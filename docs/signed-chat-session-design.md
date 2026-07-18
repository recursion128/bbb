# Signed Chat Session Engineering Contract

This document freezes the ownership, wire evidence, phases, and validation for
the authenticated signed-chat transport slice. It covers the local profile-key
session, outbound message chain, inbound chain validation needed to validate the
server echo, and `DeleteChat` lifecycle. It does not turn the native client into
an authentication launcher or duplicate Mojang's Brigadier/Authlib class trees.

Implementation status: the contract is frozen; phase 1 is current.

## Semantic Goal and Completion Criteria

An authenticated, encrypted 26.1 connection establishes one refreshable local
chat session and uses it to sign ordinary chat plus every signable command
argument. The same session's last-seen view produces both the signed payload and
wire acknowledgement update. Incoming player messages are validated against
the sender's current remote session, and `DeleteChat` either resolves one exact
signature or terminates the invalid stream before mutating visible state.

Final completion requires all of the following:

- The Services `POST /player/certificates` response is bounded before allocation,
  decoded into an RSA private/public pair plus Services-signed public-key data,
  and never exposes the bearer token or private key through debug/serde/errors.
- A local session UUID and monotonically advancing message link sign the exact
  vanilla 26.1 payload with `SHA256withRSA`: sender UUID, session UUID, chain
  index, salt, epoch seconds, UTF-8 byte length/content, and the ordered full
  last-seen signatures.
- The wire `LastSeenMessages.Update` and full signatures used for signing come
  from one atomic world operation. Plain chat consumes one chain link; a command
  consumes one link per parsed `minecraft:message` argument in command order.
- The current command tree returns the signable argument name and exact parsed
  value, including greedy message text, redirects, and literal precedence. It
  does not claim to be a general replacement for Brigadier parsing.
- Authenticated encrypted sessions fetch/refresh without blocking packet reads,
  automatically emit `ServerboundChatSessionUpdatePacket`, replace the encoder
  only when the key changes, and clear all local/remote chain state on listener,
  configuration, reconnect, and shutdown boundaries.
- Player chat with a present remote session validates timestamp order, link
  index, sender/session identity, key expiry, and RSA signature. A broken chain
  cannot silently resume; the secure-chat policy produces the vanilla-equivalent
  disconnect/error outcome rather than presenting untrusted content as signed.
- `ClientboundDeleteChatPacket` resolves its packed signature before any state
  mutation. An unresolved signature requests disconnect. A resolved signature
  leaves the last-seen pending set, is removed from any delayed presentation
  queue, or changes the visible entry to the deleted marker no earlier than 60
  GUI ticks after insertion.
- Coverage overrides and the unsupported ledger no longer report the local
  key/session, outbound signatures, server-echo validation, or `DeleteChat`
  lifecycle gaps. Focused tests, workspace full gate, and recorded/live playable
  evidence pass at delivered `HEAD`.

## Non-goals

- Full `ChatListener` presentation (localized rich HUD interactions,
  receive-ability UI, narration, abuse-report log) remains a separate screen/HUD
  slice after transport trust is authoritative.
- Abuse reporting, social blocking, realms/account UI, token acquisition or
  refresh, and in-game configuration are not added.
- Production profile-key persistence is not added. Vanilla
  `AccountProfileKeyPairManager` only writes its debug cache while running in its
  IDE mode; bbb fetches from the startup access token and retains private material
  only in the connection owner.
- Offline/unencrypted servers keep the unsigned encoder unless their secure-chat
  policy requires a signed session. No synthetic key is generated as a fallback.
- The existing packet IDs and serverbound packet wire encoders are not redesigned.

## Ownership and Integration Contract

- `bbb-net` exclusively owns the bearer-token request, private RSA key, local
  session UUID, signing chain index, refresh deadline, and automatic session
  update. Private material never crosses into `WorldStore`, control snapshots,
  protocol packet values, or native render state.
- `bbb-world` exclusively owns the command-tree projection, full last-seen
  tracker, remote player chat sessions/chains, canonical message validation, and
  deletion timing state. Its outbound operation returns an owned value containing
  the wire update plus ordered full signatures; it never signs or stores a
  private key.
- `bbb-native` submits semantic chat/command requests from the simulation owner
  and applies ordered disconnect/presentation effects. It does not construct RSA
  payloads or keep a second last-seen tracker.
- `bbb-protocol` remains the packet/wire value owner. Its existing
  `MessageSignature`, `ArgumentSignatures`, `LastSeenMessagesUpdate`, and chat
  encoders are reused; any added value surface must be wire-shaped and contain no
  network-session behavior.
