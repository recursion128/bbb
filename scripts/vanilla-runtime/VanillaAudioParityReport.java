import com.google.gson.Gson;
import com.mojang.blaze3d.audio.DeviceList;
import com.mojang.blaze3d.audio.Library;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.sounds.ChannelAccess;
import net.minecraft.client.sounds.SoundEngine;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundSource;
import org.lwjgl.openal.ALC10;
import org.lwjgl.openal.SOFTHRTF;

/**
 * Executes the official 26.1 audio owners and emits one machine-readable
 * observation. The caller runs this class in a fresh JVM for each HRTF branch.
 */
public final class VanillaAudioParityReport {
    private VanillaAudioParityReport() {
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1 || !(args[0].equals("true") || args[0].equals("false"))) {
            throw new IllegalArgumentException("usage: VanillaAudioParityReport <true|false>");
        }
        boolean requestedHrtf = Boolean.parseBoolean(args[0]);
        Map<String, Object> report = new LinkedHashMap<>();
        report.put("schema", 1);
        report.put("minecraft_version", "26.1");
        report.put("requested_hrtf", requestedHrtf);
        report.put("lifecycle", lifecycleObservation());

        DeviceList devices = DeviceList.query();
        report.put("enumerated_default_device", devices.defaultDevice());
        report.put("enumerated_devices", devices.allDevices());
        Library library = new Library();
        try {
            library.init(null, devices, requestedHrtf);
            long device = (long) field(Library.class, "currentDevice").get(library);
            Method getChannelCount = Library.class.getDeclaredMethod("getChannelCount");
            getChannelCount.setAccessible(true);
            int monoSources = (int) getChannelCount.invoke(library);
            boolean hrtfExtension = ALC10.alcIsExtensionPresent(device, "ALC_SOFT_HRTF");
            int hrtfSpecifiers = hrtfExtension
                ? ALC10.alcGetInteger(device, SOFTHRTF.ALC_NUM_HRTF_SPECIFIERS_SOFT)
                : 0;
            int hrtfStatus = hrtfExtension
                ? ALC10.alcGetInteger(device, SOFTHRTF.ALC_HRTF_STATUS_SOFT)
                : SOFTHRTF.ALC_HRTF_DISABLED_SOFT;

            report.put("status", "passed");
            report.put("current_device", library.currentDeviceName());
            report.put("mono_sources", monoSources);
            report.put("channel_debug", library.getChannelDebugString());
            report.put("hrtf_extension", hrtfExtension);
            report.put("hrtf_specifiers", hrtfSpecifiers);
            report.put("hrtf_status", hrtfStatus);
        } catch (RuntimeException error) {
            report.put("status", "failed");
            report.put("error_type", error.getClass().getName());
            report.put("error", error.getMessage());
            error.printStackTrace(System.err);
        } finally {
            if ("passed".equals(report.get("status"))) {
                library.cleanup();
            }
        }
        System.out.println(new Gson().toJson(report));
    }

    private static Map<String, Object> lifecycleObservation() throws Exception {
        SoundEngine engine = new SoundEngine(null, null, null);
        field(SoundEngine.class, "loaded").setBoolean(engine, true);
        int minimumLifetime = field(SoundEngine.class, "MIN_SOURCE_LIFETIME").getInt(null);

        SoundInstance instance = fakeSoundInstance();
        @SuppressWarnings("unchecked")
        Map<SoundInstance, ChannelAccess.ChannelHandle> channels =
            (Map<SoundInstance, ChannelAccess.ChannelHandle>) field(
                SoundEngine.class, "instanceToChannel"
            ).get(engine);
        @SuppressWarnings("unchecked")
        Map<SoundInstance, Integer> deleteTimes = (Map<SoundInstance, Integer>) field(
            SoundEngine.class, "soundDeleteTime"
        ).get(engine);
        ChannelAccess channelAccess = (ChannelAccess) field(
            SoundEngine.class, "channelAccess"
        ).get(engine);
        ChannelAccess.ChannelHandle handle = channelAccess.new ChannelHandle(null);
        field(ChannelAccess.ChannelHandle.class, "stopped").setBoolean(handle, true);
        channels.put(instance, handle);
        deleteTimes.put(instance, minimumLifetime);

        Method tickInGameSound = SoundEngine.class.getDeclaredMethod("tickInGameSound");
        tickInGameSound.setAccessible(true);
        List<Map<String, Object>> trace = new ArrayList<>();
        trace.add(lifecycleTick(engine, channels, deleteTimes, instance, 0));
        for (int tick = 1; tick <= minimumLifetime; tick++) {
            tickInGameSound.invoke(engine);
            trace.add(lifecycleTick(engine, channels, deleteTimes, instance, tick));
        }

        Map<String, Object> lifecycle = new LinkedHashMap<>();
        lifecycle.put("minimum_source_lifetime_ticks", minimumLifetime);
        lifecycle.put("trace", trace);
        return lifecycle;
    }

    private static Map<String, Object> lifecycleTick(
        SoundEngine engine,
        Map<SoundInstance, ChannelAccess.ChannelHandle> channels,
        Map<SoundInstance, Integer> deleteTimes,
        SoundInstance instance,
        int expectedTick
    ) throws Exception {
        int actualTick = field(SoundEngine.class, "tickCount").getInt(engine);
        if (actualTick != expectedTick) {
            throw new IllegalStateException(
                "unexpected SoundEngine tick " + actualTick + ", expected " + expectedTick
            );
        }
        Map<String, Object> observation = new LinkedHashMap<>();
        observation.put("tick", actualTick);
        observation.put("is_active", engine.isActive(instance));
        observation.put("channel_retained", channels.containsKey(instance));
        observation.put("delete_time_retained", deleteTimes.containsKey(instance));
        return observation;
    }

    private static SoundInstance fakeSoundInstance() {
        Identifier identifier = Identifier.parse("minecraft:bbb_audio_parity_probe");
        return (SoundInstance) Proxy.newProxyInstance(
            VanillaAudioParityReport.class.getClassLoader(),
            new Class<?>[] {SoundInstance.class},
            (proxy, method, args) -> switch (method.getName()) {
                case "hashCode" -> System.identityHashCode(proxy);
                case "equals" -> proxy == args[0];
                case "toString" -> "bbb-audio-parity-probe";
                case "getIdentifier" -> identifier;
                case "resolve", "getSound" -> null;
                case "getSource" -> SoundSource.MASTER;
                case "isLooping", "isRelative", "canStartSilent" -> false;
                case "canPlaySound" -> true;
                case "getDelay" -> 0;
                case "getVolume", "getPitch" -> 1.0F;
                case "getX", "getY", "getZ" -> 0.0D;
                case "getAttenuation" -> SoundInstance.Attenuation.NONE;
                default -> throw new UnsupportedOperationException(method.toString());
            }
        );
    }

    private static Field field(Class<?> owner, String name) throws Exception {
        Field field = owner.getDeclaredField(name);
        field.setAccessible(true);
        return field;
    }
}
