# Selected-pack post-effect graph closure

Status: completed on 2026-07-27.

Pre-start fully gated baseline:
`979459ff42b21aaadb20c6a9392cc67d639de307`.

## Goal and completion contract

Replace the fixed-resource half of the native post-processing path with one
selected-pack-owned graph generation while retaining repo-native wgpu
execution. Completion requires:

- `bbb-pack` loads the effective selected-pack `post_effect/*.json` catalog,
  referenced `shaders/core/*.vsh`, `shaders/post/*.{vsh,fsh}`, recursive
  `shaders/include/*.glsl` imports, and graph texture inputs through the
  existing precedence/filter and shared data/image budgets;
- graph decoding follows Vanilla 26.1 `PostChainConfig.CODEC`, including
  internal target dimensions, persistence and ARGB clear color, ordered passes,
  target/texture inputs, depth selection, per-input filtering, all seven
  `UniformValue` shapes, and duplicate-sampler rejection;
- the renderer compiles the selected GLSL sources into wgpu pipelines instead
  of selecting a hard-coded shader by resource id; Mojang sampler names,
  uniform-block names, `SamplerInfo`, `Globals`, stage varyings, and
  `#moj_import` results are translated to explicit WebGPU bindings without
  changing shader expressions;
- the generic executor preserves pass order, target replacement, internal
  fixed/full-size targets, persistent-target resize behavior, ARGB clears,
  input dimensions, nearest/linear clamp sampling, and full-screen triangle
  execution;
- selected generations drive every currently applicable Vanilla post-chain
  owner: `blur`, `creeper`, `spider`, `invert`, `entity_outline`, and
  `transparency`; camera selection and F4 state remain simulation-owned and no
  second graph selector is introduced;
- the existing fabulous-style main/translucent/item-entity/particles/weather/
  clouds targets supply the transparency graph. Depth inputs are exposed as
  exact sampleable scalar surfaces; unsupported filtering capability fails the
  staged generation rather than silently changing precision;
- startup may retain the checked-in Vanilla-equivalent fallback when pack roots
  are unavailable. Once a selected catalog is accepted, missing, malformed, or
  uncompilable overrides fail the same startup/reload boundary as the other
  selected resources and never mix old shaders with new graph data;
- resource push/apply/pop swaps the whole compiled post-effect generation
  inside `Renderer::transactional_resource_reload`; failure restores the exact
  previous graph generation and render counters;
- deterministic pack fixtures prove graph/shader/include/texture precedence,
  codec boundaries, malformed and compile failure, generation rollback, resize,
  custom shader pixels, pass counts, and selected-pack restoration; and
- the unsupported-feature item, contract archive, milestone index, final full
  gate, and P0 audit close at the final tip.

## Authority and non-goals

The authoritative sources are local Vanilla 26.1
`ShaderManager.prepare`, `ShaderManager.CompilationCache`,
`PostChainConfig`, `PostChain.load`, `PostPass.addToFrame`,
`RenderPipelines.POST_PROCESSING_SNIPPET`, `GlslPreprocessor`,
`UniformValue`, `GameRenderer.processBlurEffect`,
`GameRenderer.checkEntityPostEffect`,
`LevelRenderer.addMainPass`, and the six official
`assets/minecraft/post_effect/*.json` graphs plus their referenced shaders.

This slice does not add an in-game graphics or shader configuration UI, a
resource-pack editor, arbitrary compute pipelines, a second render graph, an
integrated server, or OptiFine/Iris shader-pack formats. It does not change
camera effect selection, render-type ordering, canonical world state, or the
selected resource-pack stack. General core render-pipeline overrides outside a
referenced post-chain shader remain separate work.

## Frozen Vanilla behavior

- `ShaderManager.prepare` lists effective shader and post-chain resources.
  Every shader is preprocessed with per-source import de-duplication:
  quoted imports resolve relative to the source directory and angle imports
  resolve below `shaders/include/`; selected-pack precedence and filters apply
  to both source and include.
- `PostChainConfig` defaults `targets` and `passes` to empty. Internal target
  width/height are positive when present, `persistent` defaults false, and
  `clear_color` defaults transparent black. A pass requires vertex shader,
  fragment shader, and output; inputs and uniform blocks default empty.
- An input is exactly one of:
  a texture input with sampler name, location, positive declared width/height
  and optional bilinear flag; or a target input with sampler name, target id,
  optional depth selection and optional bilinear flag. Sampler names are unique
  within a pass.
- A texture input resolves
  `textures/effect/<location-path>.png`. Target references not declared as
  internal are external and must be allowed by the execution context.
- `PostChain.load` builds passes in JSON order. Each pass declares
  `<sampler_name>Sampler`, `SamplerInfo`, and the configured uniform blocks.
  Each execution writes `SamplerInfo` as output size followed by every input
  size, binds the configured static uniform values in std140 order, then draws
  one three-vertex full-screen triangle.
- Internal targets without explicit dimensions inherit the current screen
  dimensions. Non-persistent targets are cleared from their configured ARGB
  value for each graph execution; persistent targets retain their content
  between executions and are recreated/cleared only when their descriptor
  changes.
- `blur` executes its six alternating horizontal/vertical passes and reads
  `Globals.MenuBlurRadius` when JSON radius is below `0.5`.
  `creeper`, `spider`, and `invert` retain their current camera selection and
  post-HUD ordering contract. `entity_outline` processes the outline target
  before its existing additive main composite. `transparency` combines main
  plus five premultiplied color/depth layers into `final`, then blits to main.
- Shader/reload failure cannot expose a partially prepared cache. Vanilla
  triggers resource recovery; bbb's equivalent is to reject the staged
  selected generation and retain the prior immutable generation.

