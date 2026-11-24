import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.mojang.serialization.Lifecycle;
import it.unimi.dsi.fastutil.shorts.ShortList;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.BitSet;
import java.util.Collections;
import java.util.Comparator;
import java.util.EnumSet;
import java.util.HashMap;
import java.util.HexFormat;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.function.Supplier;
import net.minecraft.SharedConstants;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Holder;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.HolderSet;
import net.minecraft.core.IdMapper;
import net.minecraft.core.MappedRegistry;
import net.minecraft.core.QuartPos;
import net.minecraft.core.Registry;
import net.minecraft.core.RegistryAccess;
import net.minecraft.core.RegistrationInfo;
import net.minecraft.core.SectionPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.network.protocol.game.ClientboundLightUpdatePacketData;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.packs.PackResources;
import net.minecraft.server.packs.PackType;
import net.minecraft.server.packs.repository.ServerPacksSource;
import net.minecraft.server.packs.resources.CloseableResourceManager;
import net.minecraft.server.packs.resources.MultiPackResourceManager;
import net.minecraft.tags.BiomeTags;
import net.minecraft.tags.FluidTags;
import net.minecraft.tags.TagKey;
import net.minecraft.tags.TagLoader;
import net.minecraft.util.RandomSource;
import net.minecraft.util.valueproviders.FloatProvider;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.LevelHeightAccessor;
import net.minecraft.world.level.StructureManager;
import net.minecraft.world.level.WorldGenLevel;
import net.minecraft.world.level.biome.Biome;
import net.minecraft.world.level.biome.BiomeManager;
import net.minecraft.world.level.biome.MultiNoiseBiomeSource;
import net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList;
import net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterLists;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.EntityBlock;
import net.minecraft.world.level.block.RailBlock;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.RailShape;
import net.minecraft.world.level.border.WorldBorder;
import net.minecraft.world.level.chunk.CarvingMask;
import net.minecraft.world.level.chunk.ChunkAccess;
import net.minecraft.world.level.chunk.LightChunk;
import net.minecraft.world.level.chunk.LightChunkGetter;
import net.minecraft.world.level.chunk.PalettedContainer;
import net.minecraft.world.level.chunk.PalettedContainerFactory;
import net.minecraft.world.level.chunk.ProtoChunk;
import net.minecraft.world.level.chunk.Strategy;
import net.minecraft.world.level.chunk.UpgradeData;
import net.minecraft.world.level.chunk.status.ChunkStatus;
import net.minecraft.world.level.dimension.DimensionType;
import net.minecraft.world.level.dimension.BuiltinDimensionTypes;
import net.minecraft.world.level.levelgen.Aquifer;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.levelgen.LegacyRandomSource;
import net.minecraft.world.level.levelgen.NoiseBasedChunkGenerator;
import net.minecraft.world.level.levelgen.NoiseChunk;
import net.minecraft.world.level.levelgen.NoiseGeneratorSettings;
import net.minecraft.world.level.levelgen.RandomState;
import net.minecraft.world.level.levelgen.WorldGenerationContext;
import net.minecraft.world.level.levelgen.WorldOptions;
import net.minecraft.world.level.levelgen.WorldgenRandom;
import net.minecraft.world.level.levelgen.XoroshiroRandomSource;
import net.minecraft.world.level.lighting.LevelLightEngine;
import net.minecraft.world.level.levelgen.blending.Blender;
import net.minecraft.world.level.levelgen.carver.CarvingContext;
import net.minecraft.world.level.levelgen.carver.CanyonCarverConfiguration;
import net.minecraft.world.level.levelgen.carver.CaveCarverConfiguration;
import net.minecraft.world.level.levelgen.carver.ConfiguredWorldCarver;
import net.minecraft.world.level.levelgen.carver.WorldCarver;
import net.minecraft.world.level.levelgen.structure.BoundingBox;
import net.minecraft.world.level.levelgen.structure.Structure;
import net.minecraft.world.level.levelgen.structure.StructurePiece;
import net.minecraft.world.level.levelgen.structure.StructureStart;
import net.minecraft.world.level.levelgen.structure.pieces.PiecesContainer;
import net.minecraft.world.level.levelgen.structure.pieces.StructurePieceSerializationContext;
import net.minecraft.world.level.levelgen.structure.pieces.StructurePiecesBuilder;
import net.minecraft.world.level.levelgen.structure.structures.MineshaftPieces;
import net.minecraft.world.level.levelgen.structure.structures.MineshaftStructure;
import net.minecraft.world.level.material.Fluid;
import net.minecraft.world.level.material.Fluids;
import net.minecraft.world.level.storage.loot.LootTable;
import net.minecraft.world.ticks.ScheduledTick;
import net.minecraft.world.ticks.TickPriority;

/** Emits a constructed Vanilla 26.1 Overworld 3x3 FEATURES component window. */
public final class NoiseOverworldFeaturesWindowReport {
    private static final long SEED = 0L;
    private static ChunkPos source;
    private static ChunkPos center;
    private static final int MIN_Y = -64;
    private static final int HEIGHT = 384;
    private static final EnumSet<Heightmap.Types> FINAL_HEIGHTMAPS = EnumSet.of(
            Heightmap.Types.MOTION_BLOCKING,
            Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,
            Heightmap.Types.OCEAN_FLOOR,
            Heightmap.Types.WORLD_SURFACE);

    private NoiseOverworldFeaturesWindowReport() {
    }

