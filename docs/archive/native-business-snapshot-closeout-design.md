# Native Business Snapshot Closeout

## Goal And Completion

Close the remaining native-owned business-snapshot inventory by proving that
client-observable server state has one canonical `WorldStore` owner and that
control reads cannot reintroduce a second mutable or independently published
value.

Completion requires:

- `ControlSnapshot` to contain only its generation plus app and runtime counter
  blocks;
- `ControlWorldSnapshot` to contain only the matching generation and immutable
  `Arc<WorldStore>`;
- every control read to acquire those two views through the generation-pairing
  load boundary before it queries business state;
- `NetCounters` fields to remain connection/runtime observations or
  command-queue projections, never gameplay, UI, inventory, entity, level, or
  other canonical business state; and
- structural tests to fail if any of those ownership surfaces expand outside
  the contract.

## State Taxonomy

- `bbb-world::WorldStore` owns packet-derived and client-derived canonical
  business state. This includes retained `last_*` event summaries when the
  latest event is itself an observable world value.
- `bbb-native::SimulationPublication` owns one immutable world generation plus
  native runtime inputs and derived presentation resources. Its exact world
  `Arc` is shared with control; it is not a second world snapshot.
- `bbb-control::ControlSnapshot` owns runtime observations only:
  `AppStatus`, `NetCounters`, `AudioCounters`, and `RendererCounters`.
- Native `last_*` fields that remain are runtime mechanics such as input
  double-click timestamps, movement-send baselines, animation samplers,
  renderer/resource cache ages, and diagnostic errors. They have no server
  authority and do not replace a `WorldStore` query.

## Non-goals

- Do not rename world-owned `last_*` query APIs merely because they retain the
  latest canonical event.
- Do not move input timing, UI editing focus, render interpolation, backend
  counters, resource handles, or cache recency into `WorldStore`.
- Do not change the control JSON schema or add a new control method.
- Do not put the live mutable `WorldStore`, a runtime request queue, or backend
  resources into a control snapshot.
- Do not merge runtime counters into canonical serialized world state.

## Ownership Audit

The pre-start source audit found:

- `SimulationPublisher::publish_control_views` publishes the exact immutable
  `Arc<WorldStore>` used by the render generation and stamps both control views
  with the same generation.
- `load_control_views` retries until the runtime and world generations match.
  The dispatcher has no other direct `state.snapshot` or `state.world` read.
- Every `world.*` control method queries the paired `WorldStore`; no business
  response is reconstructed from `NetCounters`.
- `ControlSnapshot` and `ControlWorldSnapshot` already have the target field
  split.
- Every serialized `NetCounters` field is either a fixed
  connection/resource-diagnostic field, a `*_commands_queued` projection, or a
  `simulation_*` runtime observation.

The implementation work is therefore a closeout guard, not another state
migration. If a guard exposes an exception, that value moves to its semantic
owner before this inventory can close.

## Ordered Stages And Verification

1. Add exact shape guards for both control snapshot structs and a semantic
   namespace guard for every serialized `NetCounters` field.
2. Strengthen the dispatcher source guard so reads outside
   `load_control_views` fail focused control tests.
3. Run focused `bbb-control` tests, remove the closed TODO, add the milestone
   archive entry, and run the per-commit checks.
4. This closes a core ownership invariant, so commit one reviewed change and
   run the full high-risk gate. Any failure is fixed forward before another
   slice starts.
