export const meta = {
  name: 'texture-uv-extract',
  description: 'Parallel read-only extraction of vanilla 26.1 texture-UV layouts for 17 colored-only bbb entity models',
  phases: [{ title: 'Extract', detail: 'one read-only agent per entity, schema-validated UV spec' }],
}

const ENTITIES = [
  { e: 'arrow', cls: 'ArrowModel' },
  { e: 'axolotl', cls: 'AxolotlModel' },
  { e: 'creaking', cls: 'CreakingModel' },
  { e: 'ender_dragon', cls: 'EnderDragonModel' },
  { e: 'equine', cls: 'HorseModel / AbstractEquineModel (horse, donkey, mule, skeleton/zombie horse) — note composite markings overlays in issues' },
  { e: 'evoker_fangs', cls: 'EvokerFangsModel' },
  { e: 'feline', cls: 'CatModel and OcelotModel' },
  { e: 'fox', cls: 'FoxModel (red + snow variants)' },
  { e: 'leash_knot', cls: 'LeashKnotModel' },
  { e: 'nautilus', cls: 'NautilusModel (createBodyLayer + createBabyBodyLayer)' },
  { e: 'panda', cls: 'PandaModel' },
  { e: 'parrot', cls: 'ParrotModel (5 color variants)' },
  { e: 'rabbit', cls: 'RabbitModel' },
  { e: 'shulker', cls: 'ShulkerModel (16 dye colors + default)' },
  { e: 'sniffer', cls: 'SnifferModel (adult + snifflet baby)' },
  { e: 'tadpole', cls: 'TadpoleModel' },
  { e: 'trident', cls: 'TridentModel' },
]

const CUBE_SCHEMA = {
  type: 'object',
  required: ['bbb_min', 'bbb_size', 'tex_offs', 'uv_size', 'mirror'],
  additionalProperties: false,
  properties: {
    bbb_min: { type: 'array', items: { type: 'number' }, minItems: 3, maxItems: 3 },
    bbb_size: { type: 'array', items: { type: 'number' }, minItems: 3, maxItems: 3 },
    tex_offs: { type: 'array', items: { type: 'number' }, minItems: 2, maxItems: 2 },
    uv_size: { type: 'array', items: { type: 'number' }, minItems: 3, maxItems: 3 },
    mirror: { type: 'boolean' },
    note: { type: 'string' },
  },
}

const SCHEMA = {
  type: 'object',
  required: ['entity', 'vanilla_model_class', 'atlas_size', 'textures', 'parts', 'confidence', 'issues'],
  additionalProperties: false,
  properties: {
    entity: { type: 'string' },
    vanilla_model_class: { type: 'string' },
    atlas_size: { type: 'array', items: { type: 'number' }, minItems: 2, maxItems: 2 },
    baby_atlas_size: { type: 'array', items: { type: 'number' }, minItems: 2, maxItems: 2 },
    textures: {
      type: 'array',
      items: {
        type: 'object',
        required: ['variant_key', 'path'],
        additionalProperties: false,
        properties: {
          variant_key: { type: 'string' },
          path: { type: 'string' },
          atlas_size: { type: 'array', items: { type: 'number' }, minItems: 2, maxItems: 2 },
        },
      },
    },
    parts: {
      type: 'array',
      items: {
        type: 'object',
        required: ['part_name', 'layer', 'cubes'],
        additionalProperties: false,
        properties: {
          part_name: { type: 'string' },
          layer: { type: 'string', enum: ['adult', 'baby'] },
          cubes: { type: 'array', items: CUBE_SCHEMA },
        },
      },
    },
    confidence: { type: 'string' },
    issues: { type: 'string' },
  },
}

