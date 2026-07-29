# Authenticated Player Reporting and Services Design

Engineering contract for closing the authenticated Minecraft Services block-list
and player-report composition/submission boundary left by the signed-chat,
ChatListener, and Social Interactions milestones.

## Goal and completion standard

An authenticated native client must use its existing startup credentials to:

- synchronize the Minecraft Services privacy block list and make that exact set
  the canonical blocked-player input used by chat and Social Interactions;
- create, retain, edit, discard, and resume Vanilla 26.1 chat, skin, and username
  report drafts from the existing Player Reporting and Social Interactions
  surfaces;
- build chat evidence from the canonical 1,024-event reporting log with exact
  signed-message context, limits, ordering, and base64 wire fields;
- submit the exact authlib 7.0.63 `/player/report` JSON request without exposing
  the access token to world state, snapshots, logs, or serialized configuration;
  and
- present deterministic report-type, reason, chat-selection, comments,
  attestation, sending, success, failure, and draft-discard screen states.

Completion requires bounded hostile-response/request handling, offline-mode
disabled behavior, generation-safe asynchronous completion, focused net/world/
native/renderer tests for every branch, removal of only the Services/reporting
TODO (the separate rich-hover/custom-font TODO remains), an archived contract
and milestone index entry, a clean reviewed diff, and a fully gated final HEAD.

## Authoritative 26.1 references

Minecraft sources under `~/Work/mc-code/sources/26.1/`:

- `PlayerSocialManager`
- `ReportingContext`, `ReportEnvironment`, `AbuseReportSender`
- `Report`, `ReportType`, `ReportReason`
- `ChatReport`, `ChatReportContextBuilder`, `NameReport`, `SkinReport`
- `ReportPlayerScreen`, `ChatReportScreen`, `ChatSelectionScreen`,
  `NameReportScreen`, `SkinReportScreen`, `ReportReasonSelectionScreen`,
  `AbstractReportScreen`, `ReportSendingScreen`, and `DiscardReportWarningScreen`
- `LoggedChatMessage`, `ChatLog`, `PlayerChatMessage`, `SignedMessageBody`, and
  `SignedMessageLink`

Exact account-service wire facts come from the local dependency
`~/Work/mc-code/downloads/26.1/libraries/com/mojang/authlib/7.0.63/authlib-7.0.63.jar`,
especially `YggdrasilUserApiService`, `BlockListResponse`,
`AbuseReportRequest`, and `minecraft.report.*`. The implementation must not
substitute remembered or third-party endpoint schemas for that versioned JAR.

## Frozen wire facts

- Authenticated requests use bearer authorization against
  `https://api.minecraftservices.com`.
- The privacy block list is `GET /privacy/blocklist`; its body is
  `{ "blockedProfiles": [UUID...] }`. The local zero UUID is never blocked.
  Vanilla's refresh cooldown is 120 seconds.
- Reports are `POST /player/report` with version `1`, report UUID, report
  payload, client info, third-party server address, null realm info, and
  uppercase `CHAT`, `SKIN`, or `USERNAME` report type.
- Report payload fields are `opinionComments`, nullable `reason`, nullable
  `evidence`, nullable `skinUrl`, `reportedEntity.profileId`, and
  `createdTime`. Message signatures and last-seen entries are standard base64.
- Default limits are: 1,000 comment UTF-16 code units, 4 selected messages, 40
  evidence messages, 9 leading context messages, and 0 trailing context
  messages.
- Chat evidence stores index, sender profile UUID, session UUID, timestamp,
  salt, last-seen signatures, signed plain content, nullable signature, and
  `messageReported`. Evidence is emitted oldest-to-newest after collecting
  roots newest-to-oldest.

## Ownership and state boundaries

- `bbb-net::user_api` owns the bounded HTTP transport, exact request/response
  serde types, bearer redaction, status/error classification, endpoint override
  used by tests, and the 120-second block-list request state. It exposes typed
  values only; it does not own social or screen state.
- Startup creates the Services client only for paired authenticated credentials.
  The access token remains in runtime-only credential/service owners and is
  never cloned into `WorldStore`, `ControlSnapshot`, renderer values, draft
  JSON, error text, or `Debug`.
- `bbb-world::client::social` remains the sole hidden/blocked decision owner.
  Services refresh results replace only the Services-backed blocked set;
  explicit startup blocked UUIDs remain a separate configured input whose union
  is queried through the existing canonical presentation path. Offline sessions
  never activate Services blocking.
