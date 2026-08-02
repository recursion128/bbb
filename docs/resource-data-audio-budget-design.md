# Resource Data And Audio Aggregate Budgets

> Canonical scope：资源可信链见
> [`docs/wiki/mechanisms/resources/offline-asset-trust.md`](wiki/mechanisms/resources/offline-asset-trust.md)。
> 本页只保留 data/audio budget 的详细 evidence。

This document freezes the high-risk engineering contract for bounding selected
resource JSON, model/font/sound collections, and decoded audio. It extends the
atomic owner in
[`resource-pack-lifecycle-design.md`](resource-pack-lifecycle-design.md) and the
decoded-image limits in
[`resource-image-memory-budget-design.md`](resource-image-memory-budget-design.md).
It does not change resource priority, visible fallback, audio selection, or the
simulation/publication boundary.

## Semantic goal and completion criteria

Every production pack JSON read must pass one bounded reader before allocating
its payload, then charge its parsed tree and retained derived collection to the
same immutable client-resource generation. Resource directory traversal must
have an explicit depth and result-count bound. Cloned stacks share the exact
counter; a staged reload receives only the process allowance left by the
accepted generation. A budget rejection is sticky even when an existing
optional parser normally logs and skips malformed content, so an exhausted
generation can never commit a silently incomplete mix.

Every selected OGG must be admitted before playback. Static sounds decode
through a frame-counting decoder into one generation-keyed shared owner rather
than allocating a fresh complete frame vector for every play. Streaming sounds
retain bounded decoder buffers but must still prove a bounded compressed file,
sample rate, channel shape, and finite duration before starting. Resource reload
stops old playback and clears the decoded owner as Vanilla's `SoundEngine.reload`
does; an old resource generation cannot hit the new cache.

The slice is complete when:

- one JSON file cannot exceed 16 MiB, 262,144 parsed value nodes, or 16 MiB of
  decoded UTF-8 key/value bytes;
- one resource listing cannot exceed 65,536 results or 64 directory levels;
- JSON source, parsed-tree, listing and derived-collection logical charges share
  a 512 MiB generation ceiling, and accepted plus staged generations cannot
  exceed 1 GiB;
- model, blockstate, item-model, font-provider/glyph and sound-event/entry
  collections have semantic caps before downstream bake or atlas allocation;
- every production pack JSON consumer either uses the bounded owner or is a
  checked-in/generated constant with a source guard proving the exception;
- one OGG cannot exceed 64 MiB compressed, 192 kHz, stereo, 30 minutes, or
  256 MiB of static decoded stereo frames; one decoded packet cannot expose
  more than 65,536 stereo frames to either playback path;
- the static decoded-sound owner retains at most 512 MiB, shares one exact frame
  allocation across simultaneous plays, retains at most 65,536 buffer owners,
  evicts only inactive least-recently-used owners, and rejects admission when
  active owners pin the allowance;
- reload/reset, stopped handles, malformed/truncated input, unknown duration,
  cache eviction and resource-generation reuse have focused coverage; and
- immutable diagnostics report current/peak logical data and decoded-audio
  ownership plus rejection/eviction counts without an allocator hook or
  high-frequency logging.

The limits are deliberately generous defensive ceilings rather than Vanilla
constants. Overflow and all size arithmetic use checked `u64`/`usize`
conversion. A limit is tested as an exact deterministic admission rule, not a
wall-clock CI threshold.

The model/font semantic ceilings are fixed as follows. A generation admits at
most 65,536 blockstate documents, 262,144 blockstate selection entries, 65,536
cuboid-model documents, 262,144 model elements, 1,048,576 faces, 262,144 model
texture slots, 65,536 item-model definitions and 262,144 item-model references.
One flattened font graph admits at most 256 definitions, 4,096 providers,
1,048,576 bitmap glyph slots and 65,536 space advances. Repeated/cyclic font
references retain first-visit semantics and count each visited definition once.
The retained sound catalog admits at most 65,536 event definitions and 262,144
file/event entries across all namespaces and selected layers. Replacement and
extension are checked against the resulting retained collection before each
definition is installed.

Logical retained charges use stable estimates rather than allocator layout:
512 bytes per blockstate/model/item-model/font-definition owner, 256 bytes per
blockstate selection/model element/font provider, 128 bytes per model face,
texture slot or item-model reference, and 64 bytes per font glyph slot or space
advance. Sound catalogs charge 512 bytes per retained event and 256 bytes per
retained entry. These charges are additive with bounded JSON
source/tree/listing charges and are intentionally conservative; they do not
claim to measure RSS.

