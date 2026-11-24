#!/usr/bin/env python3
"""Generate/check the checked-in vanilla 26.1 entity metadata Rust catalog."""

from __future__ import annotations

import argparse
import difflib
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root
from entity_metadata_extractor import (
    GAME_RENDERER_SOURCE,
    Accessor,
    ExtractionError,
    ExtractionResult,
    class_nearest_method_owner,
    evaluate_integer_expression,
    extract,
    source_for_owner,
)
from sound_event_catalog_extractor import direct_sound_event_entries
from tree_sitter import Node
from vanilla_java_source import (
    JavaField,
    JavaMethod,
    JavaSource,
    JavaType,
    descendants,
    resolve_type_reference,
)

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-client-world/src/entities/schema/generated_26_1.rs"
)
# The synched-data ids and flag bits are pure wire facts, so they live in the
# protocol crate: the server reads them to broadcast entity data and the client
# reads them to apply it, and neither may keep its own copy.
DEFAULT_PROTOCOL_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/entity_synched_data.rs"
DEFAULT_SOUND_SOURCE_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/entity_sound_sources.rs"


@dataclass(frozen=True)
class CameraPostEffectSpec:
    runtime_class: str
    resource_id: str


@dataclass(frozen=True)
class EntityEventHandlerSpec:
    owner: str
    super_handler_owner: str | None
    source_path: str
    direct_event_ids: tuple[int, ...]
    delegated_event_ids: tuple[int, ...]
    delegates_to: str | None
    forwards_unhandled_to_super: bool


ENTITY_EVENT_HANDLER_DELEGATIONS = {
    "net.minecraft.world.entity.vehicle.minecart.MinecartSpawner": (
        "this.spawner",
        "net.minecraft.world.level.BaseSpawner",
        "onEventTriggered",
        2,
        1,
    ),
}


# Every class that declares synched data is one metadata family, named after
# its simple class name unless listed here.
FAMILY_NAME_OVERRIDES = {
    "net.minecraft.world.entity.projectile.FireworkRocketEntity": "FireworkRocket",
}


DATA_ALIASES = {
    "VANILLA_WARDEN_ANGER_LEVEL_DATA_ID": ("net.minecraft.world.entity.monster.warden.Warden", "CLIENT_ANGER_LEVEL"),
    "VANILLA_SPIDER_FLAGS_DATA_ID": ("net.minecraft.world.entity.monster.spider.Spider", "DATA_FLAGS_ID"),
    "VANILLA_ENTITY_SHARED_FLAGS_DATA_ID": ("net.minecraft.world.entity.Entity", "DATA_SHARED_FLAGS_ID"),
    "VANILLA_ENTITY_AIR_SUPPLY_DATA_ID": ("net.minecraft.world.entity.Entity", "DATA_AIR_SUPPLY_ID"),
    "VANILLA_ENTITY_CUSTOM_NAME_DATA_ID": ("net.minecraft.world.entity.Entity", "DATA_CUSTOM_NAME"),
    "VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID": ("net.minecraft.world.entity.Entity", "DATA_CUSTOM_NAME_VISIBLE"),
    "VANILLA_ENTITY_SILENT_DATA_ID": ("net.minecraft.world.entity.Entity", "DATA_SILENT"),
    "VANILLA_ENTITY_NO_GRAVITY_DATA_ID": ("net.minecraft.world.entity.Entity", "DATA_NO_GRAVITY"),
    "VANILLA_ENTITY_POSE_DATA_ID": ("net.minecraft.world.entity.Entity", "DATA_POSE"),
    "VANILLA_ENTITY_TICKS_FROZEN_DATA_ID": ("net.minecraft.world.entity.Entity", "DATA_TICKS_FROZEN"),
    "VANILLA_LIVING_ENTITY_FLAGS_DATA_ID": ("net.minecraft.world.entity.LivingEntity", "DATA_LIVING_ENTITY_FLAGS"),
    "VANILLA_LIVING_ENTITY_HEALTH_DATA_ID": ("net.minecraft.world.entity.LivingEntity", "DATA_HEALTH_ID"),
    "VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID": ("net.minecraft.world.entity.LivingEntity", "DATA_EFFECT_PARTICLES"),
    "VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID": ("net.minecraft.world.entity.LivingEntity", "DATA_EFFECT_AMBIENCE_ID"),
    "VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID": ("net.minecraft.world.entity.LivingEntity", "DATA_ARROW_COUNT_ID"),
    "VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID": ("net.minecraft.world.entity.LivingEntity", "DATA_STINGER_COUNT_ID"),
    "VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID": ("net.minecraft.world.entity.LivingEntity", "SLEEPING_POS_ID"),
    "VANILLA_MOB_FLAGS_DATA_ID": ("net.minecraft.world.entity.Mob", "DATA_MOB_FLAGS_ID"),
    "VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID": ("net.minecraft.world.entity.animal.allay.Allay", "DATA_CAN_DUPLICATE"),
    "VANILLA_ALLAY_DANCING_DATA_ID": ("net.minecraft.world.entity.animal.allay.Allay", "DATA_DANCING"),
    "VANILLA_ARMADILLO_STATE_DATA_ID": ("net.minecraft.world.entity.animal.armadillo.Armadillo", "ARMADILLO_STATE"),
    "VANILLA_COPPER_GOLEM_STATE_DATA_ID": ("net.minecraft.world.entity.animal.golem.CopperGolem", "COPPER_GOLEM_STATE"),
    "VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID": ("net.minecraft.world.entity.animal.golem.CopperGolem", "DATA_WEATHER_STATE"),
    "VANILLA_CREEPER_SWELL_DIR_DATA_ID": ("net.minecraft.world.entity.monster.Creeper", "DATA_SWELL_DIR"),
    "VANILLA_CREEPER_POWERED_DATA_ID": ("net.minecraft.world.entity.monster.Creeper", "DATA_IS_POWERED"),
    "VANILLA_CREEPER_IGNITED_DATA_ID": ("net.minecraft.world.entity.monster.Creeper", "DATA_IS_IGNITED"),
    "VANILLA_CREAKING_CAN_MOVE_DATA_ID": ("net.minecraft.world.entity.monster.creaking.Creaking", "CAN_MOVE"),
    "VANILLA_CREAKING_IS_ACTIVE_DATA_ID": ("net.minecraft.world.entity.monster.creaking.Creaking", "IS_ACTIVE"),
    "VANILLA_CREAKING_IS_TEARING_DOWN_DATA_ID": ("net.minecraft.world.entity.monster.creaking.Creaking", "IS_TEARING_DOWN"),
    "VANILLA_CREAKING_HOME_POS_DATA_ID": ("net.minecraft.world.entity.monster.creaking.Creaking", "HOME_POS"),
    "VANILLA_WITHER_TARGET_A_DATA_ID": ("net.minecraft.world.entity.boss.wither.WitherBoss", "DATA_TARGET_A"),
    "VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID": ("net.minecraft.world.entity.boss.wither.WitherBoss", "DATA_ID_INV"),
    "VANILLA_ENDERMAN_CARRY_STATE_DATA_ID": ("net.minecraft.world.entity.monster.EnderMan", "DATA_CARRY_STATE"),
    "VANILLA_ENDERMAN_CREEPY_DATA_ID": ("net.minecraft.world.entity.monster.EnderMan", "DATA_CREEPY"),
    "VANILLA_ENDERMAN_STARED_AT_DATA_ID": ("net.minecraft.world.entity.monster.EnderMan", "DATA_STARED_AT"),
    "VANILLA_BEE_FLAGS_DATA_ID": ("net.minecraft.world.entity.animal.bee.Bee", "DATA_FLAGS_ID"),
    "VANILLA_BEE_ANGER_END_TIME_DATA_ID": ("net.minecraft.world.entity.animal.bee.Bee", "DATA_ANGER_END_TIME"),
    "VANILLA_ZOMBIE_BABY_DATA_ID": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_BABY_ID"),
    "VANILLA_ZOMBIE_SPECIAL_TYPE_DATA_ID": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_SPECIAL_TYPE_ID"),
    "VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_DROWNED_CONVERSION_ID"),
    "VANILLA_AGEABLE_MOB_BABY_DATA_ID": ("net.minecraft.world.entity.AgeableMob", "DATA_BABY_ID"),
    "VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID": ("net.minecraft.world.entity.AgeableMob", "AGE_LOCKED"),
    "VANILLA_AXOLOTL_FROM_BUCKET_DATA_ID": ("net.minecraft.world.entity.animal.axolotl.Axolotl", "FROM_BUCKET"),
    "VANILLA_AXOLOTL_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.axolotl.Axolotl", "DATA_VARIANT"),
    "VANILLA_AXOLOTL_PLAYING_DEAD_DATA_ID": ("net.minecraft.world.entity.animal.axolotl.Axolotl", "DATA_PLAYING_DEAD"),
    "VANILLA_DOLPHIN_GOT_FISH_DATA_ID": ("net.minecraft.world.entity.animal.dolphin.Dolphin", "GOT_FISH"),
    "VANILLA_DOLPHIN_MOISTNESS_DATA_ID": ("net.minecraft.world.entity.animal.dolphin.Dolphin", "MOISTNESS_LEVEL"),
    "VANILLA_GUARDIAN_MOVING_DATA_ID": ("net.minecraft.world.entity.monster.Guardian", "DATA_ID_MOVING"),
    "VANILLA_GUARDIAN_ATTACK_TARGET_DATA_ID": ("net.minecraft.world.entity.monster.Guardian", "DATA_ID_ATTACK_TARGET"),
    "VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID": ("net.minecraft.world.entity.animal.squid.GlowSquid", "DATA_DARK_TICKS_REMAINING"),
    "VANILLA_HAPPY_GHAST_IS_LEASH_HOLDER_DATA_ID": ("net.minecraft.world.entity.animal.happyghast.HappyGhast", "IS_LEASH_HOLDER"),
    "VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID": ("net.minecraft.world.entity.animal.happyghast.HappyGhast", "STAYS_STILL"),
    "VANILLA_GOAT_SCREAMING_DATA_ID": ("net.minecraft.world.entity.animal.goat.Goat", "DATA_IS_SCREAMING_GOAT"),
    "VANILLA_GOAT_LEFT_HORN_DATA_ID": ("net.minecraft.world.entity.animal.goat.Goat", "DATA_HAS_LEFT_HORN"),
    "VANILLA_GOAT_RIGHT_HORN_DATA_ID": ("net.minecraft.world.entity.animal.goat.Goat", "DATA_HAS_RIGHT_HORN"),
    "VANILLA_FOX_TYPE_DATA_ID": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_TYPE_ID"),
    "VANILLA_FOX_FLAGS_DATA_ID": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_FLAGS_ID"),
    "VANILLA_FOX_TRUSTED_DATA_ID_0": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_TRUSTED_ID_0"),
    "VANILLA_FOX_TRUSTED_DATA_ID_1": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_TRUSTED_ID_1"),
    "VANILLA_OCELOT_TRUSTING_DATA_ID": ("net.minecraft.world.entity.animal.feline.Ocelot", "DATA_TRUSTING"),
    "VANILLA_PARROT_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.parrot.Parrot", "DATA_VARIANT_ID"),
    "VANILLA_SALMON_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.fish.Salmon", "DATA_TYPE"),
    "VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID": ("net.minecraft.world.entity.animal.fish.Pufferfish", "PUFF_STATE"),
    "VANILLA_TROPICAL_FISH_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.fish.TropicalFish", "DATA_ID_TYPE_VARIANT"),
    "VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "UNHAPPY_COUNTER"),
    "VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "SNEEZE_COUNTER"),
    "VANILLA_PANDA_EAT_COUNTER_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "EAT_COUNTER"),
    "VANILLA_PANDA_MAIN_GENE_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "MAIN_GENE_ID"),
    "VANILLA_PANDA_HIDDEN_GENE_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "HIDDEN_GENE_ID"),
    "VANILLA_PANDA_FLAGS_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "DATA_ID_FLAGS"),
    "VANILLA_SHEEP_WOOL_DATA_ID": ("net.minecraft.world.entity.animal.sheep.Sheep", "DATA_WOOL_ID"),
    "VANILLA_SNIFFER_STATE_DATA_ID": ("net.minecraft.world.entity.animal.sniffer.Sniffer", "DATA_STATE"),
    "VANILLA_SNIFFER_DROP_SEED_AT_TICK_DATA_ID": ("net.minecraft.world.entity.animal.sniffer.Sniffer", "DATA_DROP_SEED_AT_TICK"),
    "VANILLA_TURTLE_HAS_EGG_DATA_ID": ("net.minecraft.world.entity.animal.turtle.Turtle", "HAS_EGG"),
    "VANILLA_TURTLE_LAYING_EGG_DATA_ID": ("net.minecraft.world.entity.animal.turtle.Turtle", "LAYING_EGG"),
    "VANILLA_RABBIT_TYPE_DATA_ID": ("net.minecraft.world.entity.animal.rabbit.Rabbit", "DATA_TYPE_ID"),
    "VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID": ("net.minecraft.world.entity.animal.equine.AbstractHorse", "DATA_ID_FLAGS"),
    "VANILLA_HORSE_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.equine.Horse", "DATA_ID_TYPE_VARIANT"),
    "VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID": ("net.minecraft.world.entity.animal.equine.AbstractChestedHorse", "DATA_ID_CHEST"),
    "VANILLA_LLAMA_STRENGTH_DATA_ID": ("net.minecraft.world.entity.animal.equine.Llama", "DATA_STRENGTH_ID"),
    "VANILLA_LLAMA_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.equine.Llama", "DATA_VARIANT_ID"),
    "VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID": ("net.minecraft.world.entity.TamableAnimal", "DATA_FLAGS_ID"),
    "VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID": ("net.minecraft.world.entity.TamableAnimal", "DATA_OWNERUUID_ID"),
    "VANILLA_CAT_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.feline.Cat", "DATA_VARIANT_ID"),
    "VANILLA_CAT_SOUND_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.feline.Cat", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_CAT_IS_LYING_DATA_ID": ("net.minecraft.world.entity.animal.feline.Cat", "IS_LYING"),
    "VANILLA_CAT_RELAX_STATE_ONE_DATA_ID": ("net.minecraft.world.entity.animal.feline.Cat", "RELAX_STATE_ONE"),
    "VANILLA_CAT_COLLAR_COLOR_DATA_ID": ("net.minecraft.world.entity.animal.feline.Cat", "DATA_COLLAR_COLOR"),
    "VANILLA_WOLF_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.wolf.Wolf", "DATA_VARIANT_ID"),
    "VANILLA_WOLF_SOUND_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.wolf.Wolf", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_CHICKEN_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.chicken.Chicken", "DATA_VARIANT_ID"),
    "VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.chicken.Chicken", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_COW_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.cow.Cow", "DATA_VARIANT_ID"),
    "VANILLA_COW_SOUND_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.cow.Cow", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_MUSHROOM_COW_TYPE_DATA_ID": ("net.minecraft.world.entity.animal.cow.MushroomCow", "DATA_TYPE"),
    "VANILLA_PIG_BOOST_TIME_DATA_ID": ("net.minecraft.world.entity.animal.pig.Pig", "DATA_BOOST_TIME"),
    "VANILLA_PIG_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.pig.Pig", "DATA_VARIANT_ID"),
    "VANILLA_PIG_SOUND_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.pig.Pig", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_FROG_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.frog.Frog", "DATA_VARIANT_ID"),
    "VANILLA_FROG_TONGUE_TARGET_DATA_ID": ("net.minecraft.world.entity.animal.frog.Frog", "DATA_TONGUE_TARGET_ID"),
    "VANILLA_CAMEL_DASH_DATA_ID": ("net.minecraft.world.entity.animal.camel.Camel", "DASH"),
    "VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID": ("net.minecraft.world.entity.animal.camel.Camel", "LAST_POSE_CHANGE_TICK"),
    "VANILLA_POLAR_BEAR_STANDING_DATA_ID": ("net.minecraft.world.entity.animal.polarbear.PolarBear", "DATA_STANDING_ID"),
    "VANILLA_WOLF_INTERESTED_DATA_ID": ("net.minecraft.world.entity.animal.wolf.Wolf", "DATA_INTERESTED_ID"),
    "VANILLA_WOLF_COLLAR_COLOR_DATA_ID": ("net.minecraft.world.entity.animal.wolf.Wolf", "DATA_COLLAR_COLOR"),
    "VANILLA_WOLF_ANGER_END_TIME_DATA_ID": ("net.minecraft.world.entity.animal.wolf.Wolf", "DATA_ANGER_END_TIME"),
    "VANILLA_SNOW_GOLEM_PUMPKIN_DATA_ID": ("net.minecraft.world.entity.animal.golem.SnowGolem", "DATA_PUMPKIN_ID"),
    "VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID": ("net.minecraft.world.entity.animal.nautilus.AbstractNautilus", "DASH"),
    "VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.nautilus.ZombieNautilus", "DATA_VARIANT_ID"),
    "VANILLA_ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID": ("net.minecraft.world.entity.npc.villager.AbstractVillager", "DATA_UNHAPPY_COUNTER"),
    "VANILLA_VILLAGER_DATA_DATA_ID": ("net.minecraft.world.entity.npc.villager.Villager", "DATA_VILLAGER_DATA"),
    "VANILLA_VILLAGER_DATA_FINALIZED_DATA_ID": ("net.minecraft.world.entity.npc.villager.Villager", "DATA_VILLAGER_DATA_FINALIZED"),
    "VANILLA_ZOMBIE_VILLAGER_CONVERTING_DATA_ID": ("net.minecraft.world.entity.monster.zombie.ZombieVillager", "DATA_CONVERTING_ID"),
    "VANILLA_ZOMBIE_VILLAGER_DATA_DATA_ID": ("net.minecraft.world.entity.monster.zombie.ZombieVillager", "DATA_VILLAGER_DATA"),
    "VANILLA_ZOMBIE_VILLAGER_DATA_FINALIZED_DATA_ID": ("net.minecraft.world.entity.monster.zombie.ZombieVillager", "DATA_VILLAGER_DATA_FINALIZED"),
    "VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_CLIENT_FLAGS"),
    "VANILLA_ARMOR_STAND_HEAD_POSE_DATA_ID": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_HEAD_POSE"),
    "VANILLA_ARMOR_STAND_BODY_POSE_DATA_ID": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_BODY_POSE"),
    "VANILLA_ARMOR_STAND_LEFT_ARM_POSE_DATA_ID": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_LEFT_ARM_POSE"),
    "VANILLA_ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_RIGHT_ARM_POSE"),
    "VANILLA_ARMOR_STAND_LEFT_LEG_POSE_DATA_ID": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_LEFT_LEG_POSE"),
    "VANILLA_ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_RIGHT_LEG_POSE"),
    "VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "ID_FLAGS"),
    "VANILLA_ABSTRACT_ARROW_PIERCE_LEVEL_DATA_ID": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "PIERCE_LEVEL"),
    "VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "IN_GROUND"),
    "VANILLA_ARROW_EFFECT_COLOR_DATA_ID": ("net.minecraft.world.entity.projectile.arrow.Arrow", "ID_EFFECT_COLOR"),
    "VANILLA_TRIDENT_LOYALTY_DATA_ID": ("net.minecraft.world.entity.projectile.arrow.ThrownTrident", "ID_LOYALTY"),
    "VANILLA_TRIDENT_FOIL_DATA_ID": ("net.minecraft.world.entity.projectile.arrow.ThrownTrident", "ID_FOIL"),
    "VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID": ("net.minecraft.world.entity.projectile.hurtingprojectile.WitherSkull", "DATA_DANGEROUS"),
    "VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID": ("net.minecraft.world.entity.boss.enderdragon.EndCrystal", "DATA_BEAM_TARGET"),
    "VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID": ("net.minecraft.world.entity.boss.enderdragon.EndCrystal", "DATA_SHOW_BOTTOM"),
    "VANILLA_INTERACTION_WIDTH_DATA_ID": ("net.minecraft.world.entity.Interaction", "DATA_WIDTH_ID"),
    "VANILLA_INTERACTION_HEIGHT_DATA_ID": ("net.minecraft.world.entity.Interaction", "DATA_HEIGHT_ID"),
    "VANILLA_INTERACTION_RESPONSE_DATA_ID": ("net.minecraft.world.entity.Interaction", "DATA_RESPONSE_ID"),
    "VANILLA_EXPERIENCE_ORB_VALUE_DATA_ID": ("net.minecraft.world.entity.ExperienceOrb", "DATA_VALUE"),
    "VANILLA_ITEM_ENTITY_ITEM_DATA_ID": ("net.minecraft.world.entity.item.ItemEntity", "DATA_ITEM"),
    "VANILLA_PRIMED_TNT_FUSE_DATA_ID": ("net.minecraft.world.entity.item.PrimedTnt", "DATA_FUSE_ID"),
    "VANILLA_PRIMED_TNT_BLOCK_STATE_DATA_ID": ("net.minecraft.world.entity.item.PrimedTnt", "DATA_BLOCK_STATE_ID"),
    "VANILLA_FIREWORK_ROCKET_ITEM_DATA_ID": ("net.minecraft.world.entity.projectile.FireworkRocketEntity", "DATA_ID_FIREWORKS_ITEM"),
    "VANILLA_FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID": ("net.minecraft.world.entity.projectile.FireworkRocketEntity", "DATA_ATTACHED_TO_TARGET"),
    "VANILLA_FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID": ("net.minecraft.world.entity.projectile.FireworkRocketEntity", "DATA_SHOT_AT_ANGLE"),
    "VANILLA_EYE_OF_ENDER_ITEM_DATA_ID": ("net.minecraft.world.entity.projectile.EyeOfEnder", "DATA_ITEM_STACK"),
    "VANILLA_FIREBALL_ITEM_DATA_ID": ("net.minecraft.world.entity.projectile.hurtingprojectile.Fireball", "DATA_ITEM_STACK"),
    "VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID": ("net.minecraft.world.entity.projectile.FishingHook", "DATA_HOOKED_ENTITY"),
    "VANILLA_FISHING_HOOK_BITING_DATA_ID": ("net.minecraft.world.entity.projectile.FishingHook", "DATA_BITING"),
    "VANILLA_OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID": ("net.minecraft.world.entity.OminousItemSpawner", "DATA_ITEM"),
    "VANILLA_BAT_FLAGS_DATA_ID": ("net.minecraft.world.entity.ambient.Bat", "DATA_ID_FLAGS"),
    "VANILLA_SLIME_SIZE_DATA_ID": ("net.minecraft.world.entity.monster.Slime", "ID_SIZE"),
    "VANILLA_PHANTOM_SIZE_DATA_ID": ("net.minecraft.world.entity.monster.Phantom", "ID_SIZE"),
    "VANILLA_BOGGED_SHEARED_DATA_ID": ("net.minecraft.world.entity.monster.skeleton.Bogged", "DATA_SHEARED"),
    "VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID": ("net.minecraft.world.entity.monster.skeleton.Skeleton", "DATA_STRAY_CONVERSION_ID"),
    "VANILLA_BLAZE_FLAGS_DATA_ID": ("net.minecraft.world.entity.monster.Blaze", "DATA_FLAGS_ID"),
    "VANILLA_SHULKER_ATTACH_FACE_DATA_ID": ("net.minecraft.world.entity.monster.Shulker", "DATA_ATTACH_FACE_ID"),
    "VANILLA_SHULKER_PEEK_DATA_ID": ("net.minecraft.world.entity.monster.Shulker", "DATA_PEEK_ID"),
    "VANILLA_SHULKER_COLOR_DATA_ID": ("net.minecraft.world.entity.monster.Shulker", "DATA_COLOR_ID"),
    "VANILLA_GHAST_CHARGING_DATA_ID": ("net.minecraft.world.entity.monster.Ghast", "DATA_IS_CHARGING"),
    "VANILLA_VEX_FLAGS_DATA_ID": ("net.minecraft.world.entity.monster.Vex", "DATA_FLAGS_ID"),
    "VANILLA_STRIDER_BOOST_TIME_DATA_ID": ("net.minecraft.world.entity.monster.Strider", "DATA_BOOST_TIME"),
    "VANILLA_STRIDER_SUFFOCATING_DATA_ID": ("net.minecraft.world.entity.monster.Strider", "DATA_SUFFOCATING"),
    "VANILLA_PIGLIN_BABY_DATA_ID": ("net.minecraft.world.entity.monster.piglin.Piglin", "DATA_BABY_ID"),
    "VANILLA_PIGLIN_DANCING_DATA_ID": ("net.minecraft.world.entity.monster.piglin.Piglin", "DATA_IS_DANCING"),
    "VANILLA_PIGLIN_CHARGING_CROSSBOW_DATA_ID": ("net.minecraft.world.entity.monster.piglin.Piglin", "DATA_IS_CHARGING_CROSSBOW"),
    "VANILLA_ABSTRACT_PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID": ("net.minecraft.world.entity.monster.piglin.AbstractPiglin", "DATA_IMMUNE_TO_ZOMBIFICATION"),
    "VANILLA_RAIDER_CELEBRATING_DATA_ID": ("net.minecraft.world.entity.raid.Raider", "IS_CELEBRATING"),
    "VANILLA_PILLAGER_CHARGING_CROSSBOW_DATA_ID": ("net.minecraft.world.entity.monster.illager.Pillager", "IS_CHARGING_CROSSBOW"),
    "VANILLA_SPELLCASTER_ILLAGER_SPELL_CASTING_DATA_ID": ("net.minecraft.world.entity.monster.illager.SpellcasterIllager", "DATA_SPELL_CASTING_ID"),
    "VANILLA_ZOGLIN_BABY_DATA_ID": ("net.minecraft.world.entity.monster.Zoglin", "DATA_BABY_ID"),
    "VANILLA_HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID": ("net.minecraft.world.entity.monster.hoglin.Hoglin", "DATA_IMMUNE_TO_ZOMBIFICATION"),
    "VANILLA_WITCH_USING_ITEM_DATA_ID": ("net.minecraft.world.entity.monster.Witch", "DATA_USING_ITEM"),
    "VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID": ("net.minecraft.world.entity.animal.fish.AbstractFish", "FROM_BUCKET"),
    "VANILLA_TADPOLE_AGE_LOCKED_DATA_ID": ("net.minecraft.world.entity.animal.frog.Tadpole", "AGE_LOCKED"),
    "VANILLA_VEHICLE_HURT_TIME_DATA_ID": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_HURT"),
    "VANILLA_VEHICLE_HURT_DIR_DATA_ID": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_HURTDIR"),
    "VANILLA_VEHICLE_DAMAGE_DATA_ID": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_DAMAGE"),
    "VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_PADDLE_LEFT"),
    "VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_PADDLE_RIGHT"),
    "VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_BUBBLE_TIME"),
    "VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID": ("net.minecraft.world.entity.vehicle.minecart.AbstractMinecart", "DATA_ID_CUSTOM_DISPLAY_BLOCK"),
    "VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID": ("net.minecraft.world.entity.vehicle.minecart.AbstractMinecart", "DATA_ID_DISPLAY_OFFSET"),
    "VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID": ("net.minecraft.world.entity.vehicle.minecart.MinecartCommandBlock", "DATA_ID_COMMAND_NAME"),
    "VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID": ("net.minecraft.world.entity.vehicle.minecart.MinecartCommandBlock", "DATA_ID_LAST_OUTPUT"),
    "VANILLA_FURNACE_MINECART_FUEL_DATA_ID": ("net.minecraft.world.entity.vehicle.minecart.MinecartFurnace", "DATA_ID_FUEL"),
    "VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID": ("net.minecraft.world.entity.decoration.HangingEntity", "DATA_DIRECTION"),
    "VANILLA_ITEM_FRAME_ITEM_DATA_ID": ("net.minecraft.world.entity.decoration.ItemFrame", "DATA_ITEM"),
    "VANILLA_ITEM_FRAME_ROTATION_DATA_ID": ("net.minecraft.world.entity.decoration.ItemFrame", "DATA_ROTATION"),
    "VANILLA_PAINTING_VARIANT_DATA_ID": ("net.minecraft.world.entity.decoration.painting.Painting", "DATA_PAINTING_VARIANT_ID"),
    "VANILLA_AVATAR_MAIN_HAND_DATA_ID": ("net.minecraft.world.entity.Avatar", "DATA_PLAYER_MAIN_HAND"),
    "VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID": ("net.minecraft.world.entity.Avatar", "DATA_PLAYER_MODE_CUSTOMISATION"),
    "VANILLA_MANNEQUIN_PROFILE_DATA_ID": ("net.minecraft.world.entity.decoration.Mannequin", "DATA_PROFILE"),
    "VANILLA_MANNEQUIN_IMMOVABLE_DATA_ID": ("net.minecraft.world.entity.decoration.Mannequin", "DATA_IMMOVABLE"),
    "VANILLA_MANNEQUIN_DESCRIPTION_DATA_ID": ("net.minecraft.world.entity.decoration.Mannequin", "DATA_DESCRIPTION"),
    "VANILLA_PLAYER_ABSORPTION_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_PLAYER_ABSORPTION_ID"),
    "VANILLA_PLAYER_SCORE_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_SCORE_ID"),
    "VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_SHOULDER_PARROT_LEFT"),
    "VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_SHOULDER_PARROT_RIGHT"),
}