    public static void main(String[] args) throws Exception {
        boolean lightMode = args.length == 1 && args[0].equals("--light");
        if (args.length != 0 && !lightMode) {
            throw new IllegalArgumentException("expected no arguments or --light");
        }
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        source = lightMode ? new ChunkPos(3, -1) : new ChunkPos(-128, -76);
        center = lightMode ? new ChunkPos(0, 0) : new ChunkPos(-131, -75);
        loadStaticTags();
        HolderLookup.Provider lookups = VanillaRegistries.createLookup();
        RegistryAccess registries = registryAccess(lookups);
        Holder<NoiseGeneratorSettings> settings = lookups.getOrThrow(NoiseGeneratorSettings.OVERWORLD);
        Holder<MultiNoiseBiomeSourceParameterList> parameters =
                lookups.getOrThrow(MultiNoiseBiomeSourceParameterLists.OVERWORLD);
        MultiNoiseBiomeSource biomeSource = MultiNoiseBiomeSource.createFromPreset(parameters);
        NoiseBasedChunkGenerator generator = new NoiseBasedChunkGenerator(biomeSource, settings);
        RandomState randomState = RandomState.create(
                settings.value(), lookups.lookupOrThrow(Registries.NOISE), SEED);
        WorldState world = new WorldState(lookups, registries, biomeSource, generator, randomState);
        if (lightMode) {
            world.chunk(center).setBlockState(
                    new BlockPos(center.getMinBlockX() + 8, 200, center.getMinBlockZ() + 8),
                    Blocks.GLOWSTONE.defaultBlockState(), 0);
            emitLightReport(world);
            return;
        }
        // Blender.java:265-306.
        if (world.blockChunks().stream().anyMatch(
                chunk -> chunk.getBlendingData() != null || chunk.isOldNoiseGeneration())) {
            throw new IllegalStateException("fresh oracle window contains blending data");
        }
        StructureManager structures = new StructureManager(
                world.proxy, new WorldOptions(SEED, true, false), null);

        StructureStart start = createMineshaftStart(lookups);
        Structure mineshaft = start.getStructure();
        world.sourceChunk.setStartForStructure(mineshaft, start);
        for (ChunkPos target : targets()) {
            if (start.getBoundingBox().intersects(writableArea(target))) {
                world.chunk(target).addReferenceForStructure(mineshaft, source.pack());
            }
        }

        for (ProtoChunk chunk : world.blockChunks()) {
            generator.createBiomes(randomState, Blender.empty(), structures, chunk).join();
            generator.fillFromNoise(Blender.empty(), randomState, structures, chunk).join();
            generator.buildSurface(
                    chunk,
                    new WorldGenerationContext(generator, chunk),
                    randomState,
                    structures,
                    world.biomeManager,
                    registries.lookupOrThrow(Registries.BIOME),
                    Blender.empty());
            carve(chunk, generator, randomState, world.biomeManager, registries, biomeSource, settings.value());
            chunk.setPersistedStatus(ChunkStatus.CARVERS);
        }

        String initialStart = pieceNbtDigest(start.getPieces());
        List<TargetFact> targetFacts = new ArrayList<>();
        List<String> transitions = new ArrayList<>();
        int ordinal = 0;
        // ChunkStatusTasks.java:132-147; ChunkGenerator.java:313-395;
        // StructureStart.java:79-99.
        for (ChunkPos target : targets()) {
            ProtoChunk chunk = world.chunk(target);
            Heightmap.primeHeightmaps(chunk, FINAL_HEIGHTMAPS);
            String before = world.aggregateBlockDigest();
            String startBefore = pieceNbtDigest(start.getPieces());
            String hpsBefore = hps(start.getPieces());
            int entityBefore = world.entityRequests.size();
            int blockEntitiesBefore = world.blockEntityPositions.size();
            world.beginTarget(target);
            generator.applyBiomeDecoration(world.proxy, chunk, structures);
            world.endTarget();
            String startAfter = pieceNbtDigest(start.getPieces());
            String hpsAfter = hps(start.getPieces());
            String after = world.aggregateBlockDigest();
            int references = chunk.getReferencesForStructure(mineshaft).size();
            transitions.add(ordinal + "|" + startBefore + "|" + startAfter);
            targetFacts.add(new TargetFact(
                    ordinal,
                    target,
                    decorationSeed(target),
                    world.takeTraceDigest(),
                    world.ownerEventCount(),
                    world.observedSteps(),
                    before,
                    after,
                    references,
                    startBefore,
                    startAfter,
                    hpsBefore,
                    hpsAfter,
                    world.blockEntityPositions.size() - blockEntitiesBefore,
                    world.entityRequests.size() - entityBefore));
            ordinal++;
        }
        if (world.blockChunks().stream().anyMatch(
                chunk -> chunk.getPersistedStatus() != ChunkStatus.CARVERS)) {
            throw new IllegalStateException("component oracle changed a chunk status");
        }
        if (world.blockChunks().stream().anyMatch(chunk -> !chunk.getEntities().isEmpty())) {
            throw new IllegalStateException("request-only oracle created a live entity payload");
        }

        Bootstrap.STDOUT.printf(
                "version=26.1 source=minecraft:overworld stage=features-window schema=2 seed=%d "
                        + "center=%d,%d targets=9 columns=25 target_order=x-major-z-minor "
                        + "status_claim=none structure_topology=constructed-mineshaft-only "
                        + "entity_mode=request-only border_mode=fresh-no-blending-proved-noop "
                        + "heightmap_mode=derived-final-four-after-window "
                        + "source_version_mode=reference-visits-not-vanilla-state "
                        + "decoration_catalog=source-anchored-steps-0-10 "
                        + "entity_obstruction=fresh-no-live-entities "
                        + "block_collision=strict-unowned%n",
                SEED, center.x(), center.z());
        for (TargetFact fact : targetFacts) {
            Bootstrap.STDOUT.printf(
                    "T\t%d\t%d\t%d\t%d\t%s\t%d\t%s\t%s\t%s\t%d\t%s\t%s\t%s\t%s\t%d\t%d%n",
                    fact.ordinal(), fact.pos().x(), fact.pos().z(), fact.decorationSeed(),
                    fact.traceDigest(), fact.ownerEventCount(), fact.observedSteps(),
                    fact.beforeDigest(), fact.afterDigest(), fact.references(),
                    fact.startBefore(), fact.startAfter(), fact.hpsBefore(), fact.hpsAfter(),
                    fact.blockEntityDelta(), fact.entityRequestDelta());
        }
        List<String> columnLines = new ArrayList<>();
        for (ProtoChunk chunk : world.blockChunks()) {
            Heightmap.primeHeightmaps(chunk, FINAL_HEIGHTMAPS);
            ColumnFact fact = world.columnFact(chunk);
            String line = fact.canonical();
            columnLines.add(line);
            Bootstrap.STDOUT.println(line);
        }
        String finalStart = pieceNbtDigest(start.getPieces());
        Bootstrap.STDOUT.printf(
                "S\t%d\t%d\t%s\t%s\t%d\t%s%n",
                source.x(), source.z(), initialStart, finalStart,
                targetFacts.stream().mapToInt(TargetFact::references).sum(), digestStrings(transitions));
        Bootstrap.STDOUT.printf(
                "A\t%s\t%s\t%s%n",
                digestStrings(targetFacts.stream().map(TargetFact::orderLine).toList()),
                world.blockDigest(world.chunk(center)),
                digestStrings(columnLines));
    }

    private static List<ChunkPos> targets() {
        List<ChunkPos> result = new ArrayList<>();
        // ChunkGenerationTask.java:35-134; StaticCache2D.java:13-61.
        for (int x = center.x() - 1; x <= center.x() + 1; x++) {
            for (int z = center.z() - 1; z <= center.z() + 1; z++) {
                result.add(new ChunkPos(x, z));
            }
        }
        return result;
    }

    private static void emitLightReport(WorldState world) throws Exception {
        LightProjection forward = lightProjection(world, false);
        LightProjection reverse = lightProjection(world, true);
        if (!forward.equals(reverse)) {
            throw new IllegalStateException("LIGHT projection depends on INIT registration order");
        }
        Bootstrap.STDOUT.printf(
                "version=26.1 source=controlled:all-air-init stage=light-component schema=1 seed=%d "
                        + "center=%d,%d dependencies=9 dependency_order=x-major-z-minor "
                        + "init_orders=forward,reverse structure_topology=disabled "
                        + "controlled_emitter=glowstone@8,200,8 "
                        + "status_claim=component-only min_light_section=-5 light_sections=26 "
                        + "projection=clientbound-light-update%n",
                SEED, center.x(), center.z());
        for (String line : forward.lines()) {
            Bootstrap.STDOUT.println(line);
        }
        String forwardDigest = digestStrings(forward.lines());
        String reverseDigest = digestStrings(reverse.lines());
        Bootstrap.STDOUT.printf("A\t%s\t%s%n", forwardDigest, reverseDigest);
    }