- `bbb-world::client::chat` remains the reporting-log authority.
  `bbb-world::client::reporting` owns only language-neutral report drafts,
  selected log ids, builder validation, exact evidence projection, environment,
  submission lifecycle, and screen generation. It receives monotonic/epoch time
  and report UUIDs from native; serializable world state reads no wall clock or
  random source.
- `bbb-native::input::reporting` owns report screen focus/editing/scroll state,
  maps existing Player Reporting/Social Interactions actions into world
  mutations, starts bounded async service work, and submits completion through
  the ordered simulation mailbox. A completion must match service generation,
  report UUID, and submission attempt before it can mutate canonical state.
- `bbb-native::runtime::render_extract` projects immutable value-only reporting
  HUD state. `bbb-renderer::hud::reporting` owns layout, clipping, draw commands,
  shared hit geometry, and deterministic offscreen pixels. Renderer hit tokens
  carry the current report-screen generation and are revalidated by simulation.
- Disconnect, Transfer, Play reconfiguration, and account/service replacement
  invalidate pending service generations. Report drafts follow Vanilla's
  `ReportingContext` environment matching: a draft cannot silently cross to a
  different server environment.

## Explicit non-goals

- No Mojang/Microsoft sign-in or token acquisition UI is added. Authenticated
  mode continues to require the existing startup `--profile-id` and
  `--access-token`.
- No in-game configuration UI is added. Reporting screens are gameplay action
  surfaces, not configuration.
- Realms environment/reporting is absent because `bbb` has no Realms connection
  owner; ordinary connections use third-party server info.
- The separate rich `show_item` / `show_entity`, `space` font, custom-font, and
  long-tail text-style TODO is not absorbed into this engineering slice.
- The Services endpoint is not exercised by ordinary tests. HTTP fixtures use a
  bounded local server; a real authenticated smoke is optional and cannot be a
  prerequisite for the deterministic gate.

## Serial stages, tests, and checkpoints

Each stage is a buildable safe stop. This is a high-risk account HTTP,
canonical-state, async-lifecycle, and screen engineering slice. The baseline
`3fe98ca60` is fully gated; run a full gate after stages 1, 3, and 5, and at any
earlier point where focused behavior becomes abnormal.

1. **Versioned Services transport.** Add the `bbb-net` user API owner, exact
   block-list/report schemas, bounded response and request limits, bearer
   redaction, cooldown clock injection, local HTTP fixtures, and offline
   disabled construction.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-net user_api`.
2. **Canonical block-list lifecycle.** Route authenticated startup/session
   refresh through the runtime owner, replace the Services set only on matching
   generations, union it with explicit startup blocking, and update chat/social
   screens without a second decision path. Cover success, failure, cooldown,
   offline, zero UUID, Transfer, reconnect, and stale completion.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-world player_social` and
   `scripts/cargo-dev.sh test -p bbb-native services_block_list`.
3. **Canonical report builders.** Add report/environment/reason/limit/draft
   values and exact chat-context collection from the existing reporting ring.
   Cover every validation error, context-chain/last-seen branch, ordering,
   nullable signature/skin URL, base64 request JSON, copy/discard, reset, and
   old-publication isolation.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-world player_reporting` and
   `scripts/cargo-dev.sh test -p bbb-net abuse_report`.
4. **Native screens and renderer surface.** Implement the report-type chooser,
   three editors, reason list, chat selection, comments, attestation,
   send/discard flow, draft badges, narration, hit revalidation, immutable
   extraction, and deterministic renderer/offscreen tests.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-native player_reporting`,
   `scripts/cargo-dev.sh test -p bbb-renderer hud_player_reporting`, and
   `scripts/cargo-dev.sh test -p bbb-renderer offscreen_player_reporting`.
5. **Asynchronous submission and closeout.** Queue service submission without
   blocking simulation/render, apply only matching completion, present
   success/failure/retry, redact all errors, close the ledger item, archive this
   contract, update the milestone index, run P0/coverage audits, review, commit,
   and gate.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-native player_reporting_submission` and
   `scripts/cargo-dev.sh test -p bbb-native simulation::mailbox`.

Any failure is fixed forward on the current stage. No unrelated slice starts
until this contract closes or a concrete external blocker is recorded.