PROTOCOL_DEFAULT_ALIASES = {
    "VANILLA_SHULKER_DEFAULT_PEEK": ("net.minecraft.world.entity.monster.Shulker", "DATA_PEEK_ID"),
    "VANILLA_SHULKER_DEFAULT_COLOR": ("net.minecraft.world.entity.monster.Shulker", "DATA_COLOR_ID"),
}


DEFAULT_ALIASES = {
    "VANILLA_ALLAY_DEFAULT_CAN_DUPLICATE": ("net.minecraft.world.entity.animal.allay.Allay", "DATA_CAN_DUPLICATE"),
    "VANILLA_CREEPER_DEFAULT_SWELL_DIR": ("net.minecraft.world.entity.monster.Creeper", "DATA_SWELL_DIR"),
    "VANILLA_CREEPER_DEFAULT_POWERED": ("net.minecraft.world.entity.monster.Creeper", "DATA_IS_POWERED"),
    "VANILLA_CREEPER_DEFAULT_IGNITED": ("net.minecraft.world.entity.monster.Creeper", "DATA_IS_IGNITED"),
    "VANILLA_WITHER_DEFAULT_TARGET_A": ("net.minecraft.world.entity.boss.wither.WitherBoss", "DATA_TARGET_A"),
    "VANILLA_WITHER_DEFAULT_INVULNERABLE_TICKS": ("net.minecraft.world.entity.boss.wither.WitherBoss", "DATA_ID_INV"),
    "VANILLA_ENDERMAN_DEFAULT_CARRY_STATE": ("net.minecraft.world.entity.monster.EnderMan", "DATA_CARRY_STATE"),
    "VANILLA_ENDERMAN_DEFAULT_CREEPY": ("net.minecraft.world.entity.monster.EnderMan", "DATA_CREEPY"),
    "VANILLA_ENDERMAN_DEFAULT_STARED_AT": ("net.minecraft.world.entity.monster.EnderMan", "DATA_STARED_AT"),
    "VANILLA_BEE_DEFAULT_FLAGS": ("net.minecraft.world.entity.animal.bee.Bee", "DATA_FLAGS_ID"),
    "VANILLA_BEE_DEFAULT_ANGER_END_TIME": ("net.minecraft.world.entity.animal.bee.Bee", "DATA_ANGER_END_TIME"),
    "VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION": ("net.minecraft.world.entity.Avatar", "DATA_PLAYER_MODE_CUSTOMISATION"),
    "VANILLA_ZOMBIE_DEFAULT_BABY": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_BABY_ID"),
    "VANILLA_ZOMBIE_DEFAULT_SPECIAL_TYPE": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_SPECIAL_TYPE_ID"),
    "VANILLA_ZOMBIE_DEFAULT_DROWNED_CONVERSION": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_DROWNED_CONVERSION_ID"),
    "VANILLA_AGEABLE_MOB_DEFAULT_BABY": ("net.minecraft.world.entity.AgeableMob", "DATA_BABY_ID"),
    "VANILLA_AGEABLE_MOB_DEFAULT_AGE_LOCKED": ("net.minecraft.world.entity.AgeableMob", "AGE_LOCKED"),
    "VANILLA_AXOLOTL_DEFAULT_FROM_BUCKET": ("net.minecraft.world.entity.animal.axolotl.Axolotl", "FROM_BUCKET"),
    "VANILLA_DOLPHIN_DEFAULT_GOT_FISH": ("net.minecraft.world.entity.animal.dolphin.Dolphin", "GOT_FISH"),
    "VANILLA_DOLPHIN_DEFAULT_MOISTNESS": ("net.minecraft.world.entity.animal.dolphin.Dolphin", "MOISTNESS_LEVEL"),
    "VANILLA_GLOW_SQUID_DEFAULT_DARK_TICKS": ("net.minecraft.world.entity.animal.squid.GlowSquid", "DATA_DARK_TICKS_REMAINING"),
    "VANILLA_HAPPY_GHAST_DEFAULT_IS_LEASH_HOLDER": ("net.minecraft.world.entity.animal.happyghast.HappyGhast", "IS_LEASH_HOLDER"),
    "VANILLA_HAPPY_GHAST_DEFAULT_STAYS_STILL": ("net.minecraft.world.entity.animal.happyghast.HappyGhast", "STAYS_STILL"),
    "VANILLA_GOAT_DEFAULT_SCREAMING": ("net.minecraft.world.entity.animal.goat.Goat", "DATA_IS_SCREAMING_GOAT"),
    "VANILLA_GOAT_DEFAULT_HAS_LEFT_HORN": ("net.minecraft.world.entity.animal.goat.Goat", "DATA_HAS_LEFT_HORN"),
    "VANILLA_GOAT_DEFAULT_HAS_RIGHT_HORN": ("net.minecraft.world.entity.animal.goat.Goat", "DATA_HAS_RIGHT_HORN"),
    "VANILLA_FOX_DEFAULT_TYPE": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_TYPE_ID"),
    "VANILLA_FOX_DEFAULT_FLAGS": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_FLAGS_ID"),
    "VANILLA_OCELOT_DEFAULT_TRUSTING": ("net.minecraft.world.entity.animal.feline.Ocelot", "DATA_TRUSTING"),
    "VANILLA_PANDA_DEFAULT_UNHAPPY_COUNTER": ("net.minecraft.world.entity.animal.panda.Panda", "UNHAPPY_COUNTER"),
    "VANILLA_PANDA_DEFAULT_SNEEZE_COUNTER": ("net.minecraft.world.entity.animal.panda.Panda", "SNEEZE_COUNTER"),
    "VANILLA_PANDA_DEFAULT_EAT_COUNTER": ("net.minecraft.world.entity.animal.panda.Panda", "EAT_COUNTER"),
    "VANILLA_PANDA_DEFAULT_MAIN_GENE": ("net.minecraft.world.entity.animal.panda.Panda", "MAIN_GENE_ID"),
    "VANILLA_PANDA_DEFAULT_HIDDEN_GENE": ("net.minecraft.world.entity.animal.panda.Panda", "HIDDEN_GENE_ID"),
    "VANILLA_PANDA_DEFAULT_FLAGS": ("net.minecraft.world.entity.animal.panda.Panda", "DATA_ID_FLAGS"),
    "VANILLA_SNIFFER_DEFAULT_DROP_SEED_AT_TICK": ("net.minecraft.world.entity.animal.sniffer.Sniffer", "DATA_DROP_SEED_AT_TICK"),
    "VANILLA_TURTLE_DEFAULT_HAS_EGG": ("net.minecraft.world.entity.animal.turtle.Turtle", "HAS_EGG"),
    "VANILLA_TURTLE_DEFAULT_LAYING_EGG": ("net.minecraft.world.entity.animal.turtle.Turtle", "LAYING_EGG"),
    "VANILLA_ABSTRACT_HORSE_DEFAULT_FLAGS": ("net.minecraft.world.entity.animal.equine.AbstractHorse", "DATA_ID_FLAGS"),
    "VANILLA_HORSE_DEFAULT_VARIANT": ("net.minecraft.world.entity.animal.equine.Horse", "DATA_ID_TYPE_VARIANT"),
    "VANILLA_ABSTRACT_CHESTED_HORSE_DEFAULT_CHEST": ("net.minecraft.world.entity.animal.equine.AbstractChestedHorse", "DATA_ID_CHEST"),
    "VANILLA_LLAMA_DEFAULT_STRENGTH": ("net.minecraft.world.entity.animal.equine.Llama", "DATA_STRENGTH_ID"),
    "VANILLA_LLAMA_DEFAULT_VARIANT": ("net.minecraft.world.entity.animal.equine.Llama", "DATA_VARIANT_ID"),
    "VANILLA_TAMABLE_ANIMAL_DEFAULT_FLAGS": ("net.minecraft.world.entity.TamableAnimal", "DATA_FLAGS_ID"),
    "VANILLA_CAT_DEFAULT_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.feline.Cat", "DATA_VARIANT_ID"),
    "VANILLA_CAT_DEFAULT_SOUND_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.feline.Cat", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_WOLF_DEFAULT_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.wolf.Wolf", "DATA_VARIANT_ID"),
    "VANILLA_WOLF_DEFAULT_SOUND_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.wolf.Wolf", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_CHICKEN_DEFAULT_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.chicken.Chicken", "DATA_VARIANT_ID"),
    "VANILLA_CHICKEN_DEFAULT_SOUND_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.chicken.Chicken", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_COW_DEFAULT_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.cow.Cow", "DATA_VARIANT_ID"),
    "VANILLA_COW_DEFAULT_SOUND_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.cow.Cow", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_PIG_DEFAULT_BOOST_TIME": ("net.minecraft.world.entity.animal.pig.Pig", "DATA_BOOST_TIME"),
    "VANILLA_PIG_DEFAULT_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.pig.Pig", "DATA_VARIANT_ID"),
    "VANILLA_PIG_DEFAULT_SOUND_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.pig.Pig", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_FROG_DEFAULT_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.frog.Frog", "DATA_VARIANT_ID"),
    "VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH": ("net.minecraft.world.entity.animal.nautilus.AbstractNautilus", "DASH"),
    "VANILLA_ZOMBIE_NAUTILUS_DEFAULT_VARIANT_RESOURCE_ID": ("net.minecraft.world.entity.animal.nautilus.ZombieNautilus", "DATA_VARIANT_ID"),
    "VANILLA_ABSTRACT_VILLAGER_DEFAULT_UNHAPPY_COUNTER": ("net.minecraft.world.entity.npc.villager.AbstractVillager", "DATA_UNHAPPY_COUNTER"),
    "VANILLA_VILLAGER_DEFAULT_DATA_FINALIZED": ("net.minecraft.world.entity.npc.villager.Villager", "DATA_VILLAGER_DATA_FINALIZED"),
    "VANILLA_ZOMBIE_VILLAGER_DEFAULT_CONVERTING": ("net.minecraft.world.entity.monster.zombie.ZombieVillager", "DATA_CONVERTING_ID"),
    "VANILLA_ZOMBIE_VILLAGER_DEFAULT_DATA_FINALIZED": ("net.minecraft.world.entity.monster.zombie.ZombieVillager", "DATA_VILLAGER_DATA_FINALIZED"),
    "VANILLA_ABSTRACT_ARROW_DEFAULT_FLAGS": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "ID_FLAGS"),
    "VANILLA_ABSTRACT_ARROW_DEFAULT_PIERCE_LEVEL": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "PIERCE_LEVEL"),
    "VANILLA_ABSTRACT_ARROW_DEFAULT_IN_GROUND": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "IN_GROUND"),
    "VANILLA_ARROW_DEFAULT_EFFECT_COLOR": ("net.minecraft.world.entity.projectile.arrow.Arrow", "ID_EFFECT_COLOR"),
    "VANILLA_TRIDENT_DEFAULT_LOYALTY": ("net.minecraft.world.entity.projectile.arrow.ThrownTrident", "ID_LOYALTY"),
    "VANILLA_TRIDENT_DEFAULT_FOIL": ("net.minecraft.world.entity.projectile.arrow.ThrownTrident", "ID_FOIL"),
    "VANILLA_WITHER_SKULL_DEFAULT_DANGEROUS": ("net.minecraft.world.entity.projectile.hurtingprojectile.WitherSkull", "DATA_DANGEROUS"),
    "VANILLA_END_CRYSTAL_DEFAULT_SHOW_BOTTOM": ("net.minecraft.world.entity.boss.enderdragon.EndCrystal", "DATA_SHOW_BOTTOM"),
    "VANILLA_INTERACTION_DEFAULT_WIDTH": ("net.minecraft.world.entity.Interaction", "DATA_WIDTH_ID"),
    "VANILLA_INTERACTION_DEFAULT_HEIGHT": ("net.minecraft.world.entity.Interaction", "DATA_HEIGHT_ID"),
    "VANILLA_INTERACTION_DEFAULT_RESPONSE": ("net.minecraft.world.entity.Interaction", "DATA_RESPONSE_ID"),
    "VANILLA_EXPERIENCE_ORB_DEFAULT_VALUE": ("net.minecraft.world.entity.ExperienceOrb", "DATA_VALUE"),
    "VANILLA_PRIMED_TNT_DEFAULT_FUSE": ("net.minecraft.world.entity.item.PrimedTnt", "DATA_FUSE_ID"),
    "VANILLA_FIREWORK_ROCKET_DEFAULT_ITEM_RESOURCE_ID": ("net.minecraft.world.entity.projectile.FireworkRocketEntity", "DATA_ID_FIREWORKS_ITEM"),
    "VANILLA_FIREWORK_ROCKET_DEFAULT_SHOT_AT_ANGLE": ("net.minecraft.world.entity.projectile.FireworkRocketEntity", "DATA_SHOT_AT_ANGLE"),
    "VANILLA_FISHING_HOOK_DEFAULT_HOOKED_ENTITY": ("net.minecraft.world.entity.projectile.FishingHook", "DATA_HOOKED_ENTITY"),
    "VANILLA_FISHING_HOOK_DEFAULT_BITING": ("net.minecraft.world.entity.projectile.FishingHook", "DATA_BITING"),
    "VANILLA_BAT_DEFAULT_FLAGS": ("net.minecraft.world.entity.ambient.Bat", "DATA_ID_FLAGS"),
    "VANILLA_SKELETON_DEFAULT_STRAY_CONVERSION": ("net.minecraft.world.entity.monster.skeleton.Skeleton", "DATA_STRAY_CONVERSION_ID"),
    "VANILLA_GHAST_DEFAULT_CHARGING": ("net.minecraft.world.entity.monster.Ghast", "DATA_IS_CHARGING"),
    "VANILLA_VEX_DEFAULT_FLAGS": ("net.minecraft.world.entity.monster.Vex", "DATA_FLAGS_ID"),
    "VANILLA_STRIDER_DEFAULT_BOOST_TIME": ("net.minecraft.world.entity.monster.Strider", "DATA_BOOST_TIME"),
    "VANILLA_STRIDER_DEFAULT_SUFFOCATING": ("net.minecraft.world.entity.monster.Strider", "DATA_SUFFOCATING"),
    "VANILLA_ZOGLIN_DEFAULT_BABY": ("net.minecraft.world.entity.monster.Zoglin", "DATA_BABY_ID"),
    "VANILLA_HOGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION": ("net.minecraft.world.entity.monster.hoglin.Hoglin", "DATA_IMMUNE_TO_ZOMBIFICATION"),
    "VANILLA_ABSTRACT_FISH_DEFAULT_FROM_BUCKET": ("net.minecraft.world.entity.animal.fish.AbstractFish", "FROM_BUCKET"),
    "VANILLA_TADPOLE_DEFAULT_AGE_LOCKED": ("net.minecraft.world.entity.animal.frog.Tadpole", "AGE_LOCKED"),
    "VANILLA_VEHICLE_DEFAULT_HURT_TIME": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_HURT"),
    "VANILLA_VEHICLE_DEFAULT_HURT_DIR": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_HURTDIR"),
    "VANILLA_VEHICLE_DEFAULT_DAMAGE": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_DAMAGE"),
    "VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_LEFT": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_PADDLE_LEFT"),
    "VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_RIGHT": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_PADDLE_RIGHT"),
    "VANILLA_ABSTRACT_BOAT_DEFAULT_BUBBLE_TIME": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_BUBBLE_TIME"),
    "VANILLA_FURNACE_MINECART_DEFAULT_FUEL": ("net.minecraft.world.entity.vehicle.minecart.MinecartFurnace", "DATA_ID_FUEL"),
    "VANILLA_ARMOR_STAND_DEFAULT_CLIENT_FLAGS": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_CLIENT_FLAGS"),
    "VANILLA_ARMOR_STAND_DEFAULT_HEAD_POSE": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_HEAD_POSE"),
    "VANILLA_ARMOR_STAND_DEFAULT_BODY_POSE": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_BODY_POSE"),
    "VANILLA_ARMOR_STAND_DEFAULT_LEFT_ARM_POSE": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_LEFT_ARM_POSE"),
    "VANILLA_ARMOR_STAND_DEFAULT_RIGHT_ARM_POSE": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_RIGHT_ARM_POSE"),
    "VANILLA_ARMOR_STAND_DEFAULT_LEFT_LEG_POSE": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_LEFT_LEG_POSE"),
    "VANILLA_ARMOR_STAND_DEFAULT_RIGHT_LEG_POSE": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_RIGHT_LEG_POSE"),
    "VANILLA_HANGING_ENTITY_DEFAULT_DIRECTION_ID": ("net.minecraft.world.entity.decoration.HangingEntity", "DATA_DIRECTION"),
    "VANILLA_ITEM_FRAME_DEFAULT_ROTATION": ("net.minecraft.world.entity.decoration.ItemFrame", "DATA_ROTATION"),
    "VANILLA_PLAYER_DEFAULT_ABSORPTION": ("net.minecraft.world.entity.player.Player", "DATA_PLAYER_ABSORPTION_ID"),
    "VANILLA_PLAYER_DEFAULT_SCORE": ("net.minecraft.world.entity.player.Player", "DATA_SCORE_ID"),
}


TYPED_DEFAULT_ALIASES = {
    "VANILLA_PRIMED_TNT_DEFAULT_BLOCK_STATE": (
        "net.minecraft.world.entity.item.PrimedTnt",
        "DATA_BLOCK_STATE_ID",
    ),
}


NAMED_FLAG_ALIASES = {
    "VANILLA_BEE_FLAG_ROLL": ("net.minecraft.world.entity.animal.bee.Bee", "DATA_FLAGS_ID", "FLAG_ROLL"),
    "VANILLA_BEE_FLAG_HAS_STUNG": ("net.minecraft.world.entity.animal.bee.Bee", "DATA_FLAGS_ID", "FLAG_HAS_STUNG"),
    "VANILLA_BEE_FLAG_HAS_NECTAR": ("net.minecraft.world.entity.animal.bee.Bee", "DATA_FLAGS_ID", "FLAG_HAS_NECTAR"),
    "VANILLA_FOX_FLAG_SITTING": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_FLAGS_ID", "FLAG_SITTING"),
    "VANILLA_FOX_FLAG_CROUCHING": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_FLAGS_ID", "FLAG_CROUCHING"),
    "VANILLA_FOX_FLAG_INTERESTED": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_FLAGS_ID", "FLAG_INTERESTED"),
    "VANILLA_FOX_FLAG_POUNCING": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_FLAGS_ID", "FLAG_POUNCING"),
    "VANILLA_FOX_FLAG_SLEEPING": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_FLAGS_ID", "FLAG_SLEEPING"),
    "VANILLA_FOX_FLAG_FACEPLANTED": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_FLAGS_ID", "FLAG_FACEPLANTED"),
    "VANILLA_FOX_FLAG_DEFENDING": ("net.minecraft.world.entity.animal.fox.Fox", "DATA_FLAGS_ID", "FLAG_DEFENDING"),
    "VANILLA_PANDA_FLAG_SNEEZING": ("net.minecraft.world.entity.animal.panda.Panda", "DATA_ID_FLAGS", "FLAG_SNEEZE"),
    "VANILLA_PANDA_FLAG_ROLLING": ("net.minecraft.world.entity.animal.panda.Panda", "DATA_ID_FLAGS", "FLAG_ROLL"),
    "VANILLA_PANDA_FLAG_SITTING": ("net.minecraft.world.entity.animal.panda.Panda", "DATA_ID_FLAGS", "FLAG_SIT"),
    "VANILLA_PANDA_FLAG_ON_BACK": ("net.minecraft.world.entity.animal.panda.Panda", "DATA_ID_FLAGS", "FLAG_ON_BACK"),
    "VANILLA_VEX_FLAG_CHARGING": ("net.minecraft.world.entity.monster.Vex", "DATA_FLAGS_ID", "FLAG_IS_CHARGING"),
    "VANILLA_BAT_FLAG_RESTING": ("net.minecraft.world.entity.ambient.Bat", "DATA_ID_FLAGS", "FLAG_RESTING"),
    "VANILLA_SNOW_GOLEM_PUMPKIN_FLAG": ("net.minecraft.world.entity.animal.golem.SnowGolem", "DATA_PUMPKIN_ID", "PUMPKIN_FLAG"),
    "VANILLA_ABSTRACT_HORSE_FLAG_TAME": ("net.minecraft.world.entity.animal.equine.AbstractHorse", "DATA_ID_FLAGS", "FLAG_TAME"),
    "VANILLA_ABSTRACT_HORSE_FLAG_BRED": ("net.minecraft.world.entity.animal.equine.AbstractHorse", "DATA_ID_FLAGS", "FLAG_BRED"),
    "VANILLA_ABSTRACT_HORSE_FLAG_EATING": ("net.minecraft.world.entity.animal.equine.AbstractHorse", "DATA_ID_FLAGS", "FLAG_EATING"),
    "VANILLA_ABSTRACT_HORSE_FLAG_STANDING": ("net.minecraft.world.entity.animal.equine.AbstractHorse", "DATA_ID_FLAGS", "FLAG_STANDING"),
    "VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH": ("net.minecraft.world.entity.animal.equine.AbstractHorse", "DATA_ID_FLAGS", "FLAG_OPEN_MOUTH"),
    "VANILLA_ABSTRACT_ARROW_FLAG_CRIT": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "ID_FLAGS", "FLAG_CRIT"),
    "VANILLA_ABSTRACT_ARROW_FLAG_NO_PHYSICS": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "ID_FLAGS", "FLAG_NOPHYSICS"),
    "VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_CLIENT_FLAGS", "CLIENT_FLAG_SMALL"),
    "VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_CLIENT_FLAGS", "CLIENT_FLAG_SHOW_ARMS"),
    "VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_CLIENT_FLAGS", "CLIENT_FLAG_NO_BASEPLATE"),
    "VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER": ("net.minecraft.world.entity.decoration.ArmorStand", "DATA_CLIENT_FLAGS", "CLIENT_FLAG_MARKER"),
}


METHOD_FLAG_ALIASES = {
    "VANILLA_SPIDER_FLAG_CLIMBING": ("net.minecraft.world.entity.monster.spider.Spider", "DATA_FLAGS_ID", "net.minecraft.world.entity.monster.spider.Spider", "isClimbing", "inline_mask"),
    "VANILLA_BLAZE_FLAG_CHARGED": ("net.minecraft.world.entity.monster.Blaze", "DATA_FLAGS_ID", "net.minecraft.world.entity.monster.Blaze", "isCharged", "inline_mask"),
    "VANILLA_IRON_GOLEM_FLAG_PLAYER_CREATED": ("net.minecraft.world.entity.animal.golem.IronGolem", "DATA_FLAGS_ID", "net.minecraft.world.entity.animal.golem.IronGolem", "isPlayerCreated", "inline_mask"),
    "VANILLA_ENTITY_SHARED_FLAG_ON_FIRE": ("net.minecraft.world.entity.Entity", "DATA_SHARED_FLAGS_ID", "net.minecraft.world.entity.Entity", "isOnFire", "shared_bit"),
    "VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN": ("net.minecraft.world.entity.Entity", "DATA_SHARED_FLAGS_ID", "net.minecraft.world.entity.Entity", "isShiftKeyDown", "shared_bit"),
    "VANILLA_ENTITY_SHARED_FLAG_SPRINTING": ("net.minecraft.world.entity.Entity", "DATA_SHARED_FLAGS_ID", "net.minecraft.world.entity.Entity", "isSprinting", "shared_bit"),
    "VANILLA_ENTITY_SHARED_FLAG_SWIMMING": ("net.minecraft.world.entity.Entity", "DATA_SHARED_FLAGS_ID", "net.minecraft.world.entity.Entity", "isSwimming", "shared_bit"),
    "VANILLA_ENTITY_SHARED_FLAG_INVISIBLE": ("net.minecraft.world.entity.Entity", "DATA_SHARED_FLAGS_ID", "net.minecraft.world.entity.Entity", "isInvisible", "shared_bit"),
    "VANILLA_ENTITY_SHARED_FLAG_GLOWING": ("net.minecraft.world.entity.Entity", "DATA_SHARED_FLAGS_ID", "net.minecraft.world.entity.Entity", "isCurrentlyGlowing", "shared_bit"),
    "VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING": ("net.minecraft.world.entity.Entity", "DATA_SHARED_FLAGS_ID", "net.minecraft.world.entity.LivingEntity", "isFallFlying", "shared_bit"),
    "VANILLA_LIVING_ENTITY_FLAG_IS_USING": ("net.minecraft.world.entity.LivingEntity", "DATA_LIVING_ENTITY_FLAGS", "net.minecraft.world.entity.LivingEntity", "isUsingItem", "binary_mask"),
    "VANILLA_LIVING_ENTITY_FLAG_OFF_HAND": ("net.minecraft.world.entity.LivingEntity", "DATA_LIVING_ENTITY_FLAGS", "net.minecraft.world.entity.LivingEntity", "getUsedItemHand", "binary_mask"),
    "VANILLA_LIVING_ENTITY_FLAG_SPIN_ATTACK": ("net.minecraft.world.entity.LivingEntity", "DATA_LIVING_ENTITY_FLAGS", "net.minecraft.world.entity.LivingEntity", "isAutoSpinAttack", "binary_mask"),
    "VANILLA_MOB_FLAG_NO_AI": ("net.minecraft.world.entity.Mob", "DATA_MOB_FLAGS_ID", "net.minecraft.world.entity.Mob", "isNoAi", "binary_mask"),
    "VANILLA_MOB_FLAG_LEFT_HANDED": ("net.minecraft.world.entity.Mob", "DATA_MOB_FLAGS_ID", "net.minecraft.world.entity.Mob", "isLeftHanded", "binary_mask"),
    "VANILLA_MOB_FLAG_AGGRESSIVE": ("net.minecraft.world.entity.Mob", "DATA_MOB_FLAGS_ID", "net.minecraft.world.entity.Mob", "isAggressive", "binary_mask"),
    "VANILLA_TAMABLE_ANIMAL_FLAG_SITTING": ("net.minecraft.world.entity.TamableAnimal", "DATA_FLAGS_ID", "net.minecraft.world.entity.TamableAnimal", "isInSittingPose", "binary_mask"),
    "VANILLA_TAMABLE_ANIMAL_FLAG_TAME": ("net.minecraft.world.entity.TamableAnimal", "DATA_FLAGS_ID", "net.minecraft.world.entity.TamableAnimal", "isTame", "binary_mask"),
    "VANILLA_SHEEP_FLAG_SHEARED": ("net.minecraft.world.entity.animal.sheep.Sheep", "DATA_WOOL_ID", "net.minecraft.world.entity.animal.sheep.Sheep", "isSheared", "inline_mask"),
}


METHOD_VALUE_MASK_ALIASES = {
    "VANILLA_SHEEP_WOOL_COLOR_MASK": (
        "net.minecraft.world.entity.animal.sheep.Sheep",
        "getColor",
        "inline_mask",
    ),
}


METHOD_PACKED_VALUE_ALIASES = {
    "VANILLA_HORSE_COLOR_MASK": (
        "net.minecraft.world.entity.animal.equine.Horse",
        "getVariant",
        0,
        "&",
    ),
    "VANILLA_HORSE_MARKINGS_MASK": (
        "net.minecraft.world.entity.animal.equine.Horse",
        "getMarkings",
        0,
        "&",
    ),
    "VANILLA_HORSE_MARKINGS_SHIFT": (
        "net.minecraft.world.entity.animal.equine.Horse",
        "getMarkings",
        0,
        ">>",
    ),
    "VANILLA_TROPICAL_FISH_BASE_COLOR_MASK": (
        "net.minecraft.world.entity.animal.fish.TropicalFish",
        "getBaseColor",
        1,
        "&",
    ),
    "VANILLA_TROPICAL_FISH_BASE_COLOR_SHIFT": (
        "net.minecraft.world.entity.animal.fish.TropicalFish",
        "getBaseColor",
        1,
        ">>",
    ),
    "VANILLA_TROPICAL_FISH_PATTERN_COLOR_MASK": (
        "net.minecraft.world.entity.animal.fish.TropicalFish",
        "getPatternColor",
        1,
        "&",
    ),
    "VANILLA_TROPICAL_FISH_PATTERN_COLOR_SHIFT": (
        "net.minecraft.world.entity.animal.fish.TropicalFish",
        "getPatternColor",
        1,
        ">>",
    ),
    "VANILLA_TROPICAL_FISH_PATTERN_MASK": (
        "net.minecraft.world.entity.animal.fish.TropicalFish",
        "getPattern",
        1,
        "&",
    ),
}


MODEL_PART_ALIASES = {
    "VANILLA_AVATAR_MODEL_PART_CAPE": "CAPE",
    "VANILLA_AVATAR_MODEL_PART_JACKET": "JACKET",
    "VANILLA_AVATAR_MODEL_PART_LEFT_SLEEVE": "LEFT_SLEEVE",
    "VANILLA_AVATAR_MODEL_PART_RIGHT_SLEEVE": "RIGHT_SLEEVE",
    "VANILLA_AVATAR_MODEL_PART_LEFT_PANTS_LEG": "LEFT_PANTS_LEG",
    "VANILLA_AVATAR_MODEL_PART_RIGHT_PANTS_LEG": "RIGHT_PANTS_LEG",
    "VANILLA_AVATAR_MODEL_PART_HAT": "HAT",
}