    private static LightProjection lightProjection(WorldState world, boolean reverse)
            throws Exception {
        Set<Long> dependencies = new LinkedHashSet<>();
        List<ChunkPos> ordered = targets();
        ordered.forEach(pos -> dependencies.add(pos.pack()));
        if (reverse) {
            Collections.reverse(ordered);
        }
        LightChunkGetter getter = new LightChunkGetter() {
            @Override
            public LightChunk getChunkForLighting(int x, int z) {
                return dependencies.contains(ChunkPos.pack(x, z))
                        ? world.chunk(new ChunkPos(x, z))
                        : null;
            }

            @Override
            public BlockGetter getLevel() {
                return world.proxy;
            }
        };
        LevelLightEngine engine = new LevelLightEngine(getter, true, true);
        // ChunkStatusTasks.java:149-164; LevelLightEngine.java:48-92.
        for (ChunkPos pos : ordered) {
            ProtoChunk chunk = world.chunk(pos);
            chunk.initializeLightSources();
            chunk.setLightEngine(engine);
            for (int index = 0; index < chunk.getSectionsCount(); index++) {
                if (!chunk.getSection(index).hasOnlyAir()) {
                    engine.updateSectionStatus(
                            SectionPos.of(pos, chunk.getSectionYFromSectionIndex(index)), false);
                }
            }
        }
        engine.runLightUpdates();
        for (ChunkPos pos : ordered) {
            engine.setLightEnabled(pos, false);
            engine.retainData(pos, false);
        }
        ProtoChunk centerChunk = world.chunk(center);
        centerChunk.setLightCorrect(false);
        engine.propagateLightSources(center);
        engine.runLightUpdates();
        centerChunk.setLightCorrect(true);
        // ClientboundLightUpdatePacketData.java:17-85.
        ClientboundLightUpdatePacketData packet =
                new ClientboundLightUpdatePacketData(center, engine, null, null);
        return LightProjection.from(packet, engine.getLightSectionCount());
    }

    private record LightProjection(
            String skyMask,
            String blockMask,
            String emptySkyMask,
            String emptyBlockMask,
            List<LightLayerFact> skyUpdates,
            List<LightLayerFact> blockUpdates) {
        static LightProjection from(ClientboundLightUpdatePacketData packet, int sectionCount)
                throws Exception {
            return new LightProjection(
                    maskBits(packet.getSkyYMask(), sectionCount),
                    maskBits(packet.getBlockYMask(), sectionCount),
                    maskBits(packet.getEmptySkyYMask(), sectionCount),
                    maskBits(packet.getEmptyBlockYMask(), sectionCount),
                    layerFacts("S", packet.getSkyYMask(), packet.getSkyUpdates()),
                    layerFacts("B", packet.getBlockYMask(), packet.getBlockUpdates()));
        }

        List<String> lines() {
            List<String> result = new ArrayList<>();
            result.add("P\t" + skyMask + "\t" + blockMask + "\t" + emptySkyMask + "\t"
                    + emptyBlockMask + "\t" + skyUpdates.size() + "\t" + blockUpdates.size());
            skyUpdates.forEach(update -> result.add(update.canonical()));
            blockUpdates.forEach(update -> result.add(update.canonical()));
            return result;
        }
    }

    private record LightLayerFact(String kind, int sectionIndex, String sha256) {
        String canonical() {
            return "L\t" + kind + "\t" + sectionIndex + "\t" + sha256;
        }
    }

    private static List<LightLayerFact> layerFacts(
            String kind, BitSet mask, List<byte[]> updates) throws Exception {
        if (mask.cardinality() != updates.size()) {
            throw new IllegalStateException(kind + " mask/update cardinality changed");
        }
        List<LightLayerFact> result = new ArrayList<>();
        int update = 0;
        for (int section = mask.nextSetBit(0); section >= 0; section = mask.nextSetBit(section + 1)) {
            byte[] bytes = updates.get(update++);
            if (bytes.length != 2048) {
                throw new IllegalStateException(kind + " update is not 2048 bytes");
            }
            result.add(new LightLayerFact(kind, section, digestBytes(bytes)));
        }
        return List.copyOf(result);
    }

    private static String maskBits(BitSet mask, int sectionCount) {
        StringBuilder result = new StringBuilder(sectionCount);
        for (int section = 0; section < sectionCount; section++) {
            result.append(mask.get(section) ? '1' : '0');
        }
        return result.toString();
    }

    private static BoundingBox writableArea(ChunkPos chunk) {
        return new BoundingBox(
                chunk.getMinBlockX(), MIN_Y + 1, chunk.getMinBlockZ(),
                chunk.getMaxBlockX(), MIN_Y + HEIGHT - 1, chunk.getMaxBlockZ());
    }

    private static StructureStart createMineshaftStart(HolderLookup.Provider lookups) throws Exception {
        WorldgenRandom random = new WorldgenRandom(new LegacyRandomSource(0L));
        random.setLargeFeatureSeed(SEED, source.x(), source.z());
        random.nextDouble();
        StructurePiecesBuilder builder = new StructurePiecesBuilder();
        MineshaftPieces.MineShaftRoom room = new MineshaftPieces.MineShaftRoom(
                0, random, source.x() * 16 + 2, source.z() * 16 + 2,
                MineshaftStructure.Type.NORMAL);
        builder.addPiece(room);
        room.addChildren(room, builder, random);
        builder.moveBelowSeaLevel(63, MIN_Y, random, 10);
        List<StructurePiece> pieces = clonePieces(builder.build().pieces());
        Structure structure = lookups.lookupOrThrow(Registries.STRUCTURE)
                .listElements()
                .filter(holder -> holder.key().identifier().toString().equals("minecraft:mineshaft"))
                .findFirst()
                .orElseThrow()
                .value();
        return new StructureStart(structure, source, 0, new PiecesContainer(pieces));
    }

    private static List<StructurePiece> clonePieces(List<StructurePiece> source) {
        StructurePieceSerializationContext context = new StructurePieceSerializationContext(null, null, null);
        List<StructurePiece> result = new ArrayList<>();
        for (StructurePiece piece : source) {
            CompoundTag tag = piece.createTag(context);
            if (piece instanceof MineshaftPieces.MineShaftCorridor) {
                result.add(new ProbeCorridor(tag));
            } else if (piece instanceof MineshaftPieces.MineShaftCrossing) {
                result.add(new MineshaftPieces.MineShaftCrossing(tag));
            } else if (piece instanceof MineshaftPieces.MineShaftRoom) {
                result.add(new MineshaftPieces.MineShaftRoom(tag));
            } else if (piece instanceof MineshaftPieces.MineShaftStairs) {
                result.add(new MineshaftPieces.MineShaftStairs(tag));
            } else {
                throw new IllegalStateException("unknown mineshaft piece " + piece.getClass());
            }
        }
        return result;
    }

