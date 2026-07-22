# Entity Equipment, Passenger, and Leash Closure

This document freezes the engineering contract for closing the Vanilla 26.1
`equipment/passenger/leash` coverage cell for all 157 fixed entity types. Work
starts from full-gated commit `edb0fd8da`.

## Goal and Completion Standard

Every fixed type must have one source-backed disposition for equipment packet
state and presentation, mount graph placement/control, and leash packet state
and presentation:

- `ClientboundSetEquipmentPacket` applies only to `LivingEntity`, retains every
  wire slot exactly, and all renderer-owned held/equipment layers consume the
  canonical stack through the generated type and renderer catalogs;
- every passenger is placed from the current vehicle render pose, generated
  attachment declarations, and the complete Vanilla override inventory, while
  controlling-passenger decisions remain separate from first-passenger and
  mount-graph membership; and
- `ClientboundSetEntityLinkPacket` applies only through the generated
  `Leashable` class graph, with every leash/rope-hold override and quad-leash
  pairing reaching the existing typed rope submission.

The category closes only after reverse tests cover all 157 generated rows, all
confirmed gaps below are removed or receive an explicit source-backed
not-needed disposition, no `equipment/passenger/leash` cell remains `partial`
or `deferred`, and focused tests, the full gate, and
`scripts/playable-gate.sh` pass at current HEAD.

## Non-Goals

- Server AI, leash physics, leash break/drop behavior, mounting eligibility,
  dismount collision search, autonomous vehicle callbacks, and server-side
  equipment selection are not simulated.
- Inventory screens, local equip interactions, item dynamic tint parity, and
  unrelated first-person item presentation remain with their existing owners.
- The renderer does not infer equipment applicability, mount control, leash
  capability, or entity families from textures or model kinds.
- Packet-authoritative mount and leash relations are not rejected merely
  because ordinary gameplay would not cause the server to send them.

## Owners and Frozen Boundaries

- `bbb-protocol` owns equipment slot wire identities and the generated fixed
  entity attachment catalog. No packet format or duplicate attachment table is
  planned.
- `bbb-world::entities::schema` owns the generated runtime class graph,
  `is_living`, `Leashable`, `canUseSlot`, renderer-family, vehicle-family, and
  control predicates. Handwritten code may consume these facts but may not add
  a second 157-type membership table.
- `bbb-world::entities::passengers` owns packet mount graphs, first-mount side
  effects, recovery, and controlling-passenger decisions. `entities::dimensions`
  owns pure attachment math; `EntityStore` projects a cycle-bounded root-to-leaf
  render position from current render poses without mutating packet state.
- `bbb-world::entities::leash` owns pure endpoint math. Endpoint inputs carry
  exact current dimensions, Camel pose animation, body rotation, player view
  rotation, and motion; the renderer receives typed rope states only.
- `bbb-native` drains the existing lossless narration/platform FIFO and maps an
  interrupting first-mount system request to `SystemNarrator::say_system_now`.
  It does not reinterpret mount state.
- `bbb-renderer` keeps equipment layers and leash geometry as submission-only
  consumers. A perfectly vertical leash retains the existing finite,
  zero-cross-section fallback: Vanilla produces NaN vertices through
  `invSqrt(0)`, while both paths rasterize no rope area; bbb avoids poisoning
  the shared mesh buffer.

## Vanilla 26.1 Evidence

The main agent rechecks local sources under
`/home/zgy/Work/mc-code/sources/26.1` before each correction. Primary anchors
are `ClientPacketListener.handleSetEquipment`,
`handleSetEntityPassengersPacket`, and `handleEntityLinkPacket`; `Entity` and
`LivingEntity` passenger placement; every `getPassengerAttachmentPoint`,
`getVehicleAttachmentPoint`, `getControllingPassenger`, `getLeashOffset`, and
`getRopeHoldPosition` declaration; and `EntityRenderer` /
`LeashFeatureRenderer` extraction.

The first reverse audit confirmed:

- `SetEquipment` already has the exact client-side `LivingEntity` gate and
  stores every decoded slot. The generated catalog already records all 157
  `canUseSlot` outcomes, and the closed renderer catalog owns held item, armor,
  saddle, body armor, wings, harness, and family-specific layer membership;