ENUM_ID_DEFAULT_RESOLVERS = (
    (
        "net.minecraft.world.entity.animal.cow.MushroomCow",
        "DATA_TYPE",
        "net.minecraft.world.entity.animal.cow.MushroomCow.Variant",
        "DEFAULT",
        1,
        "MushroomCow.Variant.DEFAULT.id",
        "the synced INT default is stored in a nested enum constant rather than an integer expression",
    ),
    (
        "net.minecraft.world.entity.animal.parrot.Parrot",
        "DATA_VARIANT_ID",
        "net.minecraft.world.entity.animal.parrot.Parrot.Variant",
        "DEFAULT",
        0,
        "Parrot.Variant.DEFAULT.id",
        "the synced INT default is stored in a nested enum constant rather than an integer expression",
    ),
    (
        "net.minecraft.world.entity.animal.rabbit.Rabbit",
        "DATA_TYPE_ID",
        "net.minecraft.world.entity.animal.rabbit.Rabbit.Variant",
        "DEFAULT",
        0,
        "Rabbit.Variant.DEFAULT.id",
        "the synced INT default is stored in a nested enum constant rather than an integer expression",
    ),
    (
        "net.minecraft.world.entity.animal.fish.Salmon",
        "DATA_TYPE",
        "net.minecraft.world.entity.animal.fish.Salmon.Variant",
        "DEFAULT",
        1,
        "Salmon.Variant.DEFAULT.id ()",
        "the synced INT default is stored in a nested enum constant rather than an integer expression",
    ),
    (
        "net.minecraft.world.entity.animal.fox.Fox",
        "DATA_TYPE_ID",
        "net.minecraft.world.entity.animal.fox.Fox.Variant",
        "DEFAULT",
        0,
        "Fox.Variant.DEFAULT.getId ()",
        "the synced INT default is stored in a nested enum constant rather than an integer expression",
    ),
)


ENUM_CONSTANT_DEFAULT_RESOLVERS = (
    (
        "net.minecraft.world.entity.animal.armadillo.Armadillo",
        "ARMADILLO_STATE",
        "ARMADILLO_STATE",
        "net.minecraft.world.entity.animal.armadillo.Armadillo.ArmadilloState",
        "IDLE",
        3,
        "Armadillo.ArmadilloState.IDLE",
        "the synced enum default is encoded by an explicit enum constructor id",
    ),
    (
        "net.minecraft.world.entity.animal.golem.CopperGolem",
        "COPPER_GOLEM_STATE",
        "COPPER_GOLEM_STATE",
        "net.minecraft.world.entity.animal.golem.CopperGolemState",
        "IDLE",
        1,
        "CopperGolemState.IDLE",
        "the synced enum default is encoded by an explicit enum constructor id",
    ),
    (
        "net.minecraft.world.entity.animal.sniffer.Sniffer",
        "DATA_STATE",
        "SNIFFER_STATE",
        "net.minecraft.world.entity.animal.sniffer.Sniffer.State",
        "IDLING",
        0,
        "Sniffer.State.IDLING",
        "the synced enum default is encoded by an explicit enum constructor id",
    ),
)


ENUM_ORDINAL_DEFAULT_RESOLVERS = (
    (
        "net.minecraft.world.entity.animal.golem.CopperGolem",
        "DATA_WEATHER_STATE",
        "WEATHERING_COPPER_STATE",
        "net.minecraft.world.level.block.WeatheringCopper.WeatherState",
        "UNAFFECTED",
        "WeatheringCopper.WeatherState.UNAFFECTED",
        "the synced enum default is encoded by the enum ordinal",
    ),
)


STATIC_FACTORY_ORDINAL_DEFAULT_RESOLVERS = (
    (
        "net.minecraft.world.entity.boss.enderdragon.EnderDragon",
        "DATA_PHASE",
        "INT",
        "EnderDragonPhase.HOVERING.getId ()",
        "net.minecraft.world.entity.boss.enderdragon.phases.EnderDragonPhase",
        "HOVERING",
        "the phase id is assigned from static create-field registration order",
    ),
)


STATIC_ITEM_STACK_DEFAULT_RESOLVERS = (
    (
        "net.minecraft.world.entity.projectile.hurtingprojectile.Fireball",
        "DATA_ITEM_STACK",
        "getDefaultItem",
        "this.getDefaultItem ()",
        "new ItemStack (Items.FIRE_CHARGE)",
        "minecraft:fire_charge",
        "the synced item-stack default is returned by a source method rather than an inline expression",
    ),
    (
        "net.minecraft.world.entity.projectile.FireworkRocketEntity",
        "DATA_ID_FIREWORKS_ITEM",
        "getDefaultItem",
        "getDefaultItem ()",
        "new ItemStack (Items.FIREWORK_ROCKET)",
        "minecraft:firework_rocket",
        "the synced item-stack default is returned by a source method rather than an inline expression",
    ),
)


STATIC_BLOCK_STATE_DEFAULT_RESOLVERS = (
    (
        "net.minecraft.world.entity.item.PrimedTnt",
        "DATA_BLOCK_STATE_ID",
        "DEFAULT_BLOCK_STATE",
        "DEFAULT_BLOCK_STATE",
        "Blocks.TNT.defaultBlockState ()",
        "net.minecraft.world.level.block.TntBlock",
        "registerDefaultState",
        "this.registerDefaultState (this.defaultBlockState ().setValue (UNSTABLE, false))",
        "minecraft:tnt",
        (("unstable", "false"),),
        "the synced block-state default is split between the entity field and the block constructor default-state declaration",
    ),
)


TROPICAL_FISH_PACKED_DEFAULT_RESOLVERS = (
    (
        "net.minecraft.world.entity.animal.fish.TropicalFish",
        "DATA_ID_TYPE_VARIANT",
        "DEFAULT_VARIANT",
        "DEFAULT_VARIANT.getPackedId ()",
        "new TropicalFish.Variant (TropicalFish.Pattern.KOB, DyeColor.WHITE, DyeColor.WHITE)",
        "net.minecraft.world.entity.animal.fish.TropicalFish.Pattern",
        "net.minecraft.world.entity.animal.fish.TropicalFish.Base",
        "net.minecraft.world.item.DyeColor",
        "this.packedId = base.id | index << 8",
        "pattern.getPackedId () & 65535 | (baseColor.getId () & 0xFF) << 16 | (patternColor.getId () & 0xFF) << 24",
        "the synced INT default is a packed record value derived from explicit pattern/base/color constructor ids",
    ),
)


STATIC_REGISTRY_RESOURCE_DEFAULT_RESOLVERS = (
    (
        "net.minecraft.world.entity.animal.feline.Cat",
        "DATA_VARIANT_ID",
        "CAT_VARIANT",
        "VariantUtils.getDefaultOrAny (this.registryAccess (), DEFAULT_VARIANT)",
        (
            (
                "net.minecraft.world.entity.animal.feline.Cat",
                "DEFAULT_VARIANT",
                "CatVariants.BLACK",
            ),
            (
                "net.minecraft.world.entity.animal.feline.CatVariants",
                "BLACK",
                'createKey ("black")',
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.feline.CatVariants",
                "createKey",
                "ResourceKey.create (Registries.CAT_VARIANT, Identifier.withDefaultNamespace (name))",
            ),
        ),
        None,
        "minecraft:black",
        "the synced holder default is a resource key selected through VariantUtils rather than a numeric registry id",
    ),
    (
        "net.minecraft.world.entity.animal.feline.Cat",
        "DATA_SOUND_VARIANT_ID",
        "CAT_SOUND_VARIANT",
        "catSoundVariants.get (CatSoundVariants.CLASSIC).or (catSoundVariants::getAny).orElseThrow ()",
        (
            (
                "net.minecraft.world.entity.animal.feline.CatSoundVariants",
                "CLASSIC",
                "createKey (CatSoundVariants.SoundSet.CLASSIC)",
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.feline.CatSoundVariants",
                "createKey",
                "ResourceKey.create (Registries.CAT_SOUND_VARIANT, Identifier.withDefaultNamespace (catSoundVariant.getIdentifier ()))",
            ),
            (
                "net.minecraft.world.entity.animal.feline.CatSoundVariants.SoundSet",
                "getIdentifier",
                "this.identifier",
            ),
        ),
        (
            "net.minecraft.world.entity.animal.feline.CatSoundVariants.SoundSet",
            "CLASSIC",
            0,
            '"classic"',
        ),
        "minecraft:classic",
        "the synced holder default is a resource key with registry fallback rather than a numeric registry id",
    ),
    (
        "net.minecraft.world.entity.animal.wolf.Wolf",
        "DATA_VARIANT_ID",
        "WOLF_VARIANT",
        "VariantUtils.getDefaultOrAny (this.registryAccess (), WolfVariants.DEFAULT)",
        (
            (
                "net.minecraft.world.entity.animal.wolf.WolfVariants",
                "DEFAULT",
                "PALE",
            ),
            (
                "net.minecraft.world.entity.animal.wolf.WolfVariants",
                "PALE",
                'createKey ("pale")',
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.wolf.WolfVariants",
                "createKey",
                "ResourceKey.create (Registries.WOLF_VARIANT, Identifier.withDefaultNamespace (name))",
            ),
        ),
        None,
        "minecraft:pale",
        "the synced holder default is a resource key selected through VariantUtils rather than a numeric registry id",
    ),
    (
        "net.minecraft.world.entity.animal.wolf.Wolf",
        "DATA_SOUND_VARIANT_ID",
        "WOLF_SOUND_VARIANT",
        "wolfSoundVariants.get (WolfSoundVariants.CLASSIC).or (wolfSoundVariants::getAny).orElseThrow ()",
        (
            (
                "net.minecraft.world.entity.animal.wolf.WolfSoundVariants",
                "CLASSIC",
                "createKey (WolfSoundVariants.SoundSet.CLASSIC)",
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.wolf.WolfSoundVariants",
                "createKey",
                "ResourceKey.create (Registries.WOLF_SOUND_VARIANT, Identifier.withDefaultNamespace (wolfSoundVariant.getIdentifier ()))",
            ),
            (
                "net.minecraft.world.entity.animal.wolf.WolfSoundVariants.SoundSet",
                "getIdentifier",
                "this.identifier",
            ),
        ),
        (
            "net.minecraft.world.entity.animal.wolf.WolfSoundVariants.SoundSet",
            "CLASSIC",
            0,
            '"classic"',
        ),
        "minecraft:classic",
        "the synced holder default is a resource key with registry fallback rather than a numeric registry id",
    ),
    (
        "net.minecraft.world.entity.animal.chicken.Chicken",
        "DATA_VARIANT_ID",
        "CHICKEN_VARIANT",
        "VariantUtils.getDefaultOrAny (this.registryAccess (), ChickenVariants.TEMPERATE)",
        (
            (
                "net.minecraft.world.entity.animal.chicken.ChickenVariants",
                "TEMPERATE",
                "createKey (TemperatureVariants.TEMPERATE)",
            ),
            (
                "net.minecraft.world.entity.animal.TemperatureVariants",
                "TEMPERATE",
                'Identifier.withDefaultNamespace ("temperate")',
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.chicken.ChickenVariants",
                "createKey",
                "ResourceKey.create (Registries.CHICKEN_VARIANT, id)",
            ),
        ),
        None,
        "minecraft:temperate",
        "the synced holder default is a resource key selected through TemperatureVariants rather than a numeric registry id",
    ),
    (
        "net.minecraft.world.entity.animal.chicken.Chicken",
        "DATA_SOUND_VARIANT_ID",
        "CHICKEN_SOUND_VARIANT",
        "chickenSoundVariants.get (ChickenSoundVariants.CLASSIC).or (chickenSoundVariants::getAny).orElseThrow ()",
        (
            (
                "net.minecraft.world.entity.animal.chicken.ChickenSoundVariants",
                "CLASSIC",
                "createKey (ChickenSoundVariants.SoundSet.CLASSIC)",
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.chicken.ChickenSoundVariants",
                "createKey",
                "ResourceKey.create (Registries.CHICKEN_SOUND_VARIANT, Identifier.withDefaultNamespace (chickenSoundVariant.getIdentifier ()))",
            ),
            (
                "net.minecraft.world.entity.animal.chicken.ChickenSoundVariants.SoundSet",
                "getIdentifier",
                "this.identifier",
            ),
        ),
        (
            "net.minecraft.world.entity.animal.chicken.ChickenSoundVariants.SoundSet",
            "CLASSIC",
            0,
            '"classic"',
        ),
        "minecraft:classic",
        "the synced holder default is a resource key with registry fallback rather than a numeric registry id",
    ),
    (
        "net.minecraft.world.entity.animal.cow.Cow",
        "DATA_VARIANT_ID",
        "COW_VARIANT",
        "VariantUtils.getDefaultOrAny (this.registryAccess (), CowVariants.TEMPERATE)",
        (
            (
                "net.minecraft.world.entity.animal.cow.CowVariants",
                "TEMPERATE",
                "createKey (TemperatureVariants.TEMPERATE)",
            ),
            (
                "net.minecraft.world.entity.animal.TemperatureVariants",
                "TEMPERATE",
                'Identifier.withDefaultNamespace ("temperate")',
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.cow.CowVariants",
                "createKey",
                "ResourceKey.create (Registries.COW_VARIANT, id)",
            ),
        ),
        None,
        "minecraft:temperate",
        "the synced holder default is a resource key selected through TemperatureVariants rather than a numeric registry id",
    ),
    (
        "net.minecraft.world.entity.animal.cow.Cow",
        "DATA_SOUND_VARIANT_ID",
        "COW_SOUND_VARIANT",
        "cowSoundVariants.get (CowSoundVariants.CLASSIC).or (cowSoundVariants::getAny).orElseThrow ()",
        (
            (
                "net.minecraft.world.entity.animal.cow.CowSoundVariants",
                "CLASSIC",
                "createKey (CowSoundVariants.SoundSet.CLASSIC)",
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.cow.CowSoundVariants",
                "createKey",
                "ResourceKey.create (Registries.COW_SOUND_VARIANT, Identifier.withDefaultNamespace (cowSoundVariant.getIdentifier ()))",
            ),
            (
                "net.minecraft.world.entity.animal.cow.CowSoundVariants.SoundSet",
                "getIdentifier",
                "this.identifier",
            ),
        ),
        (
            "net.minecraft.world.entity.animal.cow.CowSoundVariants.SoundSet",
            "CLASSIC",
            0,
            '"classic"',
        ),
        "minecraft:classic",
        "the synced holder default is a resource key with registry fallback rather than a numeric registry id",
    ),
    (
        "net.minecraft.world.entity.animal.pig.Pig",
        "DATA_VARIANT_ID",
        "PIG_VARIANT",
        "VariantUtils.getDefaultOrAny (this.registryAccess (), PigVariants.DEFAULT)",
        (
            (
                "net.minecraft.world.entity.animal.pig.PigVariants",
                "DEFAULT",
                "TEMPERATE",
            ),
            (
                "net.minecraft.world.entity.animal.pig.PigVariants",
                "TEMPERATE",
                "createKey (TemperatureVariants.TEMPERATE)",
            ),
            (
                "net.minecraft.world.entity.animal.TemperatureVariants",
                "TEMPERATE",
                'Identifier.withDefaultNamespace ("temperate")',
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.pig.PigVariants",
                "createKey",
                "ResourceKey.create (Registries.PIG_VARIANT, id)",
            ),
        ),
        None,
        "minecraft:temperate",
        "the synced holder default is a resource key selected through PigVariants.DEFAULT and TemperatureVariants",
    ),
    (
        "net.minecraft.world.entity.animal.pig.Pig",
        "DATA_SOUND_VARIANT_ID",
        "PIG_SOUND_VARIANT",
        "pigSoundVariants.get (PigSoundVariants.CLASSIC).or (pigSoundVariants::getAny).orElseThrow ()",
        (
            (
                "net.minecraft.world.entity.animal.pig.PigSoundVariants",
                "CLASSIC",
                "createKey (PigSoundVariants.SoundSet.CLASSIC)",
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.pig.PigSoundVariants",
                "createKey",
                "ResourceKey.create (Registries.PIG_SOUND_VARIANT, Identifier.withDefaultNamespace (pigSoundVariant.getIdentifier ()))",
            ),
            (
                "net.minecraft.world.entity.animal.pig.PigSoundVariants.SoundSet",
                "getIdentifier",
                "this.identifier",
            ),
        ),
        (
            "net.minecraft.world.entity.animal.pig.PigSoundVariants.SoundSet",
            "CLASSIC",
            0,
            '"classic"',
        ),
        "minecraft:classic",
        "the synced holder default is a resource key with registry fallback rather than a numeric registry id",
    ),
    (
        "net.minecraft.world.entity.animal.frog.Frog",
        "DATA_VARIANT_ID",
        "FROG_VARIANT",
        "VariantUtils.getDefaultOrAny (this.registryAccess (), DEFAULT_VARIANT)",
        (
            (
                "net.minecraft.world.entity.animal.frog.Frog",
                "DEFAULT_VARIANT",
                "FrogVariants.TEMPERATE",
            ),
            (
                "net.minecraft.world.entity.animal.frog.FrogVariants",
                "TEMPERATE",
                "createKey (TemperatureVariants.TEMPERATE)",
            ),
            (
                "net.minecraft.world.entity.animal.TemperatureVariants",
                "TEMPERATE",
                'Identifier.withDefaultNamespace ("temperate")',
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.frog.FrogVariants",
                "createKey",
                "ResourceKey.create (Registries.FROG_VARIANT, id)",
            ),
        ),
        None,
        "minecraft:temperate",
        "the synced holder default is a resource key selected through Frog.DEFAULT_VARIANT and TemperatureVariants",
    ),
    (
        "net.minecraft.world.entity.animal.nautilus.ZombieNautilus",
        "DATA_VARIANT_ID",
        "ZOMBIE_NAUTILUS_VARIANT",
        "VariantUtils.getDefaultOrAny (this.registryAccess (), ZombieNautilusVariants.TEMPERATE)",
        (
            (
                "net.minecraft.world.entity.animal.nautilus.ZombieNautilusVariants",
                "TEMPERATE",
                "createKey (TemperatureVariants.TEMPERATE)",
            ),
            (
                "net.minecraft.world.entity.animal.TemperatureVariants",
                "TEMPERATE",
                'Identifier.withDefaultNamespace ("temperate")',
            ),
        ),
        (
            (
                "net.minecraft.world.entity.animal.nautilus.ZombieNautilusVariants",
                "createKey",
                "ResourceKey.create (Registries.ZOMBIE_NAUTILUS_VARIANT, id)",
            ),
        ),
        None,
        "minecraft:temperate",
        "the synced holder default is a resource key selected through ZombieNautilusVariants.TEMPERATE and TemperatureVariants",
    ),
)


REGISTRY_ANY_DEFAULT_RESOLVERS = (
    (
        "net.minecraft.world.entity.decoration.painting.Painting",
        "DATA_PAINTING_VARIANT_ID",
        "PAINTING_VARIANT",
        "VariantUtils.getAny (this.registryAccess (), Registries.PAINTING_VARIANT)",
        "net.minecraft.core.registries.Registries",
        "PAINTING_VARIANT",
        'createRegistryKey ("painting_variant")',
        "createRegistryKey",
        "ResourceKey.createRegistryKey (Identifier.withDefaultNamespace (name))",
        "minecraft:painting_variant",
        "the synced holder default is an arbitrary entry from the named registry",
    ),
)


ENUM_FIELD_ID_DEFAULT_RESOLVERS = (
    (
        "net.minecraft.world.entity.animal.feline.Cat",
        "DATA_COLLAR_COLOR",
        "DEFAULT_COLLAR_COLOR.getId ()",
        "DEFAULT_COLLAR_COLOR",
        "DyeColor.RED",
        "net.minecraft.world.item.DyeColor",
        0,
        "the synced INT default is stored in a static enum-valued field",
    ),
    (
        "net.minecraft.world.entity.animal.wolf.Wolf",
        "DATA_COLLAR_COLOR",
        "DEFAULT_COLLAR_COLOR.getId ()",
        "DEFAULT_COLLAR_COLOR",
        "DyeColor.RED",
        "net.minecraft.world.item.DyeColor",
        0,
        "the synced INT default is stored in a static enum-valued field",
    ),
)


ENUM_CONSTRUCTOR_ID_ALIASES = {
    "VANILLA_RABBIT_VARIANT_EVIL_ID": ("net.minecraft.world.entity.animal.rabbit.Rabbit.Variant", "EVIL", 0),
    "VANILLA_ARMADILLO_STATE_IDLE_ID": ("net.minecraft.world.entity.animal.armadillo.Armadillo.ArmadilloState", "IDLE", 3),
    "VANILLA_ARMADILLO_STATE_ROLLING_ID": ("net.minecraft.world.entity.animal.armadillo.Armadillo.ArmadilloState", "ROLLING", 3),
    "VANILLA_ARMADILLO_STATE_SCARED_ID": ("net.minecraft.world.entity.animal.armadillo.Armadillo.ArmadilloState", "SCARED", 3),
    "VANILLA_ARMADILLO_STATE_UNROLLING_ID": ("net.minecraft.world.entity.animal.armadillo.Armadillo.ArmadilloState", "UNROLLING", 3),
    "VANILLA_COPPER_GOLEM_STATE_IDLE_ID": ("net.minecraft.world.entity.animal.golem.CopperGolemState", "IDLE", 1),
    "VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID": ("net.minecraft.world.entity.animal.golem.CopperGolemState", "GETTING_ITEM", 1),
    "VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID": ("net.minecraft.world.entity.animal.golem.CopperGolemState", "GETTING_NO_ITEM", 1),
    "VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID": ("net.minecraft.world.entity.animal.golem.CopperGolemState", "DROPPING_ITEM", 1),
    "VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID": ("net.minecraft.world.entity.animal.golem.CopperGolemState", "DROPPING_NO_ITEM", 1),
    "VANILLA_SNIFFER_STATE_IDLING_ID": ("net.minecraft.world.entity.animal.sniffer.Sniffer.State", "IDLING", 0),
    "VANILLA_SNIFFER_STATE_FEELING_HAPPY_ID": ("net.minecraft.world.entity.animal.sniffer.Sniffer.State", "FEELING_HAPPY", 0),
    "VANILLA_SNIFFER_STATE_SCENTING_ID": ("net.minecraft.world.entity.animal.sniffer.Sniffer.State", "SCENTING", 0),
    "VANILLA_SNIFFER_STATE_SNIFFING_ID": ("net.minecraft.world.entity.animal.sniffer.Sniffer.State", "SNIFFING", 0),
    "VANILLA_SNIFFER_STATE_SEARCHING_ID": ("net.minecraft.world.entity.animal.sniffer.Sniffer.State", "SEARCHING", 0),
    "VANILLA_SNIFFER_STATE_DIGGING_ID": ("net.minecraft.world.entity.animal.sniffer.Sniffer.State", "DIGGING", 0),
    "VANILLA_SNIFFER_STATE_RISING_ID": ("net.minecraft.world.entity.animal.sniffer.Sniffer.State", "RISING", 0),
}


ENUM_ORDINAL_ID_ALIASES = {
    "VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID": ("net.minecraft.world.level.block.WeatheringCopper.WeatherState", "UNAFFECTED"),
    "VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID": ("net.minecraft.world.level.block.WeatheringCopper.WeatherState", "EXPOSED"),
    "VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID": ("net.minecraft.world.level.block.WeatheringCopper.WeatherState", "WEATHERED"),
    "VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID": ("net.minecraft.world.level.block.WeatheringCopper.WeatherState", "OXIDIZED"),
}


STATIC_FACTORY_ORDINAL_SOURCES = {
    "net.minecraft.world.entity.boss.enderdragon.phases.EnderDragonPhase": (
        "create",
        "new EnderDragonPhase <> (phases.length, instanceClass, name)",
        "phases = Arrays.copyOf (phases, phases.length + 1)",
        "phases [phase.getId ()] = phase",
        "return phase;",
    ),
}


STATIC_FACTORY_ORDINAL_ID_ALIASES = {
    "VANILLA_ENDER_DRAGON_PHASE_HOLDING_PATTERN_ID": (
        "net.minecraft.world.entity.boss.enderdragon.phases.EnderDragonPhase",
        "HOLDING_PATTERN",
    ),
}


ENUM_CONSTRUCTOR_ID_CODEC_EXPRESSIONS = {
    "net.minecraft.world.entity.animal.rabbit.Rabbit.Variant": "ByteBufCodecs.idMapper (BY_ID, Rabbit.Variant::id)",
    "net.minecraft.world.entity.animal.armadillo.Armadillo.ArmadilloState": "ByteBufCodecs.idMapper (BY_ID, Armadillo.ArmadilloState::id)",
    "net.minecraft.world.entity.animal.golem.CopperGolemState": "ByteBufCodecs.idMapper (BY_ID, CopperGolemState::id)",
    "net.minecraft.world.entity.animal.fish.TropicalFish.Pattern": "ByteBufCodecs.idMapper (BY_ID, TropicalFish.Pattern::getPackedId)",
    "net.minecraft.world.entity.animal.sniffer.Sniffer.State": "ByteBufCodecs.idMapper (BY_ID, Sniffer.State::id)",
    "net.minecraft.world.item.DyeColor": "ByteBufCodecs.idMapper (BY_ID, DyeColor::getId)",
}


ENUM_ORDINAL_CODEC_EXPRESSIONS = {
    "net.minecraft.world.level.block.WeatheringCopper.WeatherState": "ByteBufCodecs.idMapper (BY_ID, Enum::ordinal)",
}


@dataclass(frozen=True)
class NormalizedDefault:
    rust: str
    scalar_type: str | None = None
    scalar_rust: str | None = None


@dataclass(frozen=True)
class ResolvedFlag:
    rust_name: str
    owner: str
    accessor: str
    evidence_class: str
    evidence_member: str
    expression: str
    value: int
    reason: str


@dataclass(frozen=True)
class ResolvedDefaultEvidence:
    owner: str
    accessor: str
    evidence_class: str
    evidence_member: str
    expression: str
    reason: str


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("update", "check"))
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--protocol-output", type=Path, default=DEFAULT_PROTOCOL_OUTPUT
    )
    parser.add_argument("--sound-source-output", type=Path, default=DEFAULT_SOUND_SOURCE_OUTPUT)
    args = parser.parse_args()
    args.source_root = resolve_source_root(args)
    return args


def field_for(result: ExtractionResult, owner: str, name: str) -> JavaField:
    matches = [item for item in source_for_owner(result, owner).fields_for(owner) if item.name == name]
    if len(matches) != 1:
        raise ExtractionError(f"{owner}.{name}: expected one field, found {len(matches)}")
    return matches[0]


def accessor_for(result: ExtractionResult, owner: str, name: str) -> Accessor:
    matches = [item for item in result.accessors if item.owner == owner and item.java_name == name]
    if len(matches) != 1:
        raise ExtractionError(f"{owner}.{name}: expected one accessor, found {len(matches)}")
    return matches[0]


def integer_constants(result: ExtractionResult, owner: str) -> dict[str, int]:
    source = source_for_owner(result, owner)
    fields = [
        item
        for item in source.fields_for(owner)
        if item.initializer is not None
        and item.type_spelling.replace(" ", "") in {"byte", "int", "long", "short"}
    ]
    resolved: dict[str, int] = {}
    pending = {item.name: item for item in fields}
    changed = True
    while changed:
        changed = False
        for name, item in list(pending.items()):
            try:
                value = evaluate_integer_expression(source, item.initializer.node, resolved)
            except (ExtractionError, ValueError):
                continue
            resolved[name] = value
            del pending[name]
            changed = True
    return resolved


def resolve_default_node(result: ExtractionResult, accessor: Accessor) -> tuple[JavaSource, Node]:
    if accessor.default_node is None:
        raise ExtractionError(f"{accessor.owner}.{accessor.java_name}: missing default node")
    source = result.sources[accessor.source_path]
    node = accessor.default_node
    seen: set[str] = set()
    while node.type == "identifier":
        name = source.text(node)
        if name in seen:
            raise ExtractionError(f"{accessor.owner}.{accessor.java_name}: cyclic default constant {name}")
        seen.add(name)
        field = field_for(result, accessor.owner, name)
        if field.initializer is None:
            break
        node = field.initializer.node
    return source, node


def enum_ordinals(result: ExtractionResult, fqcn: str) -> dict[str, int]:
    matches = [
        (source, java_type)
        for source in result.sources.values()
        for java_type in source.types
        if java_type.fqcn == fqcn
    ]
    if len(matches) != 1:
        raise ExtractionError(f"{fqcn}: expected one enum type, found {len(matches)}")
    source, java_type = matches[0]
    constants = [item for item in java_type.body_node.named_children if item.type == "enum_constant"]
    return {
        source.text(item.child_by_field_name("name")): ordinal
        for ordinal, item in enumerate(constants)
    }


def validate_enum_stream_codec(
    result: ExtractionResult,
    enum_fqcn: str,
    expected_expression: str,
) -> None:
    stream_codec = field_for(result, enum_fqcn, "STREAM_CODEC")
    if stream_codec.initializer is None:
        raise ExtractionError(f"{enum_fqcn}.STREAM_CODEC: missing initializer")
    if stream_codec.initializer.canonical != expected_expression:
        raise ExtractionError(
            f"{enum_fqcn}.STREAM_CODEC: stale enum id resolver expression; "
            f"expected {expected_expression!r}, found {stream_codec.initializer.canonical!r}"
        )


def enum_constant_ordinal(
    result: ExtractionResult,
    enum_fqcn: str,
    constant_name: str,
) -> int:
    expected_codec = ENUM_ORDINAL_CODEC_EXPRESSIONS.get(enum_fqcn)
    if expected_codec is None:
        raise ExtractionError(f"{enum_fqcn}: missing ordinal StreamCodec evidence")
    validate_enum_stream_codec(result, enum_fqcn, expected_codec)
    ordinals = enum_ordinals(result, enum_fqcn)
    if constant_name not in ordinals:
        raise ExtractionError(f"{enum_fqcn}: missing {constant_name} enum constant")
    return ordinals[constant_name]


