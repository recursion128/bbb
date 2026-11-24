import net.minecraft.SharedConstants;
import net.minecraft.core.BlockPos;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.EmptyBlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;

/**
 * Emits, per built-in block-state id, the two cached fields
 * {@code BlockBehaviour.getDestroyProgress} reads (BlockBehaviour.java:355-363):
 * {@code BlockState.getDestroySpeed(level, pos)} and
 * {@code BlockState.requiresCorrectToolForDrops()}.
 *
 * <p>Both are {@code BlockBehaviour.BlockStateBase} fields assigned once from
 * {@code Properties.destroyTime} and {@code Properties.requiresCorrectToolForDrops}
 * (BlockBehaviour.java:468-469), and 26.1 declares exactly one implementation of
 * each accessor (BlockBehaviour.java:639-641 and :897-899), so the
 * {@code EmptyBlockGetter} arguments of the first cannot change its answer. The
 * probe still calls the accessors rather than reading the builder chain in
 * {@code Blocks.java}, which routes through copy helpers, per-family property
 * helpers and {@code InfestedBlock} constructors that each rewrite the value.
 *
 * <p>The speed is printed as its raw IEEE-754 bits so the checked-in table is the
 * float Vanilla divides by, not a decimal that re-parses to a neighbouring value.
 */
public final class BlockStateDestroyFactsReport {
    private BlockStateDestroyFactsReport() {
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
            float destroySpeed = state.getDestroySpeed(EmptyBlockGetter.INSTANCE, BlockPos.ZERO);
            boolean requiresCorrectTool = state.requiresCorrectToolForDrops();
            Bootstrap.STDOUT.println(
                id
                    + "\t"
                    + Integer.toUnsignedString(Float.floatToRawIntBits(destroySpeed))
                    + "\t"
                    + (requiresCorrectTool ? "1" : "0")
            );
        }
    }
}
