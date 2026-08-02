# AddEntity Client Lifecycle Engineering Contract

This document freezes the high-risk engineering slice that closes the
Vanilla 26.1 `ClientboundAddEntityPacket` client lifecycle. It refines the
entity ownership described in
[`native-client-architecture.md`](../native-client-architecture.md) without
claiming that every entity-family model, animation, constructor default, or
client tick is complete.

## Semantic Goal and Completion

The packet's entity-type field remains a raw registry holder id at the wire
boundary. Before canonical mutation, the single mutable `WorldStore` owner
must resolve that holder through the current frozen RegistryAccess and then
map its exact resource identity to the generated fixed EntityType catalog.
Unknown/custom identities are ignored instead of being interpreted through a
bootstrap ordinal. A Player entity is created only after the matching current
PlayerInfo entry exists.

For an accepted entity, one ordered world-apply path must perform the same
observable lifecycle as Vanilla's `ClientPacketListener.handleAddEntity`:

1. clear matching removed-player-vehicle recovery state;
2. create the validated entity or reject it without mutating an existing
   same-id entity;
3. apply base/Living/RemotePlayer and every applicable
   `recreateFromPacket` specialization;
4. insert or replace the canonical entity;
5. record the player in the seen-player map; and
6. queue the post-add minecart/bee ticking sound owner.

Llama-spit recreation emits its seven client particles only after the entity
is present. Runtime effects are sealed before their matching immutable
publication, preserving the existing effect-before-publication gate. The
slice is complete when registry reorder/custom/unknown cases, missing
PlayerInfo, every Vanilla override family, same-id replacement, old
publication isolation, post-add effect ordering, entity removal, level reset,
and reconnect id reuse have focused tests; the AddEntity packet coverage cells
are complete; and the final full, live playable, and auto gates pass.

## Non-Goals

- No packet layout, tick cadence, input, interpolation, rendering, protocol
  compatibility, or visible-content reduction.
- No new canonical entity store, runtime-owned gameplay copy, `RwLock`,
  `unsafe`, global allocator, Rayon, configuration UI, AI, pathfinding,
  server physics, damage, or authoritative removal.
- This slice does not mark the entity matrix's broad constructor/default,
  transform, model, animation, equipment, event, save, or debug cells complete.
  It closes only facts caused directly by AddEntity creation/recreation.
- Custom EntityType identities are not synthesized into the fixed 157-type
  runtime. They remain diagnosable and ignored until the custom-presentation
  owner is implemented.
- Post-add audio must not be approximated by a one-shot sound. Minecart and bee
  loops remain entity-bound derived runtime owners and stop on removal/reset.

## Owners and Frozen Integration Boundary

- `bbb-protocol` owns the exact wire order and preserves the raw EntityType
  holder id. It does not consult world registries.
- `bbb-world` owns registry resolution, PlayerInfo gating, canonical entity
  insertion/replacement, recreation state, deterministic sound seed/state,
  counters, and read-only queries for derived ticking sounds.
- `bbb-world::PlayApplyEffects` is the only packet-apply side-effect boundary.
  Llama-spit particles use it after canonical insertion; state-only probe
  callers preserve the same world mutation and deterministic random sequence.
- `bbb-native` seals particle/audio effects before publication. The audio
  runtime may retain backend-only keys for active ticking sounds, but these
  are discardable derivatives of the immutable world generation, not a second
  gameplay owner.
- `bbb-audio` owns playback handles and stops entity-bound sounds when their
  canonical update disappears. Renderer/control continue reading the same
  exact world `Arc` and publication generation.

The accepted-type conversion is exact:

```text
wire EntityType holder id
  -> current FrozenRegistryAccess resource identity
  -> generated VANILLA_ENTITY_TYPES exact resource lookup
  -> canonical fixed entity_type_id
```

Numeric equality alone is never sufficient once synchronized contents exist.
The complete resource string, not a hash, proves identity. A rejected packet
does not despawn or reset a previously tracked same-id entity.

## Vanilla 26.1 Evidence

Local source root: `/home/zgy/Work/mc-code/sources/26.1`.

- `ClientboundAddEntityPacket` encodes its type with
  `ByteBufCodecs.registry(Registries.ENTITY_TYPE)`.
