import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.material.Fluids;

/**
 * Emits the built-in block-state ids for which Vanilla's fluid replacement
 * predicate succeeds.
 *
 * <p>This probe intentionally calls the public BlockState behavior instead of
 * reconstructing cached solidity from registration flags and collision shapes.
 */
public final class BlockStateFluidReplaceabilityReport {
    private BlockStateFluidReplaceabilityReport() {
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
            boolean water = state.canBeReplaced(Fluids.WATER);
            boolean lava = state.canBeReplaced(Fluids.LAVA);
            if (water != lava) {
                throw new IllegalStateException(
                    "fluid-dependent replacement predicate for block state id " + id
                );
            }
            Bootstrap.STDOUT.println(id + "\t" + (water ? "1" : "0"));
        }
    }
}