function buildPrompt(e, cls) {
  return [
    `You are extracting the EXACT vanilla Minecraft Java 26.1 texture-UV mapping for the bbb entity model \`${e}\`, so it can be migrated from colored-only to the textured render path. This is READ-ONLY analysis — do NOT edit, write, or build anything. Return only the structured result.`,
    ``,
    `## Sources (read both, cross-reference)`,
    `1. bbb model file: /home/zgy/Work/bbb/crates/bbb-renderer/src/entity_models/model_layers/${e}.rs`,
    `   - Read the header comment (it cites the vanilla class + atlas size).`,
    `   - Enumerate the parts and, per part, the cubes IN FILE ORDER. Each cube (\`ModelCubeDesc\` / \`cube(...)\`) gives a [min x,y,z] and a [size x,y,z]. Record these verbatim — the application aligns by them.`,
    `   - Note baby/adult layers if both exist (e.g. BABY_* consts).`,
    `   - Find the matching bbb EntityModelKind variant(s) in /home/zgy/Work/bbb/crates/bbb-renderer/src/entity_models/catalog/selection.rs and catalog.rs (search for the PascalCase variant). The variant_key strings in your \`textures\` output should describe each variant the way bbb encodes it (e.g. "red"/"snow" for fox, color name for shulker, "adult"/"baby"). If there is only one texture, use variant_key "default".`,
    ``,
    `2. Vanilla source root: /home/zgy/Work/mc-code/sources/26.1/`,
    `   - Model class \`${cls}\`: usually net/minecraft/client/model/<Class>.java. Read createBodyLayer()/createBabyLayer()/createBabyBodyLayer(). For each box: \`texOffs(u, v)\`, \`addBox(ox, oy, oz, dx, dy, dz [, CubeDeformation])\`, and the current mirror() state.`,
    `   - Renderer: net/minecraft/client/renderer/entity/<X>Renderer.java (and any *RenderState / variant texture map) for the texture Identifier path string(s) and the variant→texture mapping.`,
    `   - atlas size = the (W, H) in \`LayerDefinition.create(mesh, W, H)\`.`,
    ``,
    `## What to produce`,
    `For EACH bbb cube (every part, in file order), find the matching vanilla addBox (match by offset == bbb [min] AND box dims == bbb [size]) and record:`,
    `- tex_offs: the vanilla [u, v] from texOffs for that box.`,
    `- uv_size: the [dx, dy, dz] passed to addBox — the UV-unwrap dimensions. Usually equals [size]. If a CubeDeformation/inflate is applied, the rendered size changes but uv_size stays the un-inflated dx,dy,dz — report the un-inflated dims and mention the deformation in \`note\`.`,
    `- mirror: the box's mirror flag (true if mirror() is active for that box).`,
    `- bbb_min, bbb_size: copy from the bbb cube so the application can align.`,
    `The target application format is \`ModelCube::new([min], [size], color, [uv_size], [tex_offs], mirror)\` — your fields map directly.`,
    ``,
    `Also fill: atlas_size [W,H] (adult), baby_atlas_size if a separate baby layer exists, and \`textures\` = every variant_key → vanilla texture path string (e.g. "textures/entity/fox/fox.png").`,
    ``,
    `## Rigor`,
    `- If a bbb cube has no clean vanilla match (offset/size mismatch, extra/missing cube), still emit the cube with your best tex_offs and DESCRIBE the discrepancy in \`issues\`. Do not silently skip cubes — the part cube count MUST equal the bbb file's cube count for that part.`,
    `- Set confidence high/medium/low with a one-line reason.`,
    `- Put composite/overlay-texture facts (e.g. horse base+markings, cat collar) and any deformation/scale subtleties in \`issues\` — the application needs to know.`,
  ].join('\n')
}

phase('Extract')

const results = await parallel(
  ENTITIES.map(({ e, cls }) => () =>
    agent(buildPrompt(e, cls), { label: `extract:${e}`, phase: 'Extract', schema: SCHEMA })
  )
)

const ok = results.filter(Boolean)
log(`extracted ${ok.length}/${ENTITIES.length} entities`)
return { count: ok.length, total: ENTITIES.length, specs: ok }
