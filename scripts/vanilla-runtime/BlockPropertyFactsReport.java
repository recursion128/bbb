import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import net.minecraft.SharedConstants;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.properties.BooleanProperty;
import net.minecraft.world.level.block.state.properties.EnumProperty;
import net.minecraft.world.level.block.state.properties.IntegerProperty;
import net.minecraft.world.level.block.state.properties.Property;

public final class BlockPropertyFactsReport {
    private BlockPropertyFactsReport() {}

    private static <T extends Comparable<T>> String row(String block, Property<T> property) {
        String kind;
        if (property instanceof BooleanProperty) {
            kind = "boolean";
        } else if (property instanceof IntegerProperty) {
            kind = "integer";
        } else if (property instanceof EnumProperty<?>) {
            kind = "enum";
        } else {
            throw new IllegalStateException("unknown property implementation " + property.getClass());
        }
        List<String> values = new ArrayList<>();
        for (T value : property.getPossibleValues()) {
            int rank;
            if (value instanceof Boolean flag && kind.equals("boolean")) {
                rank = flag ? 1 : 0;
            } else if (value instanceof Integer number && kind.equals("integer")) {
                rank = number;
            } else if (value instanceof Enum<?> member && kind.equals("enum")) {
                rank = member.ordinal();
            } else {
                throw new IllegalStateException("unexpected property value type " + property.getValueClass());
            }
            values.add(property.getName(value) + ":" + rank);
        }
        return block + "\t" + property.getName() + "\t" + kind + "\t" + String.join(",", values);
    }

    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        List<String> rows = new ArrayList<>();
        for (Block block : BuiltInRegistries.BLOCK) {
            String id = BuiltInRegistries.BLOCK.getKey(block).toString();
            for (Property<?> property : block.getStateDefinition().getProperties()) {
                rows.add(row(id, property));
            }
        }
        rows.sort(Comparator.naturalOrder());
        List<String> digits = new ArrayList<>();
        for (int code = 0; code <= Character.MAX_VALUE; code++) {
            int digit = Character.digit((char) code, 10);
            if (digit >= 0) {
                digits.add("digit\t" + code + "\t" + digit);
            }
        }
        Bootstrap.STDOUT.println("version=26.1 states=" + Block.BLOCK_STATE_REGISTRY.size() + " digits=" + digits.size());
        digits.forEach(Bootstrap.STDOUT::println);
        rows.forEach(Bootstrap.STDOUT::println);
    }
}
