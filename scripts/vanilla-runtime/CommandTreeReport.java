import com.google.gson.JsonObject;
import com.mojang.brigadier.CommandDispatcher;
import com.mojang.brigadier.arguments.ArgumentType;
import com.mojang.brigadier.suggestion.SuggestionProvider;
import com.mojang.brigadier.tree.ArgumentCommandNode;
import com.mojang.brigadier.tree.CommandNode;
import com.mojang.brigadier.tree.LiteralCommandNode;
import com.mojang.brigadier.tree.RootCommandNode;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.Deque;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Predicate;
import net.minecraft.SharedConstants;
import net.minecraft.commands.CommandBuildContext;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.Commands;
import net.minecraft.commands.synchronization.ArgumentTypeInfo;
import net.minecraft.commands.synchronization.ArgumentTypeInfos;
import net.minecraft.commands.synchronization.SuggestionProviders;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.permissions.LevelBasedPermissionSet;
import net.minecraft.server.permissions.PermissionLevel;
import net.minecraft.util.profiling.jfr.JvmProfiler;

/**
 * Emits the whole Brigadier command tree a dedicated Vanilla 26.1 server
 * registers, one TSV row per node.
 *
 * <p>The tree is read out of a real {@link Commands} instance rather than
 * reconstructed from the 95 {@code net.minecraft.server.commands} registration
 * classes: the shape is produced by builder chains whose {@code then}/{@code
 * redirect}/{@code fork} calls are spread over helper methods and loops
 * (ExecuteCommand alone builds its conditionals recursively), so the only
 * faithful source is the dispatcher those builders end up in.
 *
 * <p>Construction choices, both taken from official call sites:
 *
 * <ul>
 *   <li>{@link Commands.CommandSelection#DEDICATED} is the selection
 *       {@code ReloadableServerResources} passes on a dedicated server
 *       (ReloadableServerResources.java:47), and is what gates the
 *       {@code includeDedicated} block in the {@code Commands} constructor
 *       (Commands.java:283-298) — {@code /ban}, {@code /op}, {@code /stop} and
 *       friends exist only under it, while {@code /publish} does not.
 *   <li>{@link Commands#createValidationContext} over
 *       {@link VanillaRegistries#createLookup()} is the datapack-free build
 *       context vanilla itself uses for the same job in
 *       {@code CommandsReport.run} (CommandsReport.java:28-30) and
 *       {@code Commands.validate} (Commands.java:520-522). The dedicated
 *       server's own {@code CommandBuildContext.simple(loadingContext,
 *       enabledFeatures)} needs a fully reloaded resource stack; it differs
 *       only in filtering registry lookups by feature flags and in throwing on
 *       unknown tags, and the tree shape depends on neither — argument
 *       templates carry a registry key, never registry contents.
 * </ul>
 *
 * <p>Permission levels are probed, not reflected out of the {@code requires}
 * lambdas. Vanilla 26.1 replaced numeric op levels with
 * {@code PermissionCheck}s over a {@code PermissionSet}
 * (Commands.java:162-166), and {@code requires} may compose a permission check
 * with unrelated predicates. Asking the real predicate is therefore the only
 * answer that stays correct: for each {@link PermissionLevel} this builds the
 * source {@code Commands.createCompilationContext} builds
 * (Commands.java:545-547) — the same probe vanilla's own
 * {@code COMMAND_NODE_INSPECTOR.isRestricted} uses (Commands.java:168-184) —
 * and reports the lowest level the requirement accepts.
 */
public final class CommandTreeReport {
    private CommandTreeReport() {
    }

    private static final String NO_LEVEL = "-1";

    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        CommandBuildContext context = Commands.createValidationContext(VanillaRegistries.createLookup());
        CommandDispatcher<CommandSourceStack> dispatcher =
                new Commands(Commands.CommandSelection.DEDICATED, context).getDispatcher();
        RootCommandNode<CommandSourceStack> root = dispatcher.getRoot();

        // Breadth-first so a node's children occupy one contiguous index range.
        // Siblings keep getChildren() order except the gamerule run below.
        // Redirects are not followed because they form cycles such as
        // `execute run` to the root.
        List<CommandNode<CommandSourceStack>> nodes = new ArrayList<>();
        List<Integer> parents = new ArrayList<>();
        List<String> paths = new ArrayList<>();
        Map<CommandNode<CommandSourceStack>, Integer> indices = new IdentityHashMap<>();
        Deque<Integer> pending = new ArrayDeque<>();
        nodes.add(root);
        parents.add(-1);
        paths.add("");
        indices.put(root, 0);
        pending.add(0);
        while (!pending.isEmpty()) {
            int parent = pending.remove();
            List<CommandNode<CommandSourceStack>> children = new ArrayList<>(nodes.get(parent).getChildren());
            // GameRuleCommand.java:15-25 adds rules while GameRules.java:160-165
            // iterates the Reference2ObjectOpenHashMap in GameRuleMap.java:18-45;
            // its identity-hash order varies by JVM.
            if (paths.get(parent).equals("gamerule")) {
                children.sort(Comparator.comparing(CommandNode::getName));
            }
            for (CommandNode<CommandSourceStack> child : children) {
                Integer seen = indices.get(child);
                if (seen != null) {
                    // A shared child would break the contiguous-range encoding
                    // and would give the node two paths; vanilla shares nodes
                    // through `redirect` instead, so this must not happen.
                    throw new IllegalStateException(
                            "command node reachable from two parents: " + paths.get(seen));
                }
                int index = nodes.size();
                nodes.add(child);
                parents.add(parent);
                String prefix = paths.get(parent);
                paths.add(prefix.isEmpty() ? child.getName() : prefix + " " + child.getName());
                indices.put(child, index);
                pending.add(index);
            }
        }

