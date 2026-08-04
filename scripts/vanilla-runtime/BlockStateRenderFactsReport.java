import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import net.minecraft.SharedConstants;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.tags.BlockTags;
import net.minecraft.world.level.EmptyBlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.HalfTransparentBlock;
import net.minecraft.world.level.block.IronBarsBlock;
import net.minecraft.world.level.block.LeavesBlock;
import net.minecraft.world.level.block.LiquidBlock;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.Property;
import net.minecraft.world.phys.AABB;

/**
 * Emits renderer-visible facts which Vanilla keeps independently from baked
 * block models and chunk render layers.
 *
 * <p>Face shapes are the exact cached {@code getFaceOcclusionShape} results.
 * The behavior bits describe the complete Vanilla 26.1 {@code skipRendering}
 * override set plus the water-overlay class relation used by FluidRenderer.
 */
public final class BlockStateRenderFactsReport {
    private static final Direction[] DIRECTIONS = {
        Direction.DOWN,
        Direction.UP,
        Direction.NORTH,
        Direction.SOUTH,
        Direction.WEST,
        Direction.EAST,
    };

    private BlockStateRenderFactsReport() {
    }

    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        int count = Block.BLOCK_STATE_REGISTRY.size();
        verifySkipRenderingProjection(count);
        Bootstrap.STDOUT.println("version=26.1 count=" + count);
        for (int id = 0; id < count; id++) {
            BlockState state = Block.BLOCK_STATE_REGISTRY.byId(id);
            if (state == null) {
                throw new IllegalStateException("missing block state id " + id);
            }

            Block block = state.getBlock();
            long facts = 0L;
            if (block instanceof HalfTransparentBlock || block instanceof LeavesBlock) {
                facts |= 1L;
            }
            if (block instanceof LeavesBlock) {
                facts |= 1L << 1;
            }
            if (block instanceof HalfTransparentBlock || block == Blocks.POWDER_SNOW) {
                facts |= 1L << 2;
            }
            if (block == Blocks.MANGROVE_ROOTS) {
                facts |= 1L << 3;
            }
            if (block instanceof IronBarsBlock) {
                facts |= 1L << 4;
            }
            if (block instanceof LiquidBlock) {
                facts |= 1L << 6;
            }
            facts |= booleanPropertyBit(state, "north", 7);
            facts |= booleanPropertyBit(state, "south", 8);
            facts |= booleanPropertyBit(state, "west", 9);
            facts |= booleanPropertyBit(state, "east", 10);
            facts |= booleanPropertyPresenceBit(state, "north", 11);
            facts |= booleanPropertyPresenceBit(state, "south", 12);
            facts |= booleanPropertyPresenceBit(state, "west", 13);
            facts |= booleanPropertyPresenceBit(state, "east", 14);

            StringBuilder line = new StringBuilder()
                .append(id)
                .append('\t')
                .append(BuiltInRegistries.BLOCK.getId(block))
                .append('\t')
                .append(facts)
                .append('\t')
                .append(state.getLightEmission())
                .append('\t')
                .append(state.emissiveRendering(EmptyBlockGetter.INSTANCE, BlockPos.ZERO) ? 1 : 0);
            for (Direction direction : DIRECTIONS) {
                List<AABB> boxes = state.getFaceOcclusionShape(direction).toAabbs();
                line.append('\t').append(boxes.size());
                for (AABB box : boxes) {
                    switch (direction.getAxis()) {
                        case Y -> appendRectangle(line, box.minX, box.minZ, box.maxX, box.maxZ);
                        case Z -> appendRectangle(line, box.minX, box.minY, box.maxX, box.maxY);
                        case X -> appendRectangle(line, box.minY, box.minZ, box.maxY, box.maxZ);
                    }
                }
            }
            Bootstrap.STDOUT.println(line);
        }
    }

    private static void verifySkipRenderingProjection(int count) {
        Set<String> allowedDeclarers = Set.of(
            "BlockBehaviour",
            "HalfTransparentBlock",
            "IronBarsBlock",
            "LeavesBlock",
            "LiquidBlock",
            "MangroveRootsBlock",
            "PowderSnowBlock"
        );
        List<BlockState> states = new ArrayList<>(count);
        for (int id = 0; id < count; id++) {
            BlockState state = Block.BLOCK_STATE_REGISTRY.byId(id);
            if (state == null) {
                throw new IllegalStateException("missing block state id " + id);
            }
            states.add(state);
            Class<?> declarer = skipRenderingDeclarer(state.getBlock().getClass());
            if (!allowedDeclarers.contains(declarer.getSimpleName())) {
                throw new IllegalStateException(
                    "unclassified skipRendering override " + declarer.getName()
                );
            }
        }

        List<BlockState> representatives = new ArrayList<>();
        Set<Class<?>> representedClasses = new HashSet<>();
        for (BlockState state : states) {
            if (representedClasses.add(skipRenderingDeclarer(state.getBlock().getClass()))) {
                representatives.add(state);
            }
        }

        for (boolean cutoutLeaves : new boolean[] {false, true}) {
            LeavesBlock.setCutoutLeaves(cutoutLeaves);
            for (BlockState state : states) {
                List<BlockState> candidates = new ArrayList<>(representatives);
                for (BlockState neighbor : states) {
                    if (isRelevantSkipNeighbor(state, neighbor)) {
                        candidates.add(neighbor);
                    }
                }
                for (BlockState neighbor : candidates) {
                    for (Direction direction : DIRECTIONS) {
                        boolean expected = projectedSkipRendering(
                            state,
                            neighbor,
                            direction,
                            cutoutLeaves
                        );
                        boolean actual = state.skipRendering(neighbor, direction);
                        if (actual != expected) {
                            throw new IllegalStateException(
                                "skipRendering projection mismatch: state="
                                    + Block.BLOCK_STATE_REGISTRY.getId(state)
                                    + " neighbor="
                                    + Block.BLOCK_STATE_REGISTRY.getId(neighbor)
                                    + " direction="
                                    + direction
                                    + " cutoutLeaves="
                                    + cutoutLeaves
                                    + " expected="
                                    + expected
                                    + " actual="
                                    + actual
                            );
                        }
                    }
                }
            }
        }
        LeavesBlock.setCutoutLeaves(true);
    }

    private static Class<?> skipRenderingDeclarer(Class<?> blockClass) {
        for (Class<?> current = blockClass; current != null; current = current.getSuperclass()) {
            for (Method method : current.getDeclaredMethods()) {
                if (method.getName().equals("skipRendering")
                    && method.getParameterCount() == 3) {
                    return current;
                }
            }
        }
        throw new IllegalStateException("missing skipRendering declaration for " + blockClass);
    }

    private static boolean isRelevantSkipNeighbor(BlockState state, BlockState neighbor) {
        Block block = state.getBlock();
        if (neighbor.is(block)) {
            return true;
        }
        if (block instanceof LeavesBlock && neighbor.getBlock() instanceof LeavesBlock) {
            return true;
        }
        if (block instanceof IronBarsBlock && neighbor.is(BlockTags.BARS)) {
            return true;
        }
        return block instanceof LiquidBlock && !neighbor.getFluidState().isEmpty();
    }

    private static boolean projectedSkipRendering(
        BlockState state,
        BlockState neighbor,
        Direction direction,
        boolean cutoutLeaves
    ) {
        Block block = state.getBlock();
        if (block instanceof LeavesBlock) {
            return !cutoutLeaves && neighbor.getBlock() instanceof LeavesBlock;
        }
        if (block instanceof HalfTransparentBlock || block == Blocks.POWDER_SNOW) {
            return neighbor.is(block);
        }
        if (block == Blocks.MANGROVE_ROOTS) {
            return neighbor.is(Blocks.MANGROVE_ROOTS)
                && direction.getAxis() == Direction.Axis.Y;
        }
        if (block instanceof IronBarsBlock) {
            boolean pair = neighbor.is(block)
                || state.is(BlockTags.BARS)
                    && neighbor.is(BlockTags.BARS)
                    && hasBooleanProperty(neighbor, direction.getOpposite().getName());
            if (!pair) {
                return false;
            }
            if (!direction.getAxis().isHorizontal()) {
                return true;
            }
            return booleanProperty(state, direction.getName())
                && booleanProperty(neighbor, direction.getOpposite().getName());
        }
        if (block instanceof LiquidBlock) {
            return neighbor.getFluidState().getType().isSame(state.getFluidState().getType());
        }
        return false;
    }

    private static boolean hasBooleanProperty(BlockState state, String name) {
        for (Property<?> property : state.getProperties()) {
            if (property.getName().equals(name)) {
                return true;
            }
        }
        return false;
    }

    private static boolean booleanProperty(BlockState state, String name) {
        return hasBooleanProperty(state, name) && booleanPropertyBit(state, name, 0) != 0;
    }

    private static long booleanPropertyBit(BlockState state, String name, int bit) {
        for (Property<?> property : state.getProperties()) {
            if (property.getName().equals(name)
                && state.getValue(property).toString().equals("true")) {
                return 1L << bit;
            }
        }
        return 0L;
    }

    private static long booleanPropertyPresenceBit(BlockState state, String name, int bit) {
        return hasBooleanProperty(state, name) ? 1L << bit : 0L;
    }

    private static void appendRectangle(
        StringBuilder line,
        double minA,
        double minB,
        double maxA,
        double maxB
    ) {
        appendBits(line, minA);
        appendBits(line, minB);
        appendBits(line, maxA);
        appendBits(line, maxB);
    }

    private static void appendBits(StringBuilder line, double value) {
        line.append('\t').append(
            Long.toUnsignedString(Double.doubleToRawLongBits(value))
        );
    }
}
