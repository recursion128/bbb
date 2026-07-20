# Repo-native command dispatcher engineering contract

This document freezes the high-risk engineering slice that replaces the
signable-message-only command-tree walk with the immutable client dispatcher
that Vanilla 26.1 builds from `ClientboundCommandsPacket`.

## Semantic goal and completion criteria

One world-owned immutable dispatcher must provide literal and argument
traversal, redirect resolution, executable-state validation, syntax-error
spans, ordered signable message arguments, and local/server-backed completion.
Native command entry, signing, and suggestion rendering consume that one parse
result; they do not retain parallel grammar approximations.

The slice is complete when:

- all 57 generated `ArgumentTypeInfos.bootstrap` ids decode one exact typed
  network template and instantiate against the current frozen RegistryAccess
  and enabled feature set;
- literal precedence, redirects, source cursor, parsed arguments, executable
  and restricted flags match Brigadier, while invalid indices and graph cycles
  remain packet failures;
- context-free parsers validate Vanilla lexical form and bounds, including
  coordinates, ranges, selectors, SNBT, text, UUIDs, slots, enums and time;
- block/item/particle/component/dialog/loot/generic resource parsers resolve
  through the current session registry/tag view, never bootstrap ordinal;
- one immutable parse value owns consumed spans, best error, executable state,
  ordered signable arguments and completion context;
- local literal/parser/provider suggestions avoid a round trip, while
  `minecraft:ask_server` and unknown provider ids retain the request-id-gated
  server path;
- Commands replacement, configuration re-entry, transfer, disconnect,
  registry or enabled-feature replacement cannot reuse an old dispatcher;
- render/control retain the same world Arc/publication generation and command
  submission/signing preserves the existing FIFO and packet bytes; and
- focused tests, every milestone full gate, final current-HEAD build, explicit
  live Vanilla 26.1 gate and auto gate pass.

## Baseline before modification

At gated commit `42f15dd13`, protocol command parser properties are opaque
bytes after length validation. `bbb-world` has a narrow recursive traversal
only for locating `minecraft:message`: it understands literals, redirects,
three Brigadier string modes and coordinate token counts, but it does not own
general validation or completion. Native requests server suggestions after
command-entry edits and renders request-id-matched responses.

Reproducible focused baseline:

- `cargo test -p bbb-protocol packets::commands`: 7 passed;
- `cargo test -p bbb-world commands::tests`: 6 passed;
- `cargo test -p bbb-native command_entry_`: 13 passed;
- `cargo test -p bbb-native command_suggestion`: 8 passed.

The counts are semantic records, not wall-clock thresholds.

## Non-goals

- Do not execute authoritative server commands, callbacks, permissions, score
  mutation or results locally.
- Do not copy Mojang's Brigadier dependency or Java class hierarchy. The Rust
  owner is a narrow client parser over the received immutable graph.
- Do not change chat-signing bytes, command cadence, protocol, tick/input
  semantics, or add in-game configuration UI.
- Do not invent registry entries, player/entity state or server suggestions
  absent from the immutable generation.
- Do not use global mutable state, `unsafe`, a second canonical registry, or a
  second signable/suggestion grammar after its replacement phase.

## Ownership and invalidation

- `bbb-protocol` owns framing, graph validation, the generated 57-entry id
  catalog and lossless typed templates. Raw properties remain in the packet
  summary for serde/control compatibility, but one decoder owns semantics.
- `bbb-world` owns typed graph instantiation, parse results, local completion
  inputs and registry/feature context. It reads only the same generation's
  frozen registries, tags, catalogs, scoreboard/player projections and enabled
  features.
- `bbb-native` owns command-entry editing and rendering. It asks world for one
  parse/completion projection and never caches a registry order.
- `bbb-net` and `bbb-control` retain their FIFO command/request-id boundaries;
  neither constructs a dispatcher.
- Replacement constructs a new graph owner. Parser identity includes command
  tree, registry, tag and feature revisions. Command text, entity ids, cursor
  and publication generation are not parser identity.

The simulation thread remains the sole mutable WorldStore owner. Old
publications are immutable, and render/control continue to pair one exact world
Arc with one publication generation.

## Vanilla 26.1 evidence

- `ClientPacketListener.handleCommands` creates a `CommandDispatcher` from
  `packet.getRoot(CommandBuildContext.simple(registryAccess, enabledFeatures),
  COMMAND_NODE_BUILDER)`.
- `ClientboundCommandsPacket.NodeResolver` resolves redirect targets before
  building a node and attaches resolved children afterwards. The client node
  builder configures executable and restricted predicates.
- `ArgumentTypeInfos.bootstrap` registers exactly 57 templates. Numeric and
  string serializers, `EntityArgument.Info`, `ScoreHolderArgument.Info`,
  `TimeArgument.Info`, and five generic resource infos own every non-empty wire
  property payload.
- Brigadier `CommandDispatcher.parse`, `parseNodes`,
  `findSuggestionContext` and `getCompletionSuggestions` define branch
  selection, cursor/errors, redirects and completion ranges.