    private static void carve(
            ProtoChunk chunk,
            NoiseBasedChunkGenerator generator,
            RandomState randomState,
            BiomeManager biomeManager,
            RegistryAccess registries,
            MultiNoiseBiomeSource biomeSource,
            NoiseGeneratorSettings settings) throws Exception {
        NoiseChunk noiseChunk = chunk.getOrCreateNoiseChunk(ignored -> {
            throw new IllegalStateException("noise material was not generated");
        });
        CarvingContext context = new CarvingContext(
                generator, registries, chunk.getHeightAccessorForGeneration(), noiseChunk,
                randomState, settings.surfaceRule());
        CarvingMask mask = chunk.getOrCreateCarvingMask();
        Aquifer aquifer = noiseChunk.aquifer();
        WorldgenRandom random = new WorldgenRandom(new LegacyRandomSource(0L));
        HolderSet<Block> replaceable = replaceableBlocks();
        BiomeManager correctBiomes = biomeManager.withDifferentSource(
                (x, y, z) -> biomeSource.getNoiseBiome(x, y, z, randomState.sampler()));
        for (int dx = -8; dx <= 8; dx++) {
            for (int dz = -8; dz <= 8; dz++) {
                ChunkPos source = new ChunkPos(chunk.getPos().x() + dx, chunk.getPos().z() + dz);
                Holder<Biome> sourceBiome = biomeSource.getNoiseBiome(
                        QuartPos.fromBlock(source.getMinBlockX()), 0,
                        QuartPos.fromBlock(source.getMinBlockZ()), randomState.sampler());
                int index = 0;
                for (Holder<ConfiguredWorldCarver<?>> holder
                        : generator.getBiomeGenerationSettings(sourceBiome).getCarvers()) {
                    String id = holder.unwrapKey().orElseThrow().identifier().toString();
                    ConfiguredWorldCarver<?> carver = withReplaceable(holder.value(), id, replaceable);
                    random.setLargeFeatureSeed(SEED + index, source.x(), source.z());
                    if (carver.isStartChunk(random)) {
                        carver.carve(context, chunk, correctBiomes::getBiome, random, aquifer, source, mask);
                    }
                    index++;
                }
            }
        }
    }

    private static final class ProbeCorridor extends MineshaftPieces.MineShaftCorridor {
        ProbeCorridor(CompoundTag tag) {
            super(tag);
        }

        @Override
        protected boolean createChest(
                WorldGenLevel level,
                BoundingBox bounds,
                RandomSource random,
                int x,
                int y,
                int z,
                ResourceKey<LootTable> lootTable) {
            BlockPos pos = this.getWorldPos(x, y, z);
            if (!bounds.isInside(pos)
                    || !level.getBlockState(pos).isAir()
                    || level.getBlockState(pos.below()).isAir()) {
                return false;
            }
            boolean northSouth = random.nextBoolean();
            BlockState rail = Blocks.RAIL.defaultBlockState().setValue(
                    RailBlock.SHAPE,
                    northSouth ? RailShape.NORTH_SOUTH : RailShape.EAST_WEST);
            this.placeBlock(level, rail, x, y, z, bounds);
            long lootSeed = random.nextLong();
            RailShape finalShape = level.getBlockState(pos).getValue(RailBlock.SHAPE);
            WorldState world = (WorldState)Proxy.getInvocationHandler(level);
            world.entityRequests.add(new EntityRequest(
                    pos, lootTable.identifier().toString(), lootSeed,
                    finalShape.getSerializedName()));
            return true;
        }
    }

    private static final class WorldState implements InvocationHandler {
        final HolderLookup.Provider lookups;
        final RegistryAccess registries;
        final MultiNoiseBiomeSource biomeSource;
        final NoiseBasedChunkGenerator generator;
        final RandomState randomState;
        final TreeMap<Long, ProtoChunk> chunks = new TreeMap<>();
        final ProtoChunk sourceChunk;
        final WorldGenLevel proxy;
        final BiomeManager biomeManager;
        final Set<BlockPos> blockEntityPositions = new LinkedHashSet<>();
        final List<EntityRequest> entityRequests = new ArrayList<>();
        final Map<Long, List<String>> blockTicks = new HashMap<>();
        final Map<Long, List<String>> fluidTicks = new HashMap<>();
        final Map<Holder<Biome>, Holder<Biome>> biomeHolders = new HashMap<>();
        final List<String> trace = new ArrayList<>();
        final Map<String, CatalogEntry> catalog = new HashMap<>();
        final WorldBorder worldBorder = new WorldBorder();
        ChunkPos currentTarget;
        RandomSource regionRandom;
        long nextSubTick;

        WorldState(
                HolderLookup.Provider lookups,
                RegistryAccess registries,
                MultiNoiseBiomeSource biomeSource,
                NoiseBasedChunkGenerator generator,
                RandomState randomState) {
            this.lookups = lookups;
            this.registries = registries;
            this.biomeSource = biomeSource;
            this.generator = generator;
            this.randomState = randomState;
            PalettedContainerFactory containers = containerFactory(lookups);
            // ChunkPyramid.java:9-54.
            for (int x = center.x() - 2; x <= center.x() + 2; x++) {
                for (int z = center.z() - 2; z <= center.z() + 2; z++) {
                    ProtoChunk chunk = new ProtoChunk(
                            new ChunkPos(x, z), UpgradeData.EMPTY,
                            LevelHeightAccessor.create(MIN_Y, HEIGHT), containers, null);
                    this.chunks.put(chunk.getPos().pack(), chunk);
                }
            }
            this.sourceChunk = new ProtoChunk(
                    source, UpgradeData.EMPTY, LevelHeightAccessor.create(MIN_Y, HEIGHT), containers, null);
            this.chunks.put(source.pack(), sourceChunk);
            this.proxy = (WorldGenLevel)Proxy.newProxyInstance(
                    WorldGenLevel.class.getClassLoader(), new Class<?>[] {WorldGenLevel.class}, this);
            this.biomeManager = new BiomeManager(
                    (x, y, z) -> biomeSource.getNoiseBiome(x, y, z, randomState.sampler()),
                    BiomeManager.obfuscateSeed(SEED));
            buildCatalog();
        }