def enum_constant_arguments(
    result: ExtractionResult,
    enum_fqcn: str,
    constant_name: str,
) -> tuple[JavaSource, tuple[Node, ...]]:
    matches = [
        (source, java_type)
        for source in result.sources.values()
        for java_type in source.types
        if java_type.fqcn == enum_fqcn and java_type.kind == "enum"
    ]
    if len(matches) != 1:
        raise ExtractionError(f"{enum_fqcn}: expected one enum type, found {len(matches)}")
    enum_source, java_type = matches[0]
    constants = [
        item
        for item in java_type.body_node.named_children
        if item.type == "enum_constant"
        and enum_source.text(item.child_by_field_name("name")) == constant_name
    ]
    if len(constants) != 1:
        raise ExtractionError(f"{enum_fqcn}: expected one {constant_name} enum constant")
    arguments = constants[0].child_by_field_name("arguments")
    if arguments is None:
        raise ExtractionError(f"{enum_fqcn}.{constant_name}: expected constructor arguments")
    return enum_source, tuple(arguments.named_children)


def enum_constant_constructor_integer(
    result: ExtractionResult,
    enum_fqcn: str,
    constant_name: str,
    argument_index: int,
) -> int:
    expected_codec = ENUM_CONSTRUCTOR_ID_CODEC_EXPRESSIONS.get(enum_fqcn)
    if expected_codec is not None:
        validate_enum_stream_codec(result, enum_fqcn, expected_codec)
    enum_source, arguments = enum_constant_arguments(result, enum_fqcn, constant_name)
    if argument_index < 0 or argument_index >= len(arguments):
        raise ExtractionError(
            f"{enum_fqcn}.{constant_name}: expected integer id constructor argument "
            f"at index {argument_index}"
        )
    return evaluate_integer_expression(
        enum_source,
        arguments[argument_index],
        {},
    )


def static_factory_registration_ordinals(
    result: ExtractionResult,
    owner: str,
) -> dict[str, int]:
    source_spec = STATIC_FACTORY_ORDINAL_SOURCES.get(owner)
    if source_spec is None:
        raise ExtractionError(f"{owner}: missing static-factory ordinal source evidence")
    (
        factory_name,
        expected_allocator_expression,
        expected_growth_expression,
        expected_store_expression,
        expected_return_expression,
    ) = source_spec
    source = source_for_owner(result, owner)
    _factory_source, factory = method_for(result, owner, factory_name)
    body = factory.node.child_by_field_name("body")
    if body is None:
        raise ExtractionError(f"{owner}.{factory_name}: factory lacks a body")
    allocator_expressions = [
        source.expression(node).canonical
        for node in descendants(body)
        if node.type == "object_creation_expression"
    ]
    if allocator_expressions.count(expected_allocator_expression) != 1:
        raise ExtractionError(
            f"{owner}.{factory_name}: expected one allocator {expected_allocator_expression!r}, "
            f"found {allocator_expressions}"
        )
    assignment_expressions = [
        source.expression(node).canonical
        for node in descendants(body)
        if node.type == "assignment_expression"
    ]
    for expected in (expected_growth_expression, expected_store_expression):
        if assignment_expressions.count(expected) != 1:
            raise ExtractionError(
                f"{owner}.{factory_name}: expected one assignment {expected!r}, "
                f"found {assignment_expressions}"
            )
    return_expressions = [
        source.expression(node).canonical
        for node in descendants(body)
        if node.type == "return_statement"
    ]
    if return_expressions != [expected_return_expression]:
        raise ExtractionError(
            f"{owner}.{factory_name}: expected return {expected_return_expression!r}, "
            f"found {return_expressions}"
        )

    registrations: dict[str, int] = {}
    for field in source.fields_for(owner):
        if field.initializer is None or field.initializer.node.type != "method_invocation":
            continue
        name_node = field.initializer.node.child_by_field_name("name")
        if name_node is None or source.text(name_node) != factory_name:
            continue
        registrations[field.name] = len(registrations)
    if not registrations:
        raise ExtractionError(f"{owner}.{factory_name}: no static factory registrations found")
    return registrations


def static_factory_registration_classes(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
) -> dict[str, tuple[int, str]]:
    """Resolve the implementation class passed to each ordered factory registration."""
    ordinals = static_factory_registration_ordinals(result, owner)
    source = source_for_owner(result, owner)
    owner_type = class_map.get(owner)
    if owner_type is None:
        raise ExtractionError(f"{owner}: missing factory owner class")
    factory_name = STATIC_FACTORY_ORDINAL_SOURCES[owner][0]

    registrations: dict[str, tuple[int, str]] = {}
    for field in source.fields_for(owner):
        if field.name not in ordinals or field.initializer is None:
            continue
        arguments = field.initializer.node.child_by_field_name("arguments")
        if arguments is None or len(arguments.named_children) != 2:
            raise ExtractionError(
                f"{owner}.{field.name}: {factory_name} registration requires class and name arguments"
            )
        class_argument, name_argument = arguments.named_children
        if class_argument.type != "class_literal" or name_argument.type != "string_literal":
            raise ExtractionError(
                f"{owner}.{field.name}: unsupported {factory_name} registration arguments"
            )
        class_literal = source.text(class_argument)
        if not class_literal.endswith(".class"):
            raise ExtractionError(
                f"{owner}.{field.name}: expected implementation class literal"
            )
        implementation_class = resolve_type_reference(
            class_literal.removesuffix(".class"), owner_type, class_map
        )
        if implementation_class is None:
            raise ExtractionError(
                f"{owner}.{field.name}: unresolved implementation class {class_literal}"
            )
        registrations[field.name] = (ordinals[field.name], implementation_class)

    if registrations.keys() != ordinals.keys():
        missing = sorted(ordinals.keys() - registrations.keys())
        raise ExtractionError(
            f"{owner}.{factory_name}: registrations missing implementation classes: {missing}"
        )
    return registrations


def resolve_semantic_defaults(
    result: ExtractionResult,
) -> tuple[dict[tuple[str, str], NormalizedDefault], list[ResolvedDefaultEvidence]]:
    resolved: dict[tuple[str, str], NormalizedDefault] = {}
    evidence: list[ResolvedDefaultEvidence] = []
    for (
        owner,
        accessor_name,
        enum_fqcn,
        default_field,
        id_argument_index,
        expected_expression,
        reason,
    ) in ENUM_ID_DEFAULT_RESOLVERS:
        key = (owner, accessor_name)
        if key in resolved:
            raise ExtractionError(f"duplicate enum default resolver: {owner}.{accessor_name}")
        accessor = accessor_for(result, owner, accessor_name)
        if accessor.serializer != "INT":
            raise ExtractionError(f"{owner}.{accessor_name}: enum id resolver requires INT serializer")
        if accessor.default_expression != expected_expression:
            raise ExtractionError(
                f"{owner}.{accessor_name}: stale enum default resolver expression; "
                f"expected {expected_expression!r}, found {accessor.default_expression!r}"
            )
        default = field_for(result, enum_fqcn, default_field)
        if default.initializer is None or default.initializer.node.type != "identifier":
            raise ExtractionError(f"{enum_fqcn}.{default_field}: expected enum-constant identifier initializer")
        constant_name = source_for_owner(result, enum_fqcn).text(default.initializer.node)
        value = enum_constant_constructor_integer(
            result,
            enum_fqcn,
            constant_name,
            id_argument_index,
        )
        resolved[key] = NormalizedDefault(
            f"VanillaEntityMetadataDefault::Int({value})",
            "i32",
            str(value),
        )
        evidence.append(
            ResolvedDefaultEvidence(
                owner,
                accessor_name,
                enum_fqcn,
                f"{default_field} -> {constant_name}",
                expected_expression,
                reason,
            )
        )
    for (
        owner,
        accessor_name,
        serializer,
        expected_expression,
        registry_owner,
        registry_field_name,
        expected_registry_initializer,
        registry_factory_name,
        expected_registry_factory_return,
        registry_resource_id,
        reason,
    ) in REGISTRY_ANY_DEFAULT_RESOLVERS:
        key = (owner, accessor_name)
        if key in resolved:
            raise ExtractionError(f"duplicate registry-any default resolver: {owner}.{accessor_name}")
        accessor = accessor_for(result, owner, accessor_name)
        if accessor.serializer != serializer:
            raise ExtractionError(
                f"{owner}.{accessor_name}: registry-any resolver requires {serializer} serializer"
            )
        if accessor.default_expression != expected_expression:
            raise ExtractionError(
                f"{owner}.{accessor_name}: stale registry-any expression; "
                f"expected {expected_expression!r}, found {accessor.default_expression!r}"
            )
        registry_field = field_for(result, registry_owner, registry_field_name)
        if registry_field.initializer is None:
            raise ExtractionError(f"{registry_owner}.{registry_field_name}: missing initializer")
        if registry_field.initializer.canonical != expected_registry_initializer:
            raise ExtractionError(
                f"{registry_owner}.{registry_field_name}: stale registry key initializer; "
                f"expected {expected_registry_initializer!r}, "
                f"found {registry_field.initializer.canonical!r}"
            )
        factory_return = method_single_return_expression(
            result,
            registry_owner,
            registry_factory_name,
        )
        if factory_return != expected_registry_factory_return:
            raise ExtractionError(
                f"{registry_owner}.{registry_factory_name}: stale registry key factory; "
                f"expected {expected_registry_factory_return!r}, found {factory_return!r}"
            )
        resource_rust = rust_string(registry_resource_id)
        resolved[key] = NormalizedDefault(
            f"VanillaEntityMetadataDefault::RegistryAny({resource_rust})"
        )
        evidence.append(
            ResolvedDefaultEvidence(
                owner,
                accessor_name,
                registry_owner,
                f"{registry_field_name} -> {registry_factory_name}",
                f"{expected_expression} -> {expected_registry_initializer} -> {expected_registry_factory_return}",
                reason,
            )
        )
    for (
        owner,
        accessor_name,
        serializer,
        expected_expression,
        registry_class,
        constant_name,
        reason,
    ) in STATIC_FACTORY_ORDINAL_DEFAULT_RESOLVERS:
        key = (owner, accessor_name)
        if key in resolved:
            raise ExtractionError(
                f"duplicate static-factory ordinal default resolver: {owner}.{accessor_name}"
            )
        accessor = accessor_for(result, owner, accessor_name)
        if accessor.serializer != serializer:
            raise ExtractionError(
                f"{owner}.{accessor_name}: static-factory ordinal resolver requires "
                f"{serializer} serializer"
            )
        if accessor.default_expression != expected_expression:
            raise ExtractionError(
                f"{owner}.{accessor_name}: stale static-factory ordinal expression; "
                f"expected {expected_expression!r}, found {accessor.default_expression!r}"
            )
        ordinals = static_factory_registration_ordinals(
            result,
            registry_class,
        )
        if constant_name not in ordinals:
            raise ExtractionError(f"{registry_class}: missing {constant_name} registration")
        value = ordinals[constant_name]
        resolved[key] = NormalizedDefault(
            f"VanillaEntityMetadataDefault::Int({value})",
            "i32",
            str(value),
        )
        evidence.append(
            ResolvedDefaultEvidence(
                owner,
                accessor_name,
                registry_class,
                f"{constant_name} -> {STATIC_FACTORY_ORDINAL_SOURCES[registry_class][0]} registration {value}",
                f"{expected_expression} -> {STATIC_FACTORY_ORDINAL_SOURCES[registry_class][1]}",
                reason,
            )
        )
    for (
        owner,
        accessor_name,
        serializer,
        enum_fqcn,
        constant_name,
        id_argument_index,
        expected_expression,
        reason,
    ) in ENUM_CONSTANT_DEFAULT_RESOLVERS:
        key = (owner, accessor_name)
        if key in resolved:
            raise ExtractionError(f"duplicate enum default resolver: {owner}.{accessor_name}")
        accessor = accessor_for(result, owner, accessor_name)
        if accessor.serializer != serializer:
            raise ExtractionError(
                f"{owner}.{accessor_name}: enum constant resolver requires {serializer} serializer"
            )
        if accessor.default_expression != expected_expression:
            raise ExtractionError(
                f"{owner}.{accessor_name}: stale enum default resolver expression; "
                f"expected {expected_expression!r}, found {accessor.default_expression!r}"
            )
        value = enum_constant_constructor_integer(
            result,
            enum_fqcn,
            constant_name,
            id_argument_index,
        )
        resolved[key] = NormalizedDefault(
            f"VanillaEntityMetadataDefault::Int({value})",
            "i32",
            str(value),
        )
        evidence.append(
            ResolvedDefaultEvidence(
                owner,
                accessor_name,
                enum_fqcn,
                constant_name,
                expected_expression,
                reason,
            )
        )
    for (
        owner,
        accessor_name,
        serializer,
        enum_fqcn,
        constant_name,
        expected_expression,
        reason,
    ) in ENUM_ORDINAL_DEFAULT_RESOLVERS:
        key = (owner, accessor_name)
        if key in resolved:
            raise ExtractionError(f"duplicate enum default resolver: {owner}.{accessor_name}")
        accessor = accessor_for(result, owner, accessor_name)
        if accessor.serializer != serializer:
            raise ExtractionError(
                f"{owner}.{accessor_name}: enum ordinal resolver requires {serializer} serializer"
            )
        if accessor.default_expression != expected_expression:
            raise ExtractionError(
                f"{owner}.{accessor_name}: stale enum default resolver expression; "
                f"expected {expected_expression!r}, found {accessor.default_expression!r}"
            )
        value = enum_constant_ordinal(result, enum_fqcn, constant_name)
        resolved[key] = NormalizedDefault(
            f"VanillaEntityMetadataDefault::Int({value})",
            "i32",
            str(value),
        )
        evidence.append(
            ResolvedDefaultEvidence(
                owner,
                accessor_name,
                enum_fqcn,
                constant_name,
                expected_expression,
                reason,
            )
        )
    for (
        owner,
        accessor_name,
        method_name,
        expected_expression,
        expected_return_expression,
        resource_id,
        reason,
    ) in STATIC_ITEM_STACK_DEFAULT_RESOLVERS:
        key = (owner, accessor_name)
        if key in resolved:
            raise ExtractionError(f"duplicate static item default resolver: {owner}.{accessor_name}")
        accessor = accessor_for(result, owner, accessor_name)
        if accessor.serializer != "ITEM_STACK":
            raise ExtractionError(
                f"{owner}.{accessor_name}: static item resolver requires ITEM_STACK serializer"
            )
        if accessor.default_expression != expected_expression:
            raise ExtractionError(
                f"{owner}.{accessor_name}: stale static item resolver expression; "
                f"expected {expected_expression!r}, found {accessor.default_expression!r}"
            )
        return_expression = method_single_return_expression(result, owner, method_name)
        if return_expression != expected_return_expression:
            raise ExtractionError(
                f"{owner}.{method_name}: stale static item resolver return; "
                f"expected {expected_return_expression!r}, found {return_expression!r}"
            )
        resource_rust = rust_string(resource_id)
        resolved[key] = NormalizedDefault(
            f"VanillaEntityMetadataDefault::ItemStackDefaultItem({resource_rust})",
            "&str",
            resource_rust,
        )
        evidence.append(
            ResolvedDefaultEvidence(
                owner,
                accessor_name,
                owner,
                method_name,
                f"{expected_expression} -> {expected_return_expression}",
                reason,
            )
        )
    for (
        owner,
        accessor_name,
        default_field_name,
        expected_expression,
        expected_field_expression,
        evidence_class,
        evidence_call_name,
        expected_evidence_expression,
        resource_id,
        properties,
        reason,
    ) in STATIC_BLOCK_STATE_DEFAULT_RESOLVERS:
        key = (owner, accessor_name)
        if key in resolved:
            raise ExtractionError(f"duplicate static block-state default resolver: {owner}.{accessor_name}")
        accessor = accessor_for(result, owner, accessor_name)
        if accessor.serializer != "BLOCK_STATE":
            raise ExtractionError(
                f"{owner}.{accessor_name}: static block-state resolver requires BLOCK_STATE serializer"
            )
        if accessor.default_expression != expected_expression:
            raise ExtractionError(
                f"{owner}.{accessor_name}: stale static block-state resolver expression; "
                f"expected {expected_expression!r}, found {accessor.default_expression!r}"
            )
        default_field = field_for(result, owner, default_field_name)
        if default_field.initializer is None:
            raise ExtractionError(f"{owner}.{default_field_name}: missing initializer")
        field_expression = default_field.initializer.canonical
        if field_expression != expected_field_expression:
            raise ExtractionError(
                f"{owner}.{default_field_name}: stale block-state initializer; "
                f"expected {expected_field_expression!r}, found {field_expression!r}"
            )
        evidence_source = source_for_owner(result, evidence_class)
        evidence_calls = [
            call
            for call in evidence_source.calls_for(evidence_class)
            if call.name == evidence_call_name
            and evidence_source.expression(call.node).canonical == expected_evidence_expression
        ]
        if len(evidence_calls) != 1:
            raise ExtractionError(
                f"{evidence_class}.{evidence_call_name}: expected one "
                f"{expected_evidence_expression!r} call, found {len(evidence_calls)}"
            )
        resource_rust = rust_string(resource_id)
        properties_rust = "&[" + ", ".join(
            f"({rust_string(name)}, {rust_string(value)})" for name, value in properties
        ) + "]"
        resolved[key] = NormalizedDefault(
            "VanillaEntityMetadataDefault::BlockStateDefault { "
            f"resource_id: {resource_rust}, properties: {properties_rust} "
            "}",
        )
        evidence.append(
            ResolvedDefaultEvidence(
                owner,
                accessor_name,
                evidence_class,
                evidence_call_name,
                f"{expected_expression} -> {expected_field_expression}; {expected_evidence_expression}",
                reason,
            )
        )
    for (
        owner,
        accessor_name,
        default_field_name,
        expected_expression,
        expected_default_initializer,
        pattern_enum,
        base_enum,
        dye_enum,
        expected_pattern_assignment,
        expected_pack_return,
        reason,
    ) in TROPICAL_FISH_PACKED_DEFAULT_RESOLVERS:
        key = (owner, accessor_name)
        if key in resolved:
            raise ExtractionError(f"duplicate packed variant default resolver: {owner}.{accessor_name}")
        accessor = accessor_for(result, owner, accessor_name)
        if accessor.serializer != "INT":
            raise ExtractionError(
                f"{owner}.{accessor_name}: packed variant resolver requires INT serializer"
            )
        if accessor.default_expression != expected_expression:
            raise ExtractionError(
                f"{owner}.{accessor_name}: stale packed variant default expression; "
                f"expected {expected_expression!r}, found {accessor.default_expression!r}"
            )
        default_field = field_for(result, owner, default_field_name)
        if default_field.initializer is None:
            raise ExtractionError(f"{owner}.{default_field_name}: missing initializer")
        if default_field.initializer.canonical != expected_default_initializer:
            raise ExtractionError(
                f"{owner}.{default_field_name}: stale packed variant initializer; "
                f"expected {expected_default_initializer!r}, "
                f"found {default_field.initializer.canonical!r}"
            )
        initializer = default_field.initializer.node
        arguments = initializer.child_by_field_name("arguments")
        if initializer.type != "object_creation_expression" or arguments is None or len(arguments.named_children) != 3:
            raise ExtractionError(
                f"{owner}.{default_field_name}: expected a three-argument variant constructor"
            )
        initializer_arguments = [
            source_for_owner(result, owner).expression(node).canonical
            for node in arguments.named_children
        ]
        pattern_constant = initializer_arguments[0].rsplit(".", 1)[-1]
        base_color_constant = initializer_arguments[1].rsplit(".", 1)[-1]
        pattern_color_constant = initializer_arguments[2].rsplit(".", 1)[-1]

        pattern_source, pattern_arguments = enum_constant_arguments(
            result,
            pattern_enum,
            pattern_constant,
        )
        if len(pattern_arguments) != 3:
            raise ExtractionError(
                f"{pattern_enum}.{pattern_constant}: expected three constructor arguments"
            )
        base_reference = pattern_source.expression(pattern_arguments[1]).canonical
        base_constant = base_reference.rsplit(".", 1)[-1]
        pattern_index = enum_constant_constructor_integer(
            result,
            pattern_enum,
            pattern_constant,
            2,
        )
        base_id = enum_constant_constructor_integer(result, base_enum, base_constant, 0)
        base_color_id = enum_constant_constructor_integer(
            result,
            dye_enum,
            base_color_constant,
            0,
        )
        pattern_color_id = enum_constant_constructor_integer(
            result,
            dye_enum,
            pattern_color_constant,
            0,
        )

        pattern_source, pattern_constructor = method_for(
            result,
            pattern_enum,
            pattern_enum.rsplit(".", 1)[-1],
        )
        if not pattern_constructor.is_constructor:
            raise ExtractionError(f"{pattern_enum}: expected constructor evidence")
        assignments = [
            pattern_source.expression(node).canonical
            for node in descendants(pattern_constructor.node)
            if node.type == "assignment_expression"
            and pattern_source.expression(node).canonical == expected_pattern_assignment
        ]
        if len(assignments) != 1:
            raise ExtractionError(
                f"{pattern_enum}: expected one {expected_pattern_assignment!r} assignment, "
                f"found {len(assignments)}"
            )
        pack_return = method_single_return_expression(result, owner, "packVariant")
        if pack_return != expected_pack_return:
            raise ExtractionError(
                f"{owner}.packVariant: stale packed variant expression; "
                f"expected {expected_pack_return!r}, found {pack_return!r}"
            )

        pattern_id = base_id | (pattern_index << 8)
        value = (
            (pattern_id & 0xFFFF)
            | ((base_color_id & 0xFF) << 16)
            | ((pattern_color_id & 0xFF) << 24)
        )
        resolved[key] = NormalizedDefault(
            f"VanillaEntityMetadataDefault::Int({value})",
            "i32",
            str(value),
        )
        evidence.append(
            ResolvedDefaultEvidence(
                owner,
                accessor_name,
                owner,
                default_field_name,
                f"{expected_expression} -> {expected_default_initializer}; "
                f"{expected_pattern_assignment}; {expected_pack_return}",
                reason,
            )
        )
    for (
        owner,
        accessor_name,
        serializer,
        expected_expression,
        field_evidence,
        method_evidence,
        enum_evidence,
        resource_id,
        reason,
    ) in STATIC_REGISTRY_RESOURCE_DEFAULT_RESOLVERS:
        key = (owner, accessor_name)
        if key in resolved:
            raise ExtractionError(
                f"duplicate registry resource default resolver: {owner}.{accessor_name}"
            )
        accessor = accessor_for(result, owner, accessor_name)
        if accessor.serializer != serializer:
            raise ExtractionError(
                f"{owner}.{accessor_name}: registry resource resolver requires "
                f"{serializer} serializer"
            )
        if accessor.default_expression != expected_expression:
            raise ExtractionError(
                f"{owner}.{accessor_name}: stale registry resource default expression; "
                f"expected {expected_expression!r}, found {accessor.default_expression!r}"
            )
        evidence_members: list[str] = []
        source_expressions = [expected_expression]
        for field_owner, field_name, expected_initializer in field_evidence:
            field = field_for(result, field_owner, field_name)
            if field.initializer is None:
                raise ExtractionError(f"{field_owner}.{field_name}: missing initializer")
            if field.initializer.canonical != expected_initializer:
                raise ExtractionError(
                    f"{field_owner}.{field_name}: stale registry resource initializer; "
                    f"expected {expected_initializer!r}, found {field.initializer.canonical!r}"
                )
            evidence_members.append(f"{field_owner.rsplit('.', 1)[-1]}.{field_name}")
            source_expressions.append(expected_initializer)
        for method_owner, method_name, expected_return in method_evidence:
            actual_return = method_single_return_expression(result, method_owner, method_name)
            if actual_return != expected_return:
                raise ExtractionError(
                    f"{method_owner}.{method_name}: stale registry resource return; "
                    f"expected {expected_return!r}, found {actual_return!r}"
                )
            evidence_members.append(f"{method_owner.rsplit('.', 1)[-1]}.{method_name}")
            source_expressions.append(expected_return)
        if enum_evidence is not None:
            enum_fqcn, constant_name, argument_index, expected_argument = enum_evidence
            enum_source, enum_arguments = enum_constant_arguments(
                result,
                enum_fqcn,
                constant_name,
            )
            if argument_index < 0 or argument_index >= len(enum_arguments):
                raise ExtractionError(
                    f"{enum_fqcn}.{constant_name}: missing resource constructor argument "
                    f"{argument_index}"
                )
            actual_argument = enum_source.expression(enum_arguments[argument_index]).canonical
            if actual_argument != expected_argument:
                raise ExtractionError(
                    f"{enum_fqcn}.{constant_name}: stale registry resource argument; "
                    f"expected {expected_argument!r}, found {actual_argument!r}"
                )
            evidence_members.append(f"{enum_fqcn.rsplit('.', 1)[-1]}.{constant_name}")
            source_expressions.append(expected_argument)
        resource_rust = rust_string(resource_id)
        resolved[key] = NormalizedDefault(
            f"VanillaEntityMetadataDefault::RegistryHolderResource({resource_rust})",
            "&str",
            resource_rust,
        )
        evidence.append(
            ResolvedDefaultEvidence(
                owner,
                accessor_name,
                owner.rsplit(".", 1)[0],
                " -> ".join(evidence_members),
                " -> ".join(source_expressions),
                reason,
            )
        )
    for (
        owner,
        accessor_name,
        expected_expression,
        default_field_name,
        expected_field_expression,
        enum_fqcn,
        id_argument_index,
        reason,
    ) in ENUM_FIELD_ID_DEFAULT_RESOLVERS:
        key = (owner, accessor_name)
        if key in resolved:
            raise ExtractionError(f"duplicate enum field default resolver: {owner}.{accessor_name}")
        accessor = accessor_for(result, owner, accessor_name)
        if accessor.serializer != "INT":
            raise ExtractionError(
                f"{owner}.{accessor_name}: enum field resolver requires INT serializer"
            )
        if accessor.default_expression != expected_expression:
            raise ExtractionError(
                f"{owner}.{accessor_name}: stale enum field default expression; "
                f"expected {expected_expression!r}, found {accessor.default_expression!r}"
            )
        default_field = field_for(result, owner, default_field_name)
        if default_field.initializer is None:
            raise ExtractionError(f"{owner}.{default_field_name}: missing initializer")
        if default_field.initializer.canonical != expected_field_expression:
            raise ExtractionError(
                f"{owner}.{default_field_name}: stale enum field initializer; "
                f"expected {expected_field_expression!r}, "
                f"found {default_field.initializer.canonical!r}"
            )
        constant_name = expected_field_expression.rsplit(".", 1)[-1]
        value = enum_constant_constructor_integer(
            result,
            enum_fqcn,
            constant_name,
            id_argument_index,
        )
        resolved[key] = NormalizedDefault(
            f"VanillaEntityMetadataDefault::Int({value})",
            "i32",
            str(value),
        )
        evidence.append(
            ResolvedDefaultEvidence(
                owner,
                accessor_name,
                owner,
                f"{default_field_name} -> {enum_fqcn.rsplit('.', 1)[-1]}.{constant_name}",
                f"{expected_expression} -> {expected_field_expression}",
                reason,
            )
        )
    return resolved, evidence


