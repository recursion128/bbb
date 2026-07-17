from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from entity_renderer_catalog_extractor import (  # noqa: E402
    EntityRendererCatalogExtractionError,
    extract_entity_renderer_registrations,
    extract_living_entity_upside_down_names,
)
from vanilla_java_source import parse_source  # noqa: E402


def java(path: str, source: str):
    return parse_source(path, source.encode())


class EntityRendererCatalogExtractorTests(unittest.TestCase):
    def test_extracts_living_entity_upside_down_name_membership(self) -> None:
        source = java(
            "net/minecraft/client/renderer/entity/LivingEntityRenderer.java",
            """
            package net.minecraft.client.renderer.entity;
            public class LivingEntityRenderer {
                protected static boolean isUpsideDownName(final String name) {
                    return "Dinnerbone".equals(name) || "Grumm".equals(name);
                }
            }
            """,
        )

        self.assertEqual(
            extract_living_entity_upside_down_names(source),
            ("Dinnerbone", "Grumm"),
        )

    def test_rejects_non_literal_upside_down_name_membership(self) -> None:
        source = java(
            "net/minecraft/client/renderer/entity/LivingEntityRenderer.java",
            """
            package net.minecraft.client.renderer.entity;
            public class LivingEntityRenderer {
                protected static boolean isUpsideDownName(final String name) {
                    return name.equals("Dinnerbone") || "Grumm".equals(name);
                }
            }
            """,
        )

        with self.assertRaisesRegex(
            EntityRendererCatalogExtractionError,
            r"literal\.equals\(name\)",
        ):
            extract_living_entity_upside_down_names(source)

    def test_extracts_literal_and_constructor_driven_full_brightness(self) -> None:
        entity_renderers = java(
            "net/minecraft/client/renderer/entity/EntityRenderers.java",
            """
            package net.minecraft.client.renderer.entity;
            import net.minecraft.world.entity.EntityType;
            public class EntityRenderers {
                static {
                    register(EntityType.BLAZE, BlazeRenderer::new);
                    register(EntityType.EYE_OF_ENDER,
                        context -> new ThrownItemRenderer<>(context, 1.0F, true));
                    register(EntityType.EGG, ThrownItemRenderer::new);
                    register(EntityType.FIREBALL,
                        context -> new ThrownItemRenderer<>(context, 3.0F, true));
                    register(EntityType.CAT, CatRenderer::new);
                    register(EntityType.HORSE, HorseRenderer::new);
                    register(EntityType.ALLAY, AllayRenderer::new);
                    register(EntityType.ARMOR_STAND, ArmorStandRenderer::new);
                    register(EntityType.VEX, VexRenderer::new);
                    register(EntityType.VILLAGER, VillagerRenderer::new);
                    register(EntityType.WANDERING_TRADER, WanderingTraderRenderer::new);
                    register(EntityType.EVOKER, EvokerRenderer::new);
                    register(EntityType.ILLUSIONER, IllusionerRenderer::new);
                    register(EntityType.ZOMBIE, ZombieRenderer::new);
                    register(EntityType.HUSK, HuskRenderer::new);
                    register(EntityType.DROWNED, DrownedRenderer::new);
                    register(EntityType.ZOMBIE_VILLAGER, ZombieVillagerRenderer::new);
                    register(EntityType.GLOW_ITEM_FRAME, ItemFrameRenderer::new);
                    register(EntityType.ITEM_FRAME, ItemFrameRenderer::new);
                    register(EntityType.SHEEP, SheepRenderer::new);
                    register(EntityType.PARROT, ParrotRenderer::new);
                    register(EntityType.WOLF, WolfRenderer::new);
                }
                private static void register(Object type, Object renderer) {}
            }
            """,
        )
        renderer_sources = [
            java(
                "net/minecraft/client/renderer/entity/layers/ItemInHandLayer.java",
                """
                package net.minecraft.client.renderer.entity.layers;
                public class ItemInHandLayer {
                    public ItemInHandLayer(Object renderer) {}
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/MobRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class MobRenderer extends EntityRenderer {}
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/AllayRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                import net.minecraft.client.renderer.entity.layers.ItemInHandLayer;
                public class AllayRenderer extends MobRenderer {
                    public AllayRenderer() {
                        this.addLayer(new ItemInHandLayer(this));
                    }
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.isAggressive = entity.isAggressive();
                    }
                    private void addLayer(Object layer) {}
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/ArmorStandRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                import net.minecraft.client.renderer.entity.layers.ItemInHandLayer;
                public class ArmorStandRenderer extends EntityRenderer {
                    public ArmorStandRenderer() {
                        this.addLayer(new ItemInHandLayer(this));
                    }
                    private void addLayer(Object layer) {}
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/VexRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class VexRenderer extends AllayRenderer {}
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/VillagerRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class VillagerRenderer extends MobRenderer {
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.isUnhappy = entity.getUnhappyCounter() > 0;
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/WanderingTraderRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class WanderingTraderRenderer extends MobRenderer {
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.isUnhappy = entity.getUnhappyCounter() > 0;
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/EvokerRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class EvokerRenderer extends MobRenderer {
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.isCastingSpell = entity.isCastingSpell();
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/IllusionerRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class IllusionerRenderer extends MobRenderer {
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.isCastingSpell = entity.isCastingSpell();
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/ZombieRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class ZombieRenderer extends MobRenderer {
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.isConverting = entity.isUnderWaterConverting();
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/HuskRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class HuskRenderer extends ZombieRenderer {}
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/DrownedRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class DrownedRenderer extends ZombieRenderer {}
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/ZombieVillagerRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class ZombieVillagerRenderer extends MobRenderer {
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.isConverting = entity.isConverting();
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/AbstractHorseRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public abstract class AbstractHorseRenderer extends AgeableMobRenderer {}
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/HorseRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class HorseRenderer extends AbstractHorseRenderer {}
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/CatRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                import net.minecraft.client.model.animal.feline.AbstractFelineModel;
                public class CatRenderer extends AgeableMobRenderer<
                    Object, Object, AbstractFelineModel<Object>
                > {
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.isSitting = entity.isInSittingPose();
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/EntityRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class EntityRenderer {
                    protected int getBlockLightLevel(Object entity, Object pos) {
                        return 0;
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/BlazeRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class BlazeRenderer extends EntityRenderer {
                    protected int getBlockLightLevel(Object entity, Object pos) {
                        return 15;
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/ThrownItemRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class ThrownItemRenderer<T> extends EntityRenderer {
                    private final boolean fullBright;
                    public ThrownItemRenderer(
                        final EntityRendererProvider.Context context,
                        final float scale,
                        final boolean fullBright
                    ) {
                        this.scale = scale;
                        this.fullBright = fullBright;
                    }
                    public ThrownItemRenderer(final EntityRendererProvider.Context context) {
                        this(context, 1.0F, false);
                    }
                    protected int getBlockLightLevel(final T entity, final Object blockPos) {
                        return this.fullBright
                            ? 15
                            : super.getBlockLightLevel(entity, blockPos);
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/ItemFrameRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                import net.minecraft.world.entity.EntityType;
                public class ItemFrameRenderer extends EntityRenderer {
                    protected int getBlockLightLevel(final Object entity, final Object blockPos) {
                        return entity.is(EntityType.GLOW_ITEM_FRAME)
                            ? Math.max(5, super.getBlockLightLevel(entity, blockPos))
                            : super.getBlockLightLevel(entity, blockPos);
                    }
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.isGlowFrame = entity.is(EntityType.GLOW_ITEM_FRAME);
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/SheepRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class SheepRenderer extends MobRenderer {
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.headEatAngleScale = entity.getHeadEatAngleScale(partialTicks);
                        state.headEatPositionScale = entity.getHeadEatPositionScale(partialTicks);
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/ParrotRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class ParrotRenderer extends MobRenderer {
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.pose = ParrotModel.getPose(entity);
                    }
                }
                """,
            ),
            java(
                "net/minecraft/client/renderer/entity/WolfRenderer.java",
                """
                package net.minecraft.client.renderer.entity;
                public class WolfRenderer extends MobRenderer {
                    public void extractRenderState(Object entity, Object state, float partialTicks) {
                        state.isSitting = entity.isInSittingPose();
                    }
                }
                """,
            ),
        ]

        entries = extract_entity_renderer_registrations(
            entity_renderers,
            renderer_sources,
            [
                "BLAZE",
                "EYE_OF_ENDER",
                "CAT",
                "HORSE",
                "ALLAY",
                "ARMOR_STAND",
                "VEX",
                "VILLAGER",
                "WANDERING_TRADER",
                "EVOKER",
                "ILLUSIONER",
                "EGG",
                "FIREBALL",
                "ZOMBIE",
                "HUSK",
                "DROWNED",
                "ZOMBIE_VILLAGER",
                "GLOW_ITEM_FRAME",
                "ITEM_FRAME",
                "SHEEP",
                "PARROT",
                "WOLF",
                "MANNEQUIN",
                "PLAYER",
            ],
        )

        self.assertEqual(
            [
                (
                    entry.entity_type_id,
                    entry.constant_name,
                    entry.renderer_class,
                    entry.static_full_block_light,
                    entry.minimum_block_light,
                    entry.thrown_item_scale_bits,
                    entry.uses_abstract_feline_model,
                    entry.uses_abstract_horse_renderer,
                    entry.uses_mob_item_in_hand_layer,
                    entry.extracts_mob_aggressive,
                    entry.extracts_unhappy_counter,
                    entry.extracts_casting_spell,
                )
                for entry in entries
            ],
            [
                (0, "BLAZE", "BlazeRenderer", True, None, None, False, False, False, False, False, False),
                (1, "EYE_OF_ENDER", "ThrownItemRenderer", True, None, 1065353216, False, False, False, False, False, False),
                (2, "CAT", "CatRenderer", False, None, None, True, False, False, False, False, False),
                (3, "HORSE", "HorseRenderer", False, None, None, False, True, False, False, False, False),
                (4, "ALLAY", "AllayRenderer", False, None, None, False, False, True, True, False, False),
                (5, "ARMOR_STAND", "ArmorStandRenderer", False, None, None, False, False, False, False, False, False),
                (6, "VEX", "VexRenderer", False, None, None, False, False, True, True, False, False),
                (7, "VILLAGER", "VillagerRenderer", False, None, None, False, False, False, False, True, False),
                (8, "WANDERING_TRADER", "WanderingTraderRenderer", False, None, None, False, False, False, False, True, False),
                (9, "EVOKER", "EvokerRenderer", False, None, None, False, False, False, False, False, True),
                (10, "ILLUSIONER", "IllusionerRenderer", False, None, None, False, False, False, False, False, True),
                (11, "EGG", "ThrownItemRenderer", False, None, 1065353216, False, False, False, False, False, False),
                (12, "FIREBALL", "ThrownItemRenderer", True, None, 1077936128, False, False, False, False, False, False),
                (13, "ZOMBIE", "ZombieRenderer", False, None, None, False, False, False, False, False, False),
                (14, "HUSK", "HuskRenderer", False, None, None, False, False, False, False, False, False),
                (15, "DROWNED", "DrownedRenderer", False, None, None, False, False, False, False, False, False),
                (16, "ZOMBIE_VILLAGER", "ZombieVillagerRenderer", False, None, None, False, False, False, False, False, False),
                (17, "GLOW_ITEM_FRAME", "ItemFrameRenderer", False, 5, None, False, False, False, False, False, False),
                (18, "ITEM_FRAME", "ItemFrameRenderer", False, None, None, False, False, False, False, False, False),
                (19, "SHEEP", "SheepRenderer", False, None, None, False, False, False, False, False, False),
                (20, "PARROT", "ParrotRenderer", False, None, None, False, False, False, False, False, False),
                (21, "WOLF", "WolfRenderer", False, None, None, False, False, False, False, False, False),
            ],
        )
        self.assertEqual(
            [
                entry.constant_name
                for entry in entries
                if entry.extracts_zombie_underwater_converting
            ],
            ["ZOMBIE", "HUSK", "DROWNED"],
        )
        self.assertEqual(
            [entry.constant_name for entry in entries if entry.extracts_sheep_head_eat],
            ["SHEEP"],
        )
        self.assertEqual(
            [entry.constant_name for entry in entries if entry.extracts_wolf_sitting],
            ["WOLF"],
        )
        self.assertEqual(
            [entry.constant_name for entry in entries if entry.extracts_parrot_pose],
            ["PARROT"],
        )


if __name__ == "__main__":
    unittest.main()