- `ClientPacketListener.handleAddEntity` clears matching removed-vehicle
  recovery, calls `createEntityFromPacket`, invokes `recreateFromPacket`, adds
  the entity, starts post-add sound, and only then records a Player as seen.
- `createEntityFromPacket` rejects Player before PlayerInfo and otherwise calls
  the EntityType factory with `EntitySpawnReason.LOAD`.
- The finite override inventory is `Entity`, `LivingEntity`, `RemotePlayer`,
  `AbstractMinecart`, `EnderDragon`, `ItemFrame`, `Painting`,
  `FallingBlockEntity`, `Shulker`, `Warden`, `Projectile`, `FishingHook`,
  `LlamaSpit`, and `ShulkerBullet`.
- `postAddEntitySoundInstance` starts `MinecartSoundInstance` for every
  AbstractMinecart and the aggressive or ordinary looping bee sound according
  to `Bee.isAngry`. These sounds follow the entity and stop when it is removed.

Critical source facts are rechecked by the main agent before each owning phase;
the repository does not read local Vanilla sources during normal build/test.

## Serial Phases, Tests, Gates, and Safe Stops

1. **Contract and inventory.** Record this contract and audit the generated
   EntityType lookup, current registry API, all fourteen recreation overrides,
   and ticking-audio boundary. Focused document/source checks; normal commit.
2. **Validated canonical creation.** Resolve raw holder ids by exact resource,
   enforce the PlayerInfo gate, preserve same-id state on rejection, and close
   base plus specialization recreation state. Focused `bbb-protocol` and
   `bbb-world` tests, fmt/diff/docs checks, normal commit, then full gate. This
   gated commit is the first safe stop.
3. **Ordered client side effects.** Add LlamaSpit recreation particles and
   post-add minecart/bee ticking sound owners; make backend ticking-sound
   start/switch/stop idempotent across publications, removal and reset.
   Focused world/native/audio tests, normal commit, then full gate. This gated
   commit is the second safe stop.
4. **Coverage closeout.** Update packet/entity coverage evidence,
   `unsupported-features.md`, architecture notes, and `goal-archive.md` without
   overclaiming adjacent entity-family cells. Run generator checks, focused
   crate tests, fmt/diff/docs checks, full diff review, and commit.
5. **Delivery.** At the committed final HEAD run `scripts/cargo-dev.sh gate`,
   build `bbb-native` in `/tmp/bbb-target-main`, verify its timestamp is newer
   than the HEAD commit, run the explicit Weston-backed
   `scripts/playable-gate.sh live`, then the policy auto gate.

Failures localize to registry/factory admission, canonical recreation, runtime
side effects, or coverage evidence. Once phases 2 or 3 start, do not cross
their safe-stop boundary until the phase is buildable, reviewed, committed,
and full-gated. Fix failures forward with normal commits; never amend, rebase,
reset, or rewrite history.

## Delivered Ownership and Residual Boundary

The implementation landed as the frozen serial phases rather than as a second
entity creation path. `3df38cb7d` established exact session-registry admission,
PlayerInfo/FishingHook gates, specialization recreation, same-id safety, and
retained-publication isolation. Fixture and evidence repairs followed in
`2f6496629` and `9f6a9a453`; the latter passed the phase-2 full gate.
`dae892d9a` added ordered LlamaSpit particles plus canonical minecart/bee
ticking-sound owners and passed the phase-3 full gate.

The final ownership shape is:

- one production AddEntity dispatch path calls world-owned admission and
  canonical replacement;
- static entity-type identity comes from the generated catalog only after the
  current session holder resolves to an exact resource id;
- recreation particles cross `PlayApplyEffects` after insertion and before the
  matching immutable publication;
- active minecart/bee loop state lives in `WorldStore`, is removed on entity
  replacement/removal/reset, and remains isolated in retained generations; and
- native audio retains only discardable backend keys, restarting the correct
  loop after a bee anger transition or resource reload.

This closes the five packet-coverage dimensions for AddEntity itself. It does
not close the entity-family matrix: renderer/model/texture/layer/animation,
metadata callbacks, later entity events, and family-specific client ticks stay
owned by their existing ledger cells. Custom synchronized EntityType resources
also remain rejected because the fixed client has no matching factory or
presentation owner.