def normalize_default(
    result: ExtractionResult,
    accessor: Accessor,
    resolved_defaults: dict[tuple[str, str], NormalizedDefault],
) -> NormalizedDefault | None:
    resolved = resolved_defaults.get((accessor.owner, accessor.java_name))
    if resolved is not None:
        return resolved
    source, node = resolve_default_node(result, accessor)
    canonical = source.expression(node).canonical
    compact = canonical.replace(" ", "")
    serializer = accessor.serializer
    if serializer == "BOOLEAN" and node.type in {"true", "false"}:
        return NormalizedDefault(f"VanillaEntityMetadataDefault::Boolean({compact})", "bool", compact)
    if serializer in {"BYTE", "INT", "LONG"}:
        if compact in {"this.getMaxAirSupply()", "getMaxAirSupply()"}:
            return NormalizedDefault("VanillaEntityMetadataDefault::MaxAirSupply")
        if compact in {"this.getDefaultDisplayOffset()", "getDefaultDisplayOffset()"}:
            return NormalizedDefault("VanillaEntityMetadataDefault::MinecartDefaultDisplayOffset")
        try:
            value = evaluate_integer_expression(source, node, integer_constants(result, accessor.owner))
        except (ExtractionError, ValueError):
            return None
        if serializer == "BYTE":
            if value >= 128:
                value -= 256
            return NormalizedDefault(f"VanillaEntityMetadataDefault::Byte({value})", "i8", str(value))
        if serializer == "LONG":
            return NormalizedDefault(f"VanillaEntityMetadataDefault::Long({value})", "i64", str(value))
        return NormalizedDefault(f"VanillaEntityMetadataDefault::Int({value})", "i32", str(value))
    if serializer == "FLOAT" and node.type in {"decimal_floating_point_literal", "decimal_integer_literal"}:
        value = float(source.text(node).rstrip("fFdD"))
        rust = f"{value:.1f}"
        return NormalizedDefault(f"VanillaEntityMetadataDefault::Float({rust})", "f32", rust)
    optional = {
        "OPTIONAL_COMPONENT": ("OptionalComponentNone", None, None),
        "OPTIONAL_BLOCK_POS": ("OptionalBlockPosNone", None, None),
        "OPTIONAL_BLOCK_STATE": ("OptionalBlockStateNone", "Option<i32>", "None"),
        "OPTIONAL_UNSIGNED_INT": ("OptionalUnsignedIntNone", None, None),
        "OPTIONAL_LIVING_ENTITY_REFERENCE": ("OptionalLivingEntityReferenceNone", None, None),
    }
    if serializer in optional and compact in {"Optional.empty()", "OptionalInt.empty()"}:
        variant, scalar_type, scalar_rust = optional[serializer]
        return NormalizedDefault(
            f"VanillaEntityMetadataDefault::{variant}",
            scalar_type,
            scalar_rust,
        )
    if serializer == "PARTICLES" and compact == "List.of()":
        return NormalizedDefault("VanillaEntityMetadataDefault::ParticlesEmpty")
    if serializer == "ITEM_STACK" and compact == "ItemStack.EMPTY":
        return NormalizedDefault("VanillaEntityMetadataDefault::ItemStackEmpty")
    enum_types = {
        "POSE": ("Pose", "net.minecraft.world.entity.Pose"),
        "DIRECTION": ("Direction", "net.minecraft.core.Direction"),
        "HUMANOID_ARM": ("HumanoidArm", "net.minecraft.world.entity.HumanoidArm"),
    }
    if serializer in enum_types:
        prefix, fqcn = enum_types[serializer]
        if compact.startswith(prefix + "."):
            name = compact.rsplit(".", 1)[-1]
            ordinal = enum_ordinals(result, fqcn)[name]
            return NormalizedDefault(
                f"VanillaEntityMetadataDefault::{prefix}({ordinal})",
                "i32",
                str(ordinal),
            )
    if serializer == "ROTATIONS" and node.type == "object_creation_expression":
        arguments = node.child_by_field_name("arguments")
        if arguments is not None and len(arguments.named_children) == 3:
            values = [float(source.text(item).rstrip("fFdD")) for item in arguments.named_children]
            rust_values = ", ".join(f"{value:.1f}" for value in values)
            scalar = f"[{rust_values}]"
            return NormalizedDefault(
                f"VanillaEntityMetadataDefault::Rotations({scalar})",
                "[f32; 3]",
                scalar,
            )
    return None


def method_for(
    result: ExtractionResult, owner: str, name: str, parameter_count: int | None = None
) -> tuple[JavaSource, JavaMethod]:
    source = source_for_owner(result, owner)
    matches = [
        item
        for item in source.methods_for(owner)
        if item.name == name
        and (parameter_count is None or len(item.parameters) == parameter_count)
    ]
    if len(matches) != 1:
        signature = name if parameter_count is None else f"{name}/{parameter_count}"
        raise ExtractionError(f"{owner}.{signature}: expected one method, found {len(matches)}")
    return source, matches[0]


def _unwrapped_expression(node: Node) -> Node:
    while node.type == "parenthesized_expression":
        if len(node.named_children) != 1:
            raise ExtractionError("malformed parenthesized expression")
        node = node.named_children[0]
    return node


def method_discriminant_ids(
    result: ExtractionResult,
    owner: str,
    name: str,
    parameter_count: int,
    discriminant_parameter: int,
) -> tuple[int, ...]:
    """Extract fixed integer arms selected directly from one method parameter."""
    source, method = method_for(result, owner, name, parameter_count)
    parameter = method.parameters[discriminant_parameter].node
    parameter_name_node = parameter.child_by_field_name("name")
    if parameter_name_node is None:
        raise ExtractionError(f"{owner}.{name}: discriminant parameter lacks a name")
    parameter_name = source.text(parameter_name_node)
    body = method.node.child_by_field_name("body")
    if body is None:
        raise ExtractionError(f"{owner}.{name}: method lacks body")

    candidates: list[tuple[int, int]] = []
    for node in descendants(body):
        if node.type in {"switch_expression", "switch_statement"}:
            selector = node.child_by_field_name("condition")
            if selector is None:
                raise ExtractionError(f"{owner}.{name}: switch lacks selector")
            selector = _unwrapped_expression(selector)
            if selector.type != "identifier" or source.text(selector) != parameter_name:
                continue
            for label in descendants(node):
                if label.type != "switch_label" or source.text(label).strip() == "default":
                    continue
                values = label.named_children
                if len(values) != 1:
                    raise ExtractionError(
                        f"{owner}.{name}: unsupported switch label {source.text(label)!r}"
                    )
                value = evaluate_integer_expression(source, values[0], {})
                candidates.append((label.start_byte, value))
        elif node.type == "binary_expression":
            operator = node.child_by_field_name("operator")
            left = node.child_by_field_name("left")
            right = node.child_by_field_name("right")
            if operator is None or left is None or right is None or source.text(operator) != "==":
                continue
            left = _unwrapped_expression(left)
            right = _unwrapped_expression(right)
            if left.type == "identifier" and source.text(left) == parameter_name:
                value_node = right
            elif right.type == "identifier" and source.text(right) == parameter_name:
                value_node = left
            else:
                continue
            value = evaluate_integer_expression(source, value_node, {})
            candidates.append((node.start_byte, value))

    event_ids: list[int] = []
    for _, value in sorted(candidates):
        if not 0 <= value <= 255:
            raise ExtractionError(f"{owner}.{name}: event id {value} is outside u8")
        if value not in event_ids:
            event_ids.append(value)
    return tuple(event_ids)


def method_forwards_parameter_to_super(
    result: ExtractionResult,
    owner: str,
    name: str,
    parameter_count: int,
    parameter_index: int,
) -> bool:
    source, method = method_for(result, owner, name, parameter_count)
    parameter_name_node = method.parameters[parameter_index].node.child_by_field_name("name")
    body = method.node.child_by_field_name("body")
    if parameter_name_node is None or body is None:
        raise ExtractionError(f"{owner}.{name}: malformed method declaration")
    parameter_name = source.text(parameter_name_node)
    matches = []
    for node in descendants(body):
        if node.type != "method_invocation":
            continue
        object_node = node.child_by_field_name("object")
        name_node = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if (
            object_node is not None
            and object_node.type == "super"
            and name_node is not None
            and source.text(name_node) == name
            and arguments is not None
            and len(arguments.named_children) == 1
            and source.expression(arguments.named_children[0]).canonical == parameter_name
        ):
            matches.append(node)
    if len(matches) > 1:
        raise ExtractionError(f"{owner}.{name}: multiple super forwarding calls")
    return bool(matches)


def entity_event_handler_specs(
    result: ExtractionResult,
) -> tuple[EntityEventHandlerSpec, ...]:
    class_map = {item.fqcn: item for item in result.classes}
    nearest_handler_cache: dict[str, str] = {}
    specs: list[EntityEventHandlerSpec] = []
    for java_class in result.classes:
        source = source_for_owner(result, java_class.fqcn)
        handlers = [
            method
            for method in source.methods_for(java_class.fqcn)
            if method.name == "handleEntityEvent" and len(method.parameters) == 1
        ]
        if len(handlers) > 1:
            raise ExtractionError(
                f"{java_class.fqcn}.handleEntityEvent/1: expected at most one method"
            )
        if not handlers:
            continue

        direct_event_ids = method_discriminant_ids(
            result, java_class.fqcn, "handleEntityEvent", 1, 0
        )
        delegated_event_ids: tuple[int, ...] = ()
        delegates_to = None
        delegation = ENTITY_EVENT_HANDLER_DELEGATIONS.get(java_class.fqcn)
        if delegation is not None:
            qualifier, delegated_owner, delegated_method, parameter_count, parameter_index = delegation
            handler = handlers[0]
            body = handler.node.child_by_field_name("body")
            parameter_name_node = handler.parameters[0].node.child_by_field_name("name")
            if body is None or parameter_name_node is None:
                raise ExtractionError(
                    f"{java_class.fqcn}.handleEntityEvent: malformed delegation"
                )
            parameter_name = source.text(parameter_name_node)
            matches = []
            for node in descendants(body):
                if node.type != "method_invocation":
                    continue
                object_node = node.child_by_field_name("object")
                name_node = node.child_by_field_name("name")
                arguments = node.child_by_field_name("arguments")
                if (
                    object_node is not None
                    and source.expression(object_node).canonical == qualifier
                    and name_node is not None
                    and source.text(name_node) == delegated_method
                    and arguments is not None
                    and len(arguments.named_children) == parameter_count
                    and source.expression(arguments.named_children[parameter_index]).canonical
                    == parameter_name
                ):
                    matches.append(node)
            if len(matches) != 1 or direct_event_ids:
                raise ExtractionError(
                    f"{java_class.fqcn}.handleEntityEvent: delegation shape changed"
                )
            delegated_event_ids = method_discriminant_ids(
                result,
                delegated_owner,
                delegated_method,
                parameter_count,
                parameter_index,
            )
            delegates_to = f"{delegated_owner}.{delegated_method}"

        specs.append(
            EntityEventHandlerSpec(
                owner=java_class.fqcn,
                super_handler_owner=(
                    class_nearest_method_owner(
                        result,
                        class_map,
                        java_class.superclass,
                        "handleEntityEvent",
                        1,
                        nearest_handler_cache,
                    )
                    if java_class.superclass is not None
                    else None
                ),
                source_path=java_class.source_path,
                direct_event_ids=direct_event_ids,
                delegated_event_ids=delegated_event_ids,
                delegates_to=delegates_to,
                forwards_unhandled_to_super=method_forwards_parameter_to_super(
                    result, java_class.fqcn, "handleEntityEvent", 1, 0
                ),
            )
        )
    if len(specs) != 35:
        raise ExtractionError(
            f"expected 35 handleEntityEvent declarations, extracted {len(specs)}"
        )
    return tuple(specs)


def camera_post_effect_specs(result: ExtractionResult) -> tuple[CameraPostEffectSpec, ...]:
    """Extract ``GameRenderer.checkEntityPostEffect`` pattern-switch mappings."""
    source = result.sources[GAME_RENDERER_SOURCE.as_posix()]
    owners = [
        item
        for item in source.types
        if item.fqcn == "net.minecraft.client.renderer.GameRenderer"
    ]
    if len(owners) != 1:
        raise ExtractionError(
            f"GameRenderer: expected one source owner, found {len(owners)}"
        )
    owner = owners[0]
    _, method = method_for(
        result,
        "net.minecraft.client.renderer.GameRenderer",
        "checkEntityPostEffect",
        parameter_count=1,
    )
    body = method.node.child_by_field_name("body")
    if body is None:
        raise ExtractionError("GameRenderer.checkEntityPostEffect: method lacks body")
    switches = [item for item in descendants(body) if item.type == "switch_expression"]
    if len(switches) != 1:
        raise ExtractionError(
            "GameRenderer.checkEntityPostEffect: expected one switch expression"
        )
    switch = switches[0]
    if (
        not switch.named_children
        or source.expression(switch.named_children[0]).canonical != "(cameraEntity)"
    ):
        raise ExtractionError(
            "GameRenderer.checkEntityPostEffect: expected cameraEntity selector"
        )
    blocks = [item for item in switch.named_children if item.type == "switch_block"]
    if len(blocks) != 1:
        raise ExtractionError(
            "GameRenderer.checkEntityPostEffect: expected one switch block"
        )

    known_classes = {item.fqcn: item for item in result.classes}
    specs: list[CameraPostEffectSpec] = []
    saw_null = False
    saw_default_clear = False
    for group in blocks[0].named_children:
        if group.type != "switch_block_statement_group" or not group.named_children:
            raise ExtractionError(
                "GameRenderer.checkEntityPostEffect: unsupported switch group"
            )
        label = group.named_children[0]
        if label.type != "switch_label":
            raise ExtractionError(
                "GameRenderer.checkEntityPostEffect: switch group lacks label"
            )
        label_text = source.text(label)
        patterns = [item for item in descendants(label) if item.type == "type_pattern"]
        if patterns:
            if len(patterns) != 1 or [item.type for item in group.named_children] != [
                "switch_label",
                "expression_statement",
                "break_statement",
            ]:
                raise ExtractionError(
                    "GameRenderer.checkEntityPostEffect: unsupported type-pattern arm"
                )
            pattern = patterns[0]
            if len(pattern.named_children) != 2:
                raise ExtractionError(
                    "GameRenderer.checkEntityPostEffect: malformed type pattern"
                )
            runtime_class = resolve_type_reference(
                source.text(pattern.named_children[0]), owner, known_classes
            )
            if runtime_class is None:
                raise ExtractionError(
                    "GameRenderer.checkEntityPostEffect: unresolved pattern class "
                    + source.text(pattern.named_children[0])
                )

            expression = group.named_children[1]
            calls = [
                item
                for item in descendants(expression)
                if item.type == "method_invocation"
            ]
            if len(calls) != 2:
                raise ExtractionError(
                    "GameRenderer.checkEntityPostEffect: expected setPostEffect namespace call"
                )
            outer, identifier_call = calls
            outer_name = outer.child_by_field_name("name")
            outer_object = outer.child_by_field_name("object")
            outer_arguments = outer.child_by_field_name("arguments")
            identifier_name = identifier_call.child_by_field_name("name")
            identifier_object = identifier_call.child_by_field_name("object")
            identifier_arguments = identifier_call.child_by_field_name("arguments")
            if (
                outer_name is None
                or source.text(outer_name) != "setPostEffect"
                or outer_object is None
                or source.text(outer_object) != "this"
                or outer_arguments is None
                or outer_arguments.named_children != [identifier_call]
                or identifier_name is None
                or source.text(identifier_name) != "withDefaultNamespace"
                or identifier_object is None
                or source.text(identifier_object) != "Identifier"
                or identifier_arguments is None
                or len(identifier_arguments.named_children) != 1
                or identifier_arguments.named_children[0].type != "string_literal"
            ):
                raise ExtractionError(
                    "GameRenderer.checkEntityPostEffect: unsupported post-effect call"
                )
            literal = source.text(identifier_arguments.named_children[0])
            resource_path = (
                literal[1:-1]
                if literal.startswith('"') and literal.endswith('"')
                else ""
            )
            if not resource_path or any(
                character not in "abcdefghijklmnopqrstuvwxyz0123456789_/-"
                for character in resource_path
            ):
                raise ExtractionError(
                    f"GameRenderer.checkEntityPostEffect: unsupported resource literal {literal!r}"
                )
            specs.append(
                CameraPostEffectSpec(runtime_class, f"minecraft:{resource_path}")
            )
        elif label_text == "case null":
            if len(group.named_children) != 1 or saw_null:
                raise ExtractionError(
                    "GameRenderer.checkEntityPostEffect: malformed null arm"
                )
            saw_null = True
        elif label_text == "default":
            if (
                len(group.named_children) != 2
                or source.expression(group.named_children[1]).canonical
                != "this.clearPostEffect ();"
                or saw_default_clear
            ):
                raise ExtractionError(
                    "GameRenderer.checkEntityPostEffect: malformed default clear arm"
                )
            saw_default_clear = True
        else:
            raise ExtractionError(
                f"GameRenderer.checkEntityPostEffect: unsupported label {label_text!r}"
            )

    if not specs or not saw_null or not saw_default_clear:
        raise ExtractionError(
            "GameRenderer.checkEntityPostEffect: incomplete pattern/default coverage"
        )
    if len({item.runtime_class for item in specs}) != len(specs):
        raise ExtractionError("GameRenderer.checkEntityPostEffect: duplicate pattern class")
    return tuple(specs)


def method_single_return_expression(
    result: ExtractionResult,
    owner: str,
    name: str,
    parameter_count: int | None = None,
) -> str:
    source, method = method_for(result, owner, name, parameter_count)
    body = method.node.child_by_field_name("body")
    returns = [item for item in descendants(body) if item.type == "return_statement"]
    if len(returns) != 1 or len(returns[0].named_children) != 1:
        raise ExtractionError(f"{owner}.{name}: expected one return expression")
    return source.expression(returns[0].named_children[0]).canonical


def item_tag_resources(result: ExtractionResult) -> dict[str, str]:
    owner = "net.minecraft.tags.ItemTags"
    source = source_for_owner(result, owner)
    resources: dict[str, str] = {}
    for field in source.fields_for(owner):
        if field.initializer is None:
            continue
        compact = field.initializer.canonical.replace(" ", "")
        match = re.fullmatch(r'bind\("([^"]+)"\)', compact)
        if match is None:
            raise ExtractionError(
                f"{owner}.{field.name}: expected a direct bind string, got "
                f"{field.initializer.canonical!r}"
            )
        resources[field.name] = f"minecraft:{match.group(1)}"
    if not resources:
        raise ExtractionError(f"{owner}: no item tag bindings")
    return resources


def food_predicate_rust(
    result: ExtractionResult,
    owner: str,
    item_tags: dict[str, str],
) -> str:
    expression = method_single_return_expression(result, owner, "isFood", 1).replace(" ", "")
    if expression == "false":
        return "VanillaEntityFoodPredicate::Never"
    match = re.fullmatch(r"[A-Za-z_$][A-Za-z0-9_$]*\.is\(ItemTags\.([A-Z0-9_]+)\)", expression)
    if match is not None:
        tag_name = match.group(1)
        resource_id = item_tags.get(tag_name)
        if resource_id is None:
            raise ExtractionError(f"{owner}.isFood: unknown ItemTags.{tag_name}")
        return (
            "VanillaEntityFoodPredicate::ItemTag("
            + rust_string(resource_id)
            + ")"
        )
    nautilus = (
        "!this.isTame()&&!this.isBaby()?"
        "itemStack.is(ItemTags.NAUTILUS_TAMING_ITEMS):"
        "itemStack.is(ItemTags.NAUTILUS_FOOD)"
    )
    if expression == nautilus:
        return (
            "VanillaEntityFoodPredicate::Nautilus { "
            'untamed_adult_tag: "minecraft:nautilus_taming_items", '
            'default_tag: "minecraft:nautilus_food" }'
        )
    raise ExtractionError(
        f"{owner}.isFood: unsupported source expression {expression!r}"
    )


def class_zero_argument_boolean_method(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    method_name: str,
    cache: dict[str, bool],
) -> bool:
    """Interpret the nearest literal boolean override along one class hierarchy."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(
            f"unknown class in {method_name} boolean hierarchy: {owner}"
        )
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == method_name and len(method.parameters) == 0
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.{method_name}/0: expected at most one method")
    if not methods:
        if item.superclass is None:
            raise ExtractionError(
                f"{owner}: no {method_name}/0 override or superclass"
            )
        value = class_zero_argument_boolean_method(
            result, class_map, item.superclass, method_name, cache
        )
        cache[owner] = value
        return value

    expression = method_single_return_expression(
        result, owner, method_name, parameter_count=0
    )
    if expression == "true":
        value = True
    elif expression == "false":
        value = False
    else:
        raise ExtractionError(
            f"{owner}.{method_name}: expected literal boolean return, found {expression!r}"
        )
    cache[owner] = value
    return value


def class_baby_behavior(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    cache: dict[str, str],
) -> str:
    """Interpret the nearest vanilla ``isBaby`` implementation for a living class."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(f"unknown class in isBaby hierarchy: {owner}")
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == "isBaby" and len(method.parameters) == 0
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.isBaby/0: expected at most one method")
    if not methods:
        if item.superclass is None:
            raise ExtractionError(f"{owner}: no isBaby override or superclass")
        value = class_baby_behavior(result, class_map, item.superclass, cache)
        cache[owner] = value
        return value

    expression = method_single_return_expression(result, owner, "isBaby")
    compact = expression.replace(" ", "")
    if compact == "false":
        value = "Never"
    elif compact in {
        "this.getAge()<0",
        "this.getEntityData().get(DATA_BABY_ID)",
    }:
        value = "Metadata"
    elif compact == "this.isSmall()":
        value = "ArmorStandSmall"
    else:
        raise ExtractionError(
            f"{owner}.isBaby: unsupported return expression {expression!r}"
        )
    cache[owner] = value
    return value


def class_baby_age_scale_bits(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    cache: dict[str, str],
) -> str:
    """Resolve the baby arm of the nearest vanilla ``getAgeScale`` implementation."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(f"unknown class in getAgeScale hierarchy: {owner}")
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == "getAgeScale" and len(method.parameters) == 0
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.getAgeScale/0: expected at most one method")
    if not methods:
        if item.superclass is None:
            raise ExtractionError(f"{owner}: no getAgeScale override or superclass")
        value = class_baby_age_scale_bits(result, class_map, item.superclass, cache)
        cache[owner] = value
        return value

    expression = method_single_return_expression(result, owner, "getAgeScale")
    compact = expression.replace(" ", "")
    match = re.fullmatch(
        r"this\.isBaby\(\)\?([0-9]+(?:\.[0-9]+)?)F:1\.0F", compact
    )
    if match is None:
        raise ExtractionError(
            f"{owner}.getAgeScale: unsupported return expression {expression!r}"
        )
    value = f"{match.group(1)}_f32.to_bits()"
    cache[owner] = value
    return value


def class_zero_argument_integer_method(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    method_name: str,
    cache: dict[str, int],
) -> int:
    """Interpret the nearest scalar integer override along one class hierarchy."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(
            f"unknown class in {method_name} integer hierarchy: {owner}"
        )
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == method_name and len(method.parameters) == 0
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.{method_name}/0: expected at most one method")
    if not methods:
        if item.superclass is None:
            raise ExtractionError(
                f"{owner}: no {method_name}/0 override or superclass"
            )
        value = class_zero_argument_integer_method(
            result, class_map, item.superclass, method_name, cache
        )
        cache[owner] = value
        return value

    value = method_return_integer(result, owner, method_name)
    cache[owner] = value
    return value


def class_sanitize_scale_behavior(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    cache: dict[str, str | None],
) -> str | None:
    """Resolve the nearest non-identity `LivingEntity.sanitizeScale` behavior."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(f"unknown class in sanitizeScale hierarchy: {owner}")
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == "sanitizeScale" and len(method.parameters) == 1
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.sanitizeScale/1: expected at most one method")
    if not methods:
        if item.superclass is None:
            raise ExtractionError(f"{owner}: no sanitizeScale/1 override or superclass")
        value = class_sanitize_scale_behavior(
            result, class_map, item.superclass, cache
        )
        cache[owner] = value
        return value

    body = methods[0].node.child_by_field_name("body")
    if body is None:
        raise ExtractionError(f"{owner}.sanitizeScale: method lacks body")
    returns = [item for item in descendants(body) if item.type == "return_statement"]
    if len(returns) != 1 or len(returns[0].named_children) != 1:
        raise ExtractionError(f"{owner}.sanitizeScale: expected one return expression")
    expression = returns[0].named_children[0]
    canonical = source.expression(expression).canonical
    if canonical == "scale":
        value = None
    elif expression.type in {
        "decimal_floating_point_literal",
        "decimal_integer_literal",
    }:
        literal = source.text(expression).rstrip("fFdD")
        fixed = float(literal)
        if not 0.0 < fixed < float("inf"):
            raise ExtractionError(f"{owner}.sanitizeScale: invalid fixed value {literal!r}")
        value = f"VanillaEntityScaleSanitizer::Fixed({fixed!r})"
    else:
        name = expression.child_by_field_name("name")
        object_node = expression.child_by_field_name("object")
        arguments = expression.child_by_field_name("arguments")
        if (
            expression.type != "method_invocation"
            or name is None
            or source.text(name) != "min"
            or object_node is None
            or source.text(object_node) != "Math"
            or arguments is None
            or len(arguments.named_children) != 2
            or source.expression(arguments.named_children[0]).canonical != "scale"
            or arguments.named_children[1].type
            not in {"decimal_floating_point_literal", "decimal_integer_literal"}
        ):
            raise ExtractionError(
                f"{owner}.sanitizeScale: unsupported expression "
                f"{source.expression(expression).canonical!r}"
            )
        literal = source.text(arguments.named_children[1]).rstrip("fFdD")
        cap = float(literal)
        if not 0.0 < cap < float("inf"):
            raise ExtractionError(f"{owner}.sanitizeScale: invalid cap {literal!r}")
        value = f"VanillaEntityScaleSanitizer::ClampMax({cap!r})"
    cache[owner] = value
    return value


def class_is_pickable_without_type_tag(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    cache: dict[str, bool],
) -> bool:
    """Interpret the nearest vanilla ``isPickable`` override for one entity class.

    The redirectable-projectile tag is generated by the core registry catalog and
    composed in Rust. Runtime predicates such as removed/spectator/marker state are
    likewise applied by the world store; this returns whether the class path can be
    pickable before those per-instance gates.
    """
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(f"unknown entity class in isPickable hierarchy: {owner}")
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == "isPickable" and len(method.parameters) == 0
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.isPickable/0: expected at most one method")
    if not methods:
        if item.superclass is None:
            raise ExtractionError(f"{owner}: no isPickable override or superclass")
        value = class_is_pickable_without_type_tag(
            result, class_map, item.superclass, cache
        )
        cache[owner] = value
        return value

    expression = method_single_return_expression(result, owner, "isPickable")
    compact = expression.replace(" ", "")
    if compact in {"true", "!this.isRemoved()"}:
        value = True
    elif compact in {
        "false",
        "this.is(EntityTypeTags.REDIRECTABLE_PROJECTILE)",
    }:
        value = False
    elif compact in {
        "super.isPickable()&&!this.isMarker()",
        "!this.isSpectator()&&super.isPickable()",
        "super.isPickable()&&!this.isInGround()",
    }:
        if item.superclass is None:
            raise ExtractionError(f"{owner}.isPickable: super call without superclass")
        value = class_is_pickable_without_type_tag(
            result, class_map, item.superclass, cache
        )
    else:
        raise ExtractionError(
            f"{owner}.isPickable: unsupported return expression {expression!r}"
        )
    cache[owner] = value
    return value


def class_pushability_kind(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    cache: dict[str, str],
) -> str:
    """Interpret the nearest vanilla ``isPushable`` override for one entity class."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(f"unknown entity class in isPushable hierarchy: {owner}")
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == "isPushable" and len(method.parameters) == 0
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.isPushable/0: expected at most one method")
    if not methods:
        if item.superclass is None:
            raise ExtractionError(f"{owner}: no isPushable override or superclass")
        value = class_pushability_kind(result, class_map, item.superclass, cache)
        cache[owner] = value
        return value

    expression = method_single_return_expression(result, owner, "isPushable")
    compact = expression.replace(" ", "")
    if compact == "false":
        value = "Never"
    elif compact == "true":
        value = "Always"
    elif compact == "this.isAlive()&&!this.isSpectator()&&!this.onClimbable()":
        value = "Living"
    elif compact == "!this.isVehicle()":
        value = "NotVehicle"
    elif compact == "super.isPushable()&&this.canMove()":
        if item.superclass is None or class_pushability_kind(
            result, class_map, item.superclass, cache
        ) != "Living":
            raise ExtractionError(f"{owner}.isPushable: canMove gate lacks living super behavior")
        value = "LivingCanMove"
    elif compact == "!this.isDiggingOrEmerging()&&super.isPushable()":
        if item.superclass is None or class_pushability_kind(
            result, class_map, item.superclass, cache
        ) != "Living":
            raise ExtractionError(
                f"{owner}.isPushable: digging/emerging gate lacks living super behavior"
            )
        value = "LivingNotDiggingOrEmerging"
    else:
        raise ExtractionError(
            f"{owner}.isPushable: unsupported return expression {expression!r}"
        )
    cache[owner] = value
    return value


