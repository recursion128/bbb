# Unsupported And Deferred Feature Ledger

This is the project ledger for native Minecraft Java 26.1 features that are
known to be unsupported, partial, diagnostic-only, or intentionally deferred.
Each remaining item must have an owner, status, and next action before the
rewrite can be considered complete.

When an agent adds a new unsupported behavior, preserves a diagnostic-only path,
or discovers a vanilla feature gap that cannot be closed in the current slice,
update this file in the same slice.

## Status Key

- `covered`: implemented for the currently claimed scope; keep future work
  derived from the documented owner.
- `partial`: some behavior exists, but vanilla parity is not complete.
- `diagnostic`: unsupported input is intentionally reported, not implemented.
- `deferred`: intentionally left for a later owner because the current runtime
  surface is not ready.

## Ledger

| Area | Owner | Status | Next action | Evidence / boundary |
| --- | --- | --- | --- | --- |
| Unknown clientbound packets in login, configuration, and play | `bbb-protocol` + `bbb-net` + `bbb-native` | `diagnostic` | When an unsupported packet appears in probe/control diagnostics, verify it against local vanilla 26.1 sources, then either implement protocol decode and world/runtime handling or record why it is runtime-only. | Unknown login/config/play packets preserve `packet_id` and `len`, emit `NetEvent::UnsupportedPacket`, and project into `NetCounters` / `ProbeReport`. |
| Protocol coverage for remaining required 26.1 packet families | `bbb-protocol` | `partial` | Continue auditing packet ids, field order, nullability, enum ordinals, and serverbound encoders against `<MC_CODE_ROOT>/sources/26.1/`; add focused encode/decode tests with each packet slice. | `docs/full-native-rewrite-plan.md` phase 2 remains open until required login, configuration, play, movement, inventory, chat, resource-pack, interaction, and command suggestion paths are covered. |
| Offline probe and online dispatcher parity | `bbb-net` + `bbb-native` + `bbb-world` | `partial` | Keep adding parity regression tests for every decoded packet that has a `WorldStore` apply API; prefer shared semantics or focused paired tests when probe and online handling differ. | Probe and online paths now cover many packet families, including unsupported diagnostics and play -> configuration teardown, but the final criterion requires every supported decoded packet to stay aligned. |
| Native-owned business snapshots | `bbb-world` + `bbb-native` + `bbb-control` | `partial` | Continue moving client-observable state into `WorldStore`; keep `NetCounters` for connection/runtime status and command queue projections only. | The architecture plan still calls out removal of native-only `last_*` snapshots where a world owner exists or should exist. |
| Code of Conduct presentation | `bbb-world` + `bbb-native` + `bbb-renderer` | `deferred` | Replace the native bitmap prompt with fuller vanilla screen/font rendering when the renderer UI stack is mature. | Canonical Code of Conduct UI state and control requests are covered; presentation parity is explicitly deferred in `docs/full-native-rewrite-plan.md`. |
| Crosshair entity interaction parity | `bbb-world` + `bbb-native` + `bbb-renderer` | `partial` | Validate any future `yRotA` source and add renderer-visible target coverage once a debug target overlay exists. | `bbb-world` and `bbb-native` expose many verified pick bounds and interaction packets; renderer-visible overlay coverage remains deferred. |
| Particle runtime vanilla parity | `bbb-renderer` + `bbb-native` + `bbb-pack` | `partial` | Implement provider-specific behavior, collision/player-coupled physics, particle limits, and GPU drawing as separate renderer slices; preserve missing definition/sprite diagnostics. | Current runtime drains level-particle spawn batches and advances CPU-side common particles; full vanilla provider behavior and GPU drawing are listed as follow-up work in the plan. |
| Renderer scene parity | `bbb-renderer` + `bbb-native` + `bbb-pack` + `bbb-world` | `partial` | Continue terrain, entity, HUD, overlay, screenshot, and interaction-feedback extraction from canonical world and pack data; add deterministic renderer tests or explicit manual comparison notes. | Core renderer work remains phase 6; backend GPU resources must stay outside `WorldStore`. |
| Audio runtime parity | `bbb-audio` + `bbb-native` + `bbb-pack` + `bbb-world` | `partial` | Continue validating source/category mapping, spatial/entity-following sounds, stop semantics, and device/runtime diagnostics against vanilla behavior without requiring an audio device in unit tests. | `bbb-audio` has Kira-backed command/runtime boundaries and pack-driven sound lookup, but full vanilla playback parity remains phase 7 work. |
| Official 26.1 resource-pack coverage | `bbb-pack` | `partial` | Implement unsupported atlas, item model, item tint, and registry declaration shapes as official assets or resource packs require them; keep resource-pack precedence/filter tests close to loaders. | Loaders report unsupported atlas/item declarations; sounds, generated vanilla fallback, and resource-pack filters are covered for current audio use. |
| Bundle selected-item icon state | `bbb-protocol` + `bbb-world` + `bbb-native` + `bbb-pack` | `partial` | Preserve enough bundle item-template data to render `minecraft:bundle/selected_item`, then wire native inventory UI hover/scroll/click handling to the world selection API and command queue. | Vanilla `BundleHasSelectedItem` checks `BundleItem.getSelectedItem(itemStack) != null`; `BundleContents.STREAM_CODEC` sends the item template list but not the selected index. `bbb-world` stores the local selected index per inventory/container slot, control pumping updates that canonical state before queueing `ServerboundSelectBundleItemPacket`, and the GUI item icon runtime evaluates `minecraft:bundle/has_selected_item` from that local state. |
| Native input, movement, interaction, inventory, and command flows | `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world` | `partial` | Audit native input paths against vanilla 26.1 serverbound behavior, then add focused command queue and encode tests for missing movement, inventory, interaction, chat, and command flows. | Existing input modules queue many serverbound packets, but completion requires these flows to work through encoded serverbound packets end to end. |
| Manual visual/audio comparisons | Relevant runtime owner | `deferred` | Whenever visual or audio behavior cannot be proven by automated tests, record the vanilla source path, asset path, screenshot, smoke test, or manual comparison required to close the slice. | The project gate allows manual or screenshot/audio smoke checks outside normal unit tests, but they must be documented when required. |

## Update Rules

- Do not remove a row unless the current slice proves the feature is fully
  covered for its stated scope and the proof is referenced in code or tests.
- Prefer splitting a broad row into narrower rows as soon as a feature has a
  concrete owner and testable next action.
- Keep rows scoped to semantic ownership. Do not add arbitrary line-count or
  agent-parallelism work here.
