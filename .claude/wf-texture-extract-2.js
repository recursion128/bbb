export const meta = {
  name: 'texture-extract-2',
  description: 'Extract per-cube vanilla texOffs/uv_size/mirror for the 10 remaining colored-only entities',
  phases: [{ title: 'Extract', detail: 'one agent per entity, read-only transcription' }],
}

const VANILLA = '/home/zgy/Work/mc-code/sources/26.1'
const BBB = '/home/zgy/Work/bbb/crates/bbb-renderer/src/entity_models/model_layers'

// Each entity: the vanilla model class file, the bbb colored model file, the renderer texture, and a
// hint about deferred layers/variants. The agent transcribes texOffs per cube aligned to bbb order.
const ENTITIES = [
  { entity: 'guardian', vanilla: `${VANILLA}/net/minecraft/client/model/monster/guardian/GuardianModel.java`,
    bbb: `${BBB}/guardian.rs`, texture: 'textures/entity/guardian/guardian.png',
    hint: 'GuardianModel.createBodyLayer(). Elder variant (guardian_elder.png, GUARDIAN_ELDER_SCALE) deferred — extract the base only. The single moving eye + 12 spikes + 3 tail segments are part of the base mesh.' },
  { entity: 'frog', vanilla: `${VANILLA}/net/minecraft/client/model/animal/frog/FrogModel.java`,
    bbb: `${BBB}/frog.rs`, texture: 'textures/entity/frog/frog_temperate.png',
    hint: 'FrogModel.createBodyLayer(). The warm/cold colour variants are deferred — only the default temperate texture is wired. Note the croaking-body cube + tongue (pose-gated visibility).' },
  { entity: 'warden', vanilla: `${VANILLA}/net/minecraft/client/model/monster/warden/WardenModel.java`,
    bbb: `${BBB}/warden.rs`, texture: 'textures/entity/warden/warden.png',
    hint: 'WardenModel.createBodyLayer(). The emissive overlay layers (bioluminescent, pulsating spots, heart) are DEFERRED — extract ONLY the base body texOffs.' },
  { entity: 'armadillo', vanilla: `${VANILLA}/net/minecraft/client/model/animal/armadillo/AdultArmadilloModel.java`,
    vanilla2: `${VANILLA}/net/minecraft/client/model/animal/armadillo/BabyArmadilloModel.java`,
    bbb: `${BBB}/armadillo.rs`, texture: 'textures/entity/armadillo/armadillo.png', babyTexture: 'textures/entity/armadillo/armadillo_baby.png',
    hint: 'AdultArmadilloModel + BabyArmadilloModel createBodyLayer(). Two-model entity (adult/baby). Map BOTH adult and baby cube consts. The shell-ball cube + ear planes carry their own texOffs.' },
  { entity: 'wither', vanilla: `${VANILLA}/net/minecraft/client/model/monster/wither/WitherBossModel.java`,
    bbb: `${BBB}/wither.rs`, texture: 'textures/entity/wither/wither.png',
    hint: 'WitherBossModel.createBodyLayer(). The invulnerable variant (wither_invulnerable.png) and the dither/armor are deferred — extract the base only. 3 heads (center + 2 side), shoulders, ribcage, tail.' },
  { entity: 'arrow', vanilla: `${VANILLA}/net/minecraft/client/model/object/projectile/ArrowModel.java`,
    bbb: `${BBB}/arrow.rs`, texture: 'textures/entity/projectiles/arrow.png',
    hint: 'ArrowModel.createBodyLayer(). CRITICAL: arrows use a cross of 0-width planes. Vanilla scales the V texOffs by a factor (often texOffs(0,0).addBox with a 0.8 / specific v-scale). Report the EXACT texOffs and the box dims passed to addBox for the back-cube and each cross-plane, plus any texHeight/texWidth that differs from the PNG.' },
  { entity: 'llama_spit', vanilla: `${VANILLA}/net/minecraft/client/model/animal/llama/LlamaSpitModel.java`,
    bbb: `${BBB}/llama_spit.rs`, texture: 'textures/entity/llama/llama_spit.png',
    hint: 'LlamaSpitModel.createBodyLayer(). Small single-part spit ball (several cubes).' },
  { entity: 'wither_skull', vanilla: `${VANILLA}/net/minecraft/client/model/object/skull/SkullModel.java`,
    bbb: `${BBB}/wither_skull.rs`, texture: 'textures/entity/wither/wither.png',
    hint: 'Wither skull uses SkullModel (createWitherHeadModel / the skull layer). The charged (wither_invulnerable.png) variant is deferred. Extract the single head cube texOffs and the texWidth/texHeight the skull layer declares.' },
  { entity: 'shulker_bullet', vanilla: `${VANILLA}/net/minecraft/client/model/object/projectile/ShulkerBulletModel.java`,
    bbb: `${BBB}/shulker_bullet.rs`, texture: 'textures/entity/shulker/spark.png',
    hint: 'ShulkerBulletModel.createBodyLayer(). Note the texWidth/texHeight of the spark layer (often 64x32).' },
  { entity: 'wind_charge', vanilla: `${VANILLA}/net/minecraft/client/model/object/projectile/WindChargeModel.java`,
    bbb: `${BBB}/wind_charge.rs`, texture: 'textures/entity/projectiles/wind_charge.png',
    hint: 'WindChargeModel.createBodyLayer(). The wind_charge.png is ANIMATED (a vertical frame strip via .mcmeta). Report the layer texWidth/texHeight (one frame) AND whether the PNG is taller than texHeight — if so, flag in notes that this entity may need animated-texture handling and is NOT a plain static UV.' },
]