- The net command path preserves FIFO ordering. Key refresh/session update is
  ordered before the first signature produced by the replacement encoder, and
  configuration/reconnect cannot replay a prior session's queued signature.
- Failure fallback is explicit: key fetch/parse failure retains the last still
  valid encoder, or unsigned mode when none exists, and emits a redacted
  diagnostic. Signature construction or impossible chain overflow fails the
  connection rather than sending a mismatched unsigned packet on a signed path.

## Authoritative Vanilla and Authlib Evidence

- `ClientPacketListener.handleLogin`, `tick`, `prepareKeyPair`, `setKeyPair`,
  `sendChat`, `sendCommand`, `handleDeleteChat`, `markMessageAsProcessed`, and
  `sendChatAcknowledgement` define lifecycle and ordering.
- `LocalChatSession`, `SignedMessageChain`, `SignedMessageLink`,
  `SignedMessageBody`, `PlayerChatMessage.updateSignature`, `LastSeenMessages`,
  `LastSeenMessagesTracker`, and `ArgumentSignatures.signCommand` define the
  signing bytes, chain advancement, and acknowledgement coupling.
- `ProfileKeyPair`, `ProfilePublicKey.Data`, `ProfileKeyPairManager`, and
  `AccountProfileKeyPairManager` define refresh and public-key wire data.
- The checked-in local 26.1 Authlib 7.0.63 class evidence defines the production
  Services host, bearer-authenticated `POST /player/certificates`, and response
  fields `keyPair.privateKey`, `keyPair.publicKey`,
  `publicKeySignatureV2`, `expiresAt`, and `refreshedAfter`.
- `ServerboundChatPacket`, `ServerboundChatCommandSignedPacket`,
  `ServerboundChatSessionUpdatePacket`, and `ClientboundDeleteChatPacket` remain
  the wire authorities.

## Serial Phases and Validation

1. **Key service and signing core.** Add the bounded Services client, strict PEM/
   time/RSA validation, redacted local key/session value, exact payload builder,
   RSA-SHA256 signer, verifier, and chain-overflow behavior. Deterministic fixed-key
   fixtures compare payload bytes and verify signatures. Run all `bbb-net` tests,
   non-test check, and a full gate because Cargo/cryptography changes are high risk.
2. **Outbound signing contract.** Make the world command tree return ordered
   signable name/value pairs and make the last-seen operation return the full
   signatures paired with its wire update. Introduce net-owned outbound request
   values; native stops constructing protocol packets and the net owner signs chat
   or each argument in FIFO order. Test literal/redirect/greedy parsing, identical
   last-seen inputs, multi-argument link order, unsigned fallback, and source
   ownership. Run affected world/net/native tests and a full gate.
3. **Fetch, refresh, and automatic session update.** Start key preparation only
   for authenticated encrypted connections, keep packet reads live while it runs,
   atomically install the returned session, send its public data before using the
   new encoder, refresh no more often than vanilla's one-hour check, and reset on
   reconfiguration/reconnect. Fake HTTP and duplex connection tests cover success,
   bounds/failure, ordering, refresh, and redaction. Run `bbb-net` plus integration
   tests, full gate, and `scripts/playable-gate.sh auto`.
4. **Inbound trust and deletion closeout.** Validate remote chains with the
   Services-validated player public key, propagate secure-chat errors/disconnects,
   and implement delayed/visible deletion at the world/native HUD boundary.
   Tests cover reordered/invalid/expired/missing signatures, chain break, packed
   cache miss, pending removal, delayed queue removal, and the exact 60 GUI-tick
   lower bound. Update coverage/ledger/archive evidence, run affected complete
   crate tests, final full gate, and final playable gate.

Every phase also runs formatting, diff checks, documentation lint when changed,
P0 audit, and complete diff review before its normal commit. No phase may add a
selectable parallel signing implementation.

## Failure Localization and Safe Stops

Phase 1 failures are confined to external response parsing and pure signing
bytes. Phase 2 establishes the cross-owner value contract without asynchronous
fetch. Phase 3 adds service/network scheduling only after deterministic signing
and request construction are gated. Phase 4 consumes the already frozen remote
session/signature values and presentation clock.

Each phase's clean full-gated tip is a safe stop. Once a phase begins, stop only
at its successful gate, an explicit external blocker, or a normal fix-forward
commit that restores a buildable state. Never deliver a key/session transition
where old and new encoders are both selectable or where a private key can enter
a serialized/control surface.
