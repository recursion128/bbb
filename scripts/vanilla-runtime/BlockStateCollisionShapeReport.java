import java.util.List;
import net.minecraft.SharedConstants;
import net.minecraft.core.BlockPos;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.EmptyBlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.AABB;

/**
 * Emits every built-in block state's empty-context collision AABBs.
 *
 * <p>Non-dynamic states are Vanilla's exact cached collision shapes. Dynamic
 * states are included for audit visibility but consumers must resolve them
 * against their live level, position, block entity, and collision context.
 */
public final class BlockStateCollisionShapeReport {
    private BlockStateCollisionShapeReport() {
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

            List<AABB> boxes = state.getCollisionShape(
                EmptyBlockGetter.INSTANCE,
                BlockPos.ZERO
            ).toAabbs();
            StringBuilder line = new StringBuilder()
                .append(id)
                .append('\t')
                .append(state.getBlock().hasDynamicShape() ? 1 : 0)
                .append('\t')
                .append(boxes.size());
            for (AABB box : boxes) {
                appendBits(line, box.minX);
                appendBits(line, box.minY);
                appendBits(line, box.minZ);
                appendBits(line, box.maxX);
                appendBits(line, box.maxY);
                appendBits(line, box.maxZ);
            }
            Bootstrap.STDOUT.println(line);
        }
    }

    private static void appendBits(StringBuilder line, double value) {
        line.append('\t').append(
            Long.toUnsignedString(Double.doubleToRawLongBits(value))
        );
    }
}