def class_pushes_local_player_on_client_tick(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    push_entities_owner_cache: dict[str, str],
    do_push_owner_cache: dict[str, str],
    cache: dict[str, bool],
) -> bool:
    """Interpret the client ``LivingEntity.pushEntities`` dispatch for a local Player.

    The fixed EntityType catalog carries this source-derived virtual-dispatch fact so
    the Rust tick owner never grows a hand-maintained Bat/ArmorStand/Parrot table.
    Dynamic ``isPushable`` state remains instance-owned by ``WorldStore``.
    """
    if owner in cache:
        return cache[owner]
    current: str | None = owner
    while current is not None and current != "net.minecraft.world.entity.LivingEntity":
        item = class_map.get(current)
        current = item.superclass if item is not None else None
    if current is None:
        cache[owner] = False
        return False

    push_owner = class_nearest_method_owner(
        result, class_map, owner, "pushEntities", 0, push_entities_owner_cache
    )
    push_source, push_method = method_for(result, push_owner, "pushEntities", 0)
    push_body = push_method.node.child_by_field_name("body")
    if push_body is None:
        raise ExtractionError(f"{push_owner}.pushEntities: method lacks body")
    push_canonical = push_source.expression(push_body).canonical
    if push_owner in {
        "net.minecraft.world.entity.ambient.Bat",
        "net.minecraft.world.entity.decoration.ArmorStand",
    }:
        expected = {
            "net.minecraft.world.entity.ambient.Bat": "{}",
            "net.minecraft.world.entity.decoration.ArmorStand": (
                "{for (Entity entity : this.level ().getEntities (this, "
                "this.getBoundingBox (), RIDABLE_MINECARTS)) {"
                "if (this.distanceToSqr (entity) <= 0.2) {entity.push (this);}}}"
            ),
        }[push_owner]
        if push_canonical != expected:
            raise ExtractionError(
                f"{push_owner}.pushEntities: stale local-player exclusion body; "
                f"expected {expected!r}, found {push_canonical!r}"
            )
        cache[owner] = False
        return False
    if push_owner != "net.minecraft.world.entity.LivingEntity":
        raise ExtractionError(
            f"{push_owner}.pushEntities: unsupported local-player dispatch override"
        )
    push_calls = [
        call.name
        for call in push_source.calls
        if push_body.start_byte <= call.span.start_byte
        and call.span.end_byte <= push_body.end_byte
    ]
    if push_calls.count("getPushableEntities") != 1 or push_calls.count("doPush") != 1:
        raise ExtractionError(
            "LivingEntity.pushEntities: expected one getPushableEntities/doPush path"
        )

    do_push_owner = class_nearest_method_owner(
        result, class_map, owner, "doPush", 1, do_push_owner_cache
    )
    do_push_source, do_push_method = method_for(result, do_push_owner, "doPush", 1)
    do_push_body = do_push_method.node.child_by_field_name("body")
    if do_push_body is None:
        raise ExtractionError(f"{do_push_owner}.doPush: method lacks body")
    do_push_canonical = do_push_source.expression(do_push_body).canonical
    if do_push_owner == "net.minecraft.world.entity.animal.parrot.Parrot":
        expected = "{if (! (entity instanceof Player)) {super.doPush (entity);}}"
        if do_push_canonical != expected:
            raise ExtractionError(
                f"{do_push_owner}.doPush: stale Player exclusion body; "
                f"expected {expected!r}, found {do_push_canonical!r}"
            )
        value = False
    elif do_push_owner == "net.minecraft.world.entity.LivingEntity":
        expected = "{entity.push (this);}"
        if do_push_canonical != expected:
            raise ExtractionError(
                f"LivingEntity.doPush: stale forwarding body {do_push_canonical!r}"
            )
        value = True
    elif do_push_owner in {
        "net.minecraft.world.entity.animal.golem.IronGolem",
        "net.minecraft.world.entity.monster.warden.Warden",
    }:
        if not method_forwards_parameter_to_super(
            result, do_push_owner, "doPush", 1, 0
        ):
            raise ExtractionError(
                f"{do_push_owner}.doPush: missing Entity parameter forwarding to super"
            )
        value = True
    else:
        raise ExtractionError(
            f"{do_push_owner}.doPush: unsupported local-player dispatch override"
        )
    cache[owner] = value
    return value


def class_collidability_kind(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    cache: dict[str, str],
) -> str:
    """Interpret the nearest vanilla ``canBeCollidedWith`` override."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(
            f"unknown entity class in canBeCollidedWith hierarchy: {owner}"
        )
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == "canBeCollidedWith" and len(method.parameters) == 1
    ]
    if len(methods) > 1:
        raise ExtractionError(
            f"{owner}.canBeCollidedWith/1: expected at most one method"
        )
    if not methods:
        if item.superclass is None:
            raise ExtractionError(
                f"{owner}: no canBeCollidedWith override or superclass"
            )
        value = class_collidability_kind(result, class_map, item.superclass, cache)
        cache[owner] = value
        return value

    body = methods[0].node.child_by_field_name("body")
    if body is None:
        raise ExtractionError(f"{owner}.canBeCollidedWith: method lacks body")
    canonical = source.expression(body).canonical
    kinds = {
        "{return false;}": "Never",
        "{return true;}": "Always",
        "{return this.isAlive ();}": "Alive",
        (
            "{if (! this.isBaby () && this.isAlive ()) {"
            "if (this.level ().isClientSide () && other instanceof Player && "
            "other.position ().y >= this.getBoundingBox ().maxY) {return true;} else {"
            "return this.isVehicle () && other instanceof HappyGhast ? true : "
            "this.isOnStillTimeout ();}} else {return false;}}"
        ): "HappyGhast",
    }
    value = kinds.get(canonical)
    if value is None:
        raise ExtractionError(
            f"{owner}.canBeCollidedWith: unsupported body {canonical!r}"
        )
    cache[owner] = value
    return value


def class_sound_source_kind(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    cache: dict[str, str],
) -> str:
    """Resolve the nearest vanilla ``getSoundSource`` class behavior.

    Literal enum returns are fixed per class. Rabbit is the sole 26.1 dynamic
    override and is recorded explicitly rather than guessed from its entity type.
    """
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(f"unknown entity class in getSoundSource hierarchy: {owner}")
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == "getSoundSource" and len(method.parameters) == 0
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.getSoundSource/0: expected at most one method")
    if not methods:
        if item.superclass is None:
            raise ExtractionError(f"{owner}: no getSoundSource override or superclass")
        value = class_sound_source_kind(result, class_map, item.superclass, cache)
        cache[owner] = value
        return value

    expression = method_single_return_expression(result, owner, "getSoundSource")
    compact = expression.replace(" ", "")
    literals = {
        "SoundSource.NEUTRAL": "Neutral",
        "SoundSource.HOSTILE": "Hostile",
        "SoundSource.AMBIENT": "Ambient",
        "SoundSource.WEATHER": "Weather",
        "SoundSource.PLAYERS": "Players",
    }
    if compact in literals:
        value = literals[compact]
    elif (
        owner == "net.minecraft.world.entity.animal.rabbit.Rabbit"
        and compact
        == "this.getVariant()==Rabbit.Variant.EVIL?SoundSource.HOSTILE:SoundSource.NEUTRAL"
    ):
        value = "Dynamic"
    else:
        raise ExtractionError(
            f"{owner}.getSoundSource: unsupported return expression {expression!r}"
        )
    cache[owner] = value
    return value


def sound_event_resources_by_constant(result: ExtractionResult) -> dict[str, str]:
    source = source_for_owner(result, "net.minecraft.sounds.SoundEvents")
    return {
        entry.constant_name: entry.resource_id
        for entry in direct_sound_event_entries(source)
    }


def class_hurt_sound_policy(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    sound_events: dict[str, str],
    method_owner_cache: dict[str, str],
    policy_cache: dict[str, str],
) -> str:
    """Interpret the nearest Vanilla ``LivingEntity.getHurtSound`` body.

    Fixed ``SoundEvents`` returns are open-ended and source-derived. The finite
    dynamic bodies are deliberately exact: a changed condition, branch order,
    or constant fails generation instead of silently retaining stale runtime
    behavior.
    """
    if owner in policy_cache:
        return policy_cache[owner]
    method_owner = class_nearest_method_owner(
        result, class_map, owner, "getHurtSound", 1, method_owner_cache
    )
    source, method = method_for(result, method_owner, "getHurtSound", 1)
    body = method.node.child_by_field_name("body")
    if body is None:
        raise ExtractionError(f"{method_owner}.getHurtSound: method lacks body")
    canonical = source.expression(body).canonical

    def sound(constant_name: str) -> str:
        resource_id = sound_events.get(constant_name)
        if resource_id is None:
            raise ExtractionError(
                f"{method_owner}.getHurtSound: unknown SoundEvents.{constant_name}"
            )
        return rust_string(resource_id)

    fixed = re.fullmatch(r"\{return SoundEvents\.([A-Z0-9_]+);\}", canonical)
    if fixed is not None:
        value = f"VanillaEntityHurtSoundPolicy::Fixed({sound(fixed.group(1))})"
        policy_cache[owner] = value
        return value

    variant_owners = {
        "net.minecraft.world.entity.animal.feline.Cat": "Cat",
        "net.minecraft.world.entity.animal.chicken.Chicken": "Chicken",
        "net.minecraft.world.entity.animal.cow.AbstractCow": "Cow",
        "net.minecraft.world.entity.animal.pig.Pig": "Pig",
    }
    variant_body = "{return this.getSoundSet ().hurtSound ().value ();}"
    if method_owner in variant_owners and canonical == variant_body:
        value = (
            "VanillaEntityHurtSoundPolicy::SoundVariant("
            f"VanillaEntitySoundVariantFamily::{variant_owners[method_owner]})"
        )
        policy_cache[owner] = value
        return value

    conditional_specs = {
        "net.minecraft.world.entity.animal.armadillo.Armadillo": (
            "{return this.isScared () ? SoundEvents.ARMADILLO_HURT_REDUCED : SoundEvents.ARMADILLO_HURT;}",
            "ArmadilloScared",
            "ARMADILLO_HURT_REDUCED",
            "ARMADILLO_HURT",
        ),
        "net.minecraft.world.entity.monster.zombie.Drowned": (
            "{return this.isInWater () ? SoundEvents.DROWNED_HURT_WATER : SoundEvents.DROWNED_HURT;}",
            "InWater",
            "DROWNED_HURT_WATER",
            "DROWNED_HURT",
        ),
        "net.minecraft.world.entity.monster.ElderGuardian": (
            "{return this.isInWater () ? SoundEvents.ELDER_GUARDIAN_HURT : SoundEvents.ELDER_GUARDIAN_HURT_LAND;}",
            "InWater",
            "ELDER_GUARDIAN_HURT",
            "ELDER_GUARDIAN_HURT_LAND",
        ),
        "net.minecraft.world.entity.animal.happyghast.HappyGhast": (
            "{return this.isBaby () ? SoundEvents.GHASTLING_HURT : SoundEvents.HAPPY_GHAST_HURT;}",
            "Baby",
            "GHASTLING_HURT",
            "HAPPY_GHAST_HURT",
        ),
        "net.minecraft.world.entity.animal.goat.Goat": (
            "{return this.isScreamingGoat () ? SoundEvents.GOAT_SCREAMING_HURT : SoundEvents.GOAT_HURT;}",
            "GoatScreaming",
            "GOAT_SCREAMING_HURT",
            "GOAT_HURT",
        ),
        "net.minecraft.world.entity.monster.Guardian": (
            "{return this.isInWater () ? SoundEvents.GUARDIAN_HURT : SoundEvents.GUARDIAN_HURT_LAND;}",
            "InWater",
            "GUARDIAN_HURT",
            "GUARDIAN_HURT_LAND",
        ),
        "net.minecraft.world.entity.animal.equine.Horse": (
            "{return this.isBaby () ? SoundEvents.HORSE_HURT_BABY : SoundEvents.HORSE_HURT;}",
            "Baby",
            "HORSE_HURT_BABY",
            "HORSE_HURT",
        ),
        "net.minecraft.world.entity.monster.MagmaCube": (
            "{return this.isTiny () ? SoundEvents.MAGMA_CUBE_HURT_SMALL : SoundEvents.MAGMA_CUBE_HURT;}",
            "Tiny",
            "MAGMA_CUBE_HURT_SMALL",
            "MAGMA_CUBE_HURT",
        ),
        "net.minecraft.world.entity.monster.Shulker": (
            "{return this.isClosed () ? SoundEvents.SHULKER_HURT_CLOSED : SoundEvents.SHULKER_HURT;}",
            "ShulkerClosed",
            "SHULKER_HURT_CLOSED",
            "SHULKER_HURT",
        ),
        "net.minecraft.world.entity.monster.Slime": (
            "{return this.isTiny () ? SoundEvents.SLIME_HURT_SMALL : SoundEvents.SLIME_HURT;}",
            "Tiny",
            "SLIME_HURT_SMALL",
            "SLIME_HURT",
        ),
        "net.minecraft.world.entity.animal.turtle.Turtle": (
            "{return this.isBaby () ? SoundEvents.TURTLE_HURT_BABY : SoundEvents.TURTLE_HURT;}",
            "Baby",
            "TURTLE_HURT_BABY",
            "TURTLE_HURT",
        ),
        "net.minecraft.world.entity.animal.nautilus.ZombieNautilus": (
            "{return this.isUnderWater () ? SoundEvents.ZOMBIE_NAUTILUS_HURT : SoundEvents.ZOMBIE_NAUTILUS_HURT_ON_LAND;}",
            "UnderWater",
            "ZOMBIE_NAUTILUS_HURT",
            "ZOMBIE_NAUTILUS_HURT_ON_LAND",
        ),
    }
    if method_owner in conditional_specs:
        expected, condition, when_true, when_false = conditional_specs[method_owner]
        if canonical != expected:
            raise ExtractionError(
                f"{method_owner}.getHurtSound: unsupported body {canonical!r}"
            )
        value = (
            "VanillaEntityHurtSoundPolicy::Conditional { "
            f"condition: VanillaEntitySoundCondition::{condition}, "
            f"when_true: {sound(when_true)}, when_false: {sound(when_false)} "
            "}"
        )
        policy_cache[owner] = value
        return value

    if method_owner == "net.minecraft.world.entity.monster.creaking.Creaking":
        expected = (
            "{return this.isHeartBound () ? SoundEvents.CREAKING_SWAY : "
            "super.getHurtSound (source);}"
        )
        if canonical != expected:
            raise ExtractionError(
                f"{method_owner}.getHurtSound: unsupported body {canonical!r}"
            )
        superclass = class_map[method_owner].superclass
        if superclass is None:
            raise ExtractionError(f"{method_owner}.getHurtSound: missing superclass")
        false_policy = class_hurt_sound_policy(
            result,
            class_map,
            superclass,
            sound_events,
            method_owner_cache,
            policy_cache,
        )
        expected_false = (
            "VanillaEntityHurtSoundPolicy::Fixed("
            f"{sound('HOSTILE_HURT')})"
        )
        if false_policy != expected_false:
            raise ExtractionError(
                f"{method_owner}.getHurtSound: unexpected super policy {false_policy}"
            )
        value = (
            "VanillaEntityHurtSoundPolicy::Conditional { "
            "condition: VanillaEntitySoundCondition::CreakingHeartBound, "
            f"when_true: {sound('CREAKING_SWAY')}, "
            f"when_false: {sound('HOSTILE_HURT')} "
            "}"
        )
        policy_cache[owner] = value
        return value

    if method_owner == "net.minecraft.world.entity.animal.nautilus.Nautilus":
        expected = (
            "{if (this.isBaby ()) {return this.isUnderWater () ? "
            "SoundEvents.BABY_NAUTILUS_HURT : "
            "SoundEvents.BABY_NAUTILUS_HURT_ON_LAND;} else {return "
            "this.isUnderWater () ? SoundEvents.NAUTILUS_HURT : "
            "SoundEvents.NAUTILUS_HURT_ON_LAND;}}"
        )
        if canonical != expected:
            raise ExtractionError(
                f"{method_owner}.getHurtSound: unsupported body {canonical!r}"
            )
        value = (
            "VanillaEntityHurtSoundPolicy::NautilusAgeWater { "
            f"baby_underwater: {sound('BABY_NAUTILUS_HURT')}, "
            f"baby_land: {sound('BABY_NAUTILUS_HURT_ON_LAND')}, "
            f"adult_underwater: {sound('NAUTILUS_HURT')}, "
            f"adult_land: {sound('NAUTILUS_HURT_ON_LAND')} "
            "}"
        )
        policy_cache[owner] = value
        return value

    if method_owner == "net.minecraft.world.entity.animal.wolf.Wolf":
        expected = (
            "{return this.canArmorAbsorb (source) ? SoundEvents.WOLF_ARMOR_DAMAGE : "
            "this.getSoundSet ().hurtSound ().value ();}"
        )
        if canonical != expected:
            raise ExtractionError(
                f"{method_owner}.getHurtSound: unsupported body {canonical!r}"
            )
        value = (
            "VanillaEntityHurtSoundPolicy::WolfVariant { "
            f"armor_damage: {sound('WOLF_ARMOR_DAMAGE')} "
            "}"
        )
        policy_cache[owner] = value
        return value

    if method_owner == "net.minecraft.world.entity.player.Player":
        expected = "{return source.type ().effects ().sound ();}"
        if canonical != expected:
            raise ExtractionError(
                f"{method_owner}.getHurtSound: unsupported body {canonical!r}"
            )
        value = "VanillaEntityHurtSoundPolicy::PlayerDamageType"
        policy_cache[owner] = value
        return value

    if method_owner == "net.minecraft.world.entity.animal.golem.CopperGolem":
        expected = (
            "{return CopperGolemOxidationLevels.getOxidationLevel "
            "(this.getWeatherState ()).hurtSound ();}"
        )
        if canonical != expected:
            raise ExtractionError(
                f"{method_owner}.getHurtSound: unsupported body {canonical!r}"
            )
        oxidation_owner = (
            "net.minecraft.world.entity.animal.golem.CopperGolemOxidationLevels"
        )
        oxidation_source = source_for_owner(result, oxidation_owner)
        resources = []
        for state in ("UNAFFECTED", "EXPOSED", "WEATHERED", "OXIDIZED"):
            field = next(
                (
                    item
                    for item in oxidation_source.fields_for(oxidation_owner)
                    if item.name == state
                ),
                None,
            )
            if field is None or field.initializer is None:
                raise ExtractionError(f"{oxidation_owner}.{state}: missing initializer")
            match = re.fullmatch(
                r"new CopperGolemOxidationLevel \(SoundEvents\.[A-Z0-9_]+, "
                r"SoundEvents\.([A-Z0-9_]+), .*\)",
                field.initializer.canonical,
            )
            if match is None:
                raise ExtractionError(
                    f"{oxidation_owner}.{state}: unsupported initializer "
                    f"{field.initializer.canonical!r}"
                )
            resources.append(sound(match.group(1)))
        value = (
            "VanillaEntityHurtSoundPolicy::CopperGolemOxidation { "
            f"unaffected: {resources[0]}, exposed: {resources[1]}, "
            f"weathered: {resources[2]}, oxidized: {resources[3]} "
            "}"
        )
        policy_cache[owner] = value
        return value

    raise ExtractionError(
        f"{method_owner}.getHurtSound: unsupported body {canonical!r}"
    )


def class_death_sound_policy(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    sound_events: dict[str, str],
    method_owner_cache: dict[str, str],
    policy_cache: dict[str, str],
) -> str:
    """Interpret the nearest Vanilla ``LivingEntity.getDeathSound`` body.

    Fixed ``SoundEvents`` returns are open-ended and source-derived. Dynamic
    bodies are matched exactly so a changed predicate, branch order, registry
    accessor, or sound constant fails generation.
    """
    if owner in policy_cache:
        return policy_cache[owner]
    method_owner = class_nearest_method_owner(
        result, class_map, owner, "getDeathSound", 0, method_owner_cache
    )
    source, method = method_for(result, method_owner, "getDeathSound", 0)
    body = method.node.child_by_field_name("body")
    if body is None:
        raise ExtractionError(f"{method_owner}.getDeathSound: method lacks body")
    canonical = source.expression(body).canonical

    def sound(constant_name: str) -> str:
        resource_id = sound_events.get(constant_name)
        if resource_id is None:
            raise ExtractionError(
                f"{method_owner}.getDeathSound: unknown SoundEvents.{constant_name}"
            )
        return rust_string(resource_id)

    fixed = re.fullmatch(r"\{return SoundEvents\.([A-Z0-9_]+);\}", canonical)
    if fixed is not None:
        value = f"VanillaEntityDeathSoundPolicy::Fixed({sound(fixed.group(1))})"
        policy_cache[owner] = value
        return value

    variant_owners = {
        "net.minecraft.world.entity.animal.feline.Cat": "Cat",
        "net.minecraft.world.entity.animal.chicken.Chicken": "Chicken",
        "net.minecraft.world.entity.animal.cow.AbstractCow": "Cow",
        "net.minecraft.world.entity.animal.pig.Pig": "Pig",
        "net.minecraft.world.entity.animal.wolf.Wolf": "Wolf",
    }
    variant_body = "{return this.getSoundSet ().deathSound ().value ();}"
    if method_owner in variant_owners and canonical == variant_body:
        value = (
            "VanillaEntityDeathSoundPolicy::SoundVariant("
            f"VanillaEntitySoundVariantFamily::{variant_owners[method_owner]})"
        )
        policy_cache[owner] = value
        return value

    conditional_specs = {
        "net.minecraft.world.entity.animal.equine.Horse": (
            "{return this.isBaby () ? SoundEvents.HORSE_DEATH_BABY : SoundEvents.HORSE_DEATH;}",
            "Baby",
            "HORSE_DEATH_BABY",
            "HORSE_DEATH",
        ),
        "net.minecraft.world.entity.animal.goat.Goat": (
            "{return this.isScreamingGoat () ? SoundEvents.GOAT_SCREAMING_DEATH : SoundEvents.GOAT_DEATH;}",
            "GoatScreaming",
            "GOAT_SCREAMING_DEATH",
            "GOAT_DEATH",
        ),
        "net.minecraft.world.entity.animal.happyghast.HappyGhast": (
            "{return this.isBaby () ? SoundEvents.GHASTLING_DEATH : SoundEvents.HAPPY_GHAST_DEATH;}",
            "Baby",
            "GHASTLING_DEATH",
            "HAPPY_GHAST_DEATH",
        ),
        "net.minecraft.world.entity.animal.turtle.Turtle": (
            "{return this.isBaby () ? SoundEvents.TURTLE_DEATH_BABY : SoundEvents.TURTLE_DEATH;}",
            "Baby",
            "TURTLE_DEATH_BABY",
            "TURTLE_DEATH",
        ),
        "net.minecraft.world.entity.monster.MagmaCube": (
            "{return this.isTiny () ? SoundEvents.MAGMA_CUBE_DEATH_SMALL : SoundEvents.MAGMA_CUBE_DEATH;}",
            "Tiny",
            "MAGMA_CUBE_DEATH_SMALL",
            "MAGMA_CUBE_DEATH",
        ),
        "net.minecraft.world.entity.monster.Slime": (
            "{return this.isTiny () ? SoundEvents.SLIME_DEATH_SMALL : SoundEvents.SLIME_DEATH;}",
            "Tiny",
            "SLIME_DEATH_SMALL",
            "SLIME_DEATH",
        ),
        "net.minecraft.world.entity.monster.zombie.Drowned": (
            "{return this.isInWater () ? SoundEvents.DROWNED_DEATH_WATER : SoundEvents.DROWNED_DEATH;}",
            "InWater",
            "DROWNED_DEATH_WATER",
            "DROWNED_DEATH",
        ),
        "net.minecraft.world.entity.monster.ElderGuardian": (
            "{return this.isInWater () ? SoundEvents.ELDER_GUARDIAN_DEATH : SoundEvents.ELDER_GUARDIAN_DEATH_LAND;}",
            "InWater",
            "ELDER_GUARDIAN_DEATH",
            "ELDER_GUARDIAN_DEATH_LAND",
        ),
        "net.minecraft.world.entity.monster.Guardian": (
            "{return this.isInWater () ? SoundEvents.GUARDIAN_DEATH : SoundEvents.GUARDIAN_DEATH_LAND;}",
            "InWater",
            "GUARDIAN_DEATH",
            "GUARDIAN_DEATH_LAND",
        ),
        "net.minecraft.world.entity.animal.nautilus.ZombieNautilus": (
            "{return this.isUnderWater () ? SoundEvents.ZOMBIE_NAUTILUS_DEATH : SoundEvents.ZOMBIE_NAUTILUS_DEATH_ON_LAND;}",
            "UnderWater",
            "ZOMBIE_NAUTILUS_DEATH",
            "ZOMBIE_NAUTILUS_DEATH_ON_LAND",
        ),
    }
    if method_owner in conditional_specs:
        expected, condition, when_true, when_false = conditional_specs[method_owner]
        if canonical != expected:
            raise ExtractionError(
                f"{method_owner}.getDeathSound: unsupported body {canonical!r}"
            )
        value = (
            "VanillaEntityDeathSoundPolicy::Conditional { "
            f"condition: VanillaEntitySoundCondition::{condition}, "
            f"when_true: {sound(when_true)}, when_false: {sound(when_false)} "
            "}"
        )
        policy_cache[owner] = value
        return value

    if method_owner == "net.minecraft.world.entity.animal.nautilus.Nautilus":
        expected = (
            "{if (this.isBaby ()) {return this.isUnderWater () ? "
            "SoundEvents.BABY_NAUTILUS_DEATH : "
            "SoundEvents.BABY_NAUTILUS_DEATH_ON_LAND;} else {return "
            "this.isUnderWater () ? SoundEvents.NAUTILUS_DEATH : "
            "SoundEvents.NAUTILUS_DEATH_ON_LAND;}}"
        )
        if canonical != expected:
            raise ExtractionError(
                f"{method_owner}.getDeathSound: unsupported body {canonical!r}"
            )
        value = (
            "VanillaEntityDeathSoundPolicy::NautilusAgeWater { "
            f"baby_underwater: {sound('BABY_NAUTILUS_DEATH')}, "
            f"baby_land: {sound('BABY_NAUTILUS_DEATH_ON_LAND')}, "
            f"adult_underwater: {sound('NAUTILUS_DEATH')}, "
            f"adult_land: {sound('NAUTILUS_DEATH_ON_LAND')} "
            "}"
        )
        policy_cache[owner] = value
        return value

    if method_owner == "net.minecraft.world.entity.animal.golem.CopperGolem":
        expected = (
            "{return CopperGolemOxidationLevels.getOxidationLevel "
            "(this.getWeatherState ()).deathSound ();}"
        )
        if canonical != expected:
            raise ExtractionError(
                f"{method_owner}.getDeathSound: unsupported body {canonical!r}"
            )
        oxidation_owner = (
            "net.minecraft.world.entity.animal.golem.CopperGolemOxidationLevels"
        )
        oxidation_source = source_for_owner(result, oxidation_owner)
        resources = []
        for state in ("UNAFFECTED", "EXPOSED", "WEATHERED", "OXIDIZED"):
            field = next(
                (
                    item
                    for item in oxidation_source.fields_for(oxidation_owner)
                    if item.name == state
                ),
                None,
            )
            if field is None or field.initializer is None:
                raise ExtractionError(f"{oxidation_owner}.{state}: missing initializer")
            match = re.fullmatch(
                r"new CopperGolemOxidationLevel \(SoundEvents\.[A-Z0-9_]+, "
                r"SoundEvents\.[A-Z0-9_]+, SoundEvents\.([A-Z0-9_]+), .*\)",
                field.initializer.canonical,
            )
            if match is None:
                raise ExtractionError(
                    f"{oxidation_owner}.{state}: unsupported initializer "
                    f"{field.initializer.canonical!r}"
                )
            resources.append(sound(match.group(1)))
        value = (
            "VanillaEntityDeathSoundPolicy::CopperGolemOxidation { "
            f"unaffected: {resources[0]}, exposed: {resources[1]}, "
            f"weathered: {resources[2]}, oxidized: {resources[3]} "
            "}"
        )
        policy_cache[owner] = value
        return value

    raise ExtractionError(
        f"{method_owner}.getDeathSound: unsupported body {canonical!r}"
    )


def class_sound_volume_policy(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    method_owner_cache: dict[str, str],
    policy_cache: dict[str, str],
) -> str:
    """Interpret the nearest Vanilla ``LivingEntity.getSoundVolume`` body."""
    if owner in policy_cache:
        return policy_cache[owner]
    method_owner = class_nearest_method_owner(
        result, class_map, owner, "getSoundVolume", 0, method_owner_cache
    )
    source, method = method_for(result, method_owner, "getSoundVolume", 0)
    body = method.node.child_by_field_name("body")
    if body is None:
        raise ExtractionError(f"{method_owner}.getSoundVolume: method lacks body")
    canonical = source.expression(body).canonical
    fixed = re.fullmatch(r"\{return ([0-9]+(?:\.[0-9]+)?)F;\}", canonical)
    if fixed is not None:
        value = (
            "VanillaEntitySoundVolumePolicy::Fixed("
            f"{fixed.group(1)}_f32.to_bits())"
        )
    elif (
        method_owner == "net.minecraft.world.entity.monster.Slime"
        and canonical == "{return 0.4F * this.getSize ();}"
    ):
        value = "VanillaEntitySoundVolumePolicy::SlimeSize"
    elif (
        method_owner == "net.minecraft.world.entity.animal.happyghast.HappyGhast"
        and canonical == "{return this.isBaby () ? 1.0F : 4.0F;}"
    ):
        value = "VanillaEntitySoundVolumePolicy::HappyGhastAge"
    else:
        raise ExtractionError(
            f"{method_owner}.getSoundVolume: unsupported body {canonical!r}"
        )
    policy_cache[owner] = value
    return value


def class_boat_ride_height_multiplier(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    cache: dict[str, str],
) -> str:
    """Resolve the concrete boat class ``rideHeight`` multiplier."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(f"unknown boat class in rideHeight hierarchy: {owner}")
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == "rideHeight" and len(method.parameters) == 1
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.rideHeight/1: expected at most one method")
    if not methods:
        if item.superclass is None:
            raise ExtractionError(f"{owner}: no rideHeight override or superclass")
        value = class_boat_ride_height_multiplier(
            result, class_map, item.superclass, cache
        )
        cache[owner] = value
        return value

    expression = method_single_return_expression(result, owner, "rideHeight")
    compact = expression.replace(" ", "")
    multipliers = {
        "dimensions.height()/3.0F": "(1.0_f32 / 3.0).to_bits()",
        "dimensions.height()*0.8888889F": "0.888_888_9_f32.to_bits()",
    }
    if compact not in multipliers:
        raise ExtractionError(
            f"{owner}.rideHeight: unsupported return expression {expression!r}"
        )
    value = multipliers[compact]
    cache[owner] = value
    return value


