# Smithing Screen Presentation Engineering Contract

Status: active.

## Goal and completion

Close the active Vanilla 26.1 `SmithingScreen` presentation gap as one bounded
menu engineering slice:

- render the three empty input-slot backgrounds with Vanilla's retained
  30-tick cycling and four-tick cross-fade;
- show the missing-template, template-specific base/addition, and recipe-error
  onboarding tooltips with the correct hover precedence and 115-pixel wrapping;
- keep the existing armor/equipment/skull result preview behavior; and
- draw every ordinary or special non-armor result item on the armor stand's
  left hand, or every non-armor/non-skull head result through the armor stand's
  head item transform, inside the same isolated entity PIP target.

Completion requires native state/projection tests, resource and renderer
sanitization/order tests, resident item target/owner tests, deterministic PIP
geometry or pixel proof for both hand and head paths, removal of the active
ledger item, archival of this contract, a concise milestone index entry, and a
passing full gate at the final head.

## Authoritative behavior

- `SmithingScreen.containerTick` advances three `CyclingSlotBackground`
  instances. The template slot always cycles armor-trim and netherite-upgrade
  sprites. A recognized smithing template selects its own ordered base and
  addition icon lists; other or empty template stacks select empty lists.
- `CyclingSlotBackground.tick` resets `iconIndex` but not `tick` when its icon
  list changes. Non-empty lists advance every 30 ticks. Once at least one
  advance has occurred, the first four ticks of each period cross-fade the
  previous and current 16x16 sprites using
  `min((tick % 30 + partialTick) / 4, 1)`. Occupied slots suppress both layers.
- Armor-trim base icons are helmet, chestplate, leggings, and boots; material
  icons are ingot, redstone dust, lapis lazuli, quartz, diamond, emerald, and
  amethyst shard. Netherite-upgrade base icons are helmet, sword, chestplate,
  pickaxe, leggings, axe, boots, hoe, shovel, nautilus armor, and spear; its
  addition list contains ingot.
- `extractOnboardingTooltips` first selects the error tooltip over
  `(65,46,28,21)`, then allows a hovered slot rule to replace it. Empty
  template slot 0 shows `container.upgrade.missing_template_tooltip`. With a
  recognized template and an empty hovered slot, slot 1 shows that template's
  base description and slot 2 its addition description. The resulting
  Component is split to width 115 at the pointer position.
- `updateArmorStandPreview` clears all prior preview equipment. Humanoid armor
  occupies its equipment slot; HEAD equipment that is not humanoid armor is
  resolved with `ItemDisplayContext.HEAD`; other or unequippable items are
  resolved into the left hand with
  `ItemDisplayContext.THIRD_PERSON_LEFT_HAND`. Skull item models continue
  through `SkullBlockRenderer`; all other ordinary and special item-model leaves
  use the item resolver and retain foil.
- The armor stand is full-bright at body rotation 210 degrees and head pitch 25
  degrees. `GuiEntityRenderer` renders the complete state into the private
  40x60 color/depth PIP target at `(121,20)`, scale 25,
  `translate(0,1,0)`, and `rotationXYZ(25 degrees,0,PI)`.