## Bounded resource model

Post effects are untrusted selected-pack data and share the existing aggregate
logical-byte and decoded-image generation budgets. In addition, the catalog
enforces explicit semantic ceilings before allocation:

- at most 64 graphs, 128 passes per graph, 64 internal targets per graph,
  32 inputs per pass, 32 uniform blocks per pass, and 128 values per block;
- at most 512 distinct referenced shader/include sources, import depth 32, and
  one visit per imported id within a root shader;
- positive target/texture dimensions bounded at 8192 and checked pixel/byte
  arithmetic;
- no repeated sampler name within one pass; and
- no path traversal, non-canonical resource id, undeclared external target,
  missing source/input texture, unsupported shader declaration, or
  source/config binding mismatch.

Bounds use checked counters and the shared budgets; declared counts or
dimensions never become unchecked `Vec`/texture allocation capacities.

## Frozen ownership and integration surface

### Pack preparation

- New `bbb_pack::PostEffectCatalog` is an immutable CPU generation containing
  canonical graph configs, preprocessed vertex/fragment source text, texture
  input pixels, resource ids, pack ids, and the selected resource generation.
- `PostEffectCatalog::load(&PackRoots)` is the only filesystem/resource-stack
  entry point. Renderer and native code do not open shader, graph, include, or
  texture paths directly.
- `PreparedNativeResourceReload` prepares the catalog beside particle/item/
  audio resources. Its data and images therefore participate in the same
  generation admission and failure reporting.

### Renderer compilation and execution

- A new semantic `post_effect_graph` renderer module owns GLSL binding
  adaptation, pipeline validation, std140 buffers, samplers, input textures,
  internal targets, pass bind groups, and graph execution.
- The adapter only supplies declarations that Vanilla's post pipeline supplies:
  graph samplers, `SamplerInfo`, configured blocks, and `Globals`. Unknown
  resource declarations or mismatched block/value shapes are compile errors,
  not guessed bindings.
- `PostEffectGraphGeneration` is installed through one
  `Renderer::reload_post_effect_graphs(&PostEffectCatalog) -> Result<()>`.
  It exposes graph lookup/execution internally by canonical id and returns
  executed pass counts for the existing counters.
- Each execution context supplies an explicit allow-list and typed external
  target views. Camera and blur expose only `minecraft:main`; outline exposes
  `minecraft:entity_outline`; transparency exposes its existing six layer
  targets plus `minecraft:main`. No string lookup reaches unrelated renderer
  fields.
- Existing hard-coded WGSL owners remain a startup fallback only when no pack
  roots/catalog can be prepared. They are not selectable after a catalog
  generation commits, and tests prevent resource ids from routing back to
  fixed shaders.
- Resize recreates size-dependent internal and depth-sampling targets/bind
  groups for the accepted generation while keeping pipelines, static textures,
  uniform values, graph ids, camera active state, and generation identity.

### Atomic reload

- Startup attempts one selected catalog install after renderer creation and
  before the first frame. Failure is logged with resource context and retains
  the embedded Vanilla-equivalent fallback.
- Server resource reload compiles and installs the new graph generation inside
  the existing render-thread transaction after CPU preparation. The renderer
  registers the previous generation as a lazy restore owner immediately after
  replacement; any later upload failure restores it.
- `active_roots`, simulation acknowledgement, and diagnostics advance only
  after the transaction succeeds. Failed graph parsing is a `prepare` failure;
  shader/pipeline/GPU resource validation is a `commit` failure.

## Serial stages and verification

1. **Pack catalog and preprocessing.** Add bounded typed graph/uniform/input
   decoding, effective source/include/texture resolution, immutable generation
   data, and precedence/filter/boundary tests.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-pack post_effect`.
2. **Generic wgpu graph generation.** Add explicit GLSL-to-WebGPU binding
   adaptation, pipeline error capture, std140 packing, targets/textures,
   resize, execution, and direct custom-pixel tests without changing the four
   live post-chain owners.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-renderer post_effect_graph`.
   This is the first high-risk milestone and ends with a full gate.
3. **Live owner and reload integration.** Route blur, camera, outline, and
   transparency through the accepted graph generation; add exact external
   target/depth conversion, pass/order/counter/offscreen tests, startup load,
   transactional push/apply/pop, failure rollback, and diagnostics tests.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-renderer post_effect`,
   `scripts/cargo-dev.sh test -p bbb-native resource_reload`, and
   `scripts/cargo-dev.sh test -p bbb-native camera_post_effect`.
4. **Closeout.** Remove the completed ledger item, archive this contract,
   append the engineering milestone, review the integrated diff, then run the
   final full gate and P0 audit.

Every stage is a buildable normal commit after focused tests,
`cargo fmt --all --check`, `git diff --check`, documentation lint when
applicable, and full diff review. Failures are fixed forward from the preceding
clean stage; history is never rewritten.

## Hardest-ready decision

This is the hardest bounded ready candidate at the fully gated profiler tip.
It crosses untrusted selected resources, recursive preprocessing, seven
uniform layouts, runtime shader validation, GPU target lifetime, four ordered
render owners, resize, and all-or-nothing server-pack generations. Vanilla
fixes the codecs, resource paths, graph semantics, shader inputs, and live
insertion points, so the integration contract is freezeable.

The remaining full survival physics, all-provider particle parity, and broader
dynamic-texture items are open-ended families rather than one ready semantic
contract. Equal-load Java/native performance capture still depends on a
reproducible live comparison scene. The remaining packet partials are either
explicitly deferred presentation surfaces or narrower authoritative-correction
work, and DebugOptions polish has materially lower architecture and failure
risk.