        CommandSourceStack[] probes = new CommandSourceStack[PermissionLevel.values().length];
        for (PermissionLevel level : PermissionLevel.values()) {
            probes[level.id()] = Commands.createCompilationContext(LevelBasedPermissionSet.forLevel(level));
        }

        Bootstrap.STDOUT.println("version=26.1 selection=DEDICATED nodes=" + nodes.size()
                + " roots=" + root.getChildren().size()
                + " jfr=" + (JvmProfiler.INSTANCE.isAvailable() ? "1" : "0")
                + " dev_commands=" + (SharedConstants.DEBUG_DEV_COMMANDS || SharedConstants.IS_RUNNING_IN_IDE ? "1" : "0")
                + " chase=" + (SharedConstants.DEBUG_CHASE_COMMAND ? "1" : "0"));
        for (int index = 0; index < nodes.size(); index++) {
            CommandNode<CommandSourceStack> node = nodes.get(index);
            CommandNode<CommandSourceStack> redirect = node.getRedirect();
            Integer redirectIndex = redirect == null ? null : indices.get(redirect);
            if (redirect != null && redirectIndex == null) {
                throw new IllegalStateException("redirect target outside the tree at " + paths.get(index));
            }
            Bootstrap.STDOUT.println(String.join(
                    "\t",
                    Integer.toString(index),
                    Integer.toString(parents.get(index)),
                    paths.get(index),
                    kind(node),
                    node.getName(),
                    node.getCommand() != null ? "1" : "0",
                    redirectIndex == null ? "-1" : redirectIndex.toString(),
                    minimumLevel(node.getRequirement(), probes),
                    parserId(node),
                    suggestionId(node),
                    properties(node)));
        }
    }

    private static String kind(final CommandNode<CommandSourceStack> node) {
        if (node instanceof RootCommandNode) {
            return "root";
        }
        if (node instanceof LiteralCommandNode) {
            return "literal";
        }
        if (node instanceof ArgumentCommandNode) {
            return "argument";
        }
        throw new IllegalStateException("unknown command node class " + node.getClass());
    }

    /**
     * The lowest {@link PermissionLevel} whose source satisfies {@code requires},
     * or {@code -1} when no level does. {@code PermissionLevel.values()} is
     * declared in ladder order (PermissionLevel.java:9-13), so the first hit is
     * the minimum.
     */
    private static String minimumLevel(
            final Predicate<CommandSourceStack> requirement, final CommandSourceStack[] probes) {
        for (int level = 0; level < probes.length; level++) {
            if (requirement.test(probes[level])) {
                return Integer.toString(level);
            }
        }
        return NO_LEVEL;
    }

    /**
     * The {@code minecraft:command_argument_type} registry id of the node's
     * parser, resolved the way {@code ArgumentUtils.serializeArgumentToJson}
     * resolves it (ArgumentUtils.java:55-63): unpack the live {@link
     * ArgumentType} into its template and look the template's info up by name.
     * The generator maps that id onto the ordinal already generated into
     * {@code packets/commands/generated_26_1.rs}, so both tables keep one id
     * definition.
     */
    private static String parserId(final CommandNode<CommandSourceStack> node) {
        if (!(node instanceof ArgumentCommandNode<CommandSourceStack, ?> argument)) {
            return "";
        }
        return String.valueOf(BuiltInRegistries.COMMAND_ARGUMENT_TYPE.getKey(
                ArgumentTypeInfos.unpack(argument.getType()).type()));
    }

    private static String suggestionId(final CommandNode<CommandSourceStack> node) {
        if (!(node instanceof ArgumentCommandNode<CommandSourceStack, ?> argument)) {
            return "";
        }
        SuggestionProvider<CommandSourceStack> provider = argument.getCustomSuggestions();
        if (provider == null) {
            return "";
        }
        Identifier name = SuggestionProviders.getName(provider);
        return name.toString();
    }

    /**
     * The parser's wire properties as JSON. {@code serializeToJson} is the same
     * projection {@code ClientboundCommandsPacket} writes to the network, so a
     * parser that carries no properties yields an empty object and prints as an
     * empty field.
     */
    private static String properties(final CommandNode<CommandSourceStack> node) {
        if (!(node instanceof ArgumentCommandNode<CommandSourceStack, ?> argument)) {
            return "";
        }
        JsonObject serialized = new JsonObject();
        serializeProperties(serialized, argument.getType());
        return serialized.isEmpty() ? "" : serialized.toString();
    }

    private static <A extends ArgumentType<?>> void serializeProperties(
            final JsonObject out, final A argument) {
        ArgumentTypeInfo.Template<A> template = ArgumentTypeInfos.unpack(argument);
        serializeTemplate(out, template.type(), template);
    }

    /** Reproduces {@code ArgumentUtils.serializeArgumentCap}, which is private. */
    @SuppressWarnings("unchecked")
    private static <A extends ArgumentType<?>, T extends ArgumentTypeInfo.Template<A>> void serializeTemplate(
            final JsonObject out, final ArgumentTypeInfo<A, T> info, final ArgumentTypeInfo.Template<A> template) {
        info.serializeToJson((T) template, out);
    }
}