def class_can_use_every_equipment_slot(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    cache: dict[str, bool],
) -> bool:
    """Resolve whether the nearest ``canUseSlot`` implementation is constant true."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        return False
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == "canUseSlot" and len(method.parameters) == 1
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.canUseSlot/1: expected at most one method")
    if not methods:
        if item.superclass is None:
            value = False
        else:
            value = class_can_use_every_equipment_slot(
                result, class_map, item.superclass, cache
            )
        cache[owner] = value
        return value

    expression = method_single_return_expression(result, owner, "canUseSlot")
    compact = expression.replace(" ", "")
    dynamic_expressions = {
        "slot!=EquipmentSlot.SADDLE?super.canUseSlot(slot):this.isAlive()&&!this.isBaby()&&this.isTamed()",
        "slot!=EquipmentSlot.BODY?super.canUseSlot(slot):this.isAlive()&&!this.isBaby()",
        "slot!=EquipmentSlot.SADDLE&&slot!=EquipmentSlot.BODY?super.canUseSlot(slot):this.isAlive()&&!this.isBaby()&&this.isTame()",
        "slot!=EquipmentSlot.SADDLE?super.canUseSlot(slot):this.isAlive()&&!this.isBaby()",
        "slot!=EquipmentSlot.BODY&&slot!=EquipmentSlot.SADDLE&&!this.isDisabled(slot)",
    }
    if compact == "true":
        value = True
    elif compact in dynamic_expressions:
        value = False
    else:
        raise ExtractionError(
            f"{owner}.canUseSlot: unsupported return expression {expression!r}"
        )
    cache[owner] = value
    return value


def class_illager_arm_pose_uses_celebrating(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    cache: dict[str, bool],
) -> bool:
    """Resolve whether the nearest illager ``getArmPose`` can return ``CELEBRATING``."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(f"unknown entity class in getArmPose hierarchy: {owner}")

    abstract_illager = "net.minecraft.world.entity.monster.illager.AbstractIllager"
    current: str | None = owner
    while current is not None and current != abstract_illager:
        current_item = class_map.get(current)
        current = current_item.superclass if current_item is not None else None
    if current != abstract_illager:
        cache[owner] = False
        return False

    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == "getArmPose" and len(method.parameters) == 0
    ]
    if len(methods) > 1:
        raise ExtractionError(f"{owner}.getArmPose/0: expected at most one method")
    if not methods:
        if item.superclass is None:
            raise ExtractionError(f"{owner}: no getArmPose override or superclass")
        value = class_illager_arm_pose_uses_celebrating(
            result, class_map, item.superclass, cache
        )
        cache[owner] = value
        return value

    body = methods[0].node.child_by_field_name("body")
    if body is None:
        raise ExtractionError(f"{owner}.getArmPose: method lacks a body")
    returns = [
        source.expression(node.named_children[0]).canonical.replace(" ", "")
        for node in descendants(body)
        if node.type == "return_statement" and len(node.named_children) == 1
    ]
    if not returns:
        raise ExtractionError(f"{owner}.getArmPose: expected at least one return expression")
    value = any(
        "AbstractIllager.IllagerArmPose.CELEBRATING" in expression
        for expression in returns
    )
    cache[owner] = value
    return value


def method_mask(result: ExtractionResult, owner: str, name: str, mode: str) -> tuple[int, str]:
    source, method = method_for(result, owner, name)
    body = method.node.child_by_field_name("body")
    if body is None:
        raise ExtractionError(f"{owner}.{name}: method lacks a body")
    if mode == "shared_bit":
        values: list[tuple[int, str]] = []
        for node in descendants(body):
            if node.type != "method_invocation":
                continue
            name_node = node.child_by_field_name("name")
            arguments = node.child_by_field_name("arguments")
            if name_node is None or source.text(name_node) != "getSharedFlag" or arguments is None or len(arguments.named_children) != 1:
                continue
            argument = arguments.named_children[0]
            bit = evaluate_integer_expression(source, argument, integer_constants(result, owner))
            values.append((1 << bit, source.expression(node).canonical))
        unique = list(dict.fromkeys(values))
        if len(unique) != 1:
            raise ExtractionError(f"{owner}.{name}: expected one getSharedFlag bit, found {unique}")
        return unique[0]
    values = []
    for node in descendants(body):
        if node.type != "binary_expression":
            continue
        operators = [source.text(child) for child in node.children if not child.is_named]
        if "&" not in operators:
            continue
        left = node.child_by_field_name("left")
        right = node.child_by_field_name("right")
        for candidate in (left, right):
            if candidate is None:
                continue
            try:
                value = evaluate_integer_expression(source, candidate, integer_constants(result, owner))
            except (ExtractionError, ValueError):
                continue
            values.append((value, source.expression(node).canonical))
    unique = list(dict.fromkeys(values))
    if len(unique) != 1:
        raise ExtractionError(f"{owner}.{name}: expected one inline mask, found {unique}")
    return unique[0]


def resolve_flags(result: ExtractionResult) -> list[ResolvedFlag]:
    resolved: list[ResolvedFlag] = []
    for rust_name, (owner, accessor, constant) in NAMED_FLAG_ALIASES.items():
        field = field_for(result, owner, constant)
        if field.initializer is None:
            raise ExtractionError(f"{owner}.{constant}: missing initializer")
        value = evaluate_integer_expression(
            source_for_owner(result, owner), field.initializer.node, integer_constants(result, owner)
        )
        resolved.append(ResolvedFlag(rust_name, owner, accessor, owner, constant, field.initializer.canonical, value, "named vanilla flag constant"))
    for rust_name, (owner, accessor, evidence_owner, method, mode) in METHOD_FLAG_ALIASES.items():
        value, expression = method_mask(result, evidence_owner, method, mode)
        resolved.append(ResolvedFlag(rust_name, owner, accessor, evidence_owner, method, expression, value, "vanilla uses an inline accessor mask rather than a named constant"))
    model_source = source_for_owner(result, "net.minecraft.world.entity.player.PlayerModelPart")
    model_type = next(item for item in model_source.types if item.fqcn == "net.minecraft.world.entity.player.PlayerModelPart")
    constants = [item for item in model_type.body_node.named_children if item.type == "enum_constant"]
    by_name = {model_source.text(item.child_by_field_name("name")): item for item in constants}
    for rust_name, constant in MODEL_PART_ALIASES.items():
        node = by_name[constant]
        arguments = node.child_by_field_name("arguments")
        bit_node = arguments.named_children[0]
        bit = evaluate_integer_expression(model_source, bit_node, {})
        resolved.append(
            ResolvedFlag(
                rust_name,
                "net.minecraft.world.entity.Avatar",
                "DATA_PLAYER_MODE_CUSTOMISATION",
                "net.minecraft.world.entity.player.PlayerModelPart",
                constant,
                model_source.expression(node).canonical,
                1 << bit,
                "mask is defined by PlayerModelPart enum bit rather than Avatar",
            )
        )
    names = [item.rust_name for item in resolved]
    if len(names) != len(set(names)):
        raise ExtractionError("duplicate compatibility flag resolver")
    return resolved


def resolve_value_masks(result: ExtractionResult) -> list[tuple[str, int]]:
    resolved: list[tuple[str, int]] = []
    for rust_name, (owner, method, mode) in METHOD_VALUE_MASK_ALIASES.items():
        value, _expression = method_mask(result, owner, method, mode)
        if value <= 0 or value > 0xFF:
            raise ExtractionError(f"{owner}.{method}: value mask is outside one byte: {value}")
        resolved.append((rust_name, value))
    names = [name for name, _value in resolved]
    if len(names) != len(set(names)):
        raise ExtractionError("duplicate compatibility value-mask resolver")
    return resolved


def method_binary_integer_operand(
    result: ExtractionResult, owner: str, name: str, parameter_count: int, operator: str
) -> tuple[int, str]:
    source, method = method_for(result, owner, name, parameter_count)
    body = method.node.child_by_field_name("body")
    if body is None:
        raise ExtractionError(f"{owner}.{name}: method lacks a body")
    values: list[tuple[int, str]] = []
    for node in descendants(body):
        if node.type != "binary_expression":
            continue
        operators = [source.text(child) for child in node.children if not child.is_named]
        if operators != [operator]:
            continue
        constants: list[int] = []
        for side in (node.child_by_field_name("left"), node.child_by_field_name("right")):
            if side is None:
                continue
            try:
                constants.append(
                    evaluate_integer_expression(source, side, integer_constants(result, owner))
                )
            except (ExtractionError, ValueError):
                continue
        if len(constants) != 1:
            raise ExtractionError(
                f"{owner}.{name}: expected one constant operand for {operator!r}, "
                f"found {constants} in {source.expression(node).canonical}"
            )
        values.append((constants[0], source.expression(node).canonical))
    if len(values) != 1:
        raise ExtractionError(
            f"{owner}.{name}: expected one {operator!r} expression with a constant operand, "
            f"found {values}"
        )
    return values[0]


def resolve_packed_values(result: ExtractionResult) -> list[tuple[str, int]]:
    resolved: list[tuple[str, int]] = []
    for rust_name, (owner, method, parameter_count, operator) in METHOD_PACKED_VALUE_ALIASES.items():
        value, _expression = method_binary_integer_operand(
            result, owner, method, parameter_count, operator
        )
        if value <= 0 or value > 0x7FFF_FFFF:
            raise ExtractionError(f"{owner}.{method}: packed value is outside positive i32: {value}")
        resolved.append((rust_name, value))
    names = [name for name, _value in resolved]
    if len(names) != len(set(names)):
        raise ExtractionError("duplicate compatibility packed-value resolver")
    return resolved


def method_return_integer(result: ExtractionResult, owner: str, name: str) -> int:
    source, method = method_for(result, owner, name)
    body = method.node.child_by_field_name("body")
    returns = [item for item in descendants(body) if item.type == "return_statement"]
    if len(returns) != 1 or len(returns[0].named_children) != 1:
        raise ExtractionError(f"{owner}.{name}: expected one scalar return")
    return evaluate_integer_expression(source, returns[0].named_children[0], integer_constants(result, owner))


def format_rust(source: str) -> str:
    completed = subprocess.run(
        ["rustfmt", "--emit", "stdout", "--edition", "2021"],
        input=source,
        text=True,
        capture_output=True,
        check=True,
    )
    return completed.stdout


def render_protocol(result: ExtractionResult) -> str:
    """The synched-data ids and flag bits, as the protocol crate owns them.

    `SynchedEntityData.defineId` assigns each accessor its wire id, and the
    flag accessors pack several booleans into one of those ids. Both are facts
    about what goes on the wire rather than about how a client renders it, so
    they live here and every consumer — the client world model and the
    dedicated server alike — reads them from one place.
    """
    accessors = {(item.owner, item.java_name): item for item in result.accessors}
    lines = [
        "// @generated by scripts/generate-entity-metadata-schema.py; do not edit manually.",
        "// Vanilla version: 26.1.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-entity-metadata-schema.py update",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-entity-metadata-schema.py check",
        f"// Stable source manifest SHA-256: {result.manifest_digest}",
        "",
        "//! Vanilla 26.1 synched-entity-data ids and flag bits.",
        "//!",
        "//! `SynchedEntityData.defineId` assigns each accessor the id it is",
        "//! published under, and the flag accessors pack several booleans into",
        "//! one byte behind a single id. Both are wire facts: the server writes",
        "//! them into `ClientboundSetEntityDataPacket` and the client reads them",
        "//! back out, so neither side may keep its own copy.",
        "",
    ]

    for public_name, key in DATA_ALIASES.items():
        accessor = accessors[key]
        internal = "GENERATED_" + public_name
        lines.append(f"const {internal}: u8 = {accessor.data_id};")
        lines.append(f"pub const {public_name}: u8 = {internal};")
    lines.append("")

    resolved_defaults, _ = resolve_semantic_defaults(result)
    for public_name, key in PROTOCOL_DEFAULT_ALIASES.items():
        value = normalize_default(result, accessors[key], resolved_defaults)
        if value is None or value.scalar_type is None or value.scalar_rust is None:
            raise ExtractionError(f"{public_name}: default has no scalar compatibility value")
        lines.append(f"const GENERATED_{public_name}: {value.scalar_type} = {value.scalar_rust};")
        lines.append(f"pub const {public_name}: {value.scalar_type} = GENERATED_{public_name};")
    lines.append("")

    for public_name in (
        "VANILLA_COPPER_GOLEM_STATE_IDLE_ID",
        "VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID",
        "VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID",
        "VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID",
        "VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID",
        "VANILLA_ARMADILLO_STATE_IDLE_ID",
        "VANILLA_ARMADILLO_STATE_ROLLING_ID",
        "VANILLA_ARMADILLO_STATE_SCARED_ID",
        "VANILLA_ARMADILLO_STATE_UNROLLING_ID",
        "VANILLA_SNIFFER_STATE_IDLING_ID",
        "VANILLA_SNIFFER_STATE_FEELING_HAPPY_ID",
        "VANILLA_SNIFFER_STATE_SCENTING_ID",
        "VANILLA_SNIFFER_STATE_SNIFFING_ID",
        "VANILLA_SNIFFER_STATE_SEARCHING_ID",
        "VANILLA_SNIFFER_STATE_DIGGING_ID",
        "VANILLA_SNIFFER_STATE_RISING_ID",
    ):
        enum_fqcn, constant_name, argument_index = ENUM_CONSTRUCTOR_ID_ALIASES[public_name]
        value = enum_constant_constructor_integer(
            result, enum_fqcn, constant_name, argument_index
        )
        lines.append(f"const GENERATED_{public_name}: i32 = {value};")
        lines.append(f"pub const {public_name}: i32 = GENERATED_{public_name};")
    lines.append("")

    for public_name, (enum_fqcn, constant_name) in ENUM_ORDINAL_ID_ALIASES.items():
        value = enum_constant_ordinal(result, enum_fqcn, constant_name)
        lines.append(f"const GENERATED_{public_name}: i32 = {value};")
        lines.append(f"pub const {public_name}: i32 = GENERATED_{public_name};")
    lines.append("")

    digging_event = integer_constants(result, "net.minecraft.world.entity.EntityEvent")["SNIFFER_DIGGING_SOUND"]
    lines.append(f"pub const VANILLA_SNIFFER_DIGGING_SOUND_EVENT_ID: i8 = {digging_event};")
    lines.append("")

    for flag in resolve_flags(result):
        internal = "GENERATED_" + flag.rust_name
        signed = flag.value if flag.value < 128 else flag.value - 256
        lines.append(f"const {internal}: i8 = {signed};")
        lines.append(f"pub const {flag.rust_name}: i8 = {internal};")
    lines.append("")

    return format_rust("\n".join(lines))


