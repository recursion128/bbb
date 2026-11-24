import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Rotation;
import net.minecraft.world.level.block.state.BlockState;

/** Emits the exact global-state id reached by each horizontal rotation. */
public final class BlockStateRotationReport {
    private static final Rotation[] ROTATIONS = {
        Rotation.CLOCKWISE_90,
        Rotation.CLOCKWISE_180,
        Rotation.COUNTERCLOCKWISE_90
    };

    private BlockStateRotationReport() {
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
            StringBuilder row = new StringBuilder(Integer.toString(id));
            for (Rotation rotation : ROTATIONS) {
                int rotated = Block.BLOCK_STATE_REGISTRY.getId(state.rotate(rotation));
                if (rotated < 0) {
                    throw new IllegalStateException("unregistered rotated state for id " + id);
                }
                row.append('\t').append(rotated);
            }
            Bootstrap.STDOUT.println(row);
        }
    }
}
