import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.util.List;
import net.minecraft.SharedConstants;
import net.minecraft.core.Registry;
import net.minecraft.core.RegistryAccess;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.packs.PackResources;
import net.minecraft.server.packs.PackType;
import net.minecraft.server.packs.repository.ServerPacksSource;
import net.minecraft.server.packs.resources.CloseableResourceManager;
import net.minecraft.server.packs.resources.MultiPackResourceManager;
import net.minecraft.tags.TagLoader;
import net.minecraft.world.level.block.BaseRailBlock;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.CampfireBlock;
import net.minecraft.world.level.block.DoorBlock;
import net.minecraft.world.level.block.FenceGateBlock;
import net.minecraft.world.level.block.LeavesBlock;
import net.minecraft.world.level.block.LiquidBlock;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.pathfinder.PathComputationType;

public final class BlockStatePathFactsReport {
    private BlockStatePathFactsReport() {
    }

    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        loadStaticTags();
        int count = Block.BLOCK_STATE_REGISTRY.size();
        Bootstrap.STDOUT.println("version=26.1 count=" + count);
        for (int id = 0; id < count; id++) {
            BlockState state = Block.BLOCK_STATE_REGISTRY.byId(id);
            if (state == null) {
                throw new IllegalStateException("missing block state id " + id);
            }

            Block block = state.getBlock();
            int facts = 0;
            facts |= state.isAir() ? 1 : 0;
            facts |= state.isPathfindable(PathComputationType.LAND) ? 1 << 1 : 0;
            facts |= state.isPathfindable(PathComputationType.WATER) ? 1 << 10 : 0;
            if (block instanceof DoorBlock door) {
                facts |= 1 << 2;
                facts |= door.type().canOpenByHand() ? 1 << 3 : 0;
                facts |= state.getValue(DoorBlock.OPEN) ? 1 << 4 : 0;
            }
            facts |= block instanceof BaseRailBlock ? 1 << 5 : 0;
            facts |= block instanceof LeavesBlock ? 1 << 6 : 0;
            if (block instanceof FenceGateBlock) {
                facts |= 1 << 7;
                facts |= state.getValue(FenceGateBlock.OPEN) ? 1 << 8 : 0;
            }
            facts |= block instanceof CampfireBlock && state.getValue(CampfireBlock.LIT)
                ? 1 << 9
                : 0;
            Class<?> owner = pathOwner(block.getClass());
            String sourceFluid = "-";
            if (block instanceof LiquidBlock) {
                Field field = LiquidBlock.class.getDeclaredField("fluid");
                field.setAccessible(true);
                Object registered = field.get(block);
                Object defaultFluid = block.defaultBlockState().getFluidState().getType();
                if (registered != defaultFluid) {
                    throw new IllegalStateException("liquid default fluid differs from registered source at " + id);
                }
                sourceFluid = BuiltInRegistries.FLUID.getKey(block.defaultBlockState().getFluidState().getType()).toString();
            }
            String stateFluid = BuiltInRegistries.FLUID.getKey(state.getFluidState().getType()).toString();
            Bootstrap.STDOUT.println(id + "\t" + facts + "\t" + owner.getName() + "\t" + sourceFluid + "\t" + stateFluid);
        }
    }

    private static Class<?> pathOwner(Class<?> type) throws Exception {
        for (Class<?> current = type; current != null; current = current.getSuperclass()) {
            try {
                Method method = current.getDeclaredMethod("isPathfindable", BlockState.class, PathComputationType.class);
                if (method.isSynthetic() || method.getReturnType() != boolean.class
                    || !BlockBehaviour.class.isAssignableFrom(current)) {
                    throw new IllegalStateException("unexpected path method " + method);
                }
                return current;
            } catch (NoSuchMethodException ignored) {
            }
        }
        throw new IllegalStateException("missing isPathfindable owner for " + type);
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
}
