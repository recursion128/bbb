import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Mirror;
import net.minecraft.world.level.block.state.BlockState;

/** Emits the exact global-state id reached by each horizontal mirror. */
public final class BlockStateMirrorReport {
    private static final Mirror[] MIRRORS = {
        Mirror.LEFT_RIGHT,
        Mirror.FRONT_BACK
    };

    private BlockStateMirrorReport() {
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
            for (Mirror mirror : MIRRORS) {
                int mirrored = Block.BLOCK_STATE_REGISTRY.getId(state.mirror(mirror));
                if (mirrored < 0) {
                    throw new IllegalStateException("unregistered mirrored state for id " + id);
                }
                row.append('\t').append(mirrored);
            }
            Bootstrap.STDOUT.println(row);
        }
    }
}
