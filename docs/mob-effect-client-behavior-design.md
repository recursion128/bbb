# Mob-effect client behavior closure

Status: active engineering contract.

## Goal and completion

Close the vanilla 26.1 client-visible path for
`ClientboundUpdateMobEffectPacket` and `ClientboundRemoveMobEffectPacket`.
Completion requires:

- packet updates remain fixed-registry holder ids, apply only to an existing
  living entity, preserve every flag, replace the matching effect, and remove
  only the requested effect;
- canonical effect instances tick duration and blend state like
  `MobEffectInstance.tickClient`, including infinite duration, zero-duration
  retention until the server removes the effect, update-time blend continuity,
  and the packet's new-instance `shouldBlend` behavior;
- the generated 40-entry effect catalog owns the category, color, and blend
  timing facts used by every consumer;
- the existing movement, mining, swing, heart, food, lightmap, and effect-state
  consumers remain exact, while active-effect HUD/inventory presentation,
  nausea projection, blindness/darkness fog, effect sky suppression, and
  night-vision fog brightening consume the same canonical instances;
- packet coverage, the feature ledger, focused tests, the full workspace gate,
  the applicable playable smoke, and the P0 audit all close at the final tip.

## Authority and non-goals

Authoritative sources are vanilla 26.1 `ClientboundUpdateMobEffectPacket`,
`ClientboundRemoveMobEffectPacket`, `ClientPacketListener`,
`MobEffectInstance`, `MobEffect`, `MobEffects`, `LivingEntity`, `Player`,
`LocalPlayer`, `Gui`, `EffectsInInventory`, `GameRenderer`, `Camera`,
`LevelRenderer`, `FogRenderer`, `BlindnessFogEnvironment`, and
`DarknessFogEnvironment`.

This slice does not run server-only healing, damage, hunger, saturation,
absorption initialization, attribute mutation, air-supply mutation, death or
hurt callbacks. It does not derive invisibility, glowing, or effect particles
from an effect packet: vanilla synchronizes those through entity metadata.
Fire-resistance lava fog remains outside this closure until the camera
environment can identify lava; restarting it requires a real lava camera-fog
owner rather than an effect-only approximation. Configurable FOV/effect scale
remains in its existing movement/camera queue; this slice implements the
vanilla default effect scale without an in-game settings surface.

## Ownership and frozen integration surface

- `bbb-protocol` and the Tree-sitter gameplay-id generator own fixed holder
  validation plus effect catalog facts. No consumer may add a parallel effect
  table.
- `bbb-world::entities::status` owns canonical effect instance replacement,
  removal, client ticking, blend interpolation queries, and vanilla ordering.
  Entity client ticks advance this owner once per normally running entity tick.
- `bbb-native` extracts the active camera entity's effect view once per frame,
  local-player HUD/screen projections, and any renderer-facing nausea state.
- `bbb-renderer` owns effect icon GPU resources, deterministic HUD geometry,
  nausea projection transforms, and the existing fog/sky submission inputs. It
  does not inspect protocol packets or own effect duration.
- Pack reload uploads all `mob_effect/<resource-path>` sprites transactionally
  with the existing HUD owner. Missing sprites use the established missing-HUD
  asset behavior and never a second asset cache.

The world-to-renderer path extends `RendererFrame`; no new scattered
`renderer.set_*` calls may be added outside its single commit function.

## Serial stages and verification

1. Extend the source-derived effect catalog and canonical instance lifecycle.
   Focused checks: gameplay-id extractor Python tests, generator `--check`,
   `bbb-protocol mob_effect`, and `bbb-world mob_effect`.
2. Add camera environment consumers: generic blend projection, nausea camera
   transform, blindness/darkness fog and sky suppression, and night-vision fog
   brightening. Focused checks: `bbb-world mob_effect`, `bbb-renderer camera`,
   and `bbb-native mob_effect`.
3. Add active-effect overlay and inventory effect-panel projection with
   generated category/color ordering, duration alpha, icon visibility, ambient
   backgrounds, localization, and deterministic renderer assertions. Focused
   checks: `bbb-pack mob_effect`, `bbb-renderer hud_mob_effect`, and
   `bbb-native hud_mob_effect`.
4. Update the packet matrix and TODO ledger, review the integrated diff, then
   run the full gate, playable smoke, and P0 audit.

Each stage is a buildable normal commit after focused tests, fmt, diff check,
docs lint, and review. Because stage 1 changes a generator and the later stages
cross canonical/render architecture, run a full gate after stage 1, at any
four-commit checkpoint, and at the final tip. A failed stage stops safely at
the preceding clean commit; failures are fixed forward without rewriting
history.
