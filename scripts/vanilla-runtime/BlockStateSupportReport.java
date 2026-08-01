import net.minecraft.SharedConstants;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.EmptyBlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.SupportType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;

/**
 * Emits the built-in block-state support, collision, occlusion-face,
 * replaceable, can-occlude, solid-render, redstone-conductor, suffocating,
 * view-blocking, light-dampening, shade-brightness, and legacy liquid facts
 * cached by Vanilla.
 *
 * <p>Dynamic-shape states are marked explicitly because their support and
 * collision shapes can depend on the live level and position. Consumers must
 * not treat the EmptyBlockGetter result as a live-world answer for those
 * states.
 */
public final class BlockStateSupportReport {
    private static final Direction[] DIRECTIONS = {
        Direction.DOWN,
        Direction.UP,
        Direction.NORTH,
        Direction.SOUTH,
        Direction.WEST,
        Direction.EAST,
    };
    private static final SupportType[] SUPPORT_TYPES = {
        SupportType.FULL,
        SupportType.CENTER,
        SupportType.RIGID,
    };

    private BlockStateSupportReport() {
    }

    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        int count = Block.BLOCK_STATE_REGISTRY.size();
        Bootstrap.STDOUT.println("version=26.1 count=" + count);
        for (int id = 0; id < count; id++) {
            BlockState state = Block.BLOCK_STATE_REGISTRY.byId(id);
            if (state == null) {
                throw new IllegalStateException("missing block state id " + id);
            }

            long facts = 0L;
            if (state.isSolid()) {
                facts |= 1L;
            }
            if (state.getBlock().hasDynamicShape()) {
                facts |= 1L << 1;
            }
            for (int directionIndex = 0; directionIndex < DIRECTIONS.length; directionIndex++) {
                Direction direction = DIRECTIONS[directionIndex];
                for (int typeIndex = 0; typeIndex < SUPPORT_TYPES.length; typeIndex++) {
                    if (state.isFaceSturdy(
                        EmptyBlockGetter.INSTANCE,
                        BlockPos.ZERO,
                        direction,
                        SUPPORT_TYPES[typeIndex]
                    )) {
                        facts |= 1L << (2 + directionIndex * SUPPORT_TYPES.length + typeIndex);
                    }
                }
            }

            VoxelShape collisionShape = state.getCollisionShape(
                EmptyBlockGetter.INSTANCE,
                BlockPos.ZERO
            );
            for (int directionIndex = 0; directionIndex < DIRECTIONS.length; directionIndex++) {
                Direction direction = DIRECTIONS[directionIndex];
                if (!collisionShape.getFaceShape(direction).isEmpty()) {
                    facts |= 1L << (20 + directionIndex);
                }
                if (Block.isFaceFull(collisionShape, direction)) {
                    facts |= 1L << (26 + directionIndex);
                }
                if (state.getFaceOcclusionShape(direction) == Shapes.block()) {
                    facts |= 1L << (32 + directionIndex);
                }
            }
            if (state.canBeReplaced()) {
                facts |= 1L << 38;
            }

            Bootstrap.STDOUT.println(
                id
                    + "\t"
                    + facts
                    + "\t"
                    + (state.canOcclude() ? 1 : 0)
                    + "\t"
                    + (state.isSolidRender() ? 1 : 0)
                    + "\t"
                    + (state.isRedstoneConductor(
                        EmptyBlockGetter.INSTANCE,
                        BlockPos.ZERO
                    ) ? 1 : 0)
                    + "\t"
                    + (state.isSuffocating(
                        EmptyBlockGetter.INSTANCE,
                        BlockPos.ZERO
                    ) ? 1 : 0)
                    + "\t"
                    + (state.isViewBlocking(
                        EmptyBlockGetter.INSTANCE,
                        BlockPos.ZERO
                    ) ? 1 : 0)
                    + "\t"
                    + state.getLightDampening()
                    + "\t"
                    + Math.round(state.getShadeBrightness(
                        EmptyBlockGetter.INSTANCE,
                        BlockPos.ZERO
                    ) * 255.0F)
                    + "\t"
                    + (state.liquid() ? 1 : 0)
            );
        }
    }
}
