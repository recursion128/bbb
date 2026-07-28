# Server Data Presentation And Persistence Closure

## Goal

Close the Vanilla Java 26.1 `ClientboundServerDataPacket` path from trusted
Component decode through canonical icon validation and the optional persisted
multiplayer server-list owner.

Completion requires:

- preserving the exact trusted MOTD Component rather than only its readable
  summary;
- applying `ServerData.validateIcon` before canonical icon replacement;
- retaining the prior icon when the packet omits the field or validation
  rejects it;
- updating a configured server-list owner through the shared play-packet apply
  path; and
- writing a Vanilla-compatible, uncompressed named-root `servers.dat` NBT file
  without losing unrelated entries or fields.

## Non-goals

- No in-game server-list or configuration UI.
- No status-ping row, LAN discovery, reorder, edit, or delete presentation.
- No persistence for ordinary CLI direct-connect sessions, which have no
  Vanilla `ServerData` owner.
- No MOTD serialization: Vanilla `ServerData.write` does not include MOTD.

## Ownership And Integration Contract

- `bbb-protocol` owns trusted Component wire decode and bounded named-root NBT
  file encoding/decoding.
- `bbb-world` owns canonical `ServerDataState`, including exact
  `RichTextComponent` and validated icon bytes.
- `bbb-world::apply_play_packet` remains the only clientbound play dispatch.
  It invokes `PlayApplyEffects::server_data_updated` only after canonical
  mutation.
- `bbb-native` owns the optional file side effect. Both
  `--server-list-store PATH` and `--server-list-name NAME` are required and
  only valid with `--connect-server`; this explicit startup configuration is
  the repo-native equivalent of selecting/adding a multiplayer-list entry.
- The native owner queues saves onto one ordered background I/O worker and
  joins it at shutdown, matching Vanilla's scheduled persistence without
  blocking the simulation loop or losing accepted work during clean exit.
- Probe and state-only callers retain the default no-op effect.
- Transfer keeps the same configured owner, matching Vanilla's carried
  `ServerData` behavior.

## Vanilla 26.1 Evidence

- `ClientboundServerDataPacket.STREAM_CODEC` uses
  `ComponentSerialization.TRUSTED_CONTEXT_FREE_STREAM_CODEC` followed by an
  optional byte array.
- `ClientPacketListener.handleServerData` updates only when `serverData` is
  non-null, replaces MOTD, validates a present icon, and schedules
  `ServerList.saveSingleServer`.
- `ServerData.validateIcon` calls `PngInfo.fromBytes` and accepts dimensions
  whose signed `int` values are both at most 1024.
- `PngInfo.fromStream` reads only the PNG signature, IHDR length/type, width,
  and height.
- `ServerData.write` serializes name, IP, optional base64 icon, resource-pack
  preference, and accepted code-of-conduct hash, but not MOTD.
- `ServerList.save` writes an uncompressed named-root NBT file and uses a
  temporary file plus `servers.dat_old` replacement backup.

## Ordered Stages

1. Preserve rich MOTD wire and canonical values; validate icon header/dimensions.
2. Add bounded named-root NBT file codec support and semantic tests.
3. Add the explicit native owner, safe replacement, and packet-effect wiring.
4. Close packet coverage and feature-ledger entries.

Each stage remains buildable in the integrated slice. Failure before the final
commit is localized by crate-focused tests; the gated commit is the safe stop.

## Verification

- `bbb-protocol`: rich server-data Component decode and named-root NBT
  round-trip/trailing-byte tests.
- `bbb-world`: valid, omitted, malformed, oversized, and signed-dimension icon
  behavior.
- `bbb-native`: configured-owner CLI contract, existing-entry/unknown-field
  preservation, backup replacement, and `NetEvent` through canonical apply to
  persisted icon.
- Per-commit checks and the full high-risk gate from `goal.md`.