        private void buildCatalog() {
            lookups.lookupOrThrow(Registries.STRUCTURE).listElements().forEach(holder -> {
                CatalogEntry entry = new CatalogEntry("S", holder.value().step().ordinal());
                catalog.put(holder.key().identifier().toString(), entry);
                catalog.put(holder.key().toString(), entry);
            });
            for (Holder<Biome> biome : biomeSource.possibleBiomes()) {
                List<HolderSet<net.minecraft.world.level.levelgen.placement.PlacedFeature>> steps =
                        generator.getBiomeGenerationSettings(biome).features();
                for (int step = 0; step < steps.size(); step++) {
                    int capturedStep = step;
                    steps.get(step).stream().forEach(holder -> holder.unwrapKey().ifPresent(key -> {
                        CatalogEntry entry = new CatalogEntry("F", capturedStep);
                        catalog.put(key.identifier().toString(), entry);
                        catalog.put(key.toString(), entry);
                    }));
                }
            }
        }

        List<ProtoChunk> blockChunks() {
            return chunks.values().stream()
                    .filter(chunk -> !chunk.getPos().equals(source))
                    .sorted(Comparator.comparingInt((ProtoChunk c) -> c.getPos().x())
                            .thenComparingInt(c -> c.getPos().z()))
                    .toList();
        }

        ProtoChunk chunk(ChunkPos pos) {
            ProtoChunk chunk = chunks.get(pos.pack());
            if (chunk == null) {
                throw new IllegalStateException("chunk outside oracle window " + pos);
            }
            return chunk;
        }

        void beginTarget(ChunkPos target) {
            if (currentTarget != null) {
                throw new IllegalStateException("nested target");
            }
            currentTarget = target;
            // WorldGenRegion.java:77-88.
            regionRandom = randomState.getOrCreateRandomFactory(
                    Identifier.withDefaultNamespace("worldgen_region_random"))
                    .at(target.getWorldPosition());
            trace.clear();
        }

        void endTarget() {
            String priorKind = "";
            int priorStep = -1;
            for (String row : trace) {
                String[] fields = row.split("\\|", 3);
                int step = Integer.parseInt(fields[1]);
                if (step < priorStep || (step == priorStep && priorKind.equals("F") && fields[0].equals("S"))) {
                    throw new IllegalStateException("structure/feature trace order changed: " + trace);
                }
                priorKind = fields[0];
                priorStep = step;
            }
            if (trace.isEmpty()) {
                throw new IllegalStateException("target observed no decoration owner events");
            }
            currentTarget = null;
            regionRandom = null;
        }

        String takeTraceDigest() throws Exception {
            return digestStrings(trace);
        }

        int ownerEventCount() {
            return trace.size();
        }

        String observedSteps() {
            LinkedHashSet<String> steps = new LinkedHashSet<>();
            for (String row : trace) {
                steps.add(row.split("\\|", 3)[1]);
            }
            return String.join(",", steps);
        }

        @Override
        public Object invoke(Object owner, Method method, Object[] args) throws Throwable {
            String name = method.getName();
            if (name.equals("getChunk")) {
                if (args.length == 1 && args[0] instanceof BlockPos pos) {
                    return chunk(ChunkPos.containing(pos));
                }
                return chunk(new ChunkPos((int)args[0], (int)args[1]));
            }
            if (name.equals("hasChunk")) {
                return chunks.containsKey(ChunkPos.pack((int)args[0], (int)args[1]));
            }
            if (name.equals("getBlockState")) {
                return chunk(ChunkPos.containing((BlockPos)args[0])).getBlockState((BlockPos)args[0]);
            }
            if (name.equals("isEmptyBlock")) {
                return chunk(ChunkPos.containing((BlockPos)args[0]))
                        .getBlockState((BlockPos)args[0]).isAir();
            }
            if (name.equals("getFluidState")) {
                return chunk(ChunkPos.containing((BlockPos)args[0])).getFluidState((BlockPos)args[0]);
            }
            if (name.equals("isWaterAt")) {
                return chunk(ChunkPos.containing((BlockPos)args[0]))
                        .getFluidState((BlockPos)args[0]).is(FluidTags.WATER);
            }
            if (name.equals("isStateAtPosition")) {
                @SuppressWarnings("unchecked")
                java.util.function.Predicate<BlockState> predicate =
                        (java.util.function.Predicate<BlockState>)args[1];
                return predicate.test(chunk(ChunkPos.containing((BlockPos)args[0]))
                        .getBlockState((BlockPos)args[0]));
            }
            if (name.equals("isFluidAtPosition")) {
                @SuppressWarnings("unchecked")
                java.util.function.Predicate<net.minecraft.world.level.material.FluidState> predicate =
                        (java.util.function.Predicate<net.minecraft.world.level.material.FluidState>)args[1];
                return predicate.test(chunk(ChunkPos.containing((BlockPos)args[0]))
                        .getFluidState((BlockPos)args[0]));
            }
            if (name.equals("getBlockEntity")) {
                return blockEntity((BlockPos)args[0]);
            }
            if (name.equals("setBlock")) {
                return setBlock(owner, (BlockPos)args[0], (BlockState)args[1], (int)args[2]);
            }
            if (name.equals("removeBlock")) {
                return setBlock(owner, (BlockPos)args[0], Blocks.AIR.defaultBlockState(), 3);
            }
            if (name.equals("destroyBlock")) {
                return setBlock(owner, (BlockPos)args[0], Blocks.AIR.defaultBlockState(), 3);
            }
            if (name.equals("ensureCanWrite")) {
                return canWrite((BlockPos)args[0]);
            }
            if (name.equals("getBiome")) {
                BlockPos pos = (BlockPos)args[0];
                Holder<Biome> biome = chunk(ChunkPos.containing(pos)).getNoiseBiome(
                        QuartPos.fromBlock(pos.getX()), QuartPos.fromBlock(pos.getY()), QuartPos.fromBlock(pos.getZ()));
                return biomeHolders.computeIfAbsent(biome, this::taggedBiomeHolder);
            }
            if (name.equals("getUncachedNoiseBiome")) {
                return biomeSource.getNoiseBiome((int)args[0], (int)args[1], (int)args[2], randomState.sampler());
            }
            if (name.equals("getBiomeManager")) {
                return biomeManager;
            }
            if (name.equals("getHeight") && args != null && args.length == 3) {
                return chunk(new ChunkPos(SectionPos.blockToSectionCoord((int)args[1]),
                        SectionPos.blockToSectionCoord((int)args[2])))
                        .getHeight((Heightmap.Types)args[0], (int)args[1], (int)args[2]);
            }
            if (name.equals("getHeight") && (args == null || args.length == 0)) {
                return HEIGHT;
            }
            if (name.equals("getMinY")) {
                return MIN_Y;
            }
            if (name.equals("getMaxY")) {
                return MIN_Y + HEIGHT - 1;
            }
            if (name.equals("getMinSectionY")) {
                return SectionPos.blockToSectionCoord(MIN_Y);
            }
            if (name.equals("getMaxSectionY")) {
                return SectionPos.blockToSectionCoord(MIN_Y + HEIGHT - 1) + 1;
            }
            if (name.equals("getSectionsCount")) {
                return HEIGHT / 16;
            }
            if (name.equals("getSectionIndex")) {
                return SectionPos.blockToSectionCoord((int)args[0])
                        - SectionPos.blockToSectionCoord(MIN_Y);
            }
            if (name.equals("getSectionIndexFromSectionY")) {
                return (int)args[0] - SectionPos.blockToSectionCoord(MIN_Y);
            }
            if (name.equals("getSectionYFromSectionIndex")) {
                return (int)args[0] + SectionPos.blockToSectionCoord(MIN_Y);
            }
            if (name.equals("isOutsideBuildHeight")) {
                int y = args[0] instanceof BlockPos pos ? pos.getY() : (int)args[0];
                return y < MIN_Y || y >= MIN_Y + HEIGHT;
            }
            if (name.equals("isInsideBuildHeight")) {
                int y = args[0] instanceof BlockPos pos ? pos.getY() : (int)args[0];
                return y >= MIN_Y && y < MIN_Y + HEIGHT;
            }
            if (name.equals("getSeaLevel")) {
                return 63;
            }
            if (name.equals("getSeed")) {
                return SEED;
            }
            if (name.equals("registryAccess")) {
                return registries;
            }
            if (name.equals("dimensionType")) {
                return lookups.getOrThrow(BuiltinDimensionTypes.OVERWORLD).value();
            }
            if (name.equals("enabledFeatures")) {
                return FeatureFlags.DEFAULT_FLAGS;
            }
            if (name.equals("isClientSide")) {
                return false;
            }
            if (name.equals("getSkyDarken")) {
                return 0;
            }
            if (name.equals("getWorldBorder")) {
                return worldBorder;
            }
            if (name.equals("getRandom")) {
                if (regionRandom == null) {
                    throw new IllegalStateException("region random requested outside a target");
                }
                return regionRandom;
            }
            if (name.equals("nextSubTickCount")) {
                return nextSubTick++;
            }
            if (name.equals("scheduleTick")) {
                recordTick(args);
                return null;
            }
            if (name.equals("hasScheduledTick")) {
                BlockPos pos = (BlockPos)args[0];
                ProtoChunk chunk = chunk(ChunkPos.containing(pos));
                if (args[1] instanceof Block block) {
                    return chunk.getBlockTicks().hasScheduledTick(pos, block);
                }
                if (args[1] instanceof Fluid fluid) {
                    return chunk.getFluidTicks().hasScheduledTick(pos, fluid);
                }
                throw new IllegalArgumentException("unknown scheduled tick type " + args[1]);
            }
            if (name.equals("willTickThisTick")) {
                return false;
            }
            if (name.equals("setCurrentlyGenerating")) {
                if (args[0] != null) {
                    String id = ((Supplier<?>)args[0]).get().toString();
                    CatalogEntry entry = catalog.get(id);
                    if (entry == null) {
                        throw new IllegalStateException("uncatalogued decoration owner " + id);
                    }
                    trace.add(entry.kind() + "|" + entry.step() + "|" + id);
                }
                return null;
            }
            if (name.equals("getEntities")) {
                return List.of();
            }
            if (name.equals("isUnobstructed")) {
                // Fresh ProtoChunks contain no live entities; block collision is not answered here.
                if (blockChunks().stream().anyMatch(chunk -> !chunk.getEntities().isEmpty())) {
                    throw new IllegalStateException("entity obstruction queried after an entity appeared");
                }
                return true;
            }
            if (name.equals("playSound") || name.equals("addParticle")
                    || name.equals("levelEvent") || name.equals("gameEvent")) {
                return null;
            }
            if (name.equals("toString")) {
                return "NoiseOverworldFeaturesWindowLevel";
            }
            if (name.equals("hashCode")) {
                return System.identityHashCode(owner);
            }
            if (name.equals("equals")) {
                return owner == args[0];
            }
            throw new UnsupportedOperationException(
                    "unowned WorldGenLevel method " + method + " during " + currentTarget);
        }

