import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;

/**
 * Emits, per built-in block-state id, {@code BlockState.ignitedByLava()}
 * (BlockBehaviour.java:586-588) -- the predicate {@code LavaFluid.isFlammable}
 * answers with (LavaFluid.java:135-137).
 *
 * <p>It is a distinct fact from the {@code FireBlock.bootStrap} odds that
 * {@code FireBlock.canBurn} reads (FireBlock.java:285-287): in 26.1 each set
 * holds blocks the other does not, so neither can stand in for the other.
 *
 * <p>The value is a {@code BlockBehaviour.BlockStateBase} field assigned once
 * from {@code Properties.ignitedByLava} (BlockBehaviour.java:464), but
 * {@code Blocks.java} sets it through per-family property helpers and through
 * {@code ofLegacyCopy}, which copies it from the source block
 * (BlockBehaviour.java:1049), so the registration's own builder chain does not
 * carry it. The probe calls the accessor instead.
 */
public final class BlockStateIgnitedByLavaReport {
    private BlockStateIgnitedByLavaReport() {
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
            Bootstrap.STDOUT.println(id + "\t" + (state.ignitedByLava() ? "1" : "0"));
        }
    }
}
