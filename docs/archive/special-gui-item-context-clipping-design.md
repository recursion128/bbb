# Special GUI Item Context and Clipping Contract

## Goal and completion

Close the remaining GUI presentation gap for resident `minecraft:special` and
composite item-model leaves. Every GUI item consumer must resolve its ordinary
icon and retained special tree from the same complete `ItemModelResolver`
inputs, including bundle selection, use/cooldown/time/compass state, selected /
carried / view-entity conditions, registry keys, keybinds, fishing state, and
the caller's seed. A resident item drawn inside a floating-item or stats-list
scissor must be clipped to that exact physical-pixel rectangle without
rebuilding its static geometry.

Completion requires semantic tests proving that a context-sensitive special
leaf selects identically to the flat compatibility icon and renderer tests
proving that scissor identity participates in draw batching and reaches the
GUI render pass. Unscissored items, ordinary legacy clipped meshes, blur
strata, phase order, leaf transforms, foil, atlas ownership, and explicit
capacity failure behavior must remain unchanged.

## Non-goals

- Add new item-model property families or synthesize unavailable GUI context.
- Change non-GUI living/non-living item ownership.
- Replace the HUD/screen command graph, add a second GUI item atlas, or restore
  transient CPU geometry for resident special items.
- Generalize clipping beyond resident GUI item submissions.

## Ownership and integration surface

- `bbb-item-model` owns one public context-rich resolved-model entry point that
  delegates to the same internal selection function as the existing
  compatibility-icon entry point.
- `bbb-native` owns a compact GUI item context value and supplies the same
  value to both icon and block/special projection for each HUD consumer.
- `bbb-renderer` owns an optional physical-pixel scissor on narrow item
  instance requests/records/draws. Special expansion copies it to every
  generated part. Draw coalescing requires equal asset, target, phase, and
  scissor; GUI passes set the requested rectangle before the draw and restore
  the full attachment rectangle for an unscissored draw.

No canonical world state or renderer asset identity includes the scissor.
Clipping is per-frame placement state and therefore never invalidates resident
geometry.

## Vanilla 26.1 evidence

- `GuiGraphicsExtractor.item` calls
  `ItemModelResolver.updateForTopItem(output, stack, GUI, level, owner, seed)`
  once and stores the resulting `TrackingItemStackRenderState`; special leaves
  do not use a reduced selector.
- `ItemModelResolver.appendItemLayers` passes the same stack, display context,
  client level, owner, and seed through the selected model tree.
- `GuiItemRenderState` captures the current `ScreenRectangle`, intersects it
  with transformed normal or oversized item bounds, and exposes the same
  scissor to the later blit.
- `GuiRenderer.submitBlitFromItemAtlas` applies
  `GuiItemRenderState.scissorArea`; `OversizedItemRenderer` renders oversized
  geometry to its retained texture and the picture-in-picture blit applies the
  same GUI state bounds/scissor.

## Serial stages and validation

1. Add the context-rich resolved-model API and make every native GUI
   block/special projection consume the exact context already used for its
   flat icon. Focused `bbb-item-model` and `bbb-native` tests pin bundle and
   live-condition special selection. Run format/diff/docs checks and commit.
   This clean commit is the first safe stop.
2. Add optional physical scissor state to renderer item submissions, preserve
   it through special expansion and draw coalescing, and set it in the GUI
   resident pass. Focused HUD/residency/render tests pin clipped special
   submissions, distinct batching, reset behavior, and unchanged unscissored
   order. Delete the closed ledger todo, append the milestone index, run all
   affected focused tests and per-commit checks, commit, then run the complete
   workspace gate.

A stage failure is fixed forward within that owner. A scissor that is empty or
outside the current surface emits no resident submission; invalid rectangles
must never reach `wgpu::RenderPass::set_scissor_rect`. The previous fully gated
checkpoint remains `fa2c78e84` until the final gate passes.
