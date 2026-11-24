import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import net.minecraft.SharedConstants;
import net.minecraft.commands.Commands;
import net.minecraft.core.BlockPos;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.RegistryAccess;
import net.minecraft.core.registries.Registries;
import net.minecraft.nbt.NbtOps;
import net.minecraft.nbt.Tag;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.ReloadableServerResources;
import net.minecraft.server.WorldLoader;
import net.minecraft.server.dedicated.DedicatedServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.packs.repository.ServerPacksSource;
import net.minecraft.server.packs.resources.CloseableResourceManager;
import net.minecraft.server.permissions.LevelBasedPermissionSet;
import net.minecraft.tags.TagKey;
import net.minecraft.util.Util;
import net.minecraft.util.context.ContextMap;
import net.minecraft.world.SimpleContainer;
import net.minecraft.world.flag.FeatureFlagSet;
import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.WorldDataConfiguration;
import net.minecraft.world.level.levelgen.structure.Structure;
import net.minecraft.world.level.storage.loot.LootParams;
import net.minecraft.world.level.storage.loot.LootTable;
import net.minecraft.world.level.storage.loot.parameters.LootContextParamSets;
import net.minecraft.world.level.storage.loot.parameters.LootContextParams;
import net.minecraft.world.phys.Vec3;
import sun.misc.Unsafe;

/**
 * Fills a 27-slot container from every {@code chests/*} loot table through
 * {@link LootTable#fill}, once per requested seed, and prints each written
 * slot as the SNBT {@link ItemStack#CODEC} encodes.
 *
 * <p>Registries, tags and loot tables come from the vanilla pack through
 * {@link WorldLoader#load}, the dedicated server's own loader
 * (Main.java:162-183). The level is a constructor-less {@link ServerLevel}
 * that answers the three things a chest fill asks of it: the server whose
 * reloadable registries {@code LootContext.Builder.create} resolves nested
 * tables through (LootContext.java:138-143), the enabled features the stack
 * splitter checks (LootTable.java:66-68), and
 * {@code findNearestMapStructure}, which answers {@code null} as it does for a
 * world generated without structures (ServerLevel.java:1415-1417).
 */
public final class ChestLootFillReport {
    private ChestLootFillReport() {
    }

    private static MinecraftServer server;
    private static RegistryAccess registries;

    static final class ChestLevel extends ServerLevel {
        ChestLevel() {
            super(null, null, null, null, null, null, false, 0L, null, false);
        }

        @Override
        public MinecraftServer getServer() {
            return server;
        }

        @Override
        public RegistryAccess registryAccess() {
            return registries;
        }

        @Override
        public FeatureFlagSet enabledFeatures() {
            return FeatureFlags.DEFAULT_FLAGS;
        }

        @Override
        public BlockPos findNearestMapStructure(
                final TagKey<Structure> structureTag, final BlockPos origin, final int maxSearchRadius, final boolean createReference) {
            return null;
        }
    }

    public static void main(String[] args) throws Exception {
        if (args.length < 4) {
            throw new IllegalArgumentException("usage: ChestLootFillReport <x> <y> <z> <seed>...");
        }
        Vec3 origin = new Vec3(Double.parseDouble(args[0]), Double.parseDouble(args[1]), Double.parseDouble(args[2]));
        List<Long> seeds = new ArrayList<>();
        for (int index = 3; index < args.length; index++) {
            seeds.add(Long.parseLong(args[index]));
        }
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        WorldLoader.InitConfig config = new WorldLoader.InitConfig(
                new WorldLoader.PackConfig(ServerPacksSource.createVanillaTrustedRepository(), WorldDataConfiguration.DEFAULT, false, true),
                Commands.CommandSelection.DEDICATED,
                LevelBasedPermissionSet.GAMEMASTER);
        Object[] loaded = Util.<Object[]>blockUntilDone(executor -> WorldLoader.<Object, Object[]>load(
                config,
                context -> new WorldLoader.DataLoadOutput<>(null, context.datapackDimensions()),
                (resourceManager, reloaded, layers, cookie) -> new Object[] {resourceManager, reloaded, layers.compositeAccess()},
                Util.backgroundExecutor(),
                executor)).get();
        CloseableResourceManager resources = (CloseableResourceManager) loaded[0];
        ReloadableServerResources managers = (ReloadableServerResources) loaded[1];
        registries = (RegistryAccess) loaded[2];

        Field unsafeField = Unsafe.class.getDeclaredField("theUnsafe");
        unsafeField.setAccessible(true);
        Unsafe unsafe = (Unsafe) unsafeField.get(null);
        server = (MinecraftServer) unsafe.allocateInstance(DedicatedServer.class);
        Class<?> reloadable = Class.forName("net.minecraft.server.MinecraftServer$ReloadableResources");
        Constructor<?> reloadableConstructor = reloadable.getDeclaredConstructor(CloseableResourceManager.class, ReloadableServerResources.class);
        reloadableConstructor.setAccessible(true);
        Field resourcesField = MinecraftServer.class.getDeclaredField("resources");
        resourcesField.setAccessible(true);
        resourcesField.set(server, reloadableConstructor.newInstance(resources, managers));
        ServerLevel level = (ServerLevel) unsafe.allocateInstance(ChestLevel.class);

        HolderLookup.Provider lookup = managers.fullRegistries().lookup();
        List<ResourceKey<LootTable>> tables = lookup.lookupOrThrow(Registries.LOOT_TABLE)
                .listElementIds()
                .filter(key -> key.identifier().getNamespace().equals("minecraft") && key.identifier().getPath().startsWith("chests/"))
                .sorted((a, b) -> a.identifier().toString().compareTo(b.identifier().toString()))
                .toList();
        var ops = lookup.createSerializationContext(NbtOps.INSTANCE);
        StringBuilder seedList = new StringBuilder();
        for (long seed : seeds) {
            seedList.append(seedList.isEmpty() ? "" : ",").append(seed);
        }
        Bootstrap.STDOUT.println("H\tversion=26.1\ttables=" + tables.size() + "\tseeds=" + seedList
                + "\torigin=" + origin.x + "," + origin.y + "," + origin.z + "\tslots=27");
        for (ResourceKey<LootTable> key : tables) {
            LootTable table = managers.fullRegistries().getLootTable(key);
            for (long seed : seeds) {
                // RandomizableContainer.java:83-88.
                LootParams params = new LootParams(
                        level,
                        new ContextMap.Builder().withParameter(LootContextParams.ORIGIN, origin).create(LootContextParamSets.CHEST),
                        Map.of(),
                        0.0F);
                SimpleContainer container = new SimpleContainer(27);
                table.fill(container, params, seed);
                int written = 0;
                for (int slot = 0; slot < container.getContainerSize(); slot++) {
                    ItemStack stack = container.getItem(slot);
                    if (stack.isEmpty()) {
                        continue;
                    }
                    Tag encoded = ItemStack.CODEC.encodeStart(ops, stack).getOrThrow();
                    Bootstrap.STDOUT.println("F\t" + key.identifier() + "\t" + seed + "\t" + slot + "\t" + encoded);
                    written++;
                }
                Bootstrap.STDOUT.println("E\t" + key.identifier() + "\t" + seed + "\t" + written);
            }
        }
        System.exit(0);
    }
}
