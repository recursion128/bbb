# Resource Image Input Safety

> Canonical scope：资源可信链与 fail-closed 规则见
> [`docs/wiki/mechanisms/resources/offline-asset-trust.md`](wiki/mechanisms/resources/offline-asset-trust.md)。
> 本页只保留不可信输入边界的详细 evidence。

This document freezes the bounded PNG boundary for selected client resource
packs and downloaded profile textures. It refines the ownership and atomic
generation rules in
[`resource-pack-lifecycle-design.md`](resource-pack-lifecycle-design.md). The
goal is to reject attacker-controlled declared image sizes before allocating
their pixel payload, without creating a global allocator, a second resource
owner, or a stale-on-error presentation path.

## Ownership And Failure Contract

- `bbb-pack::image` is the only production file-backed PNG decoder for atlas
  sprites, paletted/unstitch sources, colormaps and `ClientAsset.ResourceTexture`.
  Callers receive dimensions or RGBA only after the same file, dimension,
  pixel and decoder-allocation checks.
- One atlas load owns one mutable logical RGBA budget. Directory, single,
  paletted-permutation and unstitch sources all charge that budget before
  retaining or generating another image. Reading source dimensions charges the
  same budget so a later atlas allocation cannot bypass it.
- `bbb-render-types::player_skin` owns memory-backed profile PNG validation and
  Vanilla's 64x32/64x64 legacy-skin conversion. `bbb-item-model::skin_runtime`
  owns HTTP and disk-cache byte transfer. The semantic skin/cape/elytra caller
  passes its exact byte limit into the fetcher.
- HTTP bodies are read through `Read::take(max + 1)` even when `Content-Length`
  is absent or false. Cache files are checked through the opened file's
  metadata before a bounded read. A fetched image is written to disk only after
  successful decode, so malformed input does not become a persistent bad-cache
  hit.
- A selected-pack decode failure aborts preparation and keeps the prior atomic
  resource generation. A downloaded-texture failure remains an explicit failed
  cache state and uses the existing fallback; it never publishes partial RGBA
  or a stale handle.

No reader limit relies on `image::Limits::max_alloc` alone. BBB first reads the
fixed PNG signature/IHDR, checks the dimensions and logical RGBA charge, then
constructs the decoder with strict width/height and bounded decoder allocation.
The decoder's actual dimensions and native decoded byte count are checked again
before `DynamicImage` allocates its output.

## Limits

| Input owner | Compressed bytes | Per axis | Pixels | Decoder bytes | Aggregate retained charge |
| --- | ---: | ---: | ---: | ---: | ---: |
| pack-backed PNG | 64 MiB | 8,192 | 16,777,216 | 128 MiB | 256 MiB per atlas, 512 MiB per resource generation, 1 GiB accepted plus staged |
| player skin | 1 MiB | exact 64x32 or 64x64 | 4,096 | 128 MiB hard decoder ceiling; dimensions make the real bound much smaller | one request |
| cape/elytra profile texture | 16 MiB | 8,192 | 16,777,216 | 128 MiB | one result, 256 MiB ready dynamic owner |

The dimension and pixel limits are separate: a profile texture may be long and
narrow up to 8,192 on one axis, but cannot use an 8,192x8,192 pixel payload.
All arithmetic uses checked `u64`/`usize` conversion before capacity or output
allocation.

An audit of the local official Java 26.1 client resources found 3,665 PNGs. The
largest pixel payloads are 1024x256 and 512x512; the largest compressed file is
124,488 bytes. These measurements validate that the security ceilings leave a
large resource-pack scaling margin; they are evidence, not a claim that Vanilla
itself defines these BBB-specific safety constants.

## Vanilla 26.1 Evidence

- `SpriteResourceLoader` and `TextureContents` decode pack resources through
  `NativeImage.read`; atlas sources select resources before sprite loading.
- `SkinTextureDownloader.downloadSkin` loads either its disk cache or the HTTP
  body, and applies `processLegacySkin` only for `Type.SKIN`.
- `processLegacySkin` accepts only 64x32 and 64x64 skins and performs the opaque
  base/legacy overlay repair preserved by BBB.
- Cape and elytra downloads use the normal image path without legacy-skin
  conversion, so BBB retains their decoded dimensions subject to the explicit
  hostile-input ceiling.

## Regression Evidence And Remaining Risk

Focused tests use sparse oversized files, forged IHDR dimensions, aggregate
atlas charges, missing/lying body lengths, oversized cache files and failed
decode-before-cache-write. Source guards reject reintroduced direct decoders in
the migrated consumers. Local ignored coverage loads every official 26.1 atlas
source under the new budget.

The generation-wide and dynamic-owner closure is specified, implemented and
measured in
[`resource-image-memory-budget-design.md`](resource-image-memory-budget-design.md).
Every production pack decoder shares a 512 MiB generation charge, adjacent
accepted/staged generations share a 1 GiB ceiling, and the remote dynamic owner
retains at most 256 MiB of ready decoded pixels with bounded worker/delivery
handoff. These are logical decoded-pixel bounds, not whole-process RSS: decoder
scratch, derived renderer/GPU owners and driver-deferred release remain outside
the counters. JSON/model/font collections and audio decode budgets are closed
by [`resource-data-audio-budget-design.md`](resource-data-audio-budget-design.md)
and remain maintenance invariants rather than a second open queue.
