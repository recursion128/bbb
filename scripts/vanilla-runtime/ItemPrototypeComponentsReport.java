import com.google.gson.JsonElement;
import com.mojang.serialization.JsonOps;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import net.minecraft.SharedConstants;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.component.DataComponentInitializers;
import net.minecraft.core.component.TypedDataComponent;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.resources.RegistryOps;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.item.Item;

/**
 * Emits every persistent item prototype component through its Vanilla codec.
 *
 * <p>The report deliberately carries registry resource ids rather than holder
 * ordinals inside encoded values. The native client resolves those values
 * against its current frozen registry generation.
 */
public final class ItemPrototypeComponentsReport {
    private ItemPrototypeComponentsReport() {
    }

    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        HolderLookup.Provider registries = VanillaRegistries.createLookup();
        RegistryOps<JsonElement> ops =
            registries.createSerializationContext(JsonOps.INSTANCE);
        Map<Integer, ComponentValues> componentValues = new TreeMap<>();
        List<String> itemResourceIds = new ArrayList<>();

        for (int itemId = 0; itemId < BuiltInRegistries.ITEM.size(); itemId++) {
            itemResourceIds.add(
                BuiltInRegistries.ITEM.getKey(
                    BuiltInRegistries.ITEM.byIdOrThrow(itemId)
                ).toString()
            );
        }

        for (DataComponentInitializers.PendingComponents<?> pending
            : BuiltInRegistries.DATA_COMPONENT_INITIALIZERS.build(registries)) {
            if (!pending.key().equals(Registries.ITEM)) {
                continue;
            }
            pending.forEach((holder, components) -> {
                int itemId = BuiltInRegistries.ITEM.getId((Item)holder.value());
                for (TypedDataComponent<?> component : components) {
                    if (component.type().isTransient()) {
                        throw new IllegalStateException(
                            "Item prototype contains transient component "
                                + BuiltInRegistries.DATA_COMPONENT_TYPE.getKey(component.type())
                        );
                    }
                    int typeId =
                        BuiltInRegistries.DATA_COMPONENT_TYPE.getId(component.type());
                    String resourceId =
                        BuiltInRegistries.DATA_COMPONENT_TYPE.getKey(component.type()).toString();
                    JsonElement encoded = component.encodeValue(ops).getOrThrow(
                        error -> new IllegalStateException(
                            "Failed to encode " + resourceId + " for item "
                                + itemResourceIds.get(itemId) + ": " + error
                        )
                    );
                    ComponentValues values = componentValues.computeIfAbsent(
                        typeId,
                        ignored -> new ComponentValues(resourceId)
                    );
                    if (!values.resourceId.equals(resourceId)) {
                        throw new IllegalStateException(
                            "Component id " + typeId + " changed resource id"
                        );
                    }
                    values.itemsByValue
                        .computeIfAbsent(encoded.toString(), ignored -> new ArrayList<>())
                        .add(itemId);
                }
            });
        }

        int membershipCount = componentValues.values().stream()
            .flatMap(values -> values.itemsByValue.values().stream())
            .mapToInt(List::size)
            .sum();
        Bootstrap.STDOUT.println(
            "version=26.1\titems=" + itemResourceIds.size()
                + "\tcomponents=" + componentValues.size()
                + "\tmemberships=" + membershipCount
        );
        for (int itemId = 0; itemId < itemResourceIds.size(); itemId++) {
            Bootstrap.STDOUT.println(
                "item\t" + itemId + "\t" + itemResourceIds.get(itemId)
            );
        }
        componentValues.forEach((typeId, values) -> {
            values.itemsByValue.forEach((value, itemIds) -> {
                itemIds.sort(Comparator.naturalOrder());
                Bootstrap.STDOUT.println(
                    "component\t" + typeId
                        + "\t" + values.resourceId
                        + "\t" + value
                        + "\t" + itemIds.stream()
                            .map(String::valueOf)
                            .collect(java.util.stream.Collectors.joining(","))
                );
            });
        });
    }

    private static final class ComponentValues {
        private final String resourceId;
        private final Map<String, List<Integer>> itemsByValue = new TreeMap<>();

        private ComponentValues(String resourceId) {
            this.resourceId = resourceId;
        }
    }
}
