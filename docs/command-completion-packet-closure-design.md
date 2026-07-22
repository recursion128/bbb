# Command completion packet-family closure

## Goal and completion contract

Close the remaining runtime, response, state-guard, and semantic-test coverage
for `ClientboundCommandsPacket`, `ServerboundCommandSuggestionPacket`, and
`ClientboundCommandSuggestionsPacket`. The world-owned command dispatcher must
produce the same local completion states as Vanilla 26.1 for entity-selector
options, block-state properties, and item component patches; summonable entity
suggestions must carry the entity description translation key into the selected
language generation; and the server fallback must retain exactly one current
pending request/result instead of accumulating responses by transaction id.

Completion means nested selector option names, the `sort`, `gamemode`, and
`type` value families, block/tag property names and values, property/component
separators and closing delimiters, block-entity NBT openers, and persistent item
component assignment/removal states all produce local suggestions without an
unconditional server request. Suggestions obey active registry/tag order and
Vanilla applicability rules. Unknown custom providers and registries without a
local owner still use `ask_server`. A new custom request cancels the preceding
logical pending request; only its matching response is accepted; stale responses
cannot alter visible suggestions; listener reset clears pending/result state;
and retained response memory is bounded to the one Vanilla future. All coverage
cells for the three packet rows are `complete`, the finished command-completion
TODO text is removed, and full plus applicable playable gates pass at the
milestone head.

## Scope and non-goals

- `bbb-world` owns command-tree traversal, the registry-aware local completion
  grammar, selector applicability state, block property/value discovery, item
  component completion, pending request identity, and the single accepted
  response result.
- `bbb-protocol` remains the sole owner of command argument ids/templates,
  entity/block/item/component catalogs, Component wire decode, and both packet
  codecs. This slice consumes those typed owners; it does not add a second
  packet or argument-type table.
- `bbb-native` owns request scheduling, suggestion list interaction, tooltip
  localization through the active immutable resource generation, narration,
  and HUD projection. `bbb-net` continues to send the already typed command and
  dispatch the response through the existing unique play event.

This slice does not implement server command execution, permission prediction,
custom server parser semantics, selector evaluation against local entities,
server-owned objective/predicate/loot completion, a parallel native grammar, or
new language/configuration UI. Vanilla item component codecs do not suggest
inside the SNBT value after `=`; the local owner suggests the surrounding
assignment/removal and next/end states and leaves unknown/custom value grammar
to the server. Existing command syntax coloring and broader usage-box polish are
not packet-family completion requirements.

## Vanilla and integration contract

`EntitySelectorParser.parseOptions` installs option-name suggestions before a
key, clears them while each modifier parses, then suggests `,` and `]` after a
complete value. `EntitySelectorOptions.suggestNames` emits only options whose
predicate is currently applicable. `sort` suggests `nearest`, `furthest`,
`random`, and `arbitrary`; `gamemode` applies the positive/inverted repeat
rules; `type` suggests entity ids and tags with the allowed `!`, `#`, and `!#`
prefixes. Other option values have no client-local value provider.

`BlockStateParser.fillSuggestions` suggests block ids (and tags for block
predicates), `[` when the selected block/tag has properties, and `{` when it
can own a block entity. Inside properties it suggests unused names with `=`,
the exact possible values, and then `,`/`]`; tag predicates use the union of
member property names and values. These facts come from the existing embedded
26.1 block-state registry and the active frozen tag generation.

`ItemParser.fillSuggestions` suggests an item id, `[`, optional `!`, persistent
component ids, `=` for assignments, and `,`/`]` after a complete removed
component or decoded SNBT value. It intentionally installs no suggestion
provider inside the SNBT component value. Component membership continues to use
the generated data-component catalog and active registry access.

`SuggestionProviders.SUMMONABLE_ENTITIES` filters the built-in entity registry
by enabled features and `EntityType.canSummon`, and attaches
`EntityType.getDescription` to each suggestion. The description key is
`entity.<namespace>.<path>` and is resolved only in native presentation against
the selected language generation; canonical command state remains
language-neutral.

`ClientSuggestionProvider.customSuggestion` cancels its previous future,
increments an id starting at zero, sends the full input, and holds one pending
future. `completeCustomSuggestions` accepts only the current id, completes the
future, and clears the pending id/future. The bbb world state mirrors this with
one pending id and at most one accepted result; counters count packets and the
currently retained entries, not historical response accumulation.

Authoritative local sources are
`net/minecraft/commands/arguments/selector/EntitySelectorParser.java`,
`selector/options/EntitySelectorOptions.java`,
`arguments/blocks/BlockStateParser.java`,
`arguments/item/ItemParser.java`,
`commands/synchronization/SuggestionProviders.java`,
`client/multiplayer/ClientSuggestionProvider.java`, and
`client/gui/components/CommandSuggestions.java` in the Vanilla 26.1 source
tree.

## Serial stages and verification

1. Complete the world-owned nested completion grammar and registry query
   surface. Focused `bbb-world` tests cover selector applicability/value
   prefixes, concrete and tag block properties, exact property values,
   block-entity openers, component assignment/removal/next/end states, active
   registry/tag replacement, and no-server versus server-fallback decisions.
2. Carry summonable entity description keys through local suggestion state,
   localize hover/narration through `NativeItemRuntime`, and make accepted
   response storage single-result. Focused `bbb-world` and `bbb-native` tests
   cover language fallback/override, stale and matching responses, replacement,
   listener reset, bounded counters, tooltip rendering, and narration.
3. Mark all three packet rows complete, remove the closed TODO sentence, append
   the milestone index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `8f718815f` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Grammar failures stop before native localization; pending-id
or localization failures stop before coverage closeout. Failures are fixed
forward without rewriting history.