Authoritative local sources are
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/screens/inventory/SmithingScreen.java`,
`net/minecraft/client/gui/screens/inventory/CyclingSlotBackground.java`,
`net/minecraft/world/item/SmithingTemplateItem.java`,
`net/minecraft/client/renderer/entity/ArmorStandRenderer.java`,
`net/minecraft/client/renderer/entity/layers/ItemInHandLayer.java`,
`net/minecraft/client/renderer/entity/layers/CustomHeadLayer.java`, and
`net/minecraft/client/gui/render/pip/GuiEntityRenderer.java`.

## Ownership and frozen interfaces

- `bbb-world` remains the canonical owner of the open container, effective item
  stacks, components, registry access, and menu error data. It gains no
  animation clock, pointer state, decoded image, or GPU handle.
- `bbb-native::runtime::smithing_screen` becomes the semantic owner of the
  Smithing screen-local cycling state and world-to-HUD projection. Its runtime
  key is the level generation plus `open_container_revision`; it advances at
  most once per published client tick and resets when a different screen
  instance opens.
- The native frame projection carries only typed slot-icon layers, tooltip
  lines, the existing `HudEntityPreview`, and item-model residency requests.
  Template classification uses exact generated item resource identity:
  `netherite_upgrade_smithing_template` versus the complete armor-trim
  smithing-template family. It does not infer arbitrary custom items by suffix.
- `bbb-renderer` gains a bounded `HudSmithingSlotIcon` enum and one fixed array
  of optional resident 16x16 sprite owners.
  `HudInventoryBackgroundTexture::SmithingSlotIcon` selects from that array;
  reload stages and restores the array atomically with the other HUD owners.
- The existing `HudEntityPreviewItemLayer` remains the render-plan descriptor.
  Native resolves its exact result stack through the normal item runtime,
  registry context, display transforms, special-item expansion, terrain/item/
  entity atlases, foil, full-bright light, and no-overlay values.
- `ItemModelDrawTarget::EntityPreview(u32)` identifies the matching sanitized
  preview/PIP target. It is frame-varying instance metadata, not part of
  `ItemModelAssetKey`; the same resident owner remains reusable in world, GUI,
  first-person, and PIP submissions.
- `entity_preview_pip_passes` creates camera+atlas/glint bindings against the
  preview's private camera buffer, filters resident draws by the exact preview
  index, and draws their solid/translucent/special/glint phases into the same
  private color/depth attachments after the armor-stand model. This result-item
  branch is mutually exclusive with armor/wings/skull equipment, so no missing
  layer interleave is hidden by the insertion point.
- Hand placement reuses `humanoid_hand_attach_transforms` for the armor stand's
  left arm. Head placement reuses `custom_head_item_transforms`. The existing
  item display-matrix and special-item leaf paths remain the sole transform and
  model-resolution owners.

## Bounds and fallback rules

- The icon catalog is fixed at the twenty Vanilla sprite identities above.
  Missing sprite data leaves only that icon layer absent; it cannot abort or
  partially replace the accepted HUD resource generation.
- Missing item runtime, invalid item id, absent item-model leaves, or missing
  resident atlas data omits only the result item. The armor stand and static
  Smithing screen remain visible.
- A template list change resets its selected icon to index zero while retaining
  the per-screen tick, exactly like `CyclingSlotBackground`. An occupied input
  slot hides icon layers without resetting their state.
- PIP targets remain bounded by the existing 512x512 sanitizer, resident item
  owner/byte caps, dynamic texture caps, and per-frame instance limits.
  Preview-target submissions may not carry screen scissor rectangles.
- Unknown/custom non-template items in slot 0 expose neither base/addition icon
  lists nor template-specific tooltips. They do not borrow a Vanilla template
  family by ordinal or name shape.

## Non-goals

- This slice does not change smithing recipes, property sets, slot acceptance,
  quick-move/click prediction, packet handling, or server authority.
- It does not add input routing, narration, in-game configuration, new
  standalone texture ownership, sampler/mipmap policy, or a general
  picture-in-picture scene graph.
- It does not alter ordinary inventory item icons, world held items,
  equipment/skull rendering, entity model buckets, or the unrelated Beacon
  presentation item.
- It does not close generic remaining mob-armor, GPU render-state split, or
  standalone sampler ledger umbrellas.

## Serial stages and verification

1. **Screen-local projection and sprite residency.** Add the semantic native
   owner, exact template/icon catalogs, 30/4 tick transition, occupied-slot
   suppression, resource upload/rollback, and wrapped onboarding/error tooltip
   precedence. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native smithing` and
   `scripts/cargo-dev.sh test -p bbb-renderer smithing`.
2. **Resident item PIP target.** Resolve hand/head ordinary and special leaves
   into `EntityPreview(index)`, reuse exact resident owners and model attach
   transforms, bind each atlas/glint source to the PIP camera, and draw the
   phases into the isolated target. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native smithing_preview`,
   `scripts/cargo-dev.sh test -p bbb-renderer entity_preview`, and
   `scripts/cargo-dev.sh test -p bbb-renderer resident_item_model`.
3. **Closeout.** Re-run both focused families, remove the ledger item, archive
   this contract, append the milestone index, run all standard per-commit
   checks, review the integrated diff, and run the full gate.

The pre-start full gate passed at
`aa2b79e0da1e934a190b92849c443018a9c780af`. The clean contract commit and the
buildable stage-1 commit are safe stops. Any focused or gate failure is fixed
forward before another slice; history is not rewritten.