## Before baseline and cost attribution

The local official Java 26.1 `assets/**` tree contains 6,538 JSON documents,
3,254,463 source bytes, 119,497 JSON value nodes and 1,769,281 decoded UTF-8
key/value bytes. The largest document is `minecraft/lang/en_us.json` at 502,008
bytes and 7,887 nodes. The current model/font inventory includes 1,506 item
definitions, 1,284 item cuboid models, 13 font providers, 2,864 bitmap-grid
codepoints, two space advances, 1,136 explicit cuboid elements and 4,050 faces.
These counts are a reproducible filesystem/content baseline, not allocator RSS
or proof that every consumer retains each document exactly once.

Before this slice, production consumers commonly call `std::fs::read` or
`read_to_string` and then `serde_json` directly. Serde's recursion protection
bounds nesting but not file bytes, document count, wide arrays/maps, decoded
strings, directory traversal, or the sum retained by adjacent reload
generations. Model and font output can further expand one admitted tree into
typed vectors, glyph maps and baked geometry.

The local source tree contains no official OGG payloads, so this host cannot
record a representative ordinary-audio duration distribution without the
missing asset-object tree. The structural before path is still exact:
`StaticSoundData::from_file` decodes a fresh complete `Vec<Frame>` for every
non-streaming play, while `StreamingSoundData::from_file` has no BBB file or
duration admission. There is no decoded resident-byte owner, and resource
reload replaces only the catalog rather than clearing playback data. The after
record must therefore separate deterministic byte/frame ownership evidence
from any future official-asset timing capture.

## After measurements and cost ownership

On the same host, loading the representative official resource consumers into
one generation charges 34,438,925 logical bytes (32.84 MiB, 6.41% of the
512 MiB ceiling): 4,832,934 JSON source bytes, 11,339 documents, 204,611 value
nodes, 2,621,803 decoded string bytes, 11,271 listing entries and 10,505,856
derived bytes, with zero rejections. The item-runtime subset charges 16,293,223
logical bytes, including 4,661,888 derived bytes. These are repeatable owner
counters from the normal loaders, not an allocator sample.

The audio before/after result is structural because this checkout and its local
downloads contain no official OGG payload. Before, every non-streaming play
called `StaticSoundData::from_file` and constructed a fresh complete
`Vec<Frame>`. After, the first admitted play constructs one cache owner and an
exact generation/path hit adds zero decoded-frame owner bytes; focused tests
prove simultaneous returned data use the identical frame `Arc`, inactive LRU
eviction occurs in order, active pins reject pressure, and reset cannot reuse a
stale generation. No wall-clock multiplier or ordinary-sound duration
distribution is claimed without the missing payloads.

The 512 MiB audio number covers retained decoded frame owners, not total RSS.
Path/hash-table overhead, Symphonia/Kira scratch and device/backend buffers are
outside the logical counter. Reload clears the cache immediately, but a stopped
sound's old frame `Arc` can remain pinned until the audio callback observes the
stop; a conservative transient envelope is therefore old active frames up to
512 MiB plus the new 512 MiB cache plus one admitted 256 MiB static decode.
Compressed streaming input is file-backed and capped, and each exposed decoded
packet is capped at 65,536 stereo frames, but codec-private scratch remains an
external bounded-library risk rather than a measured BBB owner.

## Ownership and integration contract

- `bbb-pack::resource_budget` owns bounded reads, JSON-tree accounting,
  resource-list admission, sticky rejection and immutable observations. One
  `ClientResourceGeneration`, every `PackRoots` clone and every derived
  `PackResourceStack` share that owner.
- `bbb-pack` semantic loaders charge retained model/blockstate/item/sound
  collections before insertion. Embedded generated catalogs are not filesystem
  input and remain outside the runtime generation charge; source guards keep
  that exception narrow.
- `bbb-item-model` consumes the pack-owned bounded JSON API for font definitions
  and charges provider, reference and glyph entries through the same generation.
  Bitmap pixels continue to use the separate image budget.
- `bbb-native::resource_reload` computes the staged logical-data allowance from
  the accepted generation before preparation. Sticky failure is checked before
  renderer/audio commit, preserving the exact old generation on failure.