- first local mount already preserves the pre-packet indirect-passenger guard,
  boat yaw alignment, overlay, and removed-vehicle reset, but omits Vanilla's
  interrupting `GameNarrator.saySystemNow` call;
- generated attachment points plus Boat, Camel, Minecart, Slime, AbstractHorse,
  Llama, Spider/CaveSpider, baby, pose, and scale corrections exist, but the
  ordinary model-source path does not generally recompute passenger placement
  from the vehicle render pose. The Turtle-only render correction is not a
  complete family disposition, and the client-only Strider walk bob is absent;
- controlling-passenger code already distinguishes Boat living passengers,
  saddled Horse/Nautilus, item-steered Pig/Strider, harnessed Happy Ghast, and
  `Mob` fallback with the generated non-controlling-rider tag. Closure needs an
  exhaustive fixed-type guard rather than a parallel controller table; and
- Vanilla 26.1 has sixteen concrete `getLeashOffset` declarations. The current
  handwritten switch omits Camel, Llama, Fox, Copper Golem, Snow Golem, and
  Boat, and it omits `AbstractVillager.getRopeHoldPosition`. Player fall-flying
  roll uses the exact partial-tick view pitch/yaw, not body-yaw and rope-pitch
  substitutes. Turtle inherits `Leashable` through `Mob`, so a server-sent link
  is applied and rendered rather than rejected as a gameplay-impossible case.

Historical ledger wording is not evidence. Any additional rejected family is
verified against its current source before editing.

## Ordered Stages and Focused Tests

### A. First-mount narration and packet closure

Add an interrupting system narration kind to the existing typed request/FIFO,
enqueue it beside the first-mount overlay in Vanilla order, map it to
`say_system_now`, and close only the `SetPassengers` runtime-side-effect packet
cell. Repeated packets and reparenting inside an existing vehicle chain must not
replay either effect.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world first_local
scripts/cargo-dev.sh test -p bbb-native narration
scripts/vanilla-source-tools.sh python -m unittest scripts.tests.test_packet_coverage_ledger
```

### B. Passenger attachment and controller closure

Replace the Turtle-only render correction with one cycle-bounded immediate
vehicle/root-to-leaf projection that reuses the existing attachment math and
current partial-tick vehicle pose. Add the Strider client walk-bob override and
reverse guards for generated attachment/controller ownership. Keep canonical
packet transforms and local-player prediction owners unchanged.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world passenger_and_vehicle_movement
scripts/cargo-dev.sh test -p bbb-world controlling_passenger
scripts/cargo-dev.sh test -p bbb-native entity_scene
```

### C. Leash override closure

Complete every Vanilla leash-offset and rope-hold declaration, including Camel
partial-tick body-anchor animation and the player's exact partial-tick view
vector. Add generated-class reverse guards, retain link state for every
generated `Leashable` type including Turtle, and preserve quad-leash ordering,
lighting, slack, and the documented vertical-span fallback.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world leash
scripts/cargo-dev.sh test -p bbb-world camel_body_anchor
scripts/cargo-dev.sh test -p bbb-renderer leash
```

### D. Equipment evidence and coverage closeout

Prove the living packet gate, generated slot applicability, canonical equipment
queries, resource-pack equipment assets, and renderer layer consumers without
adding new family branches. Expand the shared evidence set, remove closed or
stale TODOs, regenerate packet/entity ledgers, and append the engineering
milestone index. No fixed row may remain non-complete.

Run:

```text
scripts/cargo-dev.sh gate
scripts/playable-gate.sh
```

## Checkpoints, Failures, and Safe Stops

The baseline is fully gated. The contract, narration correction, passenger
correction, leash correction, and coverage closeout are separate buildable
commits. Run the full gate after at most four commits and again at final
closeout. Focused failures are fixed forward; `batch-gate` is used only when the
introducing commit is unclear.

Safe stops are a clean contract checkpoint, a fully wired narration request, a
general passenger projection with the prior Turtle path removed, a complete
leash override correction, or a documented external blocker with prior
behavior intact. No world-side request or render state lands without its native
consumer in the same checkpoint.
