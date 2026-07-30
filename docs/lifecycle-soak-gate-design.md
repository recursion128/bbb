# Deterministic Lifecycle / Fault Soak Gate

This document freezes the delivery contract for the P4 lifecycle/fault soak
item in `docs/unsupported-features.md`. The gate composes the existing live
Vanilla 26.1 playable scenario; it does not replace that scenario with fixture
replay or a second client simulation.

## Goal and completion evidence

One invocation must build the current `HEAD` with the opt-in
`lifecycle-soak` validation feature, then run the checked-in manifest's fixed
number of independent live sessions. Every session must use the official local
26.1 dedicated server and the same external-multiplayer path as
`scripts/playable-gate.sh live`.

The final report must retain:

- the exact `HEAD`, manifest digest, fixed seed, iteration and timeout budgets;
- each playable evidence file and its protocol/canonical-state assertions;
- deterministic no-audio-device, resize, minimize/restore, surface
  lost/outdated and timeout injection receipts;
- the final resource/session generations and bounded queue gauges;
- final RSS, file-descriptor and thread counts before shutdown;
- the requested terminal mode, clean client exit status, and proof that the
  client process no longer exists; and
- the artifact directory for every failed or passing iteration.

The gate fails on a playable assertion, missing injection receipt, non-quiescent
terminal gauge, stale retained resource/session owner, process metric outside
the checked-in spread budget, timeout, non-zero exit, or surviving client
process. A passing process exit is the worker-termination and retained-handle
barrier: no native simulation, resource, mesh, texture, audio, or network worker
may outlive its owning client process.

## Scenario boundary

Each iteration keeps the existing playable scenario intact:

1. delegated status, Login, Configuration and Play connection;
2. movement plus chunk, block, entity, inventory, sound and particle activity;
3. death/respawn and Overworld/Nether/Overworld level replacement;
4. server resource-pack push, immutable generation commit, pop and second
   generation commit;
5. same-server reconfiguration and fresh canonical level publication;
6. same-process Transfer to the second official server; and
7. either an official-server kick followed by application shutdown, or direct
   application shutdown which requests the ordered network disconnect.

The manifest alternates both terminal modes. Independent process iterations
repeat connect and owner construction/destruction and make process leakage
mechanically visible. Within an iteration, the level, pack, reconfiguration and
transfer transitions already exercise multiple canonical and immutable
resource generations.

## Validation-only fault path

The `bbb-native/lifecycle-soak` Cargo feature enables one narrow manifest-driven
adapter. Default builds do not read the soak environment or expose fault
injection.

- Forced no-audio-device skips backend construction and publishes an exact
  disabled reason while gameplay continues.
- Resize and minimize/restore use the real winit window owner.
- Injected surface lost/outdated values enter the same swapchain
  reconfiguration branch as real `wgpu::SurfaceError` values. Timeout enters
  the same frame-skip branch.
- Renderer counters record requests, errors, reconfigurations and skipped
  frames. The live control snapshot is the sole evidence surface; no second
  diagnostics owner is introduced.

The adapter may choose when a checked-in action becomes due, but it may not
mutate world state, bypass the simulation mailbox, suppress a gate failure, or
change default client behavior.

## Manifest and bounds

`scripts/fixtures/lifecycle-soak/manifest-26.1.json` is the sole scenario
manifest. It owns the seed, three-iteration budget, per-iteration and total
timeouts, ordered fault schedule, terminal modes, terminal queue ceilings and
cross-iteration process-metric spread limits.

Queue ceilings are terminal gauges, not cumulative work counters. Safe stale
work may be counted when an old generation is rejected, but no pending result,
queue entry, server pack, or old level owner may remain at the quiescent
checkpoint. Existing playable assertions continue to prove that first-server
chunks, death state and resource packs do not cross the Transfer generation.

RSS is allocator- and driver-sensitive, so the gate does not require identical
values. After the first warm-up iteration, a strictly rising series fails only
when its total spread exceeds the manifest ceiling. File-descriptor and thread
spreads use smaller exact ceilings. Every process must still exit cleanly after
its own iteration, independent of the trend result.

## Serial implementation and checkpoints

1. **Contract and parser.** Add this design, the manifest, strict validation and
   mutation tests. Focused checks are the lifecycle-soak script tests,
   docs-lint, shell syntax where applicable, formatting and diff check.
2. **Runtime receipts.** Add feature-gated native/renderer injection, general
   renderer recovery counters, control projection and focused renderer,
   control and native tests. Run a full gate at this code checkpoint.
3. **Live termination and aggregation.** Add optional playable kick/quit
   termination evidence, the soak orchestrator, trend/quiescence assertions and
   script tests. Build current `HEAD`, run the explicit three-iteration soak,
   archive its report, close the ledger item and run the final full gate.

All commits are normal fix-forward commits. The current slice stops only at a
clean gated checkpoint, a fully passing live soak, or a documented external
environment blocker with all safely testable work committed.