- `bbb-audio` owns bounded Symphonia decode, the generation-keyed static-data
  LRU, active pins and playback observations. It receives a resource generation
  through `ResolvedSound`; paths or hashes alone never prove cache identity.
- `bbb-control` exposes serde-defaulted observations only. It owns no JSON tree,
  model, glyph, audio frame, cache key or mutable admission state.

JSON source bytes are charged before reading from an opened file. Parsed nodes,
decoded strings and derived entries are charged before the typed value enters a
catalog. Charges are conservative and monotonic for the generation: a failed
parse or dropped intermediate does not refund an allowance that hostile input
could otherwise retry. Dropping the staged generation releases the whole owner;
accepted state is never freed early to make room.

The audio cache key contains resource generation plus the exact selected file
path. Active Kira sounds clone the cache owner's immutable frame `Arc`; they do
not create another decoded frame vector. Streaming playback uses a bounded
decoder with no full-file frame owner. Driver/device/backend scratch and delayed
release remain outside logical counters and must be reported as residual risk.

## Vanilla 26.1 evidence

- `SimpleJsonResourceReloadListener.scanDirectory` parses each selected JSON
  resource into a staged map through `StrictJsonParser` and a typed Codec.
- `ModelManager.reload` prepares block models, blockstates, item definitions and
  dependencies before the reload barrier, then applies one resolved generation.
- `FontManager.prepare` loads provider stacks, resolves references, finalizes
  glyph providers and only replaces live font sets in `apply`.
- `SoundManager.prepare` merges selected `sounds.json` stacks and distinguishes
  file versus event entries plus `stream` and `preload`.
- `SoundBufferLibrary.getCompleteBuffer` fully decodes non-streaming audio and
  caches it; `getStream` returns a gradual decoder. `SoundEngine.reload` stops
  all sounds, clears decoded buffers, recreates the backend and preloads the new
  catalog.

Vanilla's current readers do not define BBB's hostile-input numbers. BBB keeps
the same selection, staging, static/streaming behavior and reload ordering while
adding explicit failure bounds.

## Non-goals

- No protocol, tick, input, resource priority, model selection, glyph metrics,
  sound selection, attenuation, pitch, render order or visible-content change.
- No global allocator, RSS assertion, `unsafe`, Rayon, second resource stack,
  second canonical world or independent audio catalog.
- No silent resource omission to fit a budget. Existing malformed optional-file
  policy remains for ordinary parse errors; a sticky admission failure rejects
  the entire staged generation.
- No ZIP/download redesign, PNG changes, dynamic texture work, shader binary
  budget, audio-device feature, or new configuration UI.
- No claim that logical charges include serde allocator overhead, temporary
  decoder buffers, GPU objects, audio backend buffers or driver-deferred release.

## Serial phases, validation and safe stops

1. **Contract and baseline.** Commit this document and the reproducible official
   filesystem/content counts. Documentation lint is the focused validation.
2. **Generation data admission.** Add the shared budget, bounded JSON reader and
   bounded resource traversal; migrate production pack JSON consumers, compute
   adjacent staged allowance, publish diagnostics, and add source guards.
   Focused `bbb-pack`, `bbb-control` and native resource-reload tests cover
   single/generation/process limits, clone sharing, sticky swallowed failure,
   rollback and new-generation reset. Commit and run the first full gate.
3. **Model and font collections.** Add semantic model/blockstate/item and font
   provider/reference/glyph admission before retained/baked output. Focused pack
   and item-model tests cover wide documents, cycles, duplicate glyphs, exact
   boundary acceptance and official catalogs. Commit normally.
4. **Audio decode and residency.** Add bounded static/streaming decoders, exact
   generation cache identity, inactive LRU eviction, active pinning, reload
   clearing and observations. Focused audio/native/control tests require no
   device and cover truncated/oversized/unknown-duration input, frame bounds,
   shared allocation, eviction/rejection and stale generation. Commit and run
   the second full gate.
5. **Closeout.** Rerun official ignored resource coverage, record after logical
   charges and structural audio bounds, audit every bypass and retained
   generation, update architecture/unsupported/archive, then run final focused
   checks and full gate. Build `bbb-native` from final HEAD and run explicit
   live plus policy auto playable gates.

Each phase uses a normal commit and failures are fixed forward. The only safe
stops after implementation begins are clean full-gated tips after phases 2 and
4. A staged resource generation that cannot prove admission is dropped before
the renderer/audio transaction; an audio value that cannot prove its bound is
rejected before playback.
