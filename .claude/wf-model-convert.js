export const meta = {
  name: 'model-uv-convert',
  description: 'Parallel read-only conversion of 12 colored-only entity model files to carry textured UV; returns converted file text',
  phases: [{ title: 'Convert', detail: 'one read-only agent per entity: produce the converted model-file text' }],
}

// Model-struct entities (existing XxxModel::new() built via *_colored builders). Each agent reads the
// model file + the verified UV spec and RETURNS the fully-converted model-file text (and, if the
// cube-type change breaks the entity's test, the converted test text). It edits/builds nothing — the
// main loop writes the files, wires the shared registrations in one batch, and runs the gate.
const ENTITIES = [
  'creaking', 'sniffer', 'parrot', 'shulker', 'nautilus', 'panda',
  'axolotl', 'fox', 'rabbit', 'feline', 'equine', 'ender_dragon',
]

const SCHEMA = {
  type: 'object',
  required: ['entity', 'converted_model_file', 'converted_test_file', 'cubes_converted', 'builders_changed', 'issues'],
  additionalProperties: false,
  properties: {
    entity: { type: 'string' },
    converted_model_file: { type: 'string' },
    converted_test_file: { type: 'string' }, // full converted test text, or exactly "UNCHANGED"
    cubes_converted: { type: 'number' },
    builders_changed: { type: 'string' },
    issues: { type: 'string' },
  },
}

function buildPrompt(e) {
  return [
    `Produce the fully-converted text of one bbb entity model file, migrating \`${e}\` from colored-only to carry textured UV. This is READ-ONLY analysis — do NOT edit, write, or build any file. Return the converted file text as a string; the orchestrator writes it.`,
    ``,
    `## Inputs (read these)`,
    `- Model file to convert: /home/zgy/Work/bbb/crates/bbb-renderer/src/entity_models/model_layers/${e}.rs`,
    `- Its test file: /home/zgy/Work/bbb/crates/bbb-renderer/src/entity_models/tests/${e}.rs`,
    `- Verified vanilla UV spec: read /home/zgy/Work/bbb/.claude/texture-specs/specs.json and take the "${e}" entry — per part (in bbb cube order): each cube's bbb_min, bbb_size, tex_offs, uv_size, mirror.`,
    `- Reference conversion (a finished example): /home/zgy/Work/bbb/crates/bbb-renderer/src/entity_models/model_layers/tadpole.rs (already converted: ModelCubeDesc cubes + leaf_colored became ModelCube cubes + leaf). Also see leash_knot.rs / trident.rs for static cases.`,
    ``,
    `## The conversion (mechanical, geometry-preserving)`,
    `1. Each cube const built via \`cube([min], [size], COLOR)\` (type ModelCubeDesc) becomes \`ModelCube::new([min], [size], COLOR, [uv_size], [tex_offs], mirror)\` (type ModelCube). Match each existing bbb cube to its spec entry by [min]+[size] (same order). Preserve [min], [size], COLOR EXACTLY; only ADD uv_size, tex_offs, mirror from the spec. Change array element types \`[ModelCubeDesc; N]\` -> \`[ModelCube; N]\`.`,
    `2. In \`XxxModel::new()\`, convert colored builders to textured: \`ModelPart::leaf_colored(pose, &CUBES)\` -> \`ModelPart::leaf(pose, CUBES.to_vec())\`; \`ModelPart::colored_named(pose, &CUBES, kids)\` -> \`ModelPart::new(pose, CUBES.to_vec(), kids)\`; \`ModelPart::colored(pose, &CUBES, kids)\` -> \`ModelPart::new(pose, CUBES.to_vec(), kids)\` (keep child names/order identical). If it builds via \`StaticModel::new(&PARTS)\` / \`root_from_colored_descs\` / \`from_colored_desc\` over a \`[ModelPartDesc; N]\` desc tree, replace that with a hand-built \`ModelPart\` tree carrying the ModelCube cubes (a small \`XxxModel\` struct like tadpole's), mirroring the desc tree's poses, child names, and nesting exactly, with a no-op or unchanged \`setup_anim\`.`,
    `3. Imports: drop now-unused \`model_cube as cube\`, \`ModelCubeDesc\`, \`ModelPartDesc\`; add \`ModelCube\`. Keep everything still used (ModelPart, PartPose, PART_POSE_ZERO, color consts, helpers).`,
    `4. Do NOT change any [min]/[size]/COLOR/pose/rotation/setup_anim — geometry and animation are unchanged. You are ONLY adding per-cube UV (uv_size/tex_offs/mirror).`,
    ``,
    `## Variants / baby layers`,
    `UV layout is identical across an entity's texture variants (only the image differs) — there is ONE set of cube UVs. Convert EVERY cube const the file declares, including any BABY_* / variant consts. If the spec marks parts as the "baby" layer, use those tex_offs/uv_size for the baby cube consts.`,
    ``,
    `## Deformation subtlety`,
    `If a spec cube notes a CubeDeformation/inflate (bbb_size already inflated, e.g. *.001), keep bbb_min/bbb_size as the file already has them but use the spec's un-inflated uv_size verbatim (uv_size may differ from size — that is correct).`,
    ``,
    `## Output`,
    `- converted_model_file: the COMPLETE converted text of ${e}.rs (every line, ready to write verbatim). It must be valid Rust that compiles with 0 warnings: no unused imports, correct types. The dispatch still calls XxxModel::new() with &[] and the colored path is unchanged, so this file compiles standalone.`,
    `- converted_test_file: if /tests/${e}.rs still compiles unchanged after the cube type changes from ModelCubeDesc to ModelCube (ModelCube has the same .min/.size fields, so simple .min/.size asserts are fine), return exactly "UNCHANGED". If it references a now-removed symbol (e.g. a removed *_PARTS const, ModelPartDesc, or a count_cubes(&[ModelPartDesc]) helper), return the COMPLETE minimally-fixed test text (keep the same assertions, retargeted to the new consts/structs). Do NOT add new textured-path tests.`,
    `- cubes_converted (count), builders_changed (which builder calls changed), issues (any cube that didn't match the spec, unusual structure, deformation, or test you rewrote — else "none").`,
  ].join('\n')
}

phase('Convert')

const results = await parallel(
  ENTITIES.map((e) => () =>
    agent(buildPrompt(e), { label: `convert:${e}`, phase: 'Convert', schema: SCHEMA })
  )
)

const ok = results.filter(Boolean)
for (const r of ok) {
  log(`${r.entity}: cubes=${r.cubes_converted} test=${r.converted_test_file === 'UNCHANGED' ? 'unchanged' : 'rewritten'}${r.issues && r.issues !== 'none' ? ' ! ' + r.issues.slice(0, 90) : ''}`)
}
return { converted: ok.length, total: ENTITIES.length, results: ok }