        private boolean canWrite(BlockPos pos) {
            if (currentTarget == null) {
                return false;
            }
            ChunkPos destination = ChunkPos.containing(pos);
            return Math.abs(destination.x() - currentTarget.x()) <= 1
                    && Math.abs(destination.z() - currentTarget.z()) <= 1
                    && pos.getY() >= MIN_Y && pos.getY() < MIN_Y + HEIGHT;
        }

        @SuppressWarnings("unchecked")
        private Holder<Biome> taggedBiomeHolder(Holder<Biome> biome) {
            return (Holder<Biome>)Proxy.newProxyInstance(
                    Holder.class.getClassLoader(), new Class<?>[] {Holder.class},
                    (owner, method, args) -> {
                        if (method.getName().equals("is")
                                && args.length == 1 && args[0] instanceof TagKey<?> tag) {
                            if (tag.equals(BiomeTags.MINESHAFT_BLOCKING)) {
                                return biome.is(net.minecraft.world.level.biome.Biomes.DEEP_DARK);
                            }
                            throw new UnsupportedOperationException("unowned biome tag " + tag);
                        }
                        if (method.getName().equals("toString")) {
                            return biome.toString();
                        }
                        return method.invoke(biome, args);
                    });
        }

        private boolean setBlock(Object owner, BlockPos pos, BlockState state, int flags) {
            if (!canWrite(pos)) {
                return false;
            }
            ProtoChunk chunk = chunk(ChunkPos.containing(pos));
            BlockState old = chunk.setBlockState(pos, state, flags);
            if (state.hasBlockEntity()) {
                CompoundTag tag = new CompoundTag();
                tag.putInt("x", pos.getX());
                tag.putInt("y", pos.getY());
                tag.putInt("z", pos.getZ());
                tag.putString("id", "DUMMY");
                chunk.setBlockEntityNbt(tag);
                blockEntityPositions.add(pos.immutable());
            } else if (old != null && old.hasBlockEntity()) {
                chunk.removeBlockEntity(pos);
                blockEntityPositions.remove(pos);
            }
            if ((flags & 16) == 0) {
                BlockPos post = state.getPostProcessPos((WorldGenLevel)owner, pos);
                if (post != null) {
                    chunk(ChunkPos.containing(post)).markPosForPostprocessing(post);
                }
            }
            return true;
        }

        private BlockEntity blockEntity(BlockPos pos) {
            ProtoChunk chunk = chunk(ChunkPos.containing(pos));
            BlockEntity entity = chunk.getBlockEntity(pos);
            if (entity != null) {
                return entity;
            }
            CompoundTag tag = chunk.getBlockEntityNbt(pos);
            BlockState state = chunk.getBlockState(pos);
            if (tag != null && "DUMMY".equals(tag.getStringOr("id", "")) && state.hasBlockEntity()) {
                entity = ((EntityBlock)state.getBlock()).newBlockEntity(pos, state);
                if (entity != null) {
                    chunk.setBlockEntity(entity);
                    blockEntityPositions.add(pos.immutable());
                }
            }
            return entity;
        }