- `CommandBuildContext.simple` applies negotiated feature filtering without
  changing synchronized registry order.
- `SuggestionProviders` registers `ask_server`, `available_sounds` and
  `summonable_entities`; unknown provider ids resolve to `ask_server`.
- `ClientPacketListener.sendCommand` and `ArgumentSignatures.signCommand`
  consume the dispatcher parse context, so signing is not a separate grammar.

Authoritative sources are under `<MC_CODE_ROOT>/sources/26.1/`, chiefly
`ClientPacketListener.java`, `ClientboundCommandsPacket.java`,
`CommandBuildContext.java`, `ArgumentTypeInfos.java`,
`SuggestionProviders.java`, the registered argument classes and the checked-in
Brigadier sources.

## Serial phases and gates

1. **Typed templates.** Decode all property payloads through one typed function,
   preserve exact raw bytes and reject a graph template that cannot be
   reinstantiated. Exhaustively test all generated ids. Run complete protocol
   command/world command tests, catalog/fmt/diff/docs checks, commit and full
   gate. This gated tip is the first safe stop.
2. **Core dispatcher.** Replace signable recursion with one parse result and
   exact root/literal/argument/redirect traversal. Implement bool, bounded
   float/double/integer/long and all string modes, UTF-8 byte spans, trailing
   data, executable/restricted nodes, literal precedence and best errors. Run
   world/native signing and entry tests, commit and full gate.
3. **Context-free families.** Implement entity/selectors, profiles,
   coordinates, text/style/SNBT/NBT path, scoreboard, slots, identifiers,
   ranges, dimensions/game modes/time, templates, heightmap and UUID. Reuse
   existing semantic parsers/catalogs. Use cohesive family commits and full
   gate no later than four commits.
4. **Context-aware families.** Bind blocks, items, particles, components,
   generic resources/tags/selectors, loot and dialogs to frozen RegistryAccess
   and tags. Test custom insertion/reorder, unknown values, feature filtering,
   replacement and retained-generation isolation; commit and full gate.
5. **Completion/native integration.** Add local literal/parser/provider
   suggestions, registered providers and exact server fallback. Native uses the
   unified parse for rendering, submission and signing; remove the old
   unconditional/special traversals. Run world/native/net/control/order tests,
   commit, full gate and explicit live playable checkpoint.
6. **Closeout.** Update coverage, unsupported ledger, goal archive and
   architecture; audit serde/control shape, stale revisions and retained
   memory. Run affected complete-crate tests, final full gate, build the final
   HEAD binary, explicit live gate and auto gate.

## Failure localization and safe stops

Phase 1 failures belong to wire templates or graph construction; phase 2 owns
only traversal/primitives; phase 3 stays inside named context-free families;
phase 4 alone changes registry resolution; phase 5 alone changes native request
policy. Before its owning phase lands, an unsupported completion fails open to
the existing server provider and never certifies an invalid command as
executable. Each phase is buildable and reviewable; superseded production paths
are removed with their replacement. Failures are fixed forward by normal
commits without amend, rebase or history rewriting.

## Delivered state

The engineering slice landed in serial commits `f5c876774`, `e2794b104`,
`bd16d596a`, `ffe216f79`, `62fe3723d`, `14369d89f`, `54f065813`, `3f277e3bc`,
`81a671c8c`, `1401af655`, and the coverage-evidence fix `170870231`.

- Protocol now decodes every generated argument-template id while preserving
  raw properties for the existing serde/control shape. World owns the one graph
  parser and returns byte-accurate consumed spans, typed validation state,
  executable/restricted state, best syntax error, signable arguments and the
  child context that owns completion.
- Context-aware parsing reads only the current `FrozenRegistryAccess`; custom
  holder order, tags, reload, configuration re-entry and retained generations
  are covered without bootstrap-ordinal substitution.
- Native command entry consumes one owned local-or-server result. Literals,
  parser candidates, registry/tag entries and sound ids are resolved locally
  where Vanilla does so, and all three registered providers follow their exact
  local/server ownership. Only `ask_server`, an unknown provider, or a parser
  whose authoritative candidates are server-owned enters the request-id-gated
  network path. Tab cycling retains one exact result value rather than rereading
  a newer world generation mid-cycle.
- `available_sounds` comes from the accepted native audio catalog and commits
  with resource reload. `summonable_entities` uses generated `noSummon`
  metadata plus the negotiated Vanilla feature gate; entity type numbers or a
  parallel membership table are not introduced.

The remaining completion-only fidelity is deliberately explicit rather than a
second grammar: nested entity-selector option values, complete block-property
and item-component-value suggestion subgrammars, and localized entity tooltip
components. The parser already validates those command families; future
completion work must extend its typed child context and cannot fall back to
unconditional requests for ordinary local nodes.

Focused closeout results were 865 `bbb-protocol`, 1,809 `bbb-world`, and 2,187
`bbb-native` tests. The phase-5 workspace full gate passed at `170870231`; an
explicit current-HEAD Vanilla 26.1 live gate completed all eight ordered phases
with artifacts at `/tmp/bbb-playable-gate.t69XK5`.
