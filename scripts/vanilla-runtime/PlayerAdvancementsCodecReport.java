import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonParser;
import com.mojang.serialization.Codec;
import com.mojang.serialization.JsonOps;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.SharedConstants;
import net.minecraft.advancements.AdvancementProgress;
import net.minecraft.advancements.AdvancementRequirements;
import net.minecraft.resources.Identifier;
import net.minecraft.server.Bootstrap;
import net.minecraft.util.datafix.DataFixTypes;
import net.minecraft.util.datafix.DataFixers;

/**
 * Emits the {@code players/advancements/<uuid>.json} document the official
 * server writes after loading one, so a round trip can be compared against it.
 *
 * <p>The load leg is {@code PlayerAdvancements.load} (:113-128) over
 * {@code Data.CODEC} (:319-320) under {@code DataFixTypes.ADVANCEMENTS.wrapCodec}
 * (:71); the fit leg is {@code startProgress} (:297-300); the save leg is
 * {@code asData} (:157-165) plus {@code save} (:130-142).
 *
 * <p>The fixture tree and document here are the ones
 * {@code player_advancements::tests::the_load_and_save_round_trip_matches_the_official_jar}
 * builds. Run it against the managed Vanilla cache with
 *
 * <pre>
 * DL=$(scripts/vanilla path 26.1)/../../downloads/26.1
 * CP=$DL/client-26.1.jar:$(find $DL/libraries -name '*.jar' | paste -sd:)
 * javac -cp "$CP" -d /tmp/probe scripts/vanilla-runtime/PlayerAdvancementsCodecReport.java
 * java -Duser.timezone=UTC -cp "/tmp/probe:$CP" PlayerAdvancementsCodecReport
 * </pre>
 *
 * The timezone is pinned because the obtained-time codec encodes through
 * {@code ZoneId.systemDefault()} ({@code AdvancementProgress.java:27-28}).
 */
public final class PlayerAdvancementsCodecReport {
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();

    /** The document read back in, keyed and spelled the way a save file is. */
    private static final String INPUT =
        """
        {
          "minecraft:story/root": {
            "criteria": {
              "a": "2026-08-31 02:30:45 +0000",
              "stale": "2026-08-31 02:30:40 +0000"
            },
            "done": true
          },
          "story/child": {
            "criteria": {"c": "2026-08-31 02:31:00 +0000"},
            "done": false
          },
          "minecraft:story/grandchild": {"criteria": {}, "done": false},
          "DataVersion": 4786
        }
        """;

    /** The live requirement grid of each advancement the fixture tree holds. */
    private static final Object[][] TREE = {
        {"minecraft:story/root", AdvancementRequirements.allOf(List.of("a", "b"))},
        {"minecraft:story/child", new AdvancementRequirements(List.of(List.of("c", "d")))},
        {"minecraft:story/grandchild", AdvancementRequirements.allOf(List.of("e"))},
    };

    private PlayerAdvancementsCodecReport() {
    }

    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Codec<Map<Identifier, AdvancementProgress>> map =
            Codec.unboundedMap(Identifier.CODEC, AdvancementProgress.CODEC);
        Codec<Map<Identifier, AdvancementProgress>> codec =
            DataFixTypes.ADVANCEMENTS.wrapCodec(map, DataFixers.getDataFixer(), 1343);

        JsonElement document = JsonParser.parseString(INPUT);
        Map<Identifier, AdvancementProgress> loaded =
            codec.parse(JsonOps.INSTANCE, document).getOrThrow();

        Map<Identifier, AdvancementProgress> saved = new LinkedHashMap<>();
        for (Object[] row : TREE) {
            Identifier id = Identifier.parse((String) row[0]);
            AdvancementProgress progress = loaded.get(id);
            if (progress == null) {
                progress = new AdvancementProgress();
            }
            progress.update((AdvancementRequirements) row[1]);
            Bootstrap.STDOUT.println(
                "progress\t" + id
                    + "\tdone=" + progress.isDone()
                    + "\thasProgress=" + progress.hasProgress()
                    + "\tpercent=" + progress.getPercent()
                    + "\tcompleted=" + progress.getCompletedCriteria()
                    + "\tremaining=" + progress.getRemainingCriteria()
                    + "\tfirstProgressDate=" + progress.getFirstProgressDate());
            if (progress.hasProgress()) {
                saved.put(id, progress);
            }
        }

        Bootstrap.STDOUT.println("dataVersion\t"
            + SharedConstants.getCurrentVersion().dataVersion().version());
        Bootstrap.STDOUT.println("document");
        Bootstrap.STDOUT.println(GSON.toJson(codec.encodeStart(JsonOps.INSTANCE, saved).getOrThrow()));
    }
}