        private void recordTick(Object[] args) {
            BlockPos pos = (BlockPos)args[0];
            Object type = args[1];
            int delay = (int)args[2];
            TickPriority priority = args.length >= 4 ? (TickPriority)args[3] : TickPriority.NORMAL;
            String id;
            Map<Long, List<String>> owner;
            if (type instanceof Block block) {
                id = BuiltInRegistries.BLOCK.getKey(block).toString();
                owner = blockTicks;
                chunk(ChunkPos.containing(pos)).getBlockTicks().schedule(
                        new ScheduledTick<>(block, pos, delay, priority, nextSubTick++));
            } else if (type instanceof Fluid fluid) {
                id = BuiltInRegistries.FLUID.getKey(fluid).toString();
                owner = fluidTicks;
                chunk(ChunkPos.containing(pos)).getFluidTicks().schedule(
                        new ScheduledTick<>(fluid, pos, delay, priority, nextSubTick++));
            } else {
                throw new IllegalArgumentException("unknown scheduled tick type " + type);
            }
            owner.computeIfAbsent(ChunkPos.containing(pos).pack(), ignored -> new ArrayList<>())
                    .add(pos.getX() + "," + pos.getY() + "," + pos.getZ() + "|" + id
                            + "|" + delay + "|" + priority.getValue());
        }

        String aggregateBlockDigest() throws Exception {
            return digestStrings(blockChunks().stream().map(chunk -> {
                try {
                    return chunk.getPos().x() + "," + chunk.getPos().z() + "|" + blockDigest(chunk);
                } catch (Exception error) {
                    throw new RuntimeException(error);
                }
            }).toList());
        }

        String blockDigest(ProtoChunk chunk) throws Exception {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            ByteBuffer value = ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN);
            BlockPos.MutableBlockPos pos = new BlockPos.MutableBlockPos();
            for (int y = MIN_Y; y < MIN_Y + HEIGHT; y++) {
                for (int z = 0; z < 16; z++) {
                    for (int x = 0; x < 16; x++) {
                        value.clear();
                        value.putInt(Block.getId(chunk.getBlockState(pos.set(
                                chunk.getPos().getMinBlockX() + x, y,
                                chunk.getPos().getMinBlockZ() + z))));
                        digest.update(value.array());
                    }
                }
            }
            return HexFormat.of().formatHex(digest.digest());
        }

