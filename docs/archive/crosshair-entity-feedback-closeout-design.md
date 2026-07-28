# Crosshair Entity Feedback Closeout

## Goal And Completion

Close the remaining crosshair/entity interaction presentation gap by replacing
the non-Vanilla entity target AABB with Vanilla 26.1's default crosshair attack
indicator.

Completion requires:

- ordinary crosshair picking to keep one target identity for entity name
  display and HUD feedback;
- `bbb-world` to own the canonical local attack-strength and living-target
  eligibility projection;
- `bbb-native` to combine that projection with the entity already selected by
  the canonical crosshair ray and suppress it outside non-spectator first
  person;
- `bbb-renderer` to own the exact full/progress sprite layout and draw order;
- selected-pack GUI atlas sprites to load and roll back with the other HUD
  resources;
- no runtime or renderer path to construct or draw an entity target AABB; and
- focused semantic tests to cover the world eligibility, native target
  selection, renderer layout, resource ownership, and removal guard.

## Authoritative Vanilla Facts

- `Minecraft.pick` stores `player.raycastHitResult(...)` and assigns
  `crosshairPickEntity` only for an entity hit
  (`Minecraft.java:2919-2927`).
- `LevelRenderer.extractLevel` passes that entity to
  `entityRenderDispatcher.prepare` before extracting visible entity models; it
  does not draw a target AABB (`LevelRenderer.java:552-565`).
- `Options.attackIndicator` defaults to `AttackIndicatorStatus.CROSSHAIR`
  (`Options.java:428-435`).
- `Gui.extractCrosshair` draws the ordinary 15-by-15 crosshair, then:
  - draws the 16-by-16 full indicator at
    `(guiWidth / 2 - 8, guiHeight / 2 + 9)` when attack strength is full, the
    target is a living and alive entity, the current attack delay is greater
    than five ticks, and any active-item `ATTACK_RANGE` accepts the hit;
  - otherwise, while strength is below one, draws the 16-by-4 background and
    the leading `(int)(strength * 17)` pixels of the progress sprite at the
    same origin; and
  - suppresses both the ordinary crosshair and indicator when the 3D debug
    crosshair replaces them (`Gui.java:420-449`).
- `Player.getCurrentItemAttackStrengthDelay` is
  `1 / ATTACK_SPEED * 20`, and `getAttackStrengthScale(0)` clamps the canonical
  attack ticker divided by that delay to `[0, 1]`
  (`Player.java:1793-1809`).

The native crosshair ray applies `LivingEntity.getActiveItem()` semantics
(empty for spectator, the current use hand while using, otherwise selected
main hand) before its `ATTACK_RANGE`, including creative reach, hitbox margin,
and movement extension. A selected entity hit therefore satisfies Vanilla's
range condition; HUD extraction must not perform a second, divergent range
test.

## Frozen Ownership And API Contract

- `bbb-world::LocalPlayerCrosshairAttackIndicatorState` contains only
  `strength_scale` and `full`.
- `WorldStore::local_player_crosshair_attack_indicator_state(target_id)`
  computes the current delay/strength from canonical player attributes and
  tick state. `full` additionally requires a present living target with
  positive synchronized health and a delay strictly greater than five.
- `bbb-native` reuses the single `targeted_entity_id` selected for the current
  frame. It maps the world projection to renderer presentation state only for
  non-spectator first person and puts that state in `RendererFrame`.
- `bbb-renderer::HudCrosshairAttackIndicator` is the presentation boundary:
  `None`, `Full`, or `Progress { strength_scale }`. Renderer layout owns the
  Vanilla pixel formula and clips the progress sprite UVs to the submitted
  width.
- Renderer resource ownership adds exactly the three selected-pack sprites
  `hud/crosshair_attack_indicator_full`,
  `hud/crosshair_attack_indicator_background`, and
  `hud/crosshair_attack_indicator_progress`.

## Non-goals

- Do not add an in-game options UI. The repo currently implements Vanilla's
  default `CROSSHAIR` mode; a future command-line option may expose `OFF` or
  `HOTBAR`.
- Do not expand this slice into spectator `MenuProvider` crosshair policy or a
  camera-mode visibility rewrite.
- Do not add a second entity raycast, attack ticker, health cache, or
  item-range implementation.
- Do not change generic entity model rendering, targeted name display,
  F3+B hitboxes, or debug entity scene outlines.
- No current native projection consumes Vanilla's historical `yRotA` field.
  The repository-wide audit found no source to validate; any future source
  must be justified at its semantic animation owner rather than introduced by
  crosshair feedback.

## Ordered Stages And Verification

1. Add the world semantic projection and focused combat tests for weak/full
   strength, slow/fast weapons, living/non-living targets, and synchronized
   death.
2. Add renderer presentation state, three HUD resources, exact layout/UV
   clipping, debug-crosshair suppression, and reload-owner coverage.
3. Reuse the native frame target identity, wire the immutable renderer frame,
   and remove the entity target outline path and its tests end to end.
4. Add structural guards against restoring the target AABB, run focused world,
   renderer, and native tests, remove the closed unsupported-feature entry,
   update the milestone index, and run the per-commit checks.
5. This is a cross-crate engineering checkpoint. Commit one reviewed change
   and run the full gate; fix failures forward before selecting the next slice.
