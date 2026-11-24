import java.util.Map;
import java.util.stream.Collectors;
import net.minecraft.SharedConstants;
import net.minecraft.core.component.DataComponentInitializers;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.component.BlockItemStateProperties;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;

/**
 * Emits the built-in block default-state ids, note-block instruments, and item
 * prototype BLOCK_STATE component properties used by BlockItem placement.
 */
public final class BlockPlacementDefaultsReport {
    private BlockPlacementDefaultsReport() {
    }

    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        for (DataComponentInitializers.PendingComponents<?> components
            : BuiltInRegistries.DATA_COMPONENT_INITIALIZERS.build(
                VanillaRegistries.createLookup()
            )) {
            components.apply();
        }
        int blockCount = BuiltInRegistries.BLOCK.size();
        int itemCount = BuiltInRegistries.ITEM.size();
        int stateCount = Block.BLOCK_STATE_REGISTRY.size();
        Bootstrap.STDOUT.println(
            "version=26.1 blocks=" + blockCount
                + " items=" + itemCount
                + " states=" + stateCount
        );

        for (int id = 0; id < blockCount; id++) {
            Block block = BuiltInRegistries.BLOCK.byId(id);
            if (block == null) {
                throw new IllegalStateException("missing block id " + id);
            }
            BlockState state = block.defaultBlockState();
            Bootstrap.STDOUT.println(
                "block\t" + id
                    + "\t" + BuiltInRegistries.BLOCK.getKey(block)
                    + "\t" + Block.getId(state)
                    + "\t" + state.instrument().getSerializedName()
            );
        }

        for (int id = 0; id < itemCount; id++) {
            Item item = BuiltInRegistries.ITEM.byId(id);
            if (item == null) {
                throw new IllegalStateException("missing item id " + id);
            }
            ItemStack stack = item.getDefaultInstance();
            BlockItemStateProperties stateProperties = stack.get(DataComponents.BLOCK_STATE);
            String properties = stateProperties == null || stateProperties.isEmpty()
                ? "-"
                : stateProperties.properties().entrySet().stream()
                    .sorted(Map.Entry.comparingByKey())
                    .map(entry -> entry.getKey() + "=" + entry.getValue())
                    .collect(Collectors.joining(","));
            Bootstrap.STDOUT.println(
                "item\t" + id
                    + "\t" + BuiltInRegistries.ITEM.getKey(item)
                    + "\t" + properties
            );
        }
    }
}
