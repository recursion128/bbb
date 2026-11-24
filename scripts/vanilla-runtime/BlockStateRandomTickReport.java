import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;

/**
 * Emits, per built-in block-state id, the two predicates
 * {@code LevelChunkSection.setBlockState} folds into its ticking counters
 * (LevelChunkSection.java:74-100): the block's own
 * {@code BlockState.isRandomlyTicking()} and the fluid's
 * {@code FluidState.isRandomlyTicking()}.
 *
 * <p>Both are per-state, not per-block: {@code BlockBehaviour.BlockStateBase}
 * caches {@code owner.isRandomlyTicking(asState())} in {@code initCache}
 * (BlockBehaviour.java:506), and subclasses override it per state — leaves by
 * {@code persistent}, crops by age, weathering copper by oxidation stage,
 * {@code LiquidBlock} by whether its fluid ticks (LiquidBlock.java:105-107).
 * Reconstructing that from the {@code randomTicks()} builder calls in
 * {@code Blocks.java} would therefore be wrong, so this probe calls the real
 * behavior instead.
 */
public final class BlockStateRandomTickReport {
    private BlockStateRandomTickReport() {
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
            boolean block = state.isRandomlyTicking();
            boolean fluid = state.getFluidState().isRandomlyTicking();
            Bootstrap.STDOUT.println(id + "\t" + (block ? "1" : "0") + "\t" + (fluid ? "1" : "0"));
        }
    }
}