const SCHEMA = {
  type: 'object',
  required: ['entity', 'texture_path', 'tex_width', 'tex_height', 'parts', 'notes'],
  additionalProperties: false,
  properties: {
    entity: { type: 'string' },
    texture_path: { type: 'string' },
    tex_width: { type: 'number', description: 'the LayerDefinition MeshDefinition texWidth that texOffs are normalized against' },
    tex_height: { type: 'number', description: 'the LayerDefinition MeshDefinition texHeight' },
    png_differs_from_tex: { type: 'boolean', description: 'true if the actual PNG size differs from tex_width/tex_height (e.g. animated)' },
    baby_texture_path: { type: ['string', 'null'] },
    baby_tex_width: { type: ['number', 'null'] },
    baby_tex_height: { type: ['number', 'null'] },
    parts: {
      type: 'array',
      description: 'one entry per bbb ModelCubeDesc const, in file order; cubes in the same order as in that const',
      items: {
        type: 'object',
        required: ['const_name', 'cubes'],
        additionalProperties: false,
        properties: {
          const_name: { type: 'string' },
          shared_across_parts: { type: 'boolean', description: 'true if this const is reused by parts that vanilla gives DIFFERENT texOffs (the shared-cube-split problem) — explain in notes' },
          cubes: {
            type: 'array',
            items: {
              type: 'object',
              required: ['tex', 'uv_size', 'mirror'],
              additionalProperties: false,
              properties: {
                tex: { type: 'array', items: { type: 'number' }, minItems: 2, maxItems: 2, description: 'texOffs [u,v]' },
                uv_size: { type: 'array', items: { type: 'number' }, minItems: 3, maxItems: 3, description: 'the integer [dx,dy,dz] passed to addBox (the un-deflated UV box size)' },
                mirror: { type: 'boolean' },
                note: { type: 'string' },
              },
            },
          },
        },
      },
    },
    notes: { type: 'string', description: 'deferred layers/variants, shared-cube splits, animated-texture flags, any cube whose vanilla match was ambiguous' },
  },
}

phase('Extract')
const results = await parallel(ENTITIES.map((e) => () =>
  agent(
    `You are extracting vanilla Minecraft 26.1 texture-UV data for the bbb native client's "${e.entity}" entity model, so its colored-only ModelCubeDesc cubes can be converted to UV-bearing ModelCube cubes.

Read the vanilla model class:
  ${e.vanilla}
${e.vanilla2 ? `  ${e.vanilla2}\n` : ''}Read the bbb colored model file:
  ${e.bbb}

Renderer texture: ${e.texture}
${e.babyTexture ? `Baby texture: ${e.babyTexture}\n` : ''}Context/hints: ${e.hint}

Your job: for EVERY cube in EVERY \`[ModelCubeDesc; N]\` const in the bbb file (in file order, cubes in array order), determine the matching vanilla \`addBox\`/\`texOffs\` call and report:
  - tex: the texOffs [u, v] (integers, in texels)
  - uv_size: the [dx, dy, dz] integer dimensions passed to addBox (the UN-deflated box size — if vanilla applies a CubeDeformation, the dx,dy,dz are still the integer pre-deformation dims; the bbb min/size already encode the deformation, so uv_size must stay the integer addBox dims)
  - mirror: whether the cube is added with mirror() set

How to match: vanilla builds each PartDefinition with CubeListBuilder.create().texOffs(u,v).addBox(...). The bbb cubes are in the SAME order as vanilla's addBox order within a part, and the bbb const names mirror the vanilla part names. Align them by part name + min/size. The bbb cube's min/size should equal vanilla's box origin/dimensions (possibly with CubeDeformation applied), which confirms the match.

CRITICAL checks:
  - If a single bbb const is reused (e.g. via .to_vec() in multiple ModelPart::leaf calls) by parts that vanilla gives DIFFERENT texOffs (e.g. left vs right legs with distinct texOffs, NOT mirrors), set shared_across_parts=true on that part and explain in notes which parts need splitting and their distinct texOffs. (If left/right genuinely share one texOffs via mirror, that's fine — shared_across_parts=false.)
  - Report tex_width/tex_height from the LayerDefinition MeshDefinition (e.g. CubeDeformation ... new LayerDefinition(mesh, texWidth, texHeight) or MeshDefinition with texWidth/texHeight). These are what texOffs normalize against.
  - If the actual PNG dimensions differ from tex_width/tex_height (animated strip), set png_differs_from_tex=true and flag it in notes.

Return ONLY the structured object. Do not edit any files. Be exact — transcribe integer texOffs verbatim from the source.`,
    { label: `extract:${e.entity}`, phase: 'Extract', schema: SCHEMA, effort: 'high' }
  )
))

return results.filter(Boolean)