def render(result: ExtractionResult) -> str:
    accessors = {(item.owner, item.java_name): item for item in result.accessors}
    resolved_defaults, _ = resolve_semantic_defaults(result)
    normalized = {
        key: normalize_default(result, item, resolved_defaults) for key, item in accessors.items()
    }
    value_masks = resolve_value_masks(result)
    packed_values = resolve_packed_values(result)
    camera_post_effects = camera_post_effect_specs(result)
    entity_event_handlers = entity_event_handler_specs(result)
    sound_events = sound_event_resources_by_constant(result)
    item_tags = item_tag_resources(result)

    class_map = {item.fqcn: item for item in result.classes}
    ender_dragon_phase_registrations = static_factory_registration_classes(
        result,
        class_map,
        "net.minecraft.world.entity.boss.enderdragon.phases.EnderDragonPhase",
    )
    ender_dragon_sitting_cache: dict[str, bool] = {}
    ender_dragon_phase_catalog = sorted(
        (
            ordinal,
            java_field_name,
            implementation_class,
            class_zero_argument_boolean_method(
                result,
                class_map,
                implementation_class,
                "isSitting",
                ender_dragon_sitting_cache,
            ),
        )
        for java_field_name, (
            ordinal,
            implementation_class,
        ) in ender_dragon_phase_registrations.items()
    )
    if [item[0] for item in ender_dragon_phase_catalog] != list(
        range(len(ender_dragon_phase_catalog))
    ):
        raise ExtractionError("EnderDragonPhase: phase registrations are not contiguous")
    ender_dragon_sitting_phase_ids = [
        ordinal
        for ordinal, _, _, is_sitting in ender_dragon_phase_catalog
        if is_sitting
    ]
    if not ender_dragon_sitting_phase_ids:
        raise ExtractionError("EnderDragonPhase: no sitting phase implementations")
    if len(ender_dragon_sitting_phase_ids) == len(ender_dragon_phase_registrations):
        raise ExtractionError("EnderDragonPhase: every phase implementation is sitting")
    pickable_cache: dict[str, bool] = {}
    pushability_cache: dict[str, str] = {}
    pushes_local_player_cache: dict[str, bool] = {}
    push_entities_owner_cache: dict[str, str] = {}
    do_push_owner_cache: dict[str, str] = {}
    collidability_cache: dict[str, str] = {}
    sound_source_cache: dict[str, str] = {}
    death_sound_owner_cache: dict[str, str] = {}
    death_sound_policy_cache: dict[str, str] = {}
    hurt_sound_owner_cache: dict[str, str] = {}
    hurt_sound_policy_cache: dict[str, str] = {}
    sound_volume_owner_cache: dict[str, str] = {}
    sound_volume_policy_cache: dict[str, str] = {}
    boat_ride_height_cache: dict[str, str] = {}
    minecart_display_offset_cache: dict[str, int] = {}
    sanitize_scale_behavior_cache: dict[str, str | None] = {}
    can_use_every_slot_cache: dict[str, bool] = {}
    illager_celebrating_cache: dict[str, bool] = {}
    handle_entity_event_owner_cache: dict[str, str] = {}
    interact_owner_cache: dict[str, str] = {}
    mob_interact_owner_cache: dict[str, str] = {}
    passenger_attachment_owner_cache: dict[str, str] = {}
    vehicle_attachment_owner_cache: dict[str, str] = {}
    controlling_passenger_owner_cache: dict[str, str] = {}
    rope_hold_position_owner_cache: dict[str, str] = {}
    ai_step_owner_cache: dict[str, str] = {}
    baby_behavior_cache: dict[str, str] = {}
    baby_age_scale_cache: dict[str, str] = {}

    def interpolation_policy(runtime_class: str | None) -> str:
        if extends_class(runtime_class, "net.minecraft.world.entity.monster.Shulker"):
            return "ShulkerAttachPosition"
        if extends_class(runtime_class, "net.minecraft.world.entity.Display"):
            return "DisplayImmediate"
        if extends_class(
            runtime_class,
            "net.minecraft.world.entity.vehicle.minecart.AbstractMinecart",
        ):
            return "MinecartFeatureGated"
        if any(
            extends_class(runtime_class, owner)
            for owner in (
                "net.minecraft.world.entity.LivingEntity",
                "net.minecraft.world.entity.ExperienceOrb",
                "net.minecraft.world.entity.projectile.FishingHook",
                "net.minecraft.world.entity.vehicle.boat.AbstractBoat",
            )
        ):
            return "GenericThreeStep"
        return "DirectSetPos"

    def client_movement_policy(runtime_class: str | None) -> str:
        if extends_class(runtime_class, "net.minecraft.world.entity.vehicle.VehicleEntity"):
            return "Vehicle"
        if extends_class(runtime_class, "net.minecraft.world.entity.item.ItemEntity"):
            return "Item"
        if extends_class(runtime_class, "net.minecraft.world.entity.ExperienceOrb"):
            return "ExperienceOrb"
        if extends_class(runtime_class, "net.minecraft.world.entity.item.FallingBlockEntity"):
            return "FallingBlock"
        if extends_class(runtime_class, "net.minecraft.world.entity.item.PrimedTnt"):
            return "PrimedTnt"
        if extends_class(
            runtime_class, "net.minecraft.world.entity.projectile.arrow.AbstractArrow"
        ):
            return "AbstractArrow"
        if extends_class(
            runtime_class,
            "net.minecraft.world.entity.projectile.hurtingprojectile.windcharge.AbstractWindCharge",
        ):
            return "WindCharge"
        exact_hurting_projectile_owners = {
            "net.minecraft.world.entity.projectile.hurtingprojectile.DragonFireball": "DragonFireball",
            "net.minecraft.world.entity.projectile.hurtingprojectile.WitherSkull": "WitherSkull",
        }
        if runtime_class in exact_hurting_projectile_owners:
            return exact_hurting_projectile_owners[runtime_class]
        if extends_class(
            runtime_class,
            "net.minecraft.world.entity.projectile.hurtingprojectile.AbstractHurtingProjectile",
        ):
            return "HurtingProjectile"
        if extends_class(
            runtime_class,
            "net.minecraft.world.entity.projectile.throwableitemprojectile.ThrownExperienceBottle",
        ):
            return "ThrownExperienceBottle"
        if extends_class(
            runtime_class,
            "net.minecraft.world.entity.projectile.throwableitemprojectile.AbstractThrownPotion",
        ):
            return "ThrownPotion"
        if extends_class(
            runtime_class, "net.minecraft.world.entity.projectile.ThrowableProjectile"
        ):
            return "ThrowableProjectile"
        exact_owners = {
            "net.minecraft.world.entity.projectile.LlamaSpit": "LlamaSpit",
            "net.minecraft.world.entity.projectile.EyeOfEnder": "EyeOfEnder",
            "net.minecraft.world.entity.projectile.FireworkRocketEntity": "FireworkRocket",
            "net.minecraft.world.entity.projectile.FishingHook": "FishingHook",
            "net.minecraft.world.entity.projectile.ShulkerBullet": "ShulkerBullet",
        }
        return exact_owners.get(runtime_class, "ServerSynchronized")

    ai_step_reaches_living_cache: dict[str, bool] = {}

    def ai_step_reaches_living_block_effect(runtime_class: str | None) -> bool:
        if runtime_class is None:
            return False
        if runtime_class in ai_step_reaches_living_cache:
            return ai_step_reaches_living_cache[runtime_class]
        owner = class_nearest_method_owner(
            result,
            class_map,
            runtime_class,
            "aiStep",
            0,
            ai_step_owner_cache,
        )
        if owner == "net.minecraft.world.entity.LivingEntity":
            ai_step_reaches_living_cache[runtime_class] = True
            return True
        source, method = method_for(result, owner, "aiStep", 0)
        calls_super = any(
            call.name == "aiStep"
            and call.qualifier is not None
            and call.qualifier.canonical == "super"
            and method.body is not None
            and method.body.start_byte <= call.span.start_byte
            and call.span.end_byte <= method.body.end_byte
            for call in source.calls
        )
        owner_type = class_map[owner]
        reaches = calls_super and ai_step_reaches_living_block_effect(
            owner_type.superclass
        )
        ai_step_reaches_living_cache[runtime_class] = reaches
        return reaches

    def block_effect_tick_policy(runtime_class: str | None) -> str:
        if extends_class(
            runtime_class, "net.minecraft.world.entity.vehicle.boat.AbstractBoat"
        ):
            return "BoatTwice"
        if extends_class(
            runtime_class, "net.minecraft.world.entity.vehicle.minecart.AbstractMinecart"
        ):
            # Both minecart behavior implementations move and apply effects on
            # the server. Client tick only interpolates; piston movement is a
            # separate block-entity callback owner.
            return "None"
        if extends_class(runtime_class, "net.minecraft.world.entity.LivingEntity"):
            return (
                "LocalAuthoritativeLiving"
                if ai_step_reaches_living_block_effect(runtime_class)
                else "None"
            )
        if extends_class(
            runtime_class, "net.minecraft.world.entity.projectile.arrow.AbstractArrow"
        ):
            return "AbstractArrowSegments"
        if extends_class(runtime_class, "net.minecraft.world.entity.item.ItemEntity"):
            return "ItemWithLastMovements"
        if runtime_class == "net.minecraft.world.entity.projectile.FireworkRocketEntity":
            return "DetachedFirework"
        if runtime_class == "net.minecraft.world.entity.boss.enderdragon.EndCrystal":
            return "OnceAfterMovement"
        movement_policy = client_movement_policy(runtime_class)
        if movement_policy in {
            "ExperienceOrb",
            "FallingBlock",
            "PrimedTnt",
            "ThrowableProjectile",
            "ThrownExperienceBottle",
            "ThrownPotion",
            "HurtingProjectile",
            "WindCharge",
            "DragonFireball",
            "WitherSkull",
            "FishingHook",
            "ShulkerBullet",
        }:
            return "OnceAfterMovement"
        return "None"

    def method_call_count(owner: str, method_name: str, call_name: str) -> int:
        source, method = method_for(result, owner, method_name, 0)
        if method.body is None:
            raise ExtractionError(f"{owner}.{method_name}: method lacks body")
        return sum(
            call.name == call_name
            and method.body.start_byte <= call.span.start_byte
            and call.span.end_byte <= method.body.end_byte
            for call in source.calls
        )

    block_effect_call_contracts = {
        ("net.minecraft.world.entity.LivingEntity", "aiStep"): 1,
        ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "tick"): 2,
        ("net.minecraft.world.entity.boss.enderdragon.EndCrystal", "tick"): 1,
        ("net.minecraft.world.entity.item.FallingBlockEntity", "tick"): 1,
        ("net.minecraft.world.entity.item.PrimedTnt", "tick"): 1,
        ("net.minecraft.world.entity.ExperienceOrb", "tick"): 1,
        ("net.minecraft.world.entity.projectile.ThrowableProjectile", "tick"): 1,
        (
            "net.minecraft.world.entity.projectile.hurtingprojectile.AbstractHurtingProjectile",
            "tick",
        ): 1,
        ("net.minecraft.world.entity.projectile.ShulkerBullet", "tick"): 1,
        ("net.minecraft.world.entity.projectile.FishingHook", "tick"): 1,
        ("net.minecraft.world.entity.projectile.FireworkRocketEntity", "tick"): 1,
        ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "tick"): 2,
        ("net.minecraft.world.entity.item.ItemEntity", "tick"): 1,
        ("net.minecraft.world.entity.boss.enderdragon.EnderDragon", "aiStep"): 1,
    }
    for (owner, method_name), expected_count in block_effect_call_contracts.items():
        actual_count = method_call_count(
            owner, method_name, "applyEffectsFromBlocks"
        )
        if actual_count != expected_count:
            raise ExtractionError(
                f"{owner}.{method_name}: expected {expected_count} "
                f"applyEffectsFromBlocks calls, found {actual_count}"
            )
    living_source, living_ai_step = method_for(
        result, "net.minecraft.world.entity.LivingEntity", "aiStep", 0
    )
    if living_ai_step.body is None or (
        "!this.level().isClientSide() || this.isLocalInstanceAuthoritative()"
        not in living_source.text(living_ai_step.body)
    ):
        raise ExtractionError(
            "LivingEntity.aiStep: missing local-authoritative block-effect guard"
        )
    firework_source, firework_tick = method_for(
        result,
        "net.minecraft.world.entity.projectile.FireworkRocketEntity",
        "tick",
        0,
    )
    if firework_tick.body is None or (
        "if (this.isAttachedToEntity())" not in firework_source.text(firework_tick.body)
        or "this.applyEffectsFromBlocks();" not in firework_source.text(firework_tick.body)
    ):
        raise ExtractionError(
            "FireworkRocketEntity.tick: missing attached/detached block-effect split"
        )
    arrow_source, arrow_step_move = method_for(
        result,
        "net.minecraft.world.entity.projectile.arrow.AbstractArrow",
        "stepMoveAndHit",
        1,
    )
    if arrow_step_move.body is None or sum(
        call.name == "applyEffectsFromBlocks"
        and arrow_step_move.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= arrow_step_move.body.end_byte
        for call in arrow_source.calls
    ) != 1:
        raise ExtractionError(
            "AbstractArrow.stepMoveAndHit: expected one explicit segment callback"
        )
    item_source, item_tick = method_for(
        result, "net.minecraft.world.entity.item.ItemEntity", "tick", 0
    )
    if item_tick.body is None or not any(
        call.name == "applyEffectsFromBlocksForLastMovements"
        and item_tick.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= item_tick.body.end_byte
        for call in item_source.calls
    ):
        raise ExtractionError(
            "ItemEntity.tick: missing resting applyEffectsFromBlocksForLastMovements call"
        )

    for java_class in class_map:
        source = source_for_owner(result, java_class)
        if any(
            method.name == "isPickable" and len(method.parameters) == 0
            for method in source.methods_for(java_class)
        ):
            class_is_pickable_without_type_tag(
                result, class_map, java_class, pickable_cache
            )
        if any(
            method.name == "getSoundSource" and len(method.parameters) == 0
            for method in source.methods_for(java_class)
        ):
            class_sound_source_kind(
                result, class_map, java_class, sound_source_cache
            )
        if any(
            method.name == "isPushable" and len(method.parameters) == 0
            for method in source.methods_for(java_class)
        ):
            class_pushability_kind(
                result, class_map, java_class, pushability_cache
            )
        if any(
            method.name == "canBeCollidedWith" and len(method.parameters) == 1
            for method in source.methods_for(java_class)
        ):
            class_collidability_kind(
                result, class_map, java_class, collidability_cache
            )
        if any(
            method.name == "canUseSlot" and len(method.parameters) == 1
            for method in source.methods_for(java_class)
        ):
            class_can_use_every_equipment_slot(
                result, class_map, java_class, can_use_every_slot_cache
            )
    own_counts: dict[str, int] = {}
    for accessor in result.accessors:
        own_counts[accessor.owner] = own_counts.get(accessor.owner, 0) + 1

    inherited_cache: dict[str, int] = {}

    def inherited_count(owner: str) -> int:
        if owner in inherited_cache:
            return inherited_cache[owner]
        item = class_map[owner]
        value = 0 if item.superclass is None else inherited_count(item.superclass) + own_counts.get(item.superclass, 0)
        inherited_cache[owner] = value
        return value

    def schema_class(runtime_class: str | None) -> str | None:
        current = runtime_class
        while current is not None:
            if own_counts.get(current, 0):
                return current
            current = class_map.get(current).superclass if current in class_map else None
        return None

    def extends_class(runtime_class: str | None, target: str) -> bool:
        current = runtime_class
        seen: set[str] = set()
        while current is not None:
            if current == target:
                return True
            if current in seen:
                raise ExtractionError(f"entity class inheritance cycle at {current}")
            seen.add(current)
            current = class_map.get(current).superclass if current in class_map else None
        return False

    def implements_interface(runtime_class: str | None, target: str) -> bool:
        seen: set[str] = set()

        def interface_reaches(interface: str) -> bool:
            if interface == target:
                return True
            if interface in seen:
                return False
            seen.add(interface)
            item = class_map.get(interface)
            return item is not None and any(interface_reaches(parent) for parent in item.interfaces)

        current = runtime_class
        while current is not None:
            item = class_map.get(current)
            if item is None:
                return False
            if any(interface_reaches(interface) for interface in item.interfaces):
                return True
            current = item.superclass
        return False

    def nearest_optional_class_method_owner(
        runtime_class: str | None,
        method_name: str,
        parameter_count: int,
    ) -> str | None:
        current = runtime_class
        while current is not None:
            source = source_for_owner(result, current)
            methods = [
                method
                for method in source.methods_for(current)
                if method.name == method_name
                and len(method.parameters) == parameter_count
            ]
            if len(methods) > 1:
                raise ExtractionError(
                    f"{current}.{method_name}/{parameter_count}: expected at most one method"
                )
            if methods:
                return current
            item = class_map.get(current)
            current = item.superclass if item is not None else None
        return None

    food_predicate_cache: dict[str, tuple[str | None, str | None]] = {}

    def food_predicate(runtime_class: str | None) -> tuple[str | None, str | None]:
        if runtime_class is None:
            return None, None
        if runtime_class in food_predicate_cache:
            return food_predicate_cache[runtime_class]
        owner = nearest_optional_class_method_owner(runtime_class, "isFood", 1)
        if owner is None:
            value = (None, None)
        else:
            source, method = method_for(result, owner, "isFood", 1)
            if method.body is None:
                raise ExtractionError(
                    f"{runtime_class}: nearest isFood declaration {owner} is abstract"
                )
            value = (owner, food_predicate_rust(result, owner, item_tags))
        food_predicate_cache[runtime_class] = value
        return value

    def effective_leash_offset_owner(runtime_class: str | None) -> str | None:
        if not implements_interface(runtime_class, "net.minecraft.world.entity.Leashable"):
            return None
        partial_tick_owner = nearest_optional_class_method_owner(
            runtime_class, "getLeashOffset", 1
        )
        if partial_tick_owner is not None:
            return partial_tick_owner
        return nearest_optional_class_method_owner(
            runtime_class, "getLeashOffset", 0
        ) or "net.minecraft.world.entity.Leashable"

    leashable_partial_offset_expression = method_single_return_expression(
        result,
        "net.minecraft.world.entity.Leashable",
        "getLeashOffset",
        1,
    ).replace(" ", "")
    if leashable_partial_offset_expression != "this.getLeashOffset()":
        raise ExtractionError(
            "Leashable.getLeashOffset/1 no longer forwards to the zero-argument virtual hook"
        )

    def camera_post_effect_id(runtime_class: str | None) -> str | None:
        matches = [
            item.resource_id
            for item in camera_post_effects
            if extends_class(runtime_class, item.runtime_class)
        ]
        if len(matches) > 1:
            raise ExtractionError(
                f"{runtime_class}: multiple GameRenderer camera post-effect arms {matches}"
            )
        return matches[0] if matches else None

    lines = [
        "// @generated by scripts/generate-entity-metadata-schema.py; do not edit manually.",
        "// Vanilla version: 26.1.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-entity-metadata-schema.py update",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-entity-metadata-schema.py check",
        f"// Stable source manifest SHA-256: {result.manifest_digest}",
        "",
        "use bbb_protocol::entity_data_serializers::*;",
        "",
        "// The synched-data ids and flag bits are wire facts, so the protocol",
        "// crate owns them and both the client and the server read them from",
        "// there; this module re-exports them so its own tables and every",
        "// existing `schema::` consumer keep resolving unchanged.",
        "pub use bbb_protocol::entity_synched_data::*;",
        "",
        "use super::*;",
        "",
    ]

    for public_name, value in value_masks:
        lines.append(f"const GENERATED_{public_name}: u8 = {value};")
        lines.append(f"pub const {public_name}: u8 = GENERATED_{public_name};")
    lines.append("")

    for public_name, value in packed_values:
        lines.append(f"const GENERATED_{public_name}: i32 = {value};")
        lines.append(f"pub const {public_name}: i32 = GENERATED_{public_name};")
    lines.append("")

    pose_ordinals = enum_ordinals(result, "net.minecraft.world.entity.HumanoidArm")
    for name in ("LEFT", "RIGHT"):
        public_name = f"VANILLA_HUMANOID_ARM_{name}_ID"
        lines.append(f"const GENERATED_{public_name}: i32 = {pose_ordinals[name]};")
        lines.append(f"pub const {public_name}: i32 = GENERATED_{public_name};")
    lines.append("")

    for public_name, (enum_fqcn, constant_name, argument_index) in ENUM_CONSTRUCTOR_ID_ALIASES.items():
        value = enum_constant_constructor_integer(
            result,
            enum_fqcn,
            constant_name,
            argument_index,
        )
        lines.append(f"const GENERATED_{public_name}: i32 = {value};")
        lines.append(f"pub const {public_name}: i32 = GENERATED_{public_name};")
    lines.append("")

    for public_name, (enum_fqcn, constant_name) in ENUM_ORDINAL_ID_ALIASES.items():
        value = enum_constant_ordinal(result, enum_fqcn, constant_name)
        lines.append(f"const GENERATED_{public_name}: i32 = {value};")
        lines.append(f"pub const {public_name}: i32 = GENERATED_{public_name};")
    lines.append("")

    for public_name, (
        owner,
        constant_name,
    ) in STATIC_FACTORY_ORDINAL_ID_ALIASES.items():
        ordinals = static_factory_registration_ordinals(
            result,
            owner,
        )
        if constant_name not in ordinals:
            raise ExtractionError(f"{owner}: missing {constant_name} registration")
        value = ordinals[constant_name]
        lines.append(f"const GENERATED_{public_name}: i32 = {value};")
        lines.append(f"pub const {public_name}: i32 = GENERATED_{public_name};")
    lines.append("")

    lines.append(
        "pub(crate) const VANILLA_ENDER_DRAGON_PHASES_26_1: &[VanillaEnderDragonPhaseCatalogEntry] = &["
    )
    for ordinal, java_field_name, implementation_class, is_sitting in ender_dragon_phase_catalog:
        lines.extend(
            [
                "    VanillaEnderDragonPhaseCatalogEntry {",
                f"        phase_id: {ordinal},",
                f"        java_field_name: {rust_string(java_field_name)},",
                f"        implementation_class: {rust_string(implementation_class)},",
                f"        is_sitting: {str(is_sitting).lower()},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "pub fn vanilla_ender_dragon_phase_is_sitting(phase_id: i32) -> bool {",
            "    usize::try_from(phase_id)",
            "        .ok()",
            "        .and_then(|phase_id| VANILLA_ENDER_DRAGON_PHASES_26_1.get(phase_id))",
            "        .is_some_and(|phase| phase.is_sitting)",
            "}",
            "",
        ]
    )

    for public_name, key in DEFAULT_ALIASES.items():
        value = normalized[key]
        if value is None or value.scalar_type is None or value.scalar_rust is None:
            raise ExtractionError(f"{public_name}: default has no scalar compatibility value")
        lines.append(f"const GENERATED_{public_name}: {value.scalar_type} = {value.scalar_rust};")
        lines.append(f"pub const {public_name}: {value.scalar_type} = GENERATED_{public_name};")

    for public_name, key in TYPED_DEFAULT_ALIASES.items():
        value = normalized[key]
        if value is None:
            raise ExtractionError(f"{public_name}: typed default unresolved")
        lines.append(
            f"const GENERATED_{public_name}: VanillaEntityMetadataDefault = {value.rust};"
        )
        lines.append(
            f"pub const {public_name}: VanillaEntityMetadataDefault = GENERATED_{public_name};"
        )

    virtual_values = {
        "VANILLA_ENTITY_DEFAULT_MAX_AIR_SUPPLY_TICKS": integer_constants(result, "net.minecraft.world.entity.Entity")["TOTAL_AIR_SUPPLY"],
    }
    for name, value in virtual_values.items():
        lines.append(f"const GENERATED_{name}: i32 = {value};")
        lines.append(f"pub const {name}: i32 = GENERATED_{name};")
    lines.append("")

    def nearest_owner_fqcn(
        method_name: str,
        parameter_count: int,
        cache: dict[str, str],
        runtime_class: str,
    ) -> str:
        return class_nearest_method_owner(
            result, class_map, runtime_class, method_name, parameter_count, cache
        )

    entity_owner_rows: list[dict[str, str | None]] = []
    owner_fqcns: set[str] = set()
    for item in entity_event_handlers:
        owner_fqcns.add(item.owner)
        if item.super_handler_owner is not None:
            owner_fqcns.add(item.super_handler_owner)
    for index, item in enumerate(result.entity_types):
        runtime_class = item.runtime_class
        if item.entity_type_id != index or runtime_class is None:
            raise ExtractionError(
                f"{item.resource_id}: entity types need contiguous ids and a runtime class"
            )
        row: dict[str, str | None] = {
            "handle_entity_event_owner": nearest_owner_fqcn(
                "handleEntityEvent", 1, handle_entity_event_owner_cache, runtime_class
            ),
            "interact_owner": nearest_owner_fqcn(
                "interact", 3, interact_owner_cache, runtime_class
            ),
            "mob_interact_owner": (
                nearest_owner_fqcn("mobInteract", 2, mob_interact_owner_cache, runtime_class)
                if extends_class(runtime_class, "net.minecraft.world.entity.Mob")
                else None
            ),
            "passenger_attachment_owner": nearest_owner_fqcn(
                "getPassengerAttachmentPoint",
                3,
                passenger_attachment_owner_cache,
                runtime_class,
            ),
            "vehicle_attachment_owner": nearest_owner_fqcn(
                "getVehicleAttachmentPoint",
                1,
                vehicle_attachment_owner_cache,
                runtime_class,
            ),
            "controlling_passenger_owner": nearest_owner_fqcn(
                "getControllingPassenger",
                0,
                controlling_passenger_owner_cache,
                runtime_class,
            ),
            "leash_offset_owner": effective_leash_offset_owner(runtime_class),
            "rope_hold_position_owner": nearest_owner_fqcn(
                "getRopeHoldPosition", 1, rope_hold_position_owner_cache, runtime_class
            ),
        }
        entity_owner_rows.append(row)
        owner_fqcns.update(value for value in row.values() if value is not None)

    owner_variant_by_fqcn: dict[str, str] = {}
    for fqcn in sorted(owner_fqcns):
        variant = fqcn.rsplit(".", 1)[-1]
        if variant in owner_variant_by_fqcn.values():
            raise ExtractionError(f"entity owner enum variant collides on simple name: {fqcn}")
        owner_variant_by_fqcn[fqcn] = variant

    def owner_expr(fqcn: str) -> str:
        return f"Owner::{owner_variant_by_fqcn[fqcn]}"

    def option_owner_expr(fqcn: str | None) -> str:
        return "None" if fqcn is None else f"Some({owner_expr(fqcn)})"

    lines.extend([
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
        "pub enum VanillaEntityOwner {",
        *(f"    {variant}," for variant in sorted(owner_variant_by_fqcn.values())),
        "}",
        "use VanillaEntityOwner as Owner;",
        "",
        "pub const VANILLA_ENTITY_EVENT_HANDLERS_26_1: &[VanillaEntityEventHandlerCatalogEntry] = &[",
    ])
    for item in entity_event_handlers:
        lines.extend(
            [
                "    VanillaEntityEventHandlerCatalogEntry {",
                f"        owner: {owner_expr(item.owner)},",
                f"        super_handler_owner: {option_owner_expr(item.super_handler_owner)},",
                "        direct_event_ids: &["
                + ", ".join(str(event_id) for event_id in item.direct_event_ids)
                + "],",
                "        delegated_event_ids: &["
                + ", ".join(str(event_id) for event_id in item.delegated_event_ids)
                + "],",
                "        forwards_unhandled_to_super: "
                + str(item.forwards_unhandled_to_super).lower()
                + ",",
                "    },",
            ]
        )
    lines.extend([
        "];",
        "",
    ])

    families = metadata_families(result)
    family_by_class = {family.java_class: family for family in families}

    def option_policy(value: str | None) -> str:
        return "None" if value is None else f"Some({value})"

    type_facts = (
        ("LIVING", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.LivingEntity")),
        ("MOB", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.Mob")),
        ("ANIMAL", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.animal.Animal")),
        ("TAMABLE_ANIMAL", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.TamableAnimal")),
        ("PROJECTILE", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.projectile.Projectile")),
        ("HURTING_PROJECTILE", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.projectile.hurtingprojectile.AbstractHurtingProjectile")),
        ("WIND_CHARGE", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.projectile.hurtingprojectile.windcharge.AbstractWindCharge")),
        ("VEHICLE", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.vehicle.VehicleEntity")),
        ("BOAT", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.vehicle.boat.AbstractBoat")),
        ("ABSTRACT_CHEST_BOAT", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.vehicle.boat.AbstractChestBoat")),
        ("MINECART", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.vehicle.minecart.AbstractMinecart")),
        ("ABSTRACT_HORSE", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.animal.equine.AbstractHorse")),
        ("ABSTRACT_CHESTED_HORSE", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.animal.equine.AbstractChestedHorse")),
        ("ABSTRACT_NAUTILUS", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.animal.nautilus.AbstractNautilus")),
        ("LLAMA", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.animal.equine.Llama")),
        ("GUARDIAN", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.monster.Guardian")),
        ("CAMEL", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.animal.camel.Camel")),
        ("SQUID", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.animal.squid.Squid")),
        ("SLIME", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.monster.Slime")),
        ("ABSTRACT_PIGLIN", lambda runtime_class: extends_class(runtime_class, "net.minecraft.world.entity.monster.piglin.AbstractPiglin")),
        ("FLYING_ANIMAL", lambda runtime_class: implements_interface(runtime_class, "net.minecraft.world.entity.animal.FlyingAnimal")),
        ("LEASHABLE", lambda runtime_class: implements_interface(runtime_class, "net.minecraft.world.entity.Leashable")),
        ("HOGLIN_BASE", lambda runtime_class: implements_interface(runtime_class, "net.minecraft.world.entity.monster.hoglin.HoglinBase")),
        ("PLAYER_RIDEABLE_JUMPING", lambda runtime_class: implements_interface(runtime_class, "net.minecraft.world.entity.PlayerRideableJumping")),
        ("HAS_CUSTOM_INVENTORY_SCREEN", lambda runtime_class: implements_interface(runtime_class, "net.minecraft.world.entity.HasCustomInventoryScreen")),
        ("MENU_PROVIDER", lambda runtime_class: implements_interface(runtime_class, "net.minecraft.world.entity.vehicle.ContainerEntity")),
        ("PICKABLE_BY_CLASS", lambda runtime_class: class_is_pickable_without_type_tag(result, class_map, runtime_class, pickable_cache)),
        ("PUSHES_LOCAL_PLAYER_ON_CLIENT_TICK", lambda runtime_class: class_pushes_local_player_on_client_tick(result, class_map, runtime_class, push_entities_owner_cache, do_push_owner_cache, pushes_local_player_cache)),
        ("CAN_USE_EVERY_EQUIPMENT_SLOT", lambda runtime_class: class_can_use_every_equipment_slot(result, class_map, runtime_class, can_use_every_slot_cache)),
        ("ILLAGER_ARM_POSE_USES_CELEBRATING", lambda runtime_class: class_illager_arm_pose_uses_celebrating(result, class_map, runtime_class, illager_celebrating_cache)),
    )
    lines.append("pub mod entity_type_facts {")
    for bit, (name, _) in enumerate(type_facts):
        lines.append(f"    pub const {name}: u64 = 1 << {bit};")
    lines.extend(["}", "use entity_type_facts::*;", ""])

    lines.append(
        "pub const VANILLA_ENTITY_TYPE_METADATA_26_1: &[VanillaEntityTypeMetadataCatalogEntry] = &["
    )
    for index, item in enumerate(result.entity_types):
        runtime_class = item.runtime_class
        if item.entity_type_id != index or runtime_class is None:
            raise ExtractionError(f"{item.resource_id}: entity types need contiguous ids and a runtime class")
        owners = entity_owner_rows[index]
        living = extends_class(runtime_class, "net.minecraft.world.entity.LivingEntity")
        schema = schema_class(runtime_class)
        facts = " | ".join(name for name, predicate in type_facts if predicate(runtime_class)) or "0"
        food_owner, food_policy = food_predicate(runtime_class)
        boat_ride_height = (
            class_boat_ride_height_multiplier(result, class_map, runtime_class, boat_ride_height_cache)
            if extends_class(runtime_class, "net.minecraft.world.entity.vehicle.boat.AbstractBoat")
            else None
        )
        handle_entity_event_owner = owners["handle_entity_event_owner"]
        interact_owner = owners["interact_owner"]
        passenger_attachment_owner = owners["passenger_attachment_owner"]
        vehicle_attachment_owner = owners["vehicle_attachment_owner"]
        controlling_passenger_owner = owners["controlling_passenger_owner"]
        rope_hold_position_owner = owners["rope_hold_position_owner"]
        assert handle_entity_event_owner is not None
        assert interact_owner is not None
        assert passenger_attachment_owner is not None
        assert vehicle_attachment_owner is not None
        assert controlling_passenger_owner is not None
        assert rope_hold_position_owner is not None
        lines.extend(
            [
                "    VanillaEntityTypeMetadataCatalogEntry {",
                f"        resource_id: {rust_string(item.resource_id)},",
                "        family: "
                + (f"Some(VanillaEntityMetadataFamily::{family_by_class[schema].rust_name})" if schema else "None")
                + ",",
                f"        facts: {facts},",
                f"        handle_entity_event_owner: {owner_expr(handle_entity_event_owner)},",
                f"        interact_owner: {owner_expr(interact_owner)},",
                f"        mob_interact_owner: {option_owner_expr(owners['mob_interact_owner'])},",
                f"        passenger_attachment_owner: {owner_expr(passenger_attachment_owner)},",
                f"        vehicle_attachment_owner: {owner_expr(vehicle_attachment_owner)},",
                f"        controlling_passenger_owner: {owner_expr(controlling_passenger_owner)},",
                f"        leash_offset_owner: {option_owner_expr(owners['leash_offset_owner'])},",
                f"        rope_hold_position_owner: {owner_expr(rope_hold_position_owner)},",
                f"        food_predicate: {option_policy(food_policy)},",
                f"        client_movement_policy: VanillaEntityClientMovementPolicy::{client_movement_policy(runtime_class)},",
                f"        block_effect_tick_policy: VanillaEntityBlockEffectTickPolicy::{block_effect_tick_policy(runtime_class)},",
                f"        interpolation_policy: VanillaEntityInterpolationPolicy::{interpolation_policy(runtime_class)},",
                f"        baby_behavior: VanillaEntityBabyBehavior::{class_baby_behavior(result, class_map, runtime_class, baby_behavior_cache) if living else 'Never'},",
                f"        baby_age_scale_bits: {class_baby_age_scale_bits(result, class_map, runtime_class, baby_age_scale_cache) if living else '1.0_f32.to_bits()'},",
                f"        death_sound_policy: {class_death_sound_policy(result, class_map, runtime_class, sound_events, death_sound_owner_cache, death_sound_policy_cache) if living else 'VanillaEntityDeathSoundPolicy::NonLiving'},",
                f"        hurt_sound_policy: {class_hurt_sound_policy(result, class_map, runtime_class, sound_events, hurt_sound_owner_cache, hurt_sound_policy_cache) if living else 'VanillaEntityHurtSoundPolicy::NonLiving'},",
                f"        sound_volume_policy: {class_sound_volume_policy(result, class_map, runtime_class, sound_volume_owner_cache, sound_volume_policy_cache) if living else 'VanillaEntitySoundVolumePolicy::NonLiving'},",
                f"        pushability_kind: VanillaEntityPushabilityKind::{class_pushability_kind(result, class_map, runtime_class, pushability_cache)},",
                f"        collidability_kind: VanillaEntityCollidabilityKind::{class_collidability_kind(result, class_map, runtime_class, collidability_cache)},",
                f"        boat_ride_height_multiplier_bits: {option_policy(boat_ride_height)},",
                "    },",
            ]
        )
        if (food_owner is None) != (food_policy is None):
            raise ExtractionError(f"{item.resource_id}: isFood owner and predicate disagree")
    lines.append("];")
    lines.append("")

    lines.append(
        "pub const VANILLA_ENTITY_CAMERA_POST_EFFECTS_26_1: &[(i32, &str)] = &["
    )
    for item in result.entity_types:
        post_effect_id = camera_post_effect_id(item.runtime_class)
        if post_effect_id is not None:
            lines.append(f"    ({item.entity_type_id}, {rust_string(post_effect_id)}),")
    lines.extend([
        "];",
        "",
    ])

    lines.append(
        "pub const VANILLA_ENTITY_MINECART_DEFAULT_DISPLAY_OFFSETS_26_1: &[(i32, i32)] = &["
    )
    for item in result.entity_types:
        if extends_class(
            item.runtime_class,
            "net.minecraft.world.entity.vehicle.minecart.AbstractMinecart",
        ):
            offset = class_zero_argument_integer_method(
                result,
                class_map,
                item.runtime_class,
                "getDefaultDisplayOffset",
                minecart_display_offset_cache,
            )
            lines.append(f"    ({item.entity_type_id}, {offset}),")
    lines.extend([
        "];",
        "",
    ])

    lines.append(
        "pub const VANILLA_ENTITY_SCALE_SANITIZERS_26_1: &[(i32, VanillaEntityScaleSanitizer)] = &["
    )
    for item in result.entity_types:
        if extends_class(
            item.runtime_class,
            "net.minecraft.world.entity.LivingEntity",
        ):
            behavior = class_sanitize_scale_behavior(
                result,
                class_map,
                item.runtime_class,
                sanitize_scale_behavior_cache,
            )
            if behavior is not None:
                lines.append(f"    ({item.entity_type_id}, {behavior}),")
    lines.extend([
        "];",
        "",
    ])

    lines.extend(render_families(result, families, normalized, inherited_count))
    return format_rust("\n".join(lines))


@dataclass(frozen=True)
class MetadataFamily:
    rust_name: str
    java_class: str
    parent: str | None


def metadata_families(result: ExtractionResult) -> list[MetadataFamily]:
    """Every class that declares synched data, parents before children."""
    class_map = {item.fqcn: item for item in result.classes}
    owners = {item.owner for item in result.accessors}

    def parent_owner(owner: str) -> str | None:
        current = class_map[owner].superclass
        while current is not None and current not in owners:
            current = class_map[current].superclass if current in class_map else None
        return current

    def depth(owner: str) -> int:
        parent = parent_owner(owner)
        return 0 if parent is None else depth(parent) + 1

    def name(owner: str) -> str:
        return FAMILY_NAME_OVERRIDES.get(owner, owner.rsplit(".", 1)[-1])

    ordered = sorted(owners, key=lambda owner: (depth(owner), name(owner)))
    names = [name(owner) for owner in ordered]
    if len(set(names)) != len(names):
        raise ExtractionError(f"metadata family names collide: {names}")
    families = [
        MetadataFamily(
            name(owner),
            owner,
            None if parent_owner(owner) is None else name(parent_owner(owner)),
        )
        for owner in ordered
    ]
    if len(families) > 128:
        raise ExtractionError("metadata family ancestor sets no longer fit in u128")
    return families


def screaming_snake(name: str) -> str:
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", name).upper()


def render_families(
    result: ExtractionResult,
    families: list[MetadataFamily],
    normalized: dict[tuple[str, str], NormalizedDefault | None],
    inherited_count,
) -> list[str]:
    index = {family.rust_name: position for position, family in enumerate(families)}

    def ancestors(family: MetadataFamily) -> int:
        bits = 0
        current: MetadataFamily | None = family
        while current is not None:
            bits |= 1 << index[current.rust_name]
            current = None if current.parent is None else families[index[current.parent]]
        return bits

    lines = [
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaEntityMetadataFamily {",
        *(f"    {family.rust_name}," for family in families),
        "}",
        "",
    ]
    for family in families:
        own = [item for item in result.accessors if item.owner == family.java_class]
        if [item.data_id for item in own] != list(
            range(inherited_count(family.java_class), inherited_count(family.java_class) + len(own))
        ):
            raise ExtractionError(f"{family.java_class}: synched data ids are not contiguous")
        for item in own:
            default = normalized[(item.owner, item.java_name)]
            lines.extend(
                [
                    f"pub(crate) const FIELD_{screaming_snake(family.rust_name)}_{item.java_name}: VanillaEntityMetadataFieldSchema = VanillaEntityMetadataFieldSchema {{",
                    f"    family: VanillaEntityMetadataFamily::{family.rust_name},",
                    f"    java_name: {rust_string(item.java_name)},",
                    f"    data_id: {item.data_id},",
                    f"    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_{item.serializer}_ID,",
                    f"    default: {'Some(' + default.rust + ')' if default else 'None'},",
                    "};",
                ]
            )
    lines.append("")
    lines.append(
        f"const FAMILY_SCHEMAS: [VanillaEntityMetadataFamilySchema; {len(families)}] = ["
    )
    for family in families:
        own = [item for item in result.accessors if item.owner == family.java_class]
        parent = f"Some(VanillaEntityMetadataFamily::{family.parent})" if family.parent else "None"
        fields = ", ".join(
            f"FIELD_{screaming_snake(family.rust_name)}_{item.java_name}" for item in own
        )
        lines.extend(
            [
                "    VanillaEntityMetadataFamilySchema {",
                f"        java_class: {rust_string(family.java_class)},",
                f"        parent: {parent},",
                f"        ancestors: {ancestors(family):#x},",
                f"        inherited_field_count: {inherited_count(family.java_class)},",
                f"        own_fields: &[{fields}],",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "pub const fn vanilla_entity_metadata_schema(",
            "    family: VanillaEntityMetadataFamily,",
            ") -> &'static VanillaEntityMetadataFamilySchema {",
            "    &FAMILY_SCHEMAS[family as usize]",
            "}",
            "",
        ]
    )
    return lines


def render_sound_sources(result: ExtractionResult) -> str:
    class_map = {item.fqcn: item for item in result.classes}
    cache: dict[str, str] = {}
    lines = [
        "// @generated by scripts/generate-entity-metadata-schema.py; do not edit manually.",
        "// Source: vanilla 26.1 Entity.getSoundSource and nearest subtype overrides.",
        "// Update/check: scripts/python scripts/generate-entity-metadata-schema.py update|check",
        "",
        "use crate::sound_sources::SoundSource;",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaEntitySoundSourceKind {",
        "    Static(SoundSource),",
        "    Dynamic,",
        "}",
        "",
        "/// Returns the inherited sound source; Dynamic requires the current rabbit variant.",
        "pub fn vanilla_entity_sound_source_kind(entity_type_id: i32) -> Option<VanillaEntitySoundSourceKind> {",
        "    ENTITY_SOUND_SOURCES.get(usize::try_from(entity_type_id).ok()?).copied()",
        "}",
        "",
        "const ENTITY_SOUND_SOURCES: &[VanillaEntitySoundSourceKind] = &[",
    ]
    for index, item in enumerate(result.entity_types):
        if item.entity_type_id != index:
            raise ExtractionError("entity sound sources require contiguous EntityType ids")
        kind = class_sound_source_kind(result, class_map, item.runtime_class, cache)
        value = "Dynamic" if kind == "Dynamic" else f"Static(SoundSource::{kind})"
        lines.append(f"    VanillaEntitySoundSourceKind::{value}, // {item.resource_id}")
    lines.extend([
        "];",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn generated_sources_cover_entity_types_and_only_rabbit_is_dynamic() {",
        "        assert_eq!(ENTITY_SOUND_SOURCES.len(), crate::entity_types::VANILLA_ENTITY_TYPES.len());",
        "        for entry in crate::entity_types::VANILLA_ENTITY_TYPES {",
        "            assert_eq!(vanilla_entity_sound_source_kind(entry.id) == Some(VanillaEntitySoundSourceKind::Dynamic), entry.resource_id == \"minecraft:rabbit\");",
        "        }",
        "        assert_eq!(vanilla_entity_sound_source_kind(-1), None);",
        "        assert_eq!(vanilla_entity_sound_source_kind(ENTITY_SOUND_SOURCES.len() as i32), None);",
        "    }",
        "}",
        "",
    ])
    return format_rust("\n".join(lines))


def main() -> int:
    args = parse_args()
    result = extract(args.source_root)
    outputs = [
        (args.protocol_output, render_protocol(result)),
        (args.output, render(result)),
        (args.sound_source_output, render_sound_sources(result)),
    ]
    if args.command == "check":
        status = 0
        for path, generated in outputs:
            current = path.read_text() if path.exists() else ""
            if current == generated:
                continue
            sys.stderr.writelines(
                difflib.unified_diff(
                    current.splitlines(keepends=True),
                    generated.splitlines(keepends=True),
                    fromfile=str(path),
                    tofile="generated",
                )
            )
            status = 1
        return status
    for path, generated in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