        ColumnFact columnFact(ProtoChunk chunk) throws Exception {
            List<String> post = new ArrayList<>();
            ShortList[] sections = chunk.getPostProcessing();
            for (int section = 0; section < sections.length; section++) {
                ShortList offsets = sections[section];
                if (offsets == null) {
                    continue;
                }
                for (short packed : offsets) {
                    post.add(section + "|" + Short.toUnsignedInt(packed));
                }
            }
            List<String> blockTickRows = blockTicks.getOrDefault(chunk.getPos().pack(), List.of());
            List<String> fluidTickRows = fluidTicks.getOrDefault(chunk.getPos().pack(), List.of());
            List<String> blockEntities = new ArrayList<>();
            for (BlockPos pos : blockEntityPositions.stream().sorted(Comparator.comparingLong(BlockPos::asLong)).toList()) {
                if (!ChunkPos.containing(pos).equals(chunk.getPos())) {
                    continue;
                }
                CompoundTag tag = chunk.getBlockEntityNbtForSaving(pos, registries);
                if (tag == null) {
                    throw new IllegalStateException("missing block entity tag " + pos);
                }
                blockEntities.add(pos.getX() + "," + pos.getY() + "," + pos.getZ() + "|" + tag);
            }
            List<String> requests = entityRequests.stream()
                    .filter(request -> ChunkPos.containing(request.pos()).equals(chunk.getPos()))
                    .map(EntityRequest::canonical)
                    .toList();
            return new ColumnFact(
                    chunk.getPos(),
                    blockDigest(chunk),
                    heightmapDigest(chunk, Heightmap.Types.MOTION_BLOCKING),
                    heightmapDigest(chunk, Heightmap.Types.MOTION_BLOCKING_NO_LEAVES),
                    heightmapDigest(chunk, Heightmap.Types.OCEAN_FLOOR),
                    heightmapDigest(chunk, Heightmap.Types.WORLD_SURFACE),
                    post.size(), digestStrings(post),
                    blockTickRows.size(), digestStrings(blockTickRows),
                    fluidTickRows.size(), digestStrings(fluidTickRows),
                    blockEntities.size(), digestStrings(blockEntities),
                    requests.size(), digestStrings(requests));
        }
    }

    private static String heightmapDigest(ProtoChunk chunk, Heightmap.Types type) throws Exception {
        long[] values = chunk.getOrCreateHeightmapUnprimed(type).getRawData();
        ByteBuffer bytes = ByteBuffer.allocate(values.length * Long.BYTES).order(ByteOrder.LITTLE_ENDIAN);
        for (long value : values) {
            bytes.putLong(value);
        }
        return digestBytes(bytes.array());
    }

    private static long decorationSeed(ChunkPos target) {
        return new WorldgenRandom(new XoroshiroRandomSource(0L)).setDecorationSeed(
                SEED, target.getMinBlockX(), target.getMinBlockZ());
    }

    private static String pieceNbtDigest(List<StructurePiece> pieces) throws Exception {
        StructurePieceSerializationContext context = new StructurePieceSerializationContext(null, null, null);
        List<String> rows = new ArrayList<>();
        for (StructurePiece piece : pieces) {
            rows.add(piece.createTag(context).toString());
        }
        return digestStrings(rows);
    }

    private static String hps(List<StructurePiece> pieces) throws Exception {
        StringBuilder bits = new StringBuilder();
        Field field = MineshaftPieces.MineShaftCorridor.class.getDeclaredField("hasPlacedSpider");
        field.setAccessible(true);
        for (StructurePiece piece : pieces) {
            if (piece instanceof MineshaftPieces.MineShaftCorridor) {
                bits.append(field.getBoolean(piece) ? '1' : '0');
            }
        }
        return bits.toString();
    }

    private static RegistryAccess registryAccess(HolderLookup.Provider lookups) {
        RegistryAccess statics = RegistryAccess.fromRegistryOfRegistries(BuiltInRegistries.REGISTRY);
        Map<ResourceKey<? extends Registry<?>>, Registry<?>> all = new HashMap<>();
        statics.registries().forEach(entry -> all.put(entry.key(), entry.value()));
        lookups.listRegistries().forEach(lookup -> {
            Registry<?> registry = copyRegistry(lookup);
            all.put(registry.key(), registry);
        });
        return new RegistryAccess.ImmutableRegistryAccess(all).freeze();
    }

    @SuppressWarnings({"rawtypes", "unchecked"})
    private static Registry<?> copyRegistry(HolderLookup.RegistryLookup<?> lookup) {
        MappedRegistry copy = new MappedRegistry(lookup.key(), lookup.registryLifecycle());
        lookup.listElements()
                .sorted(Comparator.comparing(holder -> holder.key().identifier()))
                .forEach(holder -> copy.register(
                        holder.key(), holder.value(), RegistrationInfo.BUILT_IN));
        return copy.freeze();
    }

    private static void loadStaticTags() throws Exception {
        PackResources vanilla = ServerPacksSource.createVanillaPackSource();
        try (CloseableResourceManager resources = new MultiPackResourceManager(
                PackType.SERVER_DATA, List.of(vanilla))) {
            RegistryAccess statics = RegistryAccess.fromRegistryOfRegistries(BuiltInRegistries.REGISTRY);
            for (Registry.PendingTags<?> tags : TagLoader.loadTagsForExistingRegistries(resources, statics)) {
                tags.apply();
            }
        }
    }

    private static HolderSet<Block> replaceableBlocks() throws Exception {
        LinkedHashSet<String> members = new LinkedHashSet<>();
        loadBlockTag("minecraft:overworld_carver_replaceables", members, new LinkedHashSet<>());
        List<Holder<Block>> holders = new ArrayList<>();
        for (String id : members) {
            Block block = BuiltInRegistries.BLOCK.getValue(Identifier.parse(id));
            if (block == null) {
                throw new IllegalStateException("missing block " + id);
            }
            holders.add(BuiltInRegistries.BLOCK.wrapAsHolder(block));
        }
        return HolderSet.direct(holders);
    }

    private static void loadBlockTag(String id, LinkedHashSet<String> members, Set<String> resolving)
            throws Exception {
        if (!resolving.add(id)) {
            throw new IllegalStateException("block tag cycle at " + id);
        }
        Identifier identifier = Identifier.parse(id);
        String resource = "/data/" + identifier.getNamespace() + "/tags/block/"
                + identifier.getPath() + ".json";
        try (InputStream stream = NoiseOverworldFeaturesWindowReport.class.getResourceAsStream(resource)) {
            if (stream == null) {
                throw new IllegalStateException("missing block tag resource " + resource);
            }
            JsonObject root = JsonParser.parseReader(
                    new InputStreamReader(stream, StandardCharsets.UTF_8)).getAsJsonObject();
            for (JsonElement value : root.getAsJsonArray("values")) {
                String member = value.getAsString();
                if (member.startsWith("#")) {
                    loadBlockTag(member.substring(1), members, resolving);
                } else {
                    members.add(member);
                }
            }
        }
        resolving.remove(id);
    }

    private static ConfiguredWorldCarver<?> withReplaceable(
            ConfiguredWorldCarver<?> original, String id, HolderSet<Block> replaceable)
            throws ReflectiveOperationException {
        if (original.config() instanceof CaveCarverConfiguration cave) {
            return WorldCarver.CAVE.configured(new CaveCarverConfiguration(
                    cave.probability, cave.y, cave.yScale, cave.lavaLevel, cave.debugSettings,
                    replaceable, cave.horizontalRadiusMultiplier, cave.verticalRadiusMultiplier,
                    caveFloor(cave)));
        }
        if (original.config() instanceof CanyonCarverConfiguration canyon) {
            return WorldCarver.CANYON.configured(new CanyonCarverConfiguration(
                    canyon.probability, canyon.y, canyon.yScale, canyon.lavaLevel,
                    canyon.debugSettings, replaceable, canyon.verticalRotation, canyon.shape));
        }
        throw new IllegalStateException("unexpected configured carver " + id);
    }

    private static FloatProvider caveFloor(CaveCarverConfiguration cave)
            throws ReflectiveOperationException {
        Field field = CaveCarverConfiguration.class.getDeclaredField("floorLevel");
        field.setAccessible(true);
        return (FloatProvider)field.get(cave);
    }

    private static PalettedContainerFactory containerFactory(HolderLookup.Provider lookups) {
        Strategy<BlockState> blockStrategy = Strategy.createForBlockStates(Block.BLOCK_STATE_REGISTRY);
        BlockState air = Blocks.AIR.defaultBlockState();
        HolderLookup.RegistryLookup<Biome> biomes = lookups.lookupOrThrow(Registries.BIOME);
        IdMapper<Holder<Biome>> biomeIds = new IdMapper<>();
        biomes.listElements().forEach(biomeIds::add);
        Strategy<Holder<Biome>> biomeStrategy = Strategy.createForBiomes(biomeIds);
        Holder<Biome> plains = lookups.getOrThrow(net.minecraft.world.level.biome.Biomes.PLAINS);
        return new PalettedContainerFactory(
                blockStrategy, air, PalettedContainer.codecRW(BlockState.CODEC, blockStrategy, air),
                biomeStrategy, plains, PalettedContainer.codecRO(Biome.CODEC, biomeStrategy, plains));
    }

    private static String digestStrings(List<String> rows) throws Exception {
        MessageDigest digest = MessageDigest.getInstance("SHA-256");
        for (String row : rows) {
            digest.update(row.getBytes(StandardCharsets.UTF_8));
            digest.update((byte)'\n');
        }
        return HexFormat.of().formatHex(digest.digest());
    }

    private static String digestBytes(byte[] value) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(value));
    }

    private record CatalogEntry(String kind, int step) {
    }

    private record EntityRequest(BlockPos pos, String lootTable, long seed, String railShape) {
        String canonical() {
            return pos.getX() + "," + pos.getY() + "," + pos.getZ() + "|"
                    + lootTable + "|" + seed + "|" + railShape;
        }
    }

    private record TargetFact(
            int ordinal,
            ChunkPos pos,
            long decorationSeed,
            String traceDigest,
            int ownerEventCount,
            String observedSteps,
            String beforeDigest,
            String afterDigest,
            int references,
            String startBefore,
            String startAfter,
            String hpsBefore,
            String hpsAfter,
            int blockEntityDelta,
            int entityRequestDelta) {
        String orderLine() {
            return ordinal + "|" + pos.x() + "," + pos.z();
        }
    }

    private record ColumnFact(
            ChunkPos pos,
            String blockDigest,
            String motionBlocking,
            String motionBlockingNoLeaves,
            String oceanFloor,
            String worldSurface,
            int postCount,
            String postDigest,
            int blockTickCount,
            String blockTickDigest,
            int fluidTickCount,
            String fluidTickDigest,
            int blockEntityCount,
            String blockEntityDigest,
            int entityRequestCount,
            String entityRequestDigest) {
        String canonical() {
            return "C\t" + pos.x() + "\t" + pos.z() + "\t" + blockDigest + "\t"
                    + motionBlocking + "\t" + motionBlockingNoLeaves + "\t" + oceanFloor + "\t"
                    + worldSurface + "\t" + postCount + "\t" + postDigest + "\t"
                    + blockTickCount + "\t" + blockTickDigest + "\t" + fluidTickCount + "\t"
                    + fluidTickDigest + "\t" + blockEntityCount + "\t" + blockEntityDigest + "\t"
                    + entityRequestCount + "\t" + entityRequestDigest;
        }
    }
}
