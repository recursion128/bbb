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

from tree_sitter import Node

from entity_metadata_extractor import (
    Accessor,
    ExtractionError,
    ExtractionResult,
    GAME_RENDERER_SOURCE,
    evaluate_integer_expression,
    extract,
)
from sound_event_catalog_extractor import direct_sound_event_entries
from vanilla_java_source import (
    JavaField,
    JavaMethod,
    JavaSource,
    JavaType,
    descendants,
    resolve_type_reference,
)


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-world/src/entities/schema/generated_26_1.rs"
)


@dataclass(frozen=True)
class FamilySpec:
    rust_name: str
    java_class: str
    parent: str | None


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


FAMILIES = (
    FamilySpec("Entity", "net.minecraft.world.entity.Entity", None),
    FamilySpec("LivingEntity", "net.minecraft.world.entity.LivingEntity", "Entity"),
    FamilySpec("Mob", "net.minecraft.world.entity.Mob", "LivingEntity"),
    FamilySpec("Allay", "net.minecraft.world.entity.animal.allay.Allay", "Mob"),
    FamilySpec("Creeper", "net.minecraft.world.entity.monster.Creeper", "Mob"),
    FamilySpec("Warden", "net.minecraft.world.entity.monster.warden.Warden", "Mob"),
    FamilySpec("WitherBoss", "net.minecraft.world.entity.boss.wither.WitherBoss", "Mob"),
    FamilySpec("EnderMan", "net.minecraft.world.entity.monster.EnderMan", "Mob"),
    FamilySpec("Slime", "net.minecraft.world.entity.monster.Slime", "Mob"),
    FamilySpec("Shulker", "net.minecraft.world.entity.monster.Shulker", "Mob"),
    FamilySpec("Zombie", "net.minecraft.world.entity.monster.zombie.Zombie", "Mob"),
    FamilySpec("AgeableMob", "net.minecraft.world.entity.AgeableMob", "Mob"),
    FamilySpec("Armadillo", "net.minecraft.world.entity.animal.armadillo.Armadillo", "AgeableMob"),
    FamilySpec("Axolotl", "net.minecraft.world.entity.animal.axolotl.Axolotl", "AgeableMob"),
    FamilySpec("Bee", "net.minecraft.world.entity.animal.bee.Bee", "AgeableMob"),
    FamilySpec("Dolphin", "net.minecraft.world.entity.animal.dolphin.Dolphin", "AgeableMob"),
    FamilySpec("GlowSquid", "net.minecraft.world.entity.animal.squid.GlowSquid", "AgeableMob"),
    FamilySpec(
        "HappyGhast",
        "net.minecraft.world.entity.animal.happyghast.HappyGhast",
        "AgeableMob",
    ),
    FamilySpec("Goat", "net.minecraft.world.entity.animal.goat.Goat", "AgeableMob"),
    FamilySpec("Fox", "net.minecraft.world.entity.animal.fox.Fox", "AgeableMob"),
    FamilySpec("CopperGolem", "net.minecraft.world.entity.animal.golem.CopperGolem", "Mob"),
    FamilySpec("Ocelot", "net.minecraft.world.entity.animal.feline.Ocelot", "AgeableMob"),
    FamilySpec("Panda", "net.minecraft.world.entity.animal.panda.Panda", "AgeableMob"),
    FamilySpec("PolarBear", "net.minecraft.world.entity.animal.polarbear.PolarBear", "AgeableMob"),
    FamilySpec("Sheep", "net.minecraft.world.entity.animal.sheep.Sheep", "AgeableMob"),
    FamilySpec("Sniffer", "net.minecraft.world.entity.animal.sniffer.Sniffer", "AgeableMob"),
    FamilySpec("Turtle", "net.minecraft.world.entity.animal.turtle.Turtle", "AgeableMob"),
    FamilySpec(
        "AbstractHorse",
        "net.minecraft.world.entity.animal.equine.AbstractHorse",
        "AgeableMob",
    ),
    FamilySpec("Camel", "net.minecraft.world.entity.animal.camel.Camel", "AbstractHorse"),
    FamilySpec("Horse", "net.minecraft.world.entity.animal.equine.Horse", "AbstractHorse"),
    FamilySpec(
        "AbstractChestedHorse",
        "net.minecraft.world.entity.animal.equine.AbstractChestedHorse",
        "AbstractHorse",
    ),
    FamilySpec("Llama", "net.minecraft.world.entity.animal.equine.Llama", "AbstractChestedHorse"),
    FamilySpec("TamableAnimal", "net.minecraft.world.entity.TamableAnimal", "AgeableMob"),
    FamilySpec("Cat", "net.minecraft.world.entity.animal.feline.Cat", "TamableAnimal"),
    FamilySpec("Wolf", "net.minecraft.world.entity.animal.wolf.Wolf", "TamableAnimal"),
    FamilySpec("Chicken", "net.minecraft.world.entity.animal.chicken.Chicken", "AgeableMob"),
    FamilySpec("Cow", "net.minecraft.world.entity.animal.cow.Cow", "AgeableMob"),
    FamilySpec("MushroomCow", "net.minecraft.world.entity.animal.cow.MushroomCow", "AgeableMob"),
    FamilySpec("Pig", "net.minecraft.world.entity.animal.pig.Pig", "AgeableMob"),
    FamilySpec("Frog", "net.minecraft.world.entity.animal.frog.Frog", "AgeableMob"),
    FamilySpec(
        "AbstractNautilus",
        "net.minecraft.world.entity.animal.nautilus.AbstractNautilus",
        "TamableAnimal",
    ),
    FamilySpec(
        "ZombieNautilus",
        "net.minecraft.world.entity.animal.nautilus.ZombieNautilus",
        "AbstractNautilus",
    ),
    FamilySpec("Parrot", "net.minecraft.world.entity.animal.parrot.Parrot", "TamableAnimal"),
    FamilySpec("Rabbit", "net.minecraft.world.entity.animal.rabbit.Rabbit", "AgeableMob"),
    FamilySpec(
        "AbstractVillager",
        "net.minecraft.world.entity.npc.villager.AbstractVillager",
        "AgeableMob",
    ),
    FamilySpec("ArmorStand", "net.minecraft.world.entity.decoration.ArmorStand", "LivingEntity"),
    FamilySpec(
        "AbstractArrow",
        "net.minecraft.world.entity.projectile.arrow.AbstractArrow",
        "Entity",
    ),
    FamilySpec(
        "Arrow",
        "net.minecraft.world.entity.projectile.arrow.Arrow",
        "AbstractArrow",
    ),
    FamilySpec(
        "ThrownTrident",
        "net.minecraft.world.entity.projectile.arrow.ThrownTrident",
        "AbstractArrow",
    ),
    FamilySpec(
        "WitherSkull",
        "net.minecraft.world.entity.projectile.hurtingprojectile.WitherSkull",
        "Entity",
    ),
    FamilySpec(
        "EndCrystal",
        "net.minecraft.world.entity.boss.enderdragon.EndCrystal",
        "Entity",
    ),
    FamilySpec(
        "EnderDragon",
        "net.minecraft.world.entity.boss.enderdragon.EnderDragon",
        "Mob",
    ),
    FamilySpec("Interaction", "net.minecraft.world.entity.Interaction", "Entity"),
    FamilySpec("ExperienceOrb", "net.minecraft.world.entity.ExperienceOrb", "Entity"),
    FamilySpec("ItemEntity", "net.minecraft.world.entity.item.ItemEntity", "Entity"),
    FamilySpec("PrimedTnt", "net.minecraft.world.entity.item.PrimedTnt", "Entity"),
    FamilySpec(
        "FireworkRocket",
        "net.minecraft.world.entity.projectile.FireworkRocketEntity",
        "Entity",
    ),
    FamilySpec(
        "FishingHook",
        "net.minecraft.world.entity.projectile.FishingHook",
        "Entity",
    ),
    FamilySpec(
        "OminousItemSpawner",
        "net.minecraft.world.entity.OminousItemSpawner",
        "Entity",
    ),
    FamilySpec("Guardian", "net.minecraft.world.entity.monster.Guardian", "Mob"),
    FamilySpec("SnowGolem", "net.minecraft.world.entity.animal.golem.SnowGolem", "Mob"),
    FamilySpec("IronGolem", "net.minecraft.world.entity.animal.golem.IronGolem", "Mob"),
    FamilySpec("Bat", "net.minecraft.world.entity.ambient.Bat", "Mob"),
    FamilySpec("Phantom", "net.minecraft.world.entity.monster.Phantom", "Mob"),
    FamilySpec(
        "Bogged",
        "net.minecraft.world.entity.monster.skeleton.Bogged",
        "Mob",
    ),
    FamilySpec("Skeleton", "net.minecraft.world.entity.monster.skeleton.Skeleton", "Mob"),
    FamilySpec("Ghast", "net.minecraft.world.entity.monster.Ghast", "Mob"),
    FamilySpec("Vex", "net.minecraft.world.entity.monster.Vex", "Mob"),
    FamilySpec("Creaking", "net.minecraft.world.entity.monster.creaking.Creaking", "Mob"),
    FamilySpec("Strider", "net.minecraft.world.entity.monster.Strider", "AgeableMob"),
    FamilySpec("Zoglin", "net.minecraft.world.entity.monster.Zoglin", "Mob"),
    FamilySpec("Hoglin", "net.minecraft.world.entity.monster.hoglin.Hoglin", "AgeableMob"),
    FamilySpec("AbstractPiglin", "net.minecraft.world.entity.monster.piglin.AbstractPiglin", "Mob"),
    FamilySpec("Piglin", "net.minecraft.world.entity.monster.piglin.Piglin", "AbstractPiglin"),
    FamilySpec("Raider", "net.minecraft.world.entity.raid.Raider", "Mob"),
    FamilySpec("Pillager", "net.minecraft.world.entity.monster.illager.Pillager", "Raider"),
    FamilySpec("SpellcasterIllager", "net.minecraft.world.entity.monster.illager.SpellcasterIllager", "Raider"),
    FamilySpec(
        "AbstractFish",
        "net.minecraft.world.entity.animal.fish.AbstractFish",
        "Mob",
    ),
    FamilySpec("Salmon", "net.minecraft.world.entity.animal.fish.Salmon", "AbstractFish"),
    FamilySpec(
        "TropicalFish",
        "net.minecraft.world.entity.animal.fish.TropicalFish",
        "AbstractFish",
    ),
    FamilySpec(
        "Pufferfish",
        "net.minecraft.world.entity.animal.fish.Pufferfish",
        "AbstractFish",
    ),
    FamilySpec("Tadpole", "net.minecraft.world.entity.animal.frog.Tadpole", "AbstractFish"),
    FamilySpec("VehicleEntity", "net.minecraft.world.entity.vehicle.VehicleEntity", "Entity"),
    FamilySpec(
        "AbstractBoat",
        "net.minecraft.world.entity.vehicle.boat.AbstractBoat",
        "VehicleEntity",
    ),
    FamilySpec(
        "AbstractMinecart",
        "net.minecraft.world.entity.vehicle.minecart.AbstractMinecart",
        "VehicleEntity",
    ),
    FamilySpec(
        "MinecartFurnace",
        "net.minecraft.world.entity.vehicle.minecart.MinecartFurnace",
        "AbstractMinecart",
    ),
    FamilySpec("HangingEntity", "net.minecraft.world.entity.decoration.HangingEntity", "Entity"),
    FamilySpec("ItemFrame", "net.minecraft.world.entity.decoration.ItemFrame", "HangingEntity"),
    FamilySpec(
        "Painting",
        "net.minecraft.world.entity.decoration.painting.Painting",
        "HangingEntity",
    ),
    FamilySpec("Avatar", "net.minecraft.world.entity.Avatar", "LivingEntity"),
    FamilySpec("Player", "net.minecraft.world.entity.player.Player", "Avatar"),
)


DATA_ALIASES = {
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
    "VANILLA_CREEPER_SWELL_DIR_DATA_ID": ("net.minecraft.world.entity.monster.Creeper", "DATA_SWELL_DIR"),
    "VANILLA_CREEPER_POWERED_DATA_ID": ("net.minecraft.world.entity.monster.Creeper", "DATA_IS_POWERED"),
    "VANILLA_CREEPER_IGNITED_DATA_ID": ("net.minecraft.world.entity.monster.Creeper", "DATA_IS_IGNITED"),
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
    "VANILLA_DOLPHIN_GOT_FISH_DATA_ID": ("net.minecraft.world.entity.animal.dolphin.Dolphin", "GOT_FISH"),
    "VANILLA_DOLPHIN_MOISTNESS_DATA_ID": ("net.minecraft.world.entity.animal.dolphin.Dolphin", "MOISTNESS_LEVEL"),
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
    "VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "UNHAPPY_COUNTER"),
    "VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "SNEEZE_COUNTER"),
    "VANILLA_PANDA_EAT_COUNTER_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "EAT_COUNTER"),
    "VANILLA_PANDA_MAIN_GENE_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "MAIN_GENE_ID"),
    "VANILLA_PANDA_HIDDEN_GENE_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "HIDDEN_GENE_ID"),
    "VANILLA_PANDA_FLAGS_DATA_ID": ("net.minecraft.world.entity.animal.panda.Panda", "DATA_ID_FLAGS"),
    "VANILLA_SNIFFER_DROP_SEED_AT_TICK_DATA_ID": ("net.minecraft.world.entity.animal.sniffer.Sniffer", "DATA_DROP_SEED_AT_TICK"),
    "VANILLA_TURTLE_HAS_EGG_DATA_ID": ("net.minecraft.world.entity.animal.turtle.Turtle", "HAS_EGG"),
    "VANILLA_TURTLE_LAYING_EGG_DATA_ID": ("net.minecraft.world.entity.animal.turtle.Turtle", "LAYING_EGG"),
    "VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID": ("net.minecraft.world.entity.animal.equine.AbstractHorse", "DATA_ID_FLAGS"),
    "VANILLA_HORSE_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.equine.Horse", "DATA_ID_TYPE_VARIANT"),
    "VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID": ("net.minecraft.world.entity.animal.equine.AbstractChestedHorse", "DATA_ID_CHEST"),
    "VANILLA_LLAMA_STRENGTH_DATA_ID": ("net.minecraft.world.entity.animal.equine.Llama", "DATA_STRENGTH_ID"),
    "VANILLA_LLAMA_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.equine.Llama", "DATA_VARIANT_ID"),
    "VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID": ("net.minecraft.world.entity.TamableAnimal", "DATA_FLAGS_ID"),
    "VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID": ("net.minecraft.world.entity.TamableAnimal", "DATA_OWNERUUID_ID"),
    "VANILLA_CAT_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.feline.Cat", "DATA_VARIANT_ID"),
    "VANILLA_CAT_SOUND_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.feline.Cat", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_WOLF_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.wolf.Wolf", "DATA_VARIANT_ID"),
    "VANILLA_WOLF_SOUND_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.wolf.Wolf", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_CHICKEN_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.chicken.Chicken", "DATA_VARIANT_ID"),
    "VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.chicken.Chicken", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_COW_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.cow.Cow", "DATA_VARIANT_ID"),
    "VANILLA_COW_SOUND_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.cow.Cow", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_PIG_BOOST_TIME_DATA_ID": ("net.minecraft.world.entity.animal.pig.Pig", "DATA_BOOST_TIME"),
    "VANILLA_PIG_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.pig.Pig", "DATA_VARIANT_ID"),
    "VANILLA_PIG_SOUND_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.pig.Pig", "DATA_SOUND_VARIANT_ID"),
    "VANILLA_FROG_VARIANT_DATA_ID": ("net.minecraft.world.entity.animal.frog.Frog", "DATA_VARIANT_ID"),
    "VANILLA_FROG_TONGUE_TARGET_DATA_ID": ("net.minecraft.world.entity.animal.frog.Frog", "DATA_TONGUE_TARGET_ID"),
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
    "VANILLA_PRIMED_TNT_FUSE_DATA_ID": ("net.minecraft.world.entity.item.PrimedTnt", "DATA_FUSE_ID"),
    "VANILLA_PRIMED_TNT_BLOCK_STATE_DATA_ID": ("net.minecraft.world.entity.item.PrimedTnt", "DATA_BLOCK_STATE_ID"),
    "VANILLA_FIREWORK_ROCKET_ITEM_DATA_ID": ("net.minecraft.world.entity.projectile.FireworkRocketEntity", "DATA_ID_FIREWORKS_ITEM"),
    "VANILLA_FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID": ("net.minecraft.world.entity.projectile.FireworkRocketEntity", "DATA_ATTACHED_TO_TARGET"),
    "VANILLA_FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID": ("net.minecraft.world.entity.projectile.FireworkRocketEntity", "DATA_SHOT_AT_ANGLE"),
    "VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID": ("net.minecraft.world.entity.projectile.FishingHook", "DATA_HOOKED_ENTITY"),
    "VANILLA_FISHING_HOOK_BITING_DATA_ID": ("net.minecraft.world.entity.projectile.FishingHook", "DATA_BITING"),
    "VANILLA_OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID": ("net.minecraft.world.entity.OminousItemSpawner", "DATA_ITEM"),
    "VANILLA_BAT_FLAGS_DATA_ID": ("net.minecraft.world.entity.ambient.Bat", "DATA_ID_FLAGS"),
    "VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID": ("net.minecraft.world.entity.monster.skeleton.Skeleton", "DATA_STRAY_CONVERSION_ID"),
    "VANILLA_GHAST_CHARGING_DATA_ID": ("net.minecraft.world.entity.monster.Ghast", "DATA_IS_CHARGING"),
    "VANILLA_VEX_FLAGS_DATA_ID": ("net.minecraft.world.entity.monster.Vex", "DATA_FLAGS_ID"),
    "VANILLA_STRIDER_BOOST_TIME_DATA_ID": ("net.minecraft.world.entity.monster.Strider", "DATA_BOOST_TIME"),
    "VANILLA_STRIDER_SUFFOCATING_DATA_ID": ("net.minecraft.world.entity.monster.Strider", "DATA_SUFFOCATING"),
    "VANILLA_ZOGLIN_BABY_DATA_ID": ("net.minecraft.world.entity.monster.Zoglin", "DATA_BABY_ID"),
    "VANILLA_HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID": ("net.minecraft.world.entity.monster.hoglin.Hoglin", "DATA_IMMUNE_TO_ZOMBIFICATION"),
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
    "VANILLA_PLAYER_ABSORPTION_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_PLAYER_ABSORPTION_ID"),
    "VANILLA_PLAYER_SCORE_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_SCORE_ID"),
    "VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_SHOULDER_PARROT_LEFT"),
    "VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_SHOULDER_PARROT_RIGHT"),
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

FLAG_SCHEMA_NAMES = {
    "VANILLA_ENTITY_SHARED_FLAG_ON_FIRE": "FLAG_ONFIRE",
    "VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN": "FLAG_SHIFT_KEY_DOWN",
    "VANILLA_ENTITY_SHARED_FLAG_SPRINTING": "FLAG_SPRINTING",
    "VANILLA_ENTITY_SHARED_FLAG_SWIMMING": "FLAG_SWIMMING",
    "VANILLA_ENTITY_SHARED_FLAG_INVISIBLE": "FLAG_INVISIBLE",
    "VANILLA_ENTITY_SHARED_FLAG_GLOWING": "FLAG_GLOWING",
    "VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING": "FLAG_FALL_FLYING",
    "VANILLA_LIVING_ENTITY_FLAG_IS_USING": "LIVING_ENTITY_FLAG_IS_USING",
    "VANILLA_LIVING_ENTITY_FLAG_OFF_HAND": "LIVING_ENTITY_FLAG_OFF_HAND",
    "VANILLA_LIVING_ENTITY_FLAG_SPIN_ATTACK": "LIVING_ENTITY_FLAG_SPIN_ATTACK",
    "VANILLA_MOB_FLAG_NO_AI": "MOB_FLAG_NO_AI",
    "VANILLA_MOB_FLAG_LEFT_HANDED": "MOB_FLAG_LEFTHANDED",
    "VANILLA_MOB_FLAG_AGGRESSIVE": "MOB_FLAG_AGGRESSIVE",
    "VANILLA_TAMABLE_ANIMAL_FLAG_SITTING": "FLAG_SITTING",
    "VANILLA_TAMABLE_ANIMAL_FLAG_TAME": "FLAG_TAME",
    "VANILLA_AVATAR_MODEL_PART_CAPE": "MODEL_PART_CAPE",
    "VANILLA_AVATAR_MODEL_PART_JACKET": "MODEL_PART_JACKET",
    "VANILLA_AVATAR_MODEL_PART_LEFT_SLEEVE": "MODEL_PART_LEFT_SLEEVE",
    "VANILLA_AVATAR_MODEL_PART_RIGHT_SLEEVE": "MODEL_PART_RIGHT_SLEEVE",
    "VANILLA_AVATAR_MODEL_PART_LEFT_PANTS_LEG": "MODEL_PART_LEFT_PANTS_LEG",
    "VANILLA_AVATAR_MODEL_PART_RIGHT_PANTS_LEG": "MODEL_PART_RIGHT_PANTS_LEG",
    "VANILLA_AVATAR_MODEL_PART_HAT": "MODEL_PART_HAT",
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
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def source_for_owner(result: ExtractionResult, owner: str) -> JavaSource:
    matches = [
        source
        for source in result.sources.values()
        for java_type in source.types
        if java_type.fqcn == owner
    ]
    if len(matches) != 1:
        raise ExtractionError(f"{owner}: expected one source owner, found {len(matches)}")
    return matches[0]


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


def class_nearest_method_owner(
    result: ExtractionResult,
    class_map: dict[str, JavaType],
    owner: str,
    method_name: str,
    parameter_count: int,
    cache: dict[str, str],
) -> str:
    """Resolve the nearest class declaration of one method along a class hierarchy."""
    if owner in cache:
        return cache[owner]
    item = class_map.get(owner)
    if item is None:
        raise ExtractionError(f"unknown class in {method_name} hierarchy: {owner}")
    source = source_for_owner(result, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == method_name and len(method.parameters) == parameter_count
    ]
    if len(methods) > 1:
        raise ExtractionError(
            f"{owner}.{method_name}/{parameter_count}: expected at most one method"
        )
    if methods:
        cache[owner] = owner
        return owner
    if item.superclass is None:
        raise ExtractionError(
            f"{owner}: no {method_name}/{parameter_count} declaration or superclass"
        )
    resolved = class_nearest_method_owner(
        result,
        class_map,
        item.superclass,
        method_name,
        parameter_count,
        cache,
    )
    cache[owner] = resolved
    return resolved


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
            f"condition: VanillaEntityHurtSoundCondition::{condition}, "
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
            "condition: VanillaEntityHurtSoundCondition::CreakingHeartBound, "
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


def option_string(value: str | None) -> str:
    return "None" if value is None else f"Some({rust_string(value)})"


def render(result: ExtractionResult) -> str:
    accessors = {(item.owner, item.java_name): item for item in result.accessors}
    resolved_defaults, default_evidence = resolve_semantic_defaults(result)
    normalized = {
        key: normalize_default(result, item, resolved_defaults) for key, item in accessors.items()
    }
    for family in FAMILIES:
        for accessor in result.accessors:
            if accessor.owner == family.java_class and normalized[(accessor.owner, accessor.java_name)] is None:
                raise ExtractionError(f"compatibility accessor default unresolved: {accessor.owner}.{accessor.java_name}")
    flags = resolve_flags(result)
    value_masks = resolve_value_masks(result)
    packed_values = resolve_packed_values(result)
    camera_post_effects = camera_post_effect_specs(result)
    entity_event_handlers = entity_event_handler_specs(result)
    sound_events = sound_event_resources_by_constant(result)
    flag_by_owner: dict[str, list[ResolvedFlag]] = {}
    for flag in flags:
        flag_by_owner.setdefault(flag.owner, []).append(flag)

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
    collidability_cache: dict[str, str] = {}
    sound_source_cache: dict[str, str] = {}
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
    passenger_attachment_owner_cache: dict[str, str] = {}
    vehicle_attachment_owner_cache: dict[str, str] = {}
    controlling_passenger_owner_cache: dict[str, str] = {}
    rope_hold_position_owner_cache: dict[str, str] = {}
    tick_owner_cache: dict[str, str] = {}
    base_tick_owner_cache: dict[str, str] = {}
    ai_step_owner_cache: dict[str, str] = {}
    ride_tick_owner_cache: dict[str, str] = {}
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
        "use bbb_protocol::attribute_types::*;",
        "use bbb_protocol::entity_data_serializers::*;",
        "",
        "use super::*;",
        "",
        "macro_rules! default_attribute {",
        "    ($attribute:expr) => {",
        "        VanillaEntityDefaultAttribute::registry_default($attribute)",
        "    };",
        "    ($attribute:expr, $bits:expr) => {",
        "        VanillaEntityDefaultAttribute::overridden($attribute, $bits)",
        "    };",
        "}",
        "",
    ]

    for public_name, key in DATA_ALIASES.items():
        accessor = accessors[key]
        internal = "GENERATED_" + public_name
        lines.append(f"const {internal}: u8 = {accessor.data_id};")
        lines.append(f"pub const {public_name}: u8 = {internal};")
    lines.append("")

    for flag in flags:
        internal = "GENERATED_" + flag.rust_name
        signed = flag.value if flag.value < 128 else flag.value - 256
        lines.append(f"const {internal}: i8 = {signed};")
        lines.append(f"pub const {flag.rust_name}: i8 = {internal};")
    lines.append("")

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

    lines.append("pub const VANILLA_ENTITY_METADATA_CLASSES_26_1: &[VanillaEntityMetadataClassCatalogEntry] = &[")
    for item in result.classes:
        lines.extend(
            [
                "    VanillaEntityMetadataClassCatalogEntry {",
                f"        java_class: {rust_string(item.fqcn)},",
                f"        superclass: {option_string(item.superclass)},",
                "        interfaces: &[" + ", ".join(rust_string(interface) for interface in item.interfaces) + "],",
                f"        source_path: {rust_string(item.source_path)},",
                f"        kind: {rust_string(item.kind)},",
                f"        inherited_accessor_count: {inherited_count(item.fqcn)},",
                f"        own_accessor_count: {own_counts.get(item.fqcn, 0)},",
                "    },",
            ]
        )
    lines.extend([
        "];",
        "",
        "pub const VANILLA_ENTITY_EVENT_HANDLERS_26_1: &[VanillaEntityEventHandlerCatalogEntry] = &[",
    ])
    for item in entity_event_handlers:
        lines.extend(
            [
                "    VanillaEntityEventHandlerCatalogEntry {",
                f"        owner: {rust_string(item.owner)},",
                f"        super_handler_owner: {option_string(item.super_handler_owner)},",
                f"        source_path: {rust_string(item.source_path)},",
                "        direct_event_ids: &["
                + ", ".join(str(event_id) for event_id in item.direct_event_ids)
                + "],",
                "        delegated_event_ids: &["
                + ", ".join(str(event_id) for event_id in item.delegated_event_ids)
                + "],",
                f"        delegates_to: {option_string(item.delegates_to)},",
                "        forwards_unhandled_to_super: "
                + str(item.forwards_unhandled_to_super).lower()
                + ",",
                "    },",
            ]
        )
    lines.extend([
        "];",
        "",
        "pub const VANILLA_ENTITY_METADATA_ACCESSORS_26_1: &[VanillaEntityMetadataAccessorCatalogEntry] = &[",
    ])
    serializer_ids = {name: index for index, name in enumerate(result.serializer_names)}
    for item in result.accessors:
        default = normalized[(item.owner, item.java_name)]
        lines.extend(
            [
                "    VanillaEntityMetadataAccessorCatalogEntry {",
                f"        owner: {rust_string(item.owner)},",
                f"        java_name: {rust_string(item.java_name)},",
                f"        serializer_name: {rust_string(item.serializer)},",
                f"        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_{item.serializer}_ID,",
                f"        declaration_order: {item.declaration_order},",
                f"        data_id: {item.data_id},",
                f"        source_path: {rust_string(item.source_path)},",
                f"        default_expression: {rust_string(item.default_expression)},",
                f"        typed_default: {'Some(' + default.rust + ')' if default else 'None'},",
                "    },",
            ]
        )
        if item.serializer not in serializer_ids:
            raise ExtractionError(f"missing serializer id for {item.serializer}")
    lines.extend([
        "];",
        "",
        "pub const VANILLA_ENTITY_METADATA_FLAGS_26_1: &[VanillaEntityMetadataFlagCatalogEntry] = &[",
    ])
    for item in result.flags:
        lines.extend(
            [
                "    VanillaEntityMetadataFlagCatalogEntry {",
                f"        owner: {rust_string(item.owner)},",
                f"        java_name: {rust_string(item.java_name)},",
                f"        source_expression: {rust_string(item.expression)},",
                f"        value: {'Some(' + str(item.value) + ')' if item.value is not None else 'None'},",
                f"        accessor_name: {option_string(item.accessor_name)},",
                f"        status: VanillaEntityMetadataFlagStatus::{item.status.capitalize()},",
                f"        diagnostic: {option_string(item.reason)},",
                f"        source_path: {rust_string(item.source_path)},",
                "    },",
            ]
        )
    lines.extend([
        "];",
        "",
        "pub const VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1: &[VanillaEntityMetadataResolverEvidence] = &[",
    ])
    for item in default_evidence:
        lines.extend(
            [
                "    VanillaEntityMetadataResolverEvidence {",
                f"        java_class: {rust_string(item.owner)},",
                f"        accessor_name: {rust_string(item.accessor)},",
                f"        evidence_member: {rust_string(item.evidence_class + '.' + item.evidence_member)},",
                f"        source_expression: {rust_string(item.expression)},",
                f"        reason: {rust_string(item.reason)},",
                "    },",
            ]
        )
    lines.extend([
        "];",
        "",
        "pub const VANILLA_ENTITY_METADATA_FLAG_RESOLVER_EVIDENCE_26_1: &[VanillaEntityMetadataResolverEvidence] = &[",
    ])
    for item in flags:
        lines.extend(
            [
                "    VanillaEntityMetadataResolverEvidence {",
                f"        java_class: {rust_string(item.evidence_class)},",
                f"        accessor_name: {rust_string(item.accessor)},",
                f"        evidence_member: {rust_string(item.evidence_member)},",
                f"        source_expression: {rust_string(item.expression)},",
                f"        reason: {rust_string(item.reason)},",
                "    },",
            ]
        )
    lines.extend([
        "];",
        "",
        "pub const VANILLA_ENTITY_TYPE_METADATA_26_1: &[VanillaEntityTypeMetadataCatalogEntry] = &[",
    ])
    for item in result.entity_types:
        lines.extend(
            [
                "    VanillaEntityTypeMetadataCatalogEntry {",
                f"        entity_type_id: {item.entity_type_id},",
                f"        constant_name: {rust_string(item.constant_name)},",
                f"        resource_id: {rust_string(item.resource_id)},",
                f"        runtime_class: {option_string(item.runtime_class)},",
                f"        schema_class: {option_string(schema_class(item.runtime_class))},",
                "        default_attributes: &["
                + ", ".join(
                    (
                        "default_attribute!("
                        f"VANILLA_ATTRIBUTE_{constant}_ID, {override})"
                        if override is not None
                        else "default_attribute!("
                        f"VANILLA_ATTRIBUTE_{constant}_ID)"
                    )
                    for constant, override in item.attribute_base_overrides
                )
                + "],",
                f"        handle_entity_event_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'handleEntityEvent', 1, handle_entity_event_owner_cache) if item.runtime_class is not None else None)},",
                f"        passenger_attachment_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'getPassengerAttachmentPoint', 3, passenger_attachment_owner_cache) if item.runtime_class is not None else None)},",
                f"        vehicle_attachment_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'getVehicleAttachmentPoint', 1, vehicle_attachment_owner_cache) if item.runtime_class is not None else None)},",
                f"        controlling_passenger_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'getControllingPassenger', 0, controlling_passenger_owner_cache) if item.runtime_class is not None else None)},",
                f"        leash_offset_owner: {option_string(effective_leash_offset_owner(item.runtime_class))},",
                f"        rope_hold_position_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'getRopeHoldPosition', 1, rope_hold_position_owner_cache) if item.runtime_class is not None else None)},",
                f"        tick_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'tick', 0, tick_owner_cache) if item.runtime_class is not None else None)},",
                f"        base_tick_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'baseTick', 0, base_tick_owner_cache) if item.runtime_class is not None else None)},",
                f"        ai_step_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'aiStep', 0, ai_step_owner_cache) if extends_class(item.runtime_class, 'net.minecraft.world.entity.LivingEntity') else None)},",
                f"        ride_tick_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'rideTick', 0, ride_tick_owner_cache) if item.runtime_class is not None else None)},",
                f"        client_movement_policy: VanillaEntityClientMovementPolicy::{client_movement_policy(item.runtime_class)},",
                f"        block_effect_tick_policy: VanillaEntityBlockEffectTickPolicy::{block_effect_tick_policy(item.runtime_class)},",
                f"        interpolation_policy: VanillaEntityInterpolationPolicy::{interpolation_policy(item.runtime_class)},",
                f"        baby_behavior: VanillaEntityBabyBehavior::{class_baby_behavior(result, class_map, item.runtime_class, baby_behavior_cache) if extends_class(item.runtime_class, 'net.minecraft.world.entity.LivingEntity') else 'Never'},",
                f"        baby_age_scale_bits: {class_baby_age_scale_bits(result, class_map, item.runtime_class, baby_age_scale_cache) if extends_class(item.runtime_class, 'net.minecraft.world.entity.LivingEntity') else '1.0_f32.to_bits()'},",
                f"        is_pickable_by_class: {str(class_is_pickable_without_type_tag(result, class_map, item.runtime_class, pickable_cache)).lower()},",
                f"        sound_source_kind: VanillaEntitySoundSourceKind::{class_sound_source_kind(result, class_map, item.runtime_class, sound_source_cache)},",
                f"        hurt_sound_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'getHurtSound', 1, hurt_sound_owner_cache) if extends_class(item.runtime_class, 'net.minecraft.world.entity.LivingEntity') else None)},",
                f"        hurt_sound_policy: {class_hurt_sound_policy(result, class_map, item.runtime_class, sound_events, hurt_sound_owner_cache, hurt_sound_policy_cache) if extends_class(item.runtime_class, 'net.minecraft.world.entity.LivingEntity') else 'VanillaEntityHurtSoundPolicy::NonLiving'},",
                f"        sound_volume_owner: {option_string(class_nearest_method_owner(result, class_map, item.runtime_class, 'getSoundVolume', 0, sound_volume_owner_cache) if extends_class(item.runtime_class, 'net.minecraft.world.entity.LivingEntity') else None)},",
                f"        sound_volume_policy: {class_sound_volume_policy(result, class_map, item.runtime_class, sound_volume_owner_cache, sound_volume_policy_cache) if extends_class(item.runtime_class, 'net.minecraft.world.entity.LivingEntity') else 'VanillaEntitySoundVolumePolicy::NonLiving'},",
                f"        pushability_kind: VanillaEntityPushabilityKind::{class_pushability_kind(result, class_map, item.runtime_class, pushability_cache)},",
                f"        collidability_kind: VanillaEntityCollidabilityKind::{class_collidability_kind(result, class_map, item.runtime_class, collidability_cache)},",
                f"        is_living: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.LivingEntity')).lower()},",
                f"        is_animal: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.animal.Animal')).lower()},",
                f"        is_tamable_animal: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.TamableAnimal')).lower()},",
                f"        is_projectile: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.projectile.Projectile')).lower()},",
                f"        is_hurting_projectile: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.projectile.hurtingprojectile.AbstractHurtingProjectile')).lower()},",
                f"        is_wind_charge: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.projectile.hurtingprojectile.windcharge.AbstractWindCharge')).lower()},",
                f"        is_vehicle: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.vehicle.VehicleEntity')).lower()},",
                f"        is_boat: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.vehicle.boat.AbstractBoat')).lower()},",
                f"        boat_ride_height_multiplier_bits: {'Some(' + class_boat_ride_height_multiplier(result, class_map, item.runtime_class, boat_ride_height_cache) + ')' if extends_class(item.runtime_class, 'net.minecraft.world.entity.vehicle.boat.AbstractBoat') else 'None'},",
                f"        can_use_every_equipment_slot: {str(class_can_use_every_equipment_slot(result, class_map, item.runtime_class, can_use_every_slot_cache)).lower()},",
                f"        is_abstract_chest_boat: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.vehicle.boat.AbstractChestBoat')).lower()},",
                f"        is_minecart: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.vehicle.minecart.AbstractMinecart')).lower()},",
                f"        is_abstract_horse: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.animal.equine.AbstractHorse')).lower()},",
                f"        is_abstract_chested_horse: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.animal.equine.AbstractChestedHorse')).lower()},",
                f"        is_abstract_nautilus: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.animal.nautilus.AbstractNautilus')).lower()},",
                f"        is_llama: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.animal.equine.Llama')).lower()},",
                f"        is_item_frame: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.decoration.ItemFrame')).lower()},",
                f"        is_guardian: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.monster.Guardian')).lower()},",
                f"        is_flying_animal: {str(implements_interface(item.runtime_class, 'net.minecraft.world.entity.animal.FlyingAnimal')).lower()},",
                f"        is_camel: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.animal.camel.Camel')).lower()},",
                f"        is_squid: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.animal.squid.Squid')).lower()},",
                f"        is_slime: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.monster.Slime')).lower()},",
                f"        is_abstract_piglin: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.monster.piglin.AbstractPiglin')).lower()},",
                f"        is_spider: {str(extends_class(item.runtime_class, 'net.minecraft.world.entity.monster.spider.Spider')).lower()},",
                f"        illager_arm_pose_uses_celebrating: {str(class_illager_arm_pose_uses_celebrating(result, class_map, item.runtime_class, illager_celebrating_cache)).lower() if item.runtime_class is not None else 'false'},",
                f"        is_player_rideable_jumping: {str(implements_interface(item.runtime_class, 'net.minecraft.world.entity.PlayerRideableJumping')).lower()},",
                f"        has_custom_inventory_screen: {str(implements_interface(item.runtime_class, 'net.minecraft.world.entity.HasCustomInventoryScreen')).lower()},",
                f"        diagnostic: {option_string(item.diagnostic)},",
                "    },",
            ]
        )
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

    for family in FAMILIES:
        own = [item for item in result.accessors if item.owner == family.java_class]
        lines.append(f"const {family.rust_name.upper()}_FIELDS: &[VanillaEntityMetadataFieldSchema] = &[")
        for item in own:
            default = normalized[(item.owner, item.java_name)]
            lines.extend(
                [
                    "    VanillaEntityMetadataFieldSchema {",
                    f"        name: {rust_string(item.java_name)},",
                    f"        data_id: {item.data_id},",
                    f"        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_{item.serializer}_ID,",
                    f"        default: {default.rust},",
                    "    },",
                ]
            )
        lines.append("];")
        family_flags = flag_by_owner.get(family.java_class, [])
        lines.append(f"const {family.rust_name.upper()}_FLAGS: &[VanillaEntityMetadataFlagSchema] = &[")
        for flag in family_flags:
            data_id = accessors[(flag.owner, flag.accessor)].data_id
            java_name = FLAG_SCHEMA_NAMES.get(flag.rust_name, flag.evidence_member)
            lines.extend(
                [
                    "    VanillaEntityMetadataFlagSchema {",
                    f"        name: {rust_string(java_name)},",
                    f"        data_id: {data_id},",
                    f"        mask: {flag.rust_name},",
                    "    },",
                ]
            )
        lines.append("];")
        parent = f"Some(VanillaEntityMetadataFamily::{family.parent})" if family.parent else "None"
        lines.extend(
            [
                f"const {family.rust_name.upper()}_SCHEMA: VanillaEntityMetadataFamilySchema = VanillaEntityMetadataFamilySchema {{",
                f"    family: VanillaEntityMetadataFamily::{family.rust_name},",
                f"    parent: {parent},",
                f"    inherited_field_count: {inherited_count(family.java_class)},",
                f"    own_fields: {family.rust_name.upper()}_FIELDS,",
                f"    own_flags: {family.rust_name.upper()}_FLAGS,",
                "};",
                "",
            ]
        )
    lines.append("pub(super) const fn generated_entity_metadata_schema(family: VanillaEntityMetadataFamily) -> &'static VanillaEntityMetadataFamilySchema {")
    lines.append("    match family {")
    for family in FAMILIES:
        lines.append(f"        VanillaEntityMetadataFamily::{family.rust_name} => &{family.rust_name.upper()}_SCHEMA,")
    lines.extend(["    }", "}", ""])
    lines.extend(
        [
            "#[cfg(test)]",
            "mod generated_catalog_guards {",
            "    #[test]",
            "    fn default_attribute_suppliers_cover_exactly_living_entity_types() {",
            "        let mut supplier_count = 0;",
            "        for entry in super::VANILLA_ENTITY_TYPE_METADATA_26_1 {",
            "            assert_eq!(entry.is_living, !entry.default_attributes.is_empty(), \"{}\", entry.resource_id);",
            "            let mut unique = std::collections::BTreeSet::new();",
            "            assert!(entry.default_attributes.iter().all(|attribute| unique.insert(attribute.attribute_id)), \"{}\", entry.resource_id);",
            "            supplier_count += usize::from(entry.is_living);",
            "        }",
            "        assert_eq!(supplier_count, 92);",
            "    }",
            "",
            "    #[test]",
            "    fn client_tick_method_owner_inventory_covers_every_entity_type() {",
            "        assert_eq!(super::VANILLA_ENTITY_TYPE_METADATA_26_1.len(), 157);",
            "        for entry in super::VANILLA_ENTITY_TYPE_METADATA_26_1 {",
            "            assert!(entry.tick_owner.is_some(), \"{} lacks tick owner\", entry.resource_id);",
            "            assert!(entry.base_tick_owner.is_some(), \"{} lacks baseTick owner\", entry.resource_id);",
            "            assert!(entry.ride_tick_owner.is_some(), \"{} lacks rideTick owner\", entry.resource_id);",
            "            assert_eq!(entry.is_living, entry.ai_step_owner.is_some(), \"{} aiStep/living mismatch\", entry.resource_id);",
            "            assert_eq!(entry.is_vehicle, entry.is_boat || entry.is_minecart, \"{} vehicle-family mismatch\", entry.resource_id);",
            "        }",
            "        let owner = |resource_id: &str| {",
            "            super::VANILLA_ENTITY_TYPE_METADATA_26_1",
            "                .iter()",
            "                .find(|entry| entry.resource_id == resource_id)",
            "                .unwrap_or_else(|| panic!(\"missing {resource_id}\"))",
            "        };",
            "        assert!(owner(\"minecraft:item\").tick_owner.unwrap().ends_with(\".ItemEntity\"));",
            "        assert!(owner(\"minecraft:zombie\").tick_owner.unwrap().ends_with(\".Zombie\"));",
            "        assert!(owner(\"minecraft:cow\").tick_owner.unwrap().ends_with(\".Mob\"));",
            "        assert!(owner(\"minecraft:player\").ride_tick_owner.unwrap().ends_with(\".Player\"));",
            "        assert!(owner(\"minecraft:armor_stand\").ai_step_owner.unwrap().ends_with(\".LivingEntity\"));",
            "        assert_eq!(owner(\"minecraft:item\").client_movement_policy, super::VanillaEntityClientMovementPolicy::Item);",
            "        assert_eq!(owner(\"minecraft:trident\").client_movement_policy, super::VanillaEntityClientMovementPolicy::AbstractArrow);",
            "        assert_eq!(owner(\"minecraft:experience_bottle\").client_movement_policy, super::VanillaEntityClientMovementPolicy::ThrownExperienceBottle);",
            "        assert_eq!(owner(\"minecraft:splash_potion\").client_movement_policy, super::VanillaEntityClientMovementPolicy::ThrownPotion);",
            "        assert_eq!(owner(\"minecraft:wind_charge\").client_movement_policy, super::VanillaEntityClientMovementPolicy::WindCharge);",
            "        assert_eq!(owner(\"minecraft:dragon_fireball\").client_movement_policy, super::VanillaEntityClientMovementPolicy::DragonFireball);",
            "        assert_eq!(owner(\"minecraft:wither_skull\").client_movement_policy, super::VanillaEntityClientMovementPolicy::WitherSkull);",
            "        assert_eq!(owner(\"minecraft:zombie\").client_movement_policy, super::VanillaEntityClientMovementPolicy::ServerSynchronized);",
            "        assert_eq!(owner(\"minecraft:cow\").block_effect_tick_policy, super::VanillaEntityBlockEffectTickPolicy::LocalAuthoritativeLiving);",
            "        assert_eq!(owner(\"minecraft:ender_dragon\").block_effect_tick_policy, super::VanillaEntityBlockEffectTickPolicy::None);",
            "        assert_eq!(owner(\"minecraft:oak_boat\").block_effect_tick_policy, super::VanillaEntityBlockEffectTickPolicy::BoatTwice);",
            "        assert_eq!(owner(\"minecraft:minecart\").block_effect_tick_policy, super::VanillaEntityBlockEffectTickPolicy::None);",
            "        assert_eq!(owner(\"minecraft:item\").block_effect_tick_policy, super::VanillaEntityBlockEffectTickPolicy::ItemWithLastMovements);",
            "        assert_eq!(owner(\"minecraft:arrow\").block_effect_tick_policy, super::VanillaEntityBlockEffectTickPolicy::AbstractArrowSegments);",
            "        assert_eq!(owner(\"minecraft:firework_rocket\").block_effect_tick_policy, super::VanillaEntityBlockEffectTickPolicy::DetachedFirework);",
            "        assert_eq!(owner(\"minecraft:end_crystal\").block_effect_tick_policy, super::VanillaEntityBlockEffectTickPolicy::OnceAfterMovement);",
            "    }",
            "",
            "    #[test]",
            "    fn passenger_attachment_and_controller_owners_cover_every_entity_type() {",
            "        let mut passenger_attachment_owners = std::collections::BTreeSet::new();",
            "        let mut vehicle_attachment_owners = std::collections::BTreeSet::new();",
            "        let mut controlling_passenger_owners = std::collections::BTreeSet::new();",
            "        for entry in super::VANILLA_ENTITY_TYPE_METADATA_26_1 {",
            "            passenger_attachment_owners.insert(entry.passenger_attachment_owner.expect(\"passenger attachment owner\"));",
            "            vehicle_attachment_owners.insert(entry.vehicle_attachment_owner.expect(\"vehicle attachment owner\"));",
            "            controlling_passenger_owners.insert(entry.controlling_passenger_owner.expect(\"controller owner\"));",
            "        }",
            "        assert_eq!(passenger_attachment_owners, std::collections::BTreeSet::from([",
            "            \"net.minecraft.world.entity.Entity\",",
            "            \"net.minecraft.world.entity.animal.camel.Camel\",",
            "            \"net.minecraft.world.entity.animal.equine.AbstractHorse\",",
            "            \"net.minecraft.world.entity.animal.equine.Llama\",",
            "            \"net.minecraft.world.entity.monster.Slime\",",
            "            \"net.minecraft.world.entity.monster.Strider\",",
            "            \"net.minecraft.world.entity.vehicle.boat.AbstractBoat\",",
            "            \"net.minecraft.world.entity.vehicle.minecart.AbstractMinecart\",",
            "        ]));",
            "        assert_eq!(vehicle_attachment_owners, std::collections::BTreeSet::from([",
            "            \"net.minecraft.world.entity.Entity\",",
            "            \"net.minecraft.world.entity.monster.spider.CaveSpider\",",
            "            \"net.minecraft.world.entity.monster.spider.Spider\",",
            "        ]));",
            "        assert_eq!(controlling_passenger_owners, std::collections::BTreeSet::from([",
            "            \"net.minecraft.world.entity.Entity\",",
            "            \"net.minecraft.world.entity.Mob\",",
            "            \"net.minecraft.world.entity.animal.equine.AbstractHorse\",",
            "            \"net.minecraft.world.entity.animal.happyghast.HappyGhast\",",
            "            \"net.minecraft.world.entity.animal.nautilus.AbstractNautilus\",",
            "            \"net.minecraft.world.entity.animal.pig.Pig\",",
            "            \"net.minecraft.world.entity.monster.Strider\",",
            "            \"net.minecraft.world.entity.vehicle.boat.AbstractBoat\",",
            "        ]));",
            "    }",
            "",
            "    #[test]",
            "    fn leash_offset_and_rope_holder_owners_cover_every_entity_type() {",
            "        let mut leash_offset_owners = std::collections::BTreeSet::new();",
            "        let mut rope_hold_position_owners = std::collections::BTreeSet::new();",
            "        for entry in super::VANILLA_ENTITY_TYPE_METADATA_26_1 {",
            "            assert_eq!(entry.leash_offset_owner.is_some(), super::super::vanilla_entity_metadata_type_is_leashable(entry.entity_type_id), \"{}\", entry.resource_id);",
            "            if let Some(owner) = entry.leash_offset_owner {",
            "                leash_offset_owners.insert(owner);",
            "            }",
            "            rope_hold_position_owners.insert(entry.rope_hold_position_owner.expect(\"rope holder owner\"));",
            "        }",
            "        assert_eq!(leash_offset_owners, std::collections::BTreeSet::from([",
            "            \"net.minecraft.world.entity.Leashable\",",
            "            \"net.minecraft.world.entity.animal.allay.Allay\",",
            "            \"net.minecraft.world.entity.animal.bee.Bee\",",
            "            \"net.minecraft.world.entity.animal.camel.Camel\",",
            "            \"net.minecraft.world.entity.animal.equine.Llama\",",
            "            \"net.minecraft.world.entity.animal.feline.Ocelot\",",
            "            \"net.minecraft.world.entity.animal.fox.Fox\",",
            "            \"net.minecraft.world.entity.animal.golem.CopperGolem\",",
            "            \"net.minecraft.world.entity.animal.golem.IronGolem\",",
            "            \"net.minecraft.world.entity.animal.golem.SnowGolem\",",
            "            \"net.minecraft.world.entity.animal.happyghast.HappyGhast\",",
            "            \"net.minecraft.world.entity.animal.parrot.Parrot\",",
            "            \"net.minecraft.world.entity.animal.pig.Pig\",",
            "            \"net.minecraft.world.entity.animal.rabbit.Rabbit\",",
            "            \"net.minecraft.world.entity.animal.wolf.Wolf\",",
            "            \"net.minecraft.world.entity.monster.Strider\",",
            "            \"net.minecraft.world.entity.vehicle.boat.AbstractBoat\",",
            "        ]));",
            "        assert_eq!(rope_hold_position_owners, std::collections::BTreeSet::from([",
            "            \"net.minecraft.world.entity.Entity\",",
            "            \"net.minecraft.world.entity.decoration.LeashFenceKnotEntity\",",
            "            \"net.minecraft.world.entity.npc.villager.AbstractVillager\",",
            "            \"net.minecraft.world.entity.player.Player\",",
            "        ]));",
            "    }",
            "",
            "    #[test]",
            "    fn ender_dragon_sitting_phase_membership_matches_registered_classes() {",
            "        for phase in super::VANILLA_ENDER_DRAGON_PHASES_26_1 {",
            "            assert_eq!(",
            "                super::vanilla_ender_dragon_phase_is_sitting(phase.phase_id),",
            "                phase.is_sitting,",
            '                "{} ({})",',
            "                phase.java_field_name,",
            "                phase.implementation_class",
            "            );",
            "        }",
            "        let hover = super::VANILLA_ENDER_DRAGON_PHASES_26_1",
            "            .iter()",
            '            .find(|phase| phase.implementation_class.ends_with(".DragonHoverPhase"))',
            '            .expect("generated DragonHoverPhase registration");',
            '        assert!(hover.is_sitting, "DragonHoverPhase directly overrides isSitting");',
            "        assert!(!super::vanilla_ender_dragon_phase_is_sitting(-1));",
            "        assert!(!super::vanilla_ender_dragon_phase_is_sitting(",
            "            super::VANILLA_ENDER_DRAGON_PHASES_26_1.len() as i32",
            "        ));",
            "    }",
            "",
            "    #[test]",
            "    fn dragon_animation_consumers_do_not_restore_sitting_phase_membership() {",
            '        let source = include_str!("../dragon.rs");',
            '        assert!(source.contains("vanilla_ender_dragon_phase_is_sitting(self.phase_id)"));',
            '        let fixture = source',
            '            .split("fn dragon_sitting_pose_matches_generated_phase_class_behavior")',
            "            .nth(1)",
            '            .and_then(|source| source.split("}").next())',
            '            .expect("dragon sitting-pose fixture source");',
            '        assert!(fixture.contains("VANILLA_ENDER_DRAGON_PHASES_26_1"));',
            '        assert!(!source.contains("ENDER_DRAGON_SITTING_PHASE_IDS"));',
            '        assert!(!source.contains("VANILLA_ENDER_DRAGON_PHASE_SITTING_"));',
            '        assert!(!source.contains("VANILLA_ENDER_DRAGON_PHASE_HOVERING_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_audio_does_not_restore_hostile_entity_membership() {",
            '        let source = include_str!("../../client/audio.rs");',
            '        let removed_lookup = ["is_vanilla_hostile_sound_source_", "type"].concat();',
            '        assert!(!source.contains(&format!("fn {removed_lookup}(")));',
            "    }",
            "",
            "    #[test]",
            "    fn production_camera_post_effect_does_not_restore_entity_type_dispatch() {",
            '        let source = include_str!("../../../../bbb-native/src/runtime.rs");',
            '        let body = source',
            '            .split("fn current_post_effect_id_from_world")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("camera post-effect projection source");',
            '        assert!(body.contains("vanilla_entity_camera_post_effect_id"));',
            '        assert!(!body.contains("vanilla_entity_type_is_spider"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            '        assert!(!body.contains("match entity_type_id"));',
            '        assert!(!body.contains("minecraft:creeper"));',
            '        assert!(!body.contains("minecraft:spider"));',
            '        assert!(!body.contains("minecraft:invert"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_minecart_display_offset_does_not_restore_type_dispatch() {",
            '        let source = include_str!("../schema.rs");',
            '        let body = source',
            '            .split("fn vanilla_minecart_default_display_offset")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("minecart display-offset query source");',
            '        assert!(body.contains("VANILLA_ENTITY_MINECART_DEFAULT_DISPLAY_OFFSETS_26_1"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            '        assert!(!body.contains("match entity_type_id"));',
            '        let store = include_str!("../store.rs");',
            '        assert!(store.contains("vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type"));',
            '        let display_body = store',
            '            .split("pub(crate) fn minecart_display_block_state")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(crate) fn falling_block_state").next())',
            '            .expect("minecart display block projection source");',
            '        assert!(display_body.contains("vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type"));',
            '        assert!(!display_body.contains("MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID"));',
            '        assert!(!display_body.contains("MINECART_DISPLAY_OFFSET_DATA_ID"));',
            '        assert!(!display_body.contains("VANILLA_FURNACE_MINECART_FUEL_DATA_ID"));',
            '        assert!(!display_body.contains("VANILLA_FURNACE_MINECART_DEFAULT_FUEL"));',
            '        assert!(!display_body.contains("== VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID"));',
            '        let world_tests = include_str!("../tests/vehicle_and_block_entity_projection.rs");',
            '        assert!(world_tests.contains("fn furnace_minecart_fuel_data("));',
            '        assert!(world_tests.contains("vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type"));',
            '        assert!(!world_tests.contains("FURNACE_MINECART_FUEL_DATA_ID"));',
            '        assert!(!world_tests.contains("FURNACE_MINECART_DEFAULT_FUEL"));',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        assert!(input.contains("vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type"));',
            '        assert!(!input.contains("MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID"));',
            '        assert!(!input.contains("MINECART_DISPLAY_OFFSET_DATA_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_primed_tnt_projection_does_not_restore_fixed_accessors() {",
            '        let store = include_str!("../store.rs");',
            '        let lifecycle_tests = include_str!("../tests/lifecycle_and_queries.rs");',
            '        let projection_tests = include_str!("../tests/vehicle_and_block_entity_projection.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let native_item_tests = include_str!("../../../../bbb-native/src/item_models/tests.rs");',
            '        assert_eq!(store.matches("vanilla_entity_metadata_primed_tnt_accessors_for_entity_type").count(), 4);',
            '        assert!(!store.contains("VANILLA_ENTITY_TYPE_TNT_ID"));',
            '        for source in [lifecycle_tests, projection_tests, native_input, native_input_tests, native_item_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_primed_tnt_accessors_for_entity_type"));',
            "        }",
            '        for source in [store, lifecycle_tests, projection_tests, native_input, native_input_tests, native_item_tests] {',
            '            for removed in [',
            '                "VANILLA_PRIMED_TNT_FUSE_DATA_ID",',
            '                "VANILLA_PRIMED_TNT_BLOCK_STATE_DATA_ID",',
            '                "VANILLA_PRIMED_TNT_DEFAULT_FUSE",',
            '                "VANILLA_PRIMED_TNT_DEFAULT_BLOCK_STATE",',
            '                "PRIMED_TNT_FUSE_DATA_ID",',
            '                "PRIMED_TNT_BLOCK_STATE_DATA_ID",',
            '                "PRIMED_TNT_DEFAULT_FUSE",',
            "            ] {",
            '                assert!(!source.contains(removed), "PrimedTnt projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_firework_rocket_projection_does_not_restore_fixed_accessors() {",
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/item_entity_and_pickup_projection.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let native_runtime_tests = include_str!("../../../../bbb-native/src/runtime/events/tests.rs");',
            '        for source in [store, world_tests, native_input, native_input_tests, native_runtime_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_firework_rocket_accessors_for_entity_type"));',
            '            for removed in [',
            '                "VANILLA_FIREWORK_ROCKET_ITEM_DATA_ID",',
            '                "VANILLA_FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID",',
            '                "VANILLA_FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID",',
            '                "VANILLA_FIREWORK_ROCKET_DEFAULT_ITEM_RESOURCE_ID",',
            '                "VANILLA_FIREWORK_ROCKET_DEFAULT_SHOT_AT_ANGLE",',
            '                "FIREWORK_ROCKET_ITEM_DATA_ID",',
            '                "FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID",',
            '                "FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID",',
            '                "FIREWORK_ROCKET_DEFAULT_ITEM_RESOURCE_ID",',
            "            ] {",
            '                assert!(!source.contains(removed), "FireworkRocket projection restored {removed}");',
            "            }",
            "        }",
            '        assert!(!store.contains("VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_experience_orb_value_does_not_restore_fixed_metadata_facts() {",
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/item_entity_and_pickup_projection.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [store, world_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type"));',
            '            for removed in ["EXPERIENCE_ORB_VALUE_DATA_ID", "EXPERIENCE_ORB_DEFAULT_VALUE"] {',
            '                assert!(!source.contains(removed), "ExperienceOrb projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_entity_scale_does_not_restore_type_dispatch() {",
            '        let source = include_str!("../dimensions.rs");',
            '        let body = source',
            '            .split("fn entity_scale")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("entity scale projection source");',
            '        assert!(body.contains("vanilla_entity_scale_sanitizer"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            '        assert!(!body.contains("HAPPY_GHAST_MAX_SCALE"));',
            '        assert!(!body.contains("SHULKER_MAX_SCALE"));',
            '        assert!(!body.contains("match entity_type_id"));',
            '        let metadata = include_str!("../metadata.rs");',
            '        let attribute_body = metadata',
            '            .split("pub fn apply_update_attributes")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n    }"))',
            "            .map(|(body, _)| body)",
            '            .expect("attribute apply source");',
            '        assert!(attribute_body.contains("vanilla_entity_metadata_type_is_living"));',
            '        assert!(!attribute_body.contains("vanilla_living_entity_type"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_taming_particles_do_not_restore_entity_type_membership() {",
            '        let source = include_str!("../../entities.rs");',
            '        let body = source',
            '            .split("fn is_vanilla_taming_particle_entity_type")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("taming particle type query source");',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_boat_ride_height_does_not_restore_raft_membership() {",
            '        let source = include_str!("../dimensions.rs");',
            '        let removed_lookup = ["fn is_vanilla_", "raft_type("].concat();',
            "        assert!(!source.contains(&removed_lookup));",
            "    }",
            "",
            "    #[test]",
            "    fn production_mount_slots_do_not_restore_always_active_type_membership() {",
            '        let source = include_str!("../../entities.rs");',
            '        let removed_lookup = ["is_vanilla_horse_slot_", "always_active_type"].concat();',
            "        assert!(!source.contains(&removed_lookup));",
            "    }",
            "",
            "    #[test]",
            "    fn production_end_crystal_projection_uses_generated_accessors() {",
            '        let store = include_str!("../store.rs");',
            '        let beam_body = store',
            '            .split("fn end_crystal_beam_source")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn ender_dragon_beam_source").next())',
            '            .expect("end crystal beam projection source");',
            '        assert!(beam_body.contains("vanilla_entity_metadata_end_crystal_accessors_for_entity_type"));',
            '        assert!(!beam_body.contains("END_CRYSTAL_BEAM_TARGET_DATA_ID"));',
            '        assert!(!beam_body.contains("VANILLA_ENTITY_TYPE_END_CRYSTAL_ID"));',
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let show_bottom_body = variants',
            '            .split("fn end_crystal_shows_bottom")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}").map(|(body, _)| body))',
            '            .expect("end crystal show-bottom projection source");',
            '        assert!(show_bottom_body.contains("vanilla_entity_metadata_end_crystal_accessors_for_entity_type"));',
            '        assert!(!show_bottom_body.contains("END_CRYSTAL_SHOW_BOTTOM_DATA_ID"));',
            '        assert!(!show_bottom_body.contains("END_CRYSTAL_DEFAULT_SHOW_BOTTOM"));',
            '        assert!(!show_bottom_body.contains("VANILLA_ENTITY_TYPE_END_CRYSTAL_ID"));',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let debug_body = input',
            '            .split("fn debug_push_end_crystal_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_ghast_additional_save_data").next())',
            '            .expect("end crystal debug projection source");',
            '        assert!(debug_body.contains("vanilla_entity_metadata_end_crystal_accessors_for_entity_type"));',
            '        assert!(!debug_body.contains("END_CRYSTAL_BEAM_TARGET_DATA_ID"));',
            '        assert!(!debug_body.contains("END_CRYSTAL_SHOW_BOTTOM_DATA_ID"));',
            '        assert!(!debug_body.contains("END_CRYSTAL_DEFAULT_SHOW_BOTTOM"));',
            '        let world_tests = include_str!("../tests/aquatic_and_beam_creature_animation.rs");',
            '        assert!(world_tests.contains("fn end_crystal_beam_target_data("));',
            '        assert!(world_tests.contains("fn end_crystal_beam_target_collision_data("));',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        assert!(native_scene_tests.contains("fn end_crystal_metadata_accessors("));',
            '        assert!(native_scene_tests.contains("fn end_crystal_beam_target_collision_data("));',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        assert!(native_input_tests.contains("fn end_crystal_metadata_accessors("));',
            '        for source in [world_tests, native_scene_tests, input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_end_crystal_accessors_for_entity_type"));',
            "        }",
            '        for source in [world_tests, native_scene, native_scene_tests, input, native_input_tests] {',
            '            for removed in ["END_CRYSTAL_BEAM_TARGET_DATA_ID", "END_CRYSTAL_SHOW_BOTTOM_DATA_ID", "END_CRYSTAL_DEFAULT_SHOW_BOTTOM"] {',
            '                assert!(!source.contains(removed), "EndCrystal consumer restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_ghast_charging_uses_generated_accessor() {",
            '        let source = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let collision = include_str!("../../client/local_player_collision.rs");',
            '        let body = source',
            '            .split("VANILLA_ENTITY_TYPE_GHAST_ID")',
            "            .nth(1)",
            '            .and_then(|source| source.split("VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID").next())',
            '            .expect("Ghast model projection source");',
            '        assert!(body.contains("vanilla_entity_metadata_ghast_charging_accessor_for_entity_type"));',
            '        assert!(collision.contains("vanilla_entity_metadata_happy_ghast_accessors_for_entity_type"));',
            '        assert!(collision.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        for removed in [',
            '            "GHAST_IS_CHARGING_DATA_ID",',
            '            "GHAST_DEFAULT_CHARGING",',
            '            "VANILLA_GHAST_CHARGING_DATA_ID",',
            '            "VANILLA_GHAST_DEFAULT_CHARGING",',
            '            "HAPPY_GHAST_IS_LEASH_HOLDER_DATA_ID",',
            '            "HAPPY_GHAST_DEFAULT_IS_LEASH_HOLDER",',
            '            "HAPPY_GHAST_STAYS_STILL_DATA_ID",',
            '            "HAPPY_GHAST_DEFAULT_STAYS_STILL",',
            '            "VANILLA_HAPPY_GHAST_IS_LEASH_HOLDER_DATA_ID",',
            '            "VANILLA_HAPPY_GHAST_DEFAULT_IS_LEASH_HOLDER",',
            '            "VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID",',
            '            "VANILLA_HAPPY_GHAST_DEFAULT_STAYS_STILL",',
            "        ] {",
            '            assert!(!body.contains(removed), "Ghast projection restored {removed}");',
            '            assert!(!native_scene.contains(removed), "native scene restored {removed}");',
            '            assert!(!collision.contains(removed), "collision restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_glow_squid_dark_ticks_uses_generated_accessor() {",
            '        let instance = include_str!("../../../../bbb-native/src/entity_scene/instance.rs");',
            '        let light_body = instance',
            '            .split("pub(super) fn entity_light_coords")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn sheep_head_eat_pose").next())',
            '            .expect("entity light projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let debug_body = input',
            '            .split("fn debug_push_glow_squid_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_happy_ghast_additional_save_data").next())',
            '            .expect("GlowSquid debug projection source");',
            '        for source in [light_body, debug_body] {',
            '            assert!(source.contains("vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type"));',
            '            for removed in [',
            '                "GLOW_SQUID_DARK_TICKS_DATA_ID",',
            '                "GLOW_SQUID_DEFAULT_DARK_TICKS",',
            '                "VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID",',
            '                "VANILLA_GLOW_SQUID_DEFAULT_DARK_TICKS",',
            "            ] {",
            '                assert!(!source.contains(removed), "GlowSquid projection restored {removed}");',
            "            }",
            "        }",
            '        assert!(!light_body.contains("VANILLA_ENTITY_TYPE_GLOW_SQUID_ID"));',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type"));',
            '            for removed in ["GLOW_SQUID_DARK_TICKS_DATA_ID", "GLOW_SQUID_DEFAULT_DARK_TICKS", "VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID", "VANILLA_GLOW_SQUID_DEFAULT_DARK_TICKS"] {',
            '                assert!(!source.contains(removed), "GlowSquid fixture restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_goat_metadata_uses_generated_accessors() {",
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let model_body = variants',
            '            .split("pub(super) fn goat_model_kind")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn goat_ramming_x_head_rot").next())',
            '            .expect("Goat model projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let debug_body = input',
            '            .split("fn debug_push_goat_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_dolphin_additional_save_data").next())',
            '            .expect("Goat debug projection source");',
            '        for source in [model_body, debug_body] {',
            '            assert!(source.contains("vanilla_entity_metadata_goat_accessors_for_entity_type"));',
            '            for removed in [',
            '                "GOAT_SCREAMING_DATA_ID",',
            '                "GOAT_LEFT_HORN_DATA_ID",',
            '                "GOAT_RIGHT_HORN_DATA_ID",',
            '                "GOAT_DEFAULT_SCREAMING",',
            '                "GOAT_DEFAULT_HAS_LEFT_HORN",',
            '                "GOAT_DEFAULT_HAS_RIGHT_HORN",',
            '                "VANILLA_GOAT_",',
            "            ] {",
            '                assert!(!source.contains(removed), "Goat projection restored {removed}");',
            "            }",
            "        }",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_goat_accessors_for_entity_type"));',
            '            for removed in ["GOAT_SCREAMING_DATA_ID", "GOAT_LEFT_HORN_DATA_ID", "GOAT_RIGHT_HORN_DATA_ID", "GOAT_DEFAULT_SCREAMING", "GOAT_DEFAULT_HAS_LEFT_HORN", "GOAT_DEFAULT_HAS_RIGHT_HORN", "VANILLA_GOAT_"] {',
            '                assert!(!source.contains(removed), "Goat fixture restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_turtle_metadata_uses_generated_accessors() {",
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let model_body = variants',
            '            .split("pub(super) fn turtle_has_egg")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn end_crystal_shows_bottom").next())',
            '            .expect("Turtle model projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let debug_body = input',
            '            .split("fn debug_push_turtle_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_axolotl_additional_save_data").next())',
            '            .expect("Turtle debug projection source");',
            '        for source in [model_body, debug_body] {',
            '            assert!(source.contains("vanilla_entity_metadata_turtle_accessors_for_entity_type"));',
            '            for removed in [',
            '                "TURTLE_HAS_EGG_DATA_ID",',
            '                "TURTLE_LAYING_EGG_DATA_ID",',
            '                "TURTLE_DEFAULT_HAS_EGG",',
            '                "TURTLE_DEFAULT_LAYING_EGG",',
            '                "VANILLA_TURTLE_",',
            "            ] {",
            '                assert!(!source.contains(removed), "Turtle projection restored {removed}");',
            "            }",
            "        }",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_turtle_accessors_for_entity_type"));',
            '            for removed in ["TURTLE_HAS_EGG_DATA_ID", "TURTLE_LAYING_EGG_DATA_ID", "TURTLE_DEFAULT_HAS_EGG", "TURTLE_DEFAULT_LAYING_EGG", "VANILLA_TURTLE_"] {',
            '                assert!(!source.contains(removed), "Turtle fixture restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_wither_skull_dangerous_uses_generated_accessor() {",
            '        let kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let model_body = kind',
            '            .split("VANILLA_ENTITY_TYPE_WITHER_SKULL_ID")',
            "            .nth(1)",
            '            .and_then(|source| source.split("_ => placeholder").next())',
            '            .expect("WitherSkull model projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let debug_body = input',
            '            .split("fn debug_push_wither_skull_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_shulker_bullet_additional_save_data").next())',
            '            .expect("WitherSkull debug projection source");',
            '        for source in [model_body, debug_body] {',
            '            assert!(source.contains("vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type"));',
            '            for removed in [',
            '                "WITHER_SKULL_DANGEROUS_DATA_ID",',
            '                "WITHER_SKULL_DEFAULT_DANGEROUS",',
            '                "VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID",',
            '                "VANILLA_WITHER_SKULL_DEFAULT_DANGEROUS",',
            "            ] {",
            '                assert!(!source.contains(removed), "WitherSkull projection restored {removed}");',
            "            }",
            "        }",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type"));',
            '            for removed in ["WITHER_SKULL_DANGEROUS_DATA_ID", "WITHER_SKULL_DEFAULT_DANGEROUS", "VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID", "VANILLA_WITHER_SKULL_DEFAULT_DANGEROUS"] {',
            '                assert!(!source.contains(removed), "WitherSkull fixture restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_block_attached_metadata_uses_generated_accessors() {",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        assert!(dimensions.contains("vanilla_entity_metadata_item_frame_accessors_for_entity_type"));',
            '        assert!(dimensions.contains("vanilla_entity_metadata_hanging_direction_accessor_for_entity_type"));',
            '        let block_attached = include_str!("../dimensions/block_attached.rs");',
            '        let block_attached = block_attached.split("#[cfg(test)]").next().unwrap();',
            '        assert!(block_attached.contains("VanillaItemFrameMetadataAccessors"));',
            '        assert!(block_attached.contains("VanillaHangingEntityMetadataAccessor"));',
            '        let store = include_str!("../store.rs");',
            '        let render_body = store',
            '            .split("pub(crate) fn item_frame_render_states")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(crate) fn held_item").next())',
            '            .expect("item-frame render-state source");',
            '        assert!(render_body.contains("vanilla_entity_metadata_item_frame_accessors_for_entity_type"));',
            '        assert!(!render_body.contains("vanilla_entity_metadata_type_is_item_frame"));',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let item_frame_debug_body = input',
            '            .split("fn debug_push_item_frame_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_hanging_facing_legacy_id").next())',
            '            .expect("item-frame debug source");',
            '        assert!(item_frame_debug_body.contains("vanilla_entity_metadata_item_frame_accessors_for_entity_type"));',
            '        let painting_debug_body = input',
            '            .split("fn debug_push_painting_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_painting_facing_legacy_2d_id").next())',
            '            .expect("painting debug source");',
            '        assert!(painting_debug_body.contains("vanilla_entity_metadata_hanging_direction_accessor_for_entity_type"));',
            '        let native_item_frames = include_str!("../../../../bbb-native/src/item_frames.rs");',
            '        assert!(native_item_frames.contains("vanilla_entity_metadata_item_frame_accessors_for_entity_type"));',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        assert!(native_input_tests.contains("fn item_frame_metadata_accessors("));',
            '        assert!(native_input_tests.contains("vanilla_entity_metadata_item_frame_accessors_for_entity_type"));',
            '        let world_tests = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        assert!(world_tests.contains("fn item_frame_item_data("));',
            '        assert!(world_tests.contains("vanilla_entity_metadata_item_frame_accessors_for_entity_type"));',
            '        for removed in [',
            '            "HANGING_DATA_DIRECTION_ID",',
            '            "HANGING_ENTITY_DIRECTION_DATA_ID",',
            '            "VANILLA_HANGING_ENTITY_DEFAULT_DIRECTION_ID",',
            '            "ITEM_FRAME_DATA_ITEM_ID",',
            '            "ITEM_FRAME_DATA_ROTATION_ID",',
            '            "ITEM_FRAME_ITEM_DATA_ID",',
            '            "ITEM_FRAME_ROTATION_DATA_ID",',
            '            "VANILLA_ITEM_FRAME_DEFAULT_ROTATION",',
            "        ] {",
            '            assert!(!block_attached.contains(removed), "block-attached projection restored {removed}");',
            '            assert!(!item_frame_debug_body.contains(removed), "item-frame debug restored {removed}");',
            '            assert!(!painting_debug_body.contains(removed), "painting debug restored {removed}");',
            '            assert!(!native_item_frames.contains(removed), "native ItemFrame fixture restored {removed}");',
            '            assert!(!native_input_tests.contains(removed), "native input ItemFrame fixture restored {removed}");',
            '            assert!(!world_tests.contains(removed), "world ItemFrame fixture restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_painting_variant_projection_uses_generated_accessor() {",
            '        let metadata = include_str!("../metadata.rs");',
            '        let apply_body = metadata',
            '            .split("pub fn apply_set_entity_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn painting_variant_projection_from_metadata").next())',
            '            .expect("entity metadata apply source");',
            '        assert!(apply_body.contains("self.entities.entity_type_id(id)"));',
            '        assert!(apply_body.contains("painting_variant_projection_from_metadata(type_id"));',
            '        let projection_body = metadata',
            '            .split("fn painting_variant_projection_from_metadata")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn sync_local_using_item_from_living_entity_flags").next())',
            '            .expect("painting variant projection source");',
            '        assert!(projection_body.contains("vanilla_entity_metadata_painting_variant_accessor_for_entity_type"));',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let debug_body = input',
            '            .split("fn debug_entity_data_painting_variant_named_reference")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_vanilla_direction_legacy_id").next())',
            '            .expect("painting variant debug source");',
            '        assert!(debug_body.contains("vanilla_entity_metadata_painting_variant_accessor_for_entity_type"));',
            '        let world_tests = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        assert!(world_tests.contains("fn painting_variant_data("));',
            '        assert!(world_tests.contains("fn painting_variant_collision_data("));',
            '        assert!(world_tests.contains("vanilla_entity_metadata_painting_variant_accessor_for_entity_type"));',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        assert!(native_input_tests.contains("fn painting_variant_data("));',
            '        assert!(native_input_tests.contains("vanilla_entity_metadata_painting_variant_accessor_for_entity_type"));',
            '        for removed in ["PAINTING_VARIANT_DATA_ID", "VANILLA_PAINTING_VARIANT_DATA_ID"] {',
            '            assert!(!projection_body.contains(removed), "world painting projection restored {removed}");',
            '            assert!(!debug_body.contains(removed), "painting debug restored {removed}");',
            '            assert!(!world_tests.contains(removed), "world painting fixture restored {removed}");',
            '            assert!(!native_input_tests.contains(removed), "native painting fixture restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn ominous_item_spawner_consumers_use_generated_accessor() {",
            '        let store = include_str!("../store.rs");',
            '        assert!(store.contains("vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type"));',
            '        let render_body = store',
            '            .split("pub(crate) fn ominous_item_spawner_item_states_at_partial_tick")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(crate) fn append_ominous_item_spawner_particle_states").next())',
            '            .expect("OminousItemSpawner item render projection source");',
            '        assert!(render_body.contains("ominous_item_spawner_render_stack("));',
            '        assert!(render_body.contains("identity.entity_type_id"));',
            '        assert!(!render_body.contains("VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID"));',
            '        let world_tests = include_str!("../tests/item_entity_and_pickup_projection.rs");',
            '        assert!(world_tests.contains("fn ominous_item_stack_entity_data("));',
            '        assert!(world_tests.contains("fn ominous_item_stack_collision_entity_data("));',
            '        assert!(world_tests.contains("vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type"));',
            '        let native_item_model_tests = include_str!("../../../../bbb-native/src/item_models/tests.rs");',
            '        assert!(native_item_model_tests.contains("vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type"));',
            '        for source in [store, world_tests, native_item_model_tests] {',
            '            assert!(!source.contains("OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID"));',
            '            assert!(!source.contains("OMINOUS_ITEM_SPAWNER_DATA_ITEM_ID"));',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn item_entity_consumers_use_generated_accessor() {",
            '        let store = include_str!("../store.rs");',
            '        assert!(store.contains("vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type"));',
            '        assert!(store.contains("vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer"));',
            '        let entities = include_str!("../../entities.rs");',
            '        assert!(entities.contains("vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type"));',
            '        let world_tests = include_str!("../tests.rs");',
            '        assert!(world_tests.contains("vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type"));',
            '        let play_apply = include_str!("../../play_apply.rs");',
            '        assert!(play_apply.contains("vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer"));',
            '        let native_item_model_tests = include_str!("../../../../bbb-native/src/item_models/tests.rs");',
            '        assert!(native_item_model_tests.contains("vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type"));',
            '        let native_runtime_tests = include_str!("../../../../bbb-native/src/runtime/events/tests.rs");',
            '        assert!(native_runtime_tests.contains("vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type"));',
            '        let net_probe_tests = include_str!("../../../../bbb-net/src/probe/play.rs");',
            '        assert!(net_probe_tests.contains("vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type"));',
            '        for source in [store, entities, world_tests, play_apply, native_item_model_tests, native_runtime_tests, net_probe_tests] {',
            '            for removed in ["VANILLA_ITEM_ENTITY_STACK_DATA_ID", "ITEM_ENTITY_STACK_DATA_ID", "ITEM_ENTITY_DATA_ITEM_STACK_ID"] {',
            '                assert!(!source.contains(removed), "ItemEntity projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn spellcaster_projection_consumers_use_generated_accessor() {",
            '        let source = include_str!("../store.rs");',
            '        let body = source',
            '            .split("let illager_spellcasting")',
            "            .nth(1)",
            '            .and_then(|source| source.split("let villager_unhappy").next())',
            '            .expect("spellcaster render-state projection source");',
            '        assert!(body.contains("vanilla_entity_renderer_extracts_casting_spell"));',
            '        assert!(body.contains("vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type"));',
            '        assert!(!body.contains("vanilla_entity_metadata_type_is_spellcaster_illager"));',
            '        assert!(!body.contains("VANILLA_SPELLCASTER_ILLAGER_CASTING_DATA_ID"));',
            '        assert!(!body.contains("VANILLA_SPELLCASTER_ILLAGER_DEFAULT_CASTING"));',
            '        let native_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let fixture = native_tests',
            '            .split("fn entity_model_instances_project_illager_spellcasting_flag_from_world")',
            '            .nth(1)',
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("spellcaster projection fixture source");',
            '        assert!(fixture.contains("spellcaster_casting_data_id"));',
            '        assert!(!fixture.contains("const VANILLA_SPELLCASTER_CASTING_DATA_ID"));',
            '        assert!(native_tests.contains("vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_villager_unhappy_projection_uses_generated_accessor() {",
            '        let source = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        let native_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let body = source',
            '            .split("let villager_unhappy")',
            "            .nth(1)",
            '            .and_then(|source| source.split("let enderman_accessors").next())',
            '            .expect("villager unhappy render-state projection source");',
            '        assert!(body.contains("vanilla_entity_renderer_extracts_unhappy_counter"));',
            '        assert!(body.contains("vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type"));',
            '        assert!(!body.contains("vanilla_entity_metadata_accessor_for_entity_type"));',
            '        assert!(!body.contains("VANILLA_ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID"));',
            '        assert!(!body.contains("VANILLA_ABSTRACT_VILLAGER_DEFAULT_UNHAPPY_COUNTER"));',
            '        for tests in [world_tests, native_tests] {',
            '            assert!(tests.contains("vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type"));',
            '            assert!(tests.contains("abstract_villager_unhappy_collision_data"));',
            '            assert!(!tests.contains("ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID"));',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_debug_save_data_does_not_restore_spellcaster_type_membership() {",
            '        let source = include_str!("../../../../bbb-native/src/input.rs");',
            '        assert!(!source.contains("VANILLA_ENTITY_TYPE_EVOKER_ID | VANILLA_ENTITY_TYPE_ILLUSIONER_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_baby_projection_does_not_restore_entity_type_membership() {",
            '        let schema = include_str!("../schema.rs");',
            '        let source = include_str!("../dimensions.rs");',
            '        let fixture = include_str!("../tests/specialized_pick_bounds_and_render_scale.rs");',
            '        let model_source_fixture = include_str!("../tests/vehicle_and_block_entity_projection.rs");',
            '        let animation_fixture = include_str!("../tests/small_mob_and_walk_animation.rs");',
            '        let creature_bounds_fixture = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        let visibility_fixture = include_str!("../tests/visibility_equipment_and_mount_projection.rs");',
            '        let world_inventory_fixture = include_str!("../../client/inventory/tests.rs");',
            '        let world_inventory = include_str!("../../client/inventory.rs");',
            '        let world_inventory_slot_ops = include_str!("../../client/inventory/slot_ops.rs");',
            '        let world_audio_fixture = include_str!("../../client/audio.rs");',
            '        let native_inventory_fixture = include_str!("../../../../bbb-native/src/input/inventory/tests.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let native_scene_baby_helper = native_scene_tests',
            '            .split("fn baby_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn llama_variant_data").next())',
            '            .expect("native scene baby helper source");',
            '        let native_scene_body_layer_fixtures = native_scene_tests',
            '            .split("fn entity_model_instances_project_nautilus_body_armor_render_state")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn entity_model_instances_project_armor_stand_flags_and_pose").next())',
            '            .expect("native scene body-layer baby fixture source");',
            '        let native_input_baby_helper = native_input_tests',
            '            .split("fn baby_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn sniffer_baby_data").next())',
            '            .expect("native input baby helper source");',
            '        let native_input_age_locked_helper = native_input_tests',
            '            .split("fn age_locked_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn mob_flags_data").next())',
            '            .expect("native input age-locked helper source");',
            '        let body = source',
            '            .split("fn vanilla_is_baby")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("baby projection source");',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            '        assert!(!source.contains("VANILLA_AGEABLE_MOB_BABY_DATA_ID"));',
            '        assert!(!source.contains("VANILLA_AGEABLE_MOB_DEFAULT_BABY"));',
            '        assert!(source.contains("goat_pick_bounds(entity_type_id, data_values)"));',
            '        assert!(source.contains("polar_bear_pick_bounds("));',
            '        assert_eq!(',
            '            source.matches("vanilla_is_baby(entity_type_id, data_values)").count(),',
            '            // Bounds branches plus the variant eye-height and baby',
            '            // passenger-attachment basis lookups.',
            '            9',
            '        );',
            '        assert!(fixture.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(model_source_fixture.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(animation_fixture.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(creature_bounds_fixture.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(visibility_fixture.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(world_inventory_fixture.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(world_inventory.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(world_inventory_slot_ops.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(world_inventory_slot_ops.contains("mount_entity_is_ageable_baby(entity_type_id"));',
            '        let world_audio_baby_helper = world_audio_fixture',
            '            .split("fn baby_entity_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn cat_sound_variant_entity_data").next())',
            '            .expect("world audio baby helper source");',
            '        assert!(world_audio_baby_helper.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(world_audio_baby_helper.contains("assert!(!default"));',
            '        let world_audio_builtin_fixture = world_audio_fixture',
            '            .split("fn built_in_animal_sound_variants_drive_adult_and_baby_consumed_sounds")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn synchronized_sound_variant_registry_uses_server_order_and_typed_payload").next())',
            '            .expect("world audio built-in baby fixture source");',
            '        assert_eq!(world_audio_builtin_fixture.matches("baby_entity_data(").count(), 1);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!world_audio_builtin_fixture.contains(removed), "world audio built-in baby fixture restored {removed}");',
            "        }",
            '        let world_audio_synchronized_fixture = world_audio_fixture',
            '            .split("fn synchronized_sound_variant_registry_uses_server_order_and_typed_payload")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn synchronized_sound_variant_stubs_use_resource_fallback_in_server_order").next())',
            '            .expect("world audio synchronized baby fixture source");',
            '        assert_eq!(world_audio_synchronized_fixture.matches("baby_entity_data(").count(), 1);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!world_audio_fixture.contains(removed), "world audio baby fixtures restored {removed}");',
            "        }",
            '        assert!(native_inventory_fixture.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        let native_scene_production_helper = native_variants',
            '            .split("pub(super) fn ageable_baby")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn armadillo_rolled_up").next())',
            '            .expect("native scene production baby helper source");',
            '        assert!(native_scene.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(native_scene_production_helper.contains("entity_type_id"));',
            '        assert!(native_scene_production_helper.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert_eq!(',
            '            native_kind.matches("ageable_baby(").count(),',
            '            native_kind',
            '                .matches("ageable_baby(entity_type_id, data_values)")',
            '                .count()',
            '        );',
            '        assert_eq!(',
            '            native_variants.matches("ageable_baby(").count(),',
            '            native_variants',
            '                .matches("ageable_baby(entity_type_id, values)")',
            '                .count()',
            '                + 1',
            '        );',
            '        assert!(!native_kind.contains("ageable_baby(data_values)"));',
            '        assert!(!native_variants.contains("ageable_baby(values)"));',
            '        assert!(native_scene_baby_helper.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(native_input_baby_helper.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(native_input_baby_helper.contains("assert!(!default"));',
            '        let native_input_debug_save = native_input',
            '            .split("fn debug_push_ageable_mob_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_animal_additional_save_data").next())',
            '            .expect("native input ageable debug-save source");',
            '        assert!(native_input_debug_save.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(native_input_debug_save.contains("entity.entity_type_id"));',
            '        let native_input_age_locked_debug_save = native_input',
            '            .split("fn debug_push_age_locked_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_animal_additional_save_data").next())',
            '            .expect("native input age-locked debug-save source");',
            '        assert!(schema.contains("pub fn vanilla_entity_metadata_age_locked_accessor_for_entity_type"));',
            '        assert!(schema.contains("vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, \\"AGE_LOCKED\\")"));',
            '        assert!(native_input_age_locked_helper.contains("entity_type_id(entity_id)"));',
            '        assert!(native_input_age_locked_helper.contains("vanilla_entity_metadata_age_locked_accessor_for_entity_type"));',
            '        assert!(native_input_age_locked_helper.contains("assert!(!default"));',
            '        assert!(native_input_age_locked_debug_save.contains("vanilla_entity_metadata_age_locked_accessor_for_entity_type"));',
            '        assert!(native_input_age_locked_debug_save.contains("entity.entity_type_id"));',
            '        assert_eq!(native_input_tests.matches("age_locked_data(&world").count(), 39);',
            '        for removed in ["AGEABLE_MOB_AGE_LOCKED_DATA_ID", "TADPOLE_AGE_LOCKED_DATA_ID"] {',
            '            assert!(!native_input.contains(removed), "native input age-locked production restored {removed}");',
            '            assert!(!native_input_tests.contains(removed), "native input age-locked fixtures restored {removed}");',
            "        }",
            '        assert_eq!(native_scene_body_layer_fixtures.matches("baby_data(").count(), 4);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_scene_body_layer_fixtures.contains(removed), "native scene body-layer baby fixtures restored {removed}");',
            "        }",
            '        for (start, end) in [',
            '            ("fn entity_model_instances_project_chicken_adult_and_baby_models", "fn entity_model_instances_project_sheep_eat_grass_head_pose"),',
            '            ("fn entity_model_instances_project_turtle_has_egg", "fn entity_model_instances_project_pillager_charging_crossbow"),',
            '            ("fn entity_model_instances_project_goat_ramming_head_tilt", "fn entity_model_instances_project_turtle_laying_egg"),',
            '            ("fn entity_model_instances_project_turtle_laying_egg", "fn entity_model_instances_project_end_crystal_shows_bottom"),',
            '        ] {',
            '            let fixture = native_scene_tests',
            '                .split(start)',
            "                .nth(1)",
            '                .and_then(|source| source.split(end).next())',
            '                .expect("native scene early baby fixture source");',
            '            assert_eq!(fixture.matches("baby_data(").count(), 1);',
            '            for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '                assert!(!fixture.contains(removed), "native scene early baby fixture restored {removed}");',
            "            }",
            "        }",
            '        let native_scene_farm_variant_fixtures = native_scene_tests',
            '            .split("fn entity_model_kind_uses_vanilla_chicken_variant_metadata")',
            "            .nth(1)",
            '            .and_then(|source| source.split("struct ReadyFarmAnimalPresentationLookup").next())',
            '            .expect("native scene farm-variant baby fixture source");',
            '        assert_eq!(native_scene_farm_variant_fixtures.matches("baby_data(").count(), 6);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_scene_farm_variant_fixtures.contains(removed), "native scene farm-variant baby fixtures restored {removed}");',
            "        }",
            '        let native_scene_base_animal_fixtures = native_scene_tests',
            '            .split("fn entity_model_kind_projects_bee_baby_from_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn entity_model_kind_projects_sheep_jeb_custom_name_and_age").next())',
            '            .expect("native scene base-animal baby fixture source");',
            '        assert_eq!(native_scene_base_animal_fixtures.matches("baby_data(").count(), 10);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_scene_base_animal_fixtures.contains(removed), "native scene base-animal baby fixtures restored {removed}");',
            "        }",
            '        let native_scene_specialized_animal_fixtures = native_scene_tests',
            '            .split("fn entity_model_kind_uses_exact_models_for_goats")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn entity_model_kind_maps_tadpole_to_real_model").next())',
            '            .expect("native scene specialized-animal baby fixture source");',
            '        assert_eq!(native_scene_specialized_animal_fixtures.matches("baby_data(").count(), 7);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_scene_specialized_animal_fixtures.contains(removed), "native scene specialized-animal baby fixtures restored {removed}");',
            "        }",
            '        let native_scene_bear_fixtures = native_scene_tests',
            '            .split("fn entity_model_kind_uses_exact_models_for_polar_bears")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn entity_model_kind_projects_panda_gene_variant_from_data").next())',
            '            .expect("native scene bear baby fixture source");',
            '        assert_eq!(native_scene_bear_fixtures.matches("baby_data(").count(), 2);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_scene_bear_fixtures.contains(removed), "native scene bear baby fixtures restored {removed}");',
            "        }",
            '        let native_scene_villager_fixtures = native_scene_tests',
            '            .split("fn entity_model_kind_uses_exact_models_for_villagers")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn villager_model_data_reads_vanilla_serializer_and_static_fallback_order").next())',
            '            .expect("native scene villager baby fixture source");',
            '        assert_eq!(native_scene_villager_fixtures.matches("baby_data(").count(), 2);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_scene_villager_fixtures.contains(removed), "native scene villager baby fixtures restored {removed}");',
            "        }",
            '        let native_scene_companion_animal_fixtures = native_scene_tests',
            '            .split("fn entity_model_kind_uses_exact_models_for_wolves")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn entity_model_kind_projects_wolf_variant_from_registry_and_fallback").next())',
            '            .expect("native scene companion-animal baby fixture source");',
            '        assert_eq!(native_scene_companion_animal_fixtures.matches("baby_data(").count(), 5);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_scene_companion_animal_fixtures.contains(removed), "native scene companion-animal baby fixtures restored {removed}");',
            "        }",
            '        let native_scene_cat_registry_fixture = native_scene_tests',
            '            .split("fn entity_model_instances_project_cat_variants_from_world_registry_order")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn entity_model_instances_project_feline_crouch_sprint_and_cat_sitting_from_world").next())',
            '            .expect("native scene cat-registry baby fixture source");',
            '        assert_eq!(native_scene_cat_registry_fixture.matches("baby_data(").count(), 1);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_scene_cat_registry_fixture.contains(removed), "native scene cat-registry baby fixture restored {removed}");',
            "        }",
            '        let native_scene_mount_fixtures = native_scene_tests',
            '            .split("fn entity_model_kind_uses_exact_models_for_horses")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn entity_model_kind_uses_exact_models_for_illagers_and_witch").next())',
            '            .expect("native scene mount baby fixture source");',
            '        assert_eq!(native_scene_mount_fixtures.matches("baby_data(").count(), 8);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_scene_mount_fixtures.contains(removed), "native scene mount baby fixtures restored {removed}");',
            "        }",
            '        for (start, end) in [',
            '            ("fn shift_f3_i_with_permission_copies_local_axolotl_save_nbt_to_clipboard", "fn shift_f3_i_with_permission_copies_local_nautilus_save_nbt_to_clipboard"),',
            '            ("fn shift_f3_i_with_permission_copies_local_nautilus_save_nbt_to_clipboard", "fn shift_f3_i_with_permission_copies_local_zombie_nautilus_save_nbt_to_clipboard"),',
            '            ("fn shift_f3_i_with_permission_copies_local_dolphin_save_nbt_to_clipboard", "fn shift_f3_i_with_permission_copies_local_frog_save_nbt_to_clipboard"),',
            '            ("fn shift_f3_i_with_permission_copies_local_frog_save_nbt_to_clipboard", "fn local_frog_save_uses_generated_default"),',
            '        ] {',
            '            let fixture = native_input_tests',
            '                .split(start)',
            "                .nth(1)",
            '                .and_then(|source| source.split(end).next())',
            '                .expect("native input aquatic baby fixture source");',
            '            assert_eq!(fixture.matches("baby_data(").count(), 1);',
            '            for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '                assert!(!fixture.contains(removed), "native input aquatic baby fixture restored {removed}");',
            "            }",
            "        }",
            '        let native_input_early_animal_fixtures = native_input_tests',
            '            .split("fn shift_f3_i_with_permission_copies_local_rabbit_save_nbt_to_clipboard")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_registry_resource_ids_use_the_value_local_session_identity").next())',
            '            .expect("native input early-animal baby fixture source");',
            '        assert_eq!(native_input_early_animal_fixtures.matches("baby_data(").count(), 4);',
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_input_early_animal_fixtures.contains(removed), "native input early-animal baby fixtures restored {removed}");',
            "        }",
            '        for (start, end) in [',
            '            ("fn shift_f3_i_with_permission_copies_local_chicken_save_nbt_to_clipboard", "fn local_chicken_save_uses_generated_resource_defaults"),',
            '            ("fn shift_f3_i_with_permission_copies_local_cow_save_nbt_to_clipboard", "fn local_cow_branch_save_uses_generated_defaults"),',
            '            ("fn shift_f3_i_with_permission_copies_local_mooshroom_save_nbt_to_clipboard", "fn shift_f3_i_with_permission_copies_local_goat_save_nbt_to_clipboard"),',
            '            ("fn shift_f3_i_with_permission_copies_local_goat_save_nbt_to_clipboard", "fn shift_f3_i_with_permission_copies_local_polar_bear_save_nbt_to_clipboard"),',
            '        ] {',
            '            let fixture = native_input_tests',
            '                .split(start)',
            "                .nth(1)",
            '                .and_then(|source| source.split(end).next())',
            '                .expect("native input farm baby fixture source");',
            '            assert_eq!(fixture.matches("baby_data(").count(), 1);',
            '            for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '                assert!(!fixture.contains(removed), "native input farm baby fixture restored {removed}");',
            "            }",
            "        }",
            '        for (start, end, expected_count) in [',
            '            ("fn shift_f3_i_with_permission_copies_local_polar_bear_save_nbt_to_clipboard", "fn shift_f3_i_with_permission_copies_local_allay_save_nbt_to_clipboard", 3),',
            '            ("fn shift_f3_i_with_permission_copies_local_strider_save_nbt_to_clipboard", "fn shift_f3_i_with_permission_copies_local_horse_save_nbt_to_clipboard", 2),',
            '        ] {',
            '            let fixtures = native_input_tests',
            '                .split(start)',
            "                .nth(1)",
            '                .and_then(|source| source.split(end).next())',
            '                .expect("native input specialized-animal baby fixture source");',
            '            assert_eq!(fixtures.matches("baby_data(").count(), expected_count);',
            '            for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '                assert!(!fixtures.contains(removed), "native input specialized-animal baby fixtures restored {removed}");',
            "            }",
            "        }",
            '        for (start, end) in [',
            '            ("fn shift_f3_i_with_permission_copies_local_squid_save_nbt_to_clipboard", "fn shift_f3_i_with_permission_copies_local_interaction_save_nbt_to_clipboard"),',
            '            ("fn shift_f3_i_with_permission_copies_local_wandering_trader_save_nbt_to_clipboard", "fn shift_f3_i_with_permission_copies_local_witch_save_nbt_to_clipboard"),',
            '        ] {',
            '            let fixture = native_input_tests',
            '                .split(start)',
            "                .nth(1)",
            '                .and_then(|source| source.split(end).next())',
            '                .expect("native input late baby fixture source");',
            '            assert_eq!(fixture.matches("baby_data(").count(), 1);',
            "        }",
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            assert!(!native_input_tests.contains(removed), "native input baby fixtures restored {removed}");',
            "        }",
            '        for removed in ["AGEABLE_BABY_DATA_ID", "AGEABLE_MOB_BABY_DATA_ID"] {',
            '            for fixture in [fixture, model_source_fixture, animation_fixture, creature_bounds_fixture, visibility_fixture, world_inventory_fixture, world_inventory, world_inventory_slot_ops, native_inventory_fixture, native_scene, native_kind, native_variants, native_input] {',
            '                assert!(!fixture.contains(removed), "baby fixture restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_age_scale_projection_stays_source_generated_and_distinct_from_scale() {",
            '        let schema = include_str!("../schema.rs");',
            '        let dimensions = include_str!("../dimensions.rs");',
            '        let entities = include_str!("../../entities.rs");',
            '        let store = include_str!("../store.rs");',
            '        let native_instance = include_str!("../../../../bbb-native/src/entity_scene/instance.rs");',
            '        let renderer_instances = include_str!("../../../../bbb-renderer/src/entity_models/instances.rs");',
            '        let age_scale_body = dimensions',
            '            .split("fn vanilla_age_scale")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("age-scale projection source");',
            '        assert!(schema.contains("vanilla_entity_metadata_baby_behavior_and_age_scale"));',
            '        assert!(age_scale_body.contains("vanilla_entity_metadata_baby_behavior_and_age_scale"));',
            '        assert!(age_scale_body.contains("vanilla_is_baby(entity_type_id, data_values)"));',
            '        assert!(!age_scale_body.contains("VANILLA_ENTITY_TYPE_"));',
            '        assert!(entities.contains("pub age_scale: f32"));',
            '        assert!(store.contains("age_scale: vanilla_age_scale(identity.entity_type_id, &metadata.data_values)"));',
            '        assert!(native_instance.contains("with_age_scale age_scale"));',
            '        assert!(renderer_instances.contains("(with_age_scale) age_scale: f32 = 1.0"));',
            '        for model in [',
            '            include_str!("../../../../bbb-renderer/src/entity_models/colored/mounts.rs"),',
            '            include_str!("../../../../bbb-renderer/src/entity_models/textured.rs"),',
            '            include_str!("../../../../bbb-renderer/src/entity_models/model_layers/feline.rs"),',
            '            include_str!("../../../../bbb-renderer/src/entity_models/model_layers/fox.rs"),',
            '            include_str!("../../../../bbb-renderer/src/entity_models/model_layers/piglin.rs"),',
            '            include_str!("../../../../bbb-renderer/src/entity_models/model_layers/player.rs"),',
            '            include_str!("../../../../bbb-renderer/src/entity_models/model_layers/polar_bear.rs"),',
            '            include_str!("../../../../bbb-renderer/src/entity_models/model_layers/sheep.rs"),',
            '            include_str!("../../../../bbb-renderer/src/entity_models/model_layers/skeleton.rs"),',
            '            include_str!("../../../../bbb-renderer/src/entity_models/model_layers/wolf.rs"),',
            '            include_str!("../../../../bbb-renderer/src/entity_models/model_layers/zombie.rs"),',
            "        ] {",
            '            assert!(model.contains("age_scale"), "setupAnim consumer dropped ageScale");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn saved_projectile_membership_consumers_do_not_restore_type_tables() {",
            '        let source = include_str!("../projectiles.rs");',
            '        let body = source',
            '            .split("fn is_vanilla_saved_projectile_type")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("saved projectile membership source");',
            '        assert!(body.contains("vanilla_entity_metadata_type_is_projectile"));',
            '        assert!(body.contains("vanilla_entity_type_is_serializable"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            '        assert!(!body.contains("matches!"));',
            '        let fixture = source',
            '            .split("fn saveable_projectile_membership_combines_class_and_builder_metadata")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("saved projectile membership fixture source");',
            '        assert!(fixture.contains("VANILLA_ENTITY_TYPES"));',
            '        assert!(fixture.contains("vanilla_entity_metadata_type_is_projectile"));',
            '        assert!(!fixture.contains("0..157"));',
            '        assert!(!fixture.contains("VANILLA_ENTITY_TYPE_"));',
            "    }",
            "",
            "    #[test]",
            "    fn owner_collision_consumers_do_not_restore_pickable_type_membership() {",
            '        let source = include_str!("../projectiles.rs");',
            '        let body = source',
            '            .split("fn entity_is_pickable_for_owner_collision")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n    }"))',
            "            .map(|(body, _)| body)",
            '            .expect("owner collision pickability source");',
            '        assert!(body.contains("vanilla_entity_metadata_type_is_pickable"));',
            '        assert!(body.contains("spectator_player_uuids"));',
            '        assert!(body.contains("vanilla_entity_metadata_armor_stand_accessors_for_entity_type"));',
            '        for removed in [',
            '            "vanilla_living_entity_type",',
            '            "is_vanilla_boat_type",',
            '            "is_vanilla_minecart_type",',
            '            "vanilla_entity_type_is_wind_charge",',
            '            "matches!",',
            '        ] {',
            '            assert!(!body.contains(removed), "restored {removed}");',
            "        }",
            '        let fixture = source',
            '            .split("fn owner_collision_pickability_uses_generated_class_and_tag_membership")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("owner collision pickability fixture source");',
            '        assert!(fixture.contains("VANILLA_ENTITY_TYPE_METADATA_26_1"));',
            '        assert!(fixture.contains("is_pickable_by_class"));',
            '        assert!(fixture.contains("redirectable_projectile"));',
            '        for removed in [',
            '            "VANILLA_ENTITY_TYPE_COW_ID",',
            '            "VANILLA_ENTITY_TYPE_OAK_BOAT_ID",',
            '            "VANILLA_ENTITY_TYPE_WIND_CHARGE_ID",',
            '            "VANILLA_ENTITY_TYPE_ARROW_ID",',
            '        ] {',
            '            assert!(!fixture.contains(removed), "fixture restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn collision_debug_collidability_consumers_do_not_restore_type_membership() {",
            '        let source = include_str!("../../client/local_player_collision.rs");',
            '        let body = source',
            '            .split("fn debug_entity_can_be_collided_with")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n    }"))',
            "            .map(|(body, _)| body)",
            '            .expect("collision debug collidability source");',
            '        assert!(body.contains("vanilla_entity_metadata_type_collidability_kind"));',
            '        for required in [',
            '            "living_entity_health",',
            '            "is_baby(target_id)",',
            '            "VANILLA_ENTITY_TYPE_PLAYER_ID",',
            '            "VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID",',
            '            "mount(target_id)",',
            '            "vanilla_entity_metadata_happy_ghast_accessors_for_entity_type",',
            '        ] {',
            '            assert!(body.contains(required), "missing instance gate {required}");',
            "        }",
            '        for removed in [',
            '            "is_vanilla_boat_type(target_type)",',
            '            "target_type == VANILLA_ENTITY_TYPE_SHULKER_ID",',
            '            "target_type != VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID",',
            '        ] {',
            '            assert!(!body.contains(removed), "restored {removed}");',
            "        }",
            '        let fixture = source',
            '            .split("fn collision_debug_collidability_matches_generated_class_dispatch")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("collision debug collidability fixture source");',
            '        assert!(fixture.contains("VANILLA_ENTITY_TYPES"));',
            '        assert!(fixture.contains("vanilla_entity_metadata_type_collidability_kind"));',
            '        assert!(fixture.contains("VanillaEntityCollidabilityKind::Always"));',
            '        assert!(fixture.contains("VanillaEntityCollidabilityKind::Alive"));',
            '        assert!(!fixture.contains("VANILLA_ENTITY_TYPE_"));',
            "    }",
            "",
            "    #[test]",
            "    fn collision_debug_pushability_consumers_do_not_restore_type_membership() {",
            '        let source = include_str!("../../client/local_player_collision.rs");',
            '        let body = source',
            '            .split("fn debug_entity_is_pushable")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n    }"))',
            "            .map(|(body, _)| body)",
            '            .expect("collision debug pushability source");',
            '        assert!(body.contains("vanilla_entity_metadata_type_pushability_kind"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            '        for required in [',
            '            "living_entity_health",',
            '            "debug_living_entity_on_climbable",',
            '            "mount(target_id)",',
            '            "pose(target_id)",',
            '            "vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type",',
            '        ] {',
            '            assert!(body.contains(required), "missing instance gate {required}");',
            "        }",
            '        for removed in [',
            '            "is_vanilla_abstract_horse_type",',
            '            "is_vanilla_boat_type",',
            '            "is_vanilla_minecart_type",',
            '            "vanilla_entity_type_is_living",',
            '        ] {',
            '            assert!(!body.contains(removed), "restored {removed}");',
            "        }",
            '        let fixture = source',
            '            .split("fn collision_debug_pushability_matches_generated_class_dispatch")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("collision debug pushability fixture source");',
            '        assert!(fixture.contains("VANILLA_ENTITY_TYPES"));',
            '        assert!(fixture.contains("vanilla_entity_metadata_type_pushability_kind"));',
            '        assert!(fixture.contains("VanillaEntityPushabilityKind::Never"));',
            '        assert!(!fixture.contains("VANILLA_ENTITY_TYPE_"));',
            "    }",
            "",
            "    #[test]",
            "    fn hoglin_animation_consumers_do_not_restore_entity_type_membership() {",
            '        let source = include_str!("../animations.rs");',
            '        assert_eq!(',
            '            source.matches("vanilla_entity_metadata_type_is_hoglin_base").count(),',
            '            2,',
            '            "import plus tick dispatch"',
            '        );',
            '        for removed in ["VANILLA_ENTITY_TYPE_HOGLIN_ID", "VANILLA_ENTITY_TYPE_ZOGLIN_ID"] {',
            '            assert!(!source.contains(removed), "production restored {removed}");',
            '        }',
            '        let tests = include_str!("../tests/transient_events_and_animation_triggers.rs");',
            '        let fixture = tests',
            '            .split("fn hoglin_attack_event_and_tick_match_generated_interface_graph")',
            '            .nth(1)',
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("hoglin event/tick fixture source");',
            '        assert!(fixture.contains("VANILLA_ENTITY_TYPES"));',
            '        assert!(fixture.contains("vanilla_entity_metadata_type_is_hoglin_base"));',
            '        for removed in ["VANILLA_ENTITY_TYPE_HOGLIN_ID", "VANILLA_ENTITY_TYPE_ZOGLIN_ID"] {',
            '            assert!(!fixture.contains(removed), "fixture restored {removed}");',
            '        }',
            "    }",
            "",
            "    #[test]",
            "    fn entity_event_owner_resolution_consumes_generated_super_dispatch() {",
            '        let source = include_str!("../schema.rs");',
            '        let body = source',
            '            .split("pub(crate) fn vanilla_entity_metadata_handle_entity_event_owner_for_event")',
            "            .nth(1)",
            '            .and_then(|source| source.split("\n}\n").next())',
            '            .expect("entity event owner resolution source");',
            '        for required in [',
            '            "VANILLA_ENTITY_EVENT_HANDLERS_26_1",',
            '            "direct_event_ids",',
            '            "delegated_event_ids",',
            '            "forwards_unhandled_to_super",',
            '            "super_handler_owner",',
            '        ] {',
            '            assert!(body.contains(required), "missing generated dispatch fact {required}");',
            "        }",
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            "    }",
            "",
            "    #[test]",
            "    fn entity_event_semantic_consumers_use_generated_declaration_inventory() {",
            '        let events = include_str!("../events.rs");',
            '        assert!(events.contains("VANILLA_ENTITY_EVENT_HANDLERS_26_1"));',
            '        assert!(events.contains("declaration_event_kind(handler.owner, *event_id)"));',
            '        assert!(events.contains("vanilla_entity_metadata_handle_entity_event_owner_for_event"));',
            '        let play_apply = include_str!("../../play_apply.rs")',
            '            .split("#[cfg(test)]")',
            '            .next()',
            '            .expect("production play apply source");',
            '        assert!(play_apply.contains("vanilla_entity_event_kind(entity_type_id, update.event_id)"));',
            '        assert!(play_apply.contains("VanillaEntityEventKind::"));',
            '        assert!(!play_apply.contains("update.event_id =="));',
            '        let updates = include_str!("../updates.rs");',
            '        assert!(updates.contains("vanilla_entity_event_kind(entity_type_id, packet.event_id)"));',
            '        assert!(updates.contains("VanillaEntityEventKind::LivingDeath"));',
            '        assert!(updates.contains("VanillaEntityEventKind::LivingSwapHandItems"));',
            '        let animation_body = include_str!("../animations.rs")',
            '            .split("pub(crate) fn handle_entity_event")',
            '            .nth(1)',
            '            .and_then(|source| source.split("    pub(crate) fn ").next())',
            '            .expect("entity event animation source");',
            '        assert!(animation_body.contains("event_kind: VanillaEntityEventKind"));',
            '        assert!(!animation_body.contains("event_id"));',
            "    }",
            "",
            "    #[test]",
            "    fn squid_entity_event_consumer_uses_generated_override_dispatch() {",
            '        let source = include_str!("../animations.rs");',
            '        let body = source',
            '            .split("pub(crate) fn handle_entity_event")',
            "            .nth(1)",
            '            .and_then(|source| source.split("    pub(crate) fn ").next())',
            '            .expect("entity event dispatch source");',
            '        assert!(body.contains("VanillaEntityEventKind::SquidResetMovement"));',
            '        assert!(!body.contains("vanilla_entity_metadata_type_uses_squid_entity_event_handler"));',
            '        for removed in ["VANILLA_ENTITY_TYPE_SQUID_ID", "VANILLA_ENTITY_TYPE_GLOW_SQUID_ID"] {',
            '            assert!(!body.contains(removed), "event dispatch restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn ocelot_trusting_consumer_uses_generated_override_dispatch() {",
            '        let source = include_str!("../../entities.rs");',
            '        let body = source',
            '            .split("pub fn ocelot_trusting_particle_state")',
            "            .nth(1)",
            '            .and_then(|source| source.split("    pub fn ").next())',
            '            .expect("ocelot trusting particle projection source");',
            '        assert!(body.contains("vanilla_entity_metadata_type_uses_ocelot_entity_event_handler"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_OCELOT_ID"));',
            '        let play_apply = include_str!("../../play_apply.rs");',
            '        assert!(play_apply.contains("VanillaEntityEventKind::OcelotTrusting { succeeded }"));',
            '        assert!(play_apply.contains("ocelot_trusting_particle_state(update.entity_id, succeeded)"));',
            "    }",
            "",
            "    #[test]",
            "    fn wolf_shake_events_use_generated_override_dispatch() {",
            '        let source = include_str!("../animations.rs");',
            '        let body = source',
            '            .split("pub(crate) fn handle_entity_event")',
            "            .nth(1)",
            '            .and_then(|source| source.split("    pub(crate) fn ").next())',
            '            .expect("entity event dispatch source");',
            '        assert!(body.contains("VanillaEntityEventKind::WolfStartShaking"));',
            '        assert!(body.contains("VanillaEntityEventKind::WolfCancelShaking"));',
            '        assert!(!body.contains("vanilla_entity_metadata_type_uses_wolf_entity_event_handler"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_WOLF_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn fishing_hook_pull_event_uses_generated_override_dispatch() {",
            '        let source = include_str!("../../client/effects.rs");',
            '        let body = source',
            '            .split("pub(crate) fn apply_fishing_hook_pull")',
            "            .nth(1)",
            '            .and_then(|source| source.split("    pub fn ").next())',
            '            .expect("fishing hook pull source");',
            '        assert!(body.contains("vanilla_entity_metadata_type_uses_fishing_hook_entity_event_handler"));',
            '        assert!(body.contains("VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID"));',
            '        let play_apply = include_str!("../../play_apply.rs");',
            '        assert!(play_apply.contains("VanillaEntityEventKind::FishingHookPull"));',
            '        assert!(play_apply.contains("apply_fishing_hook_pull(update.entity_id)"));',
            "    }",
            "",
            "    #[test]",
            "    fn player_events_use_generated_override_dispatch() {",
            '        let source = include_str!("../updates.rs");',
            '        let body = source',
            '            .split("pub fn apply_entity_event")',
            "            .nth(1)",
            '            .and_then(|source| source.split("    pub fn ").next())',
            '            .expect("entity event update source");',
            '        assert!(body.contains("vanilla_entity_metadata_type_uses_player_entity_event_handler"));',
            '        assert!(body.contains("VanillaEntityEventKind::PlayerFinishUsingItem"));',
            '        assert!(body.contains("VanillaEntityEventKind::PlayerEnableReducedDebug"));',
            '        assert!(body.contains("VanillaEntityEventKind::PlayerDisableReducedDebug"));',
            '        assert!(body.contains("take_local_using_item"));',
            '        assert!(body.contains("set_local_player_reduced_debug_info_from_entity_event"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_PLAYER_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn mob_spawn_event_consumer_uses_generated_class_membership() {",
            '        let source = include_str!("../../entities.rs");',
            '        let body = source',
            '            .split("pub fn mob_spawn_particle_state")',
            "            .nth(1)",
            '            .and_then(|source| source.split("    pub fn ").next())',
            '            .expect("mob spawn particle projection source");',
            '        assert!(body.contains("vanilla_entity_metadata_type_is_mob"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            '        let play_apply = include_str!("../../play_apply.rs");',
            '        assert!(play_apply.contains("VanillaEntityEventKind::MobSpawn"));',
            '        assert!(play_apply.contains("mob_spawn_particle_state(update.entity_id)"));',
            "    }",
            "",
            "    #[test]",
            "    fn living_swap_hand_event_uses_generated_class_membership() {",
            '        let source = include_str!("../metadata.rs");',
            '        let body = source',
            '            .split("pub(crate) fn apply_living_swap_hand_items_event")',
            "            .nth(1)",
            '            .and_then(|source| source.split("    pub fn ").next())',
            '            .expect("living swap-hand event source");',
            '        assert!(body.contains("vanilla_entity_metadata_type_is_living"));',
            '        assert!(body.contains("ProtocolEquipmentSlot::MainHand"));',
            '        assert!(body.contains("ProtocolEquipmentSlot::OffHand"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            '        let updates = include_str!("../updates.rs");',
            '        assert!(updates.contains("VanillaEntityEventKind::LivingSwapHandItems"));',
            '        assert!(updates.contains("apply_living_swap_hand_items_event(packet.entity_id)"));',
            "    }",
            "",
            "    #[test]",
            "    fn living_item_break_events_use_generated_class_membership() {",
            '        let source = include_str!("../../entities.rs");',
            '        let body = source',
            '            .split("pub fn living_entity_item_break_state")',
            "            .nth(1)",
            '            .and_then(|source| source.split("    pub fn ").next())',
            '            .expect("living item-break projection source");',
            '        assert!(body.contains("vanilla_living_entity_type"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            '        let play_apply = include_str!("../../play_apply.rs");',
            '        assert!(play_apply.contains("VanillaEntityEventKind::LivingItemBreak(slot)"));',
            '        assert!(play_apply.contains("living_entity_item_break_state(update.entity_id, slot)"));',
            '        assert!(!play_apply.contains("fn living_entity_item_break_slot"));',
            "    }",
            "",
            "    #[test]",
            "    fn illager_celebrating_consumers_do_not_restore_parallel_metadata_or_types() {",
            '        let source = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_instance = include_str!("../../../../bbb-native/src/entity_scene/instance.rs");',
            '        let body = source',
            '            .split("fn illager_celebrating")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("illager celebrating projection source");',
            '        assert!(body.contains("vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type"));',
            '        assert!(body.contains("vanilla_entity_type_illager_arm_pose_uses_celebrating"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_"));',
            '        assert!(native_instance.contains(".with_illager_celebrating(illager_celebrating("));',
            '        let tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let fixture = tests',
            '            .split("fn illager_celebrating_matches_generated_accessor_and_arm_pose_catalogs")',
            '            .nth(1)',
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("illager celebrating fixture source");',
            '        assert!(fixture.contains("VANILLA_ENTITY_TYPES"));',
            '        assert!(fixture.contains("vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type"));',
            '        assert!(fixture.contains("vanilla_entity_type_illager_arm_pose_uses_celebrating"));',
            '        assert!(!fixture.contains("VANILLA_ENTITY_TYPE_"));',
            '        for removed in ["RAIDER_IS_CELEBRATING_DATA_ID", "RAIDER_DEFAULT_CELEBRATING"] {',
            '            assert!(!body.contains(removed), "celebrating projection restored {removed}");',
            '            assert!(!native_scene.contains(removed), "native scene restored {removed}");',
            '            assert!(!native_instance.contains(removed), "native instance restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_camel_animation_does_not_restore_entity_type_membership() {",
            '        let source = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let body = source',
            '            .split("fn camel_sit_state")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("camel sit projection source");',
            '        assert!(body.contains("vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_CAMEL_ID"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID"));',
            '        assert!(!body.contains("CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID"));',
            '        assert!(!body.contains("CAMEL_DEFAULT_LAST_POSE_CHANGE_TICK"));',
            '        let entity_source = include_str!("../../entities.rs");',
            '        let removed_lookup = ["fn is_vanilla_", "camel_type("].concat();',
            '        assert!(!entity_source.contains(&removed_lookup));',
            '        let animations = include_str!("../animations.rs");',
            '        let walk_body = animations',
            '            .split("fn walk_update_target_speed")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(crate) fn wither_side_head_target_ids").next())',
            '            .expect("Camel walk update source");',
            '        assert!(walk_body.contains("vanilla_entity_metadata_type_is_camel"));',
            '        assert!(!walk_body.contains("VANILLA_ENTITY_TYPE_CAMEL_ID"));',
            '        assert!(!walk_body.contains("VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_villager_model_data_does_not_restore_accessor_id_switch() {",
            '        let source = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let body = source',
            '            .split("fn villager_model_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("villager model-data projection source");',
            '        assert!(body.contains("vanilla_entity_metadata_villager_data_accessor_for_entity_type"));',
            '        assert!(!body.contains("VILLAGER_DATA_DATA_ID"));',
            '        assert!(!body.contains("ZOMBIE_VILLAGER_DATA_DATA_ID"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_VILLAGER_ID"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID"));',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        for source in [native_scene, native_scene_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_villager_data_accessor_for_entity_type"));',
            '            assert!(!source.contains("VILLAGER_DATA_DATA_ID"));',
            '            assert!(!source.contains("ZOMBIE_VILLAGER_DATA_DATA_ID"));',
            "        }",
            '        assert!(native_scene_tests.contains("vanilla_entity_metadata_accessor_for_entity_type"));',
            '        assert!(native_scene_tests.contains("DATA_UNHAPPY_COUNTER"));',
            '        for removed in ["protocol_villager_data(18", "protocol_villager_data_at(18"] {',
            '            assert!(!native_scene_tests.contains(removed), "villager fixture restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_skeleton_shaking_does_not_restore_entity_type_membership() {",
            '        let source = include_str!("../../../../bbb-native/src/entity_scene/instance.rs");',
            '        let body = source',
            '            .split("fn entity_shaking")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("entity shaking projection source");',
            '        assert!(body.contains("vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_SKELETON_ID"));',
            '        assert!(!body.contains("SKELETON_STRAY_CONVERSION_DATA_ID"));',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        assert!(!native_scene.contains("SKELETON_STRAY_CONVERSION_DATA_ID"));',
            '        assert!(native_scene_tests.contains("vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type"));',
            '        assert!(!native_scene_tests.contains("SKELETON_STRAY_CONVERSION_DATA_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_strider_shaking_does_not_restore_entity_type_membership() {",
            '        let source = include_str!("../../../../bbb-native/src/entity_scene/instance.rs");',
            '        let body = source',
            '            .split("fn entity_shaking")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("entity shaking projection source");',
            '        let kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let model_body = kind',
            '            .split("VANILLA_ENTITY_TYPE_STRIDER_ID")',
            "            .nth(1)",
            '            .and_then(|source| source.split("VANILLA_ENTITY_TYPE_TADPOLE_ID").next())',
            '            .expect("Strider model projection source");',
            '        assert!(body.contains("vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type"));',
            '        assert!(model_body.contains("vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_STRIDER_ID"));',
            '        assert!(!body.contains("STRIDER_SUFFOCATING_DATA_ID"));',
            '        for removed in ["STRIDER_SUFFOCATING_DATA_ID", "STRIDER_DEFAULT_SUFFOCATING", "VANILLA_STRIDER_"] {',
            '            assert!(!model_body.contains(removed), "Strider model projection restored {removed}");',
            "        }",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_scene_tests, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type"));',
            '            assert!(!source.contains("STRIDER_SUFFOCATING_DATA_ID"));',
            "        }",
            '        assert!(native_input_tests.contains("vanilla_entity_metadata_strider_boost_time_accessor_for_entity_type"));',
            '        for removed in ["STRIDER_BOOST_TIME_DATA_ID", "STRIDER_DEFAULT_BOOST_TIME"] {',
            '            assert!(!native_input_tests.contains(removed), "Strider boost fixture restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn avatar_fixtures_do_not_restore_fixed_accessor_arm_or_model_part_ids() {",
            '        let combat_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        for required in ["VANILLA_AVATAR_MAIN_HAND_DATA_ID", "VANILLA_HUMANOID_ARM_LEFT_ID", "VANILLA_HUMANOID_ARM_RIGHT_ID"] {',
            '            assert!(combat_tests.contains(required), "Avatar main-arm fixture missing {required}");',
            "        }",
            '        for removed in ["const VANILLA_AVATAR_MAIN_HAND_DATA_ID", "const VANILLA_HUMANOID_ARM_LEFT_ID", "const VANILLA_HUMANOID_ARM_RIGHT_ID"] {',
            '            assert!(!combat_tests.contains(removed), "Avatar main-arm fixture restored {removed}");',
            "        }",
            '        let player_pose_tests = include_str!("../tests/player_pose_and_sleeping_projection.rs");',
            '        assert!(player_pose_tests.contains("VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID"));',
            '        assert!(player_pose_tests.contains("VANILLA_AVATAR_MODEL_PART_CAPE"));',
            '        for removed in ["const VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID", "const VANILLA_AVATAR_CAPE_PART_MASK"] {',
            '            assert!(!player_pose_tests.contains(removed), "Avatar model-parts fixture restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn local_player_metadata_consumers_do_not_restore_fixed_accessor_ids() {",
            '        let local_player = include_str!("../../client/local_player.rs");',
            '        assert!(local_player.contains("vanilla_entity_metadata_entity_accessors"));',
            '        assert!(local_player.contains("vanilla_entity_metadata_living_entity_accessors_for_entity_type"));',
            '        assert!(local_player.contains("vanilla_entity_metadata_player_absorption_accessor_for_entity_type"));',
            '        for removed in ["const VANILLA_ENTITY_AIR_SUPPLY_DATA_ID", "const VANILLA_ENTITY_TICKS_FROZEN_DATA_ID", "const VANILLA_LIVING_ENTITY_HEALTH_DATA_ID", "const VANILLA_PLAYER_ABSORPTION_DATA_ID"] {',
            '            assert!(!local_player.contains(removed), "local-player fixture restored {removed}");',
            "        }",
            '        let store = include_str!("../store.rs");',
            '        assert!(store.contains("vanilla_entity_metadata_player_absorption_accessor_for_entity_type"));',
            '        for removed in ["VANILLA_PLAYER_ABSORPTION_DATA_ID", "VANILLA_PLAYER_DEFAULT_ABSORPTION"] {',
            '            assert!(!store.contains(removed), "Player absorption projection restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn player_shoulder_parrot_fixtures_do_not_restore_fixed_accessor_ids() {",
            '        let world_tests = include_str!("../tests/player_pose_and_sleeping_projection.rs");',
            '        let native_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            "        for source in [world_tests, native_tests] {",
            '            assert!(source.contains("VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID"));',
            '            assert!(source.contains("VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID"));',
            '            for removed in ["const PLAYER_SHOULDER_PARROT_LEFT_DATA_ID", "const PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID"] {',
            '                assert!(!source.contains(removed), "Player shoulder-parrot fixture restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn native_witch_debug_fixture_does_not_restore_fixed_using_item_metadata() {",
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        assert!(native_input_tests.contains("vanilla_entity_metadata_witch_using_item_accessor_for_entity_type"));',
            '        for removed in ["WITCH_USING_ITEM_DATA_ID", "WITCH_DEFAULT_USING_ITEM"] {',
            '            assert!(!native_input_tests.contains(removed), "Witch debug fixture restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_zombie_villager_shaking_does_not_restore_entity_type_membership() {",
            '        let source = include_str!("../../../../bbb-native/src/entity_scene/instance.rs");',
            '        let body = source',
            '            .split("fn entity_shaking")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("entity shaking projection source");',
            '        assert!(body.contains("vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID"));',
            '        assert!(!body.contains("ZOMBIE_VILLAGER_CONVERTING_DATA_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_zombie_family_does_not_restore_accessor_constants() {",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_instance = include_str!("../../../../bbb-native/src/entity_scene/instance.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        assert!(native_scene.contains("vanilla_entity_metadata_zombie_accessors_for_entity_type"));',
            '        assert_eq!(native_kind.matches("zombie_baby(entity_type_id, data_values)").count(), 5);',
            '        assert!(native_variants.contains("vanilla_entity_metadata_zombie_accessors_for_entity_type"));',
            '        assert!(native_instance.contains("vanilla_entity_metadata_zombie_accessors_for_entity_type"));',
            '        assert!(native_input.contains("vanilla_entity_metadata_zombie_accessors_for_entity_type"));',
            '        for removed in ["ZOMBIE_BABY_DATA_ID", "ZOMBIE_DEFAULT_BABY", "ZOMBIE_DROWNED_CONVERSION_DATA_ID", "ZOMBIE_DEFAULT_DROWNED_CONVERSION"] {',
            '            assert!(!native_scene.contains(removed), "native scene restored {removed}");',
            '            assert!(!native_kind.contains(removed), "native kind restored {removed}");',
            '            assert!(!native_variants.contains(removed), "native variants restored {removed}");',
            '            assert!(!native_instance.contains(removed), "native instance restored {removed}");',
            '            assert!(!native_input.contains(removed), "native input restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_zombification_shaking_does_not_restore_entity_type_dispatch() {",
            '        let source = include_str!("../../../../bbb-native/src/entity_scene/instance.rs");',
            '        let body = source',
            '            .split("fn entity_shaking")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("entity shaking projection source");',
            '        assert!(body.contains("vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type"));',
            '        assert!(!body.contains("vanilla_entity_type_is_abstract_piglin"));',
            '        assert!(!body.contains("VANILLA_ENTITY_TYPE_HOGLIN_ID"));',
            '        assert!(!body.contains("PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID"));',
            '        assert!(!body.contains("HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID"));',
            '        assert!(!body.contains("match entity_type_id"));',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let hoglin_debug = native_input',
            '            .split("fn debug_push_hoglin_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_abstract_piglin_additional_save_data").next())',
            '            .expect("Hoglin debug projection source");',
            '        for removed in ["HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID", "HOGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION"] {',
            '            assert!(!native_scene.contains(removed), "zombification fixture restored {removed}");',
            "        }",
            '        for source in [native_scene_tests, hoglin_debug, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type"));',
            '            for removed in ["HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID", "HOGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION"] {',
            '                assert!(!source.contains(removed), "zombification fixture restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_wither_projection_does_not_restore_type_or_accessor_constants() {",
            '        let source = include_str!("../store.rs");',
            '        assert!(source.contains("vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type"));',
            '        assert!(source.contains("wither_side_head_target_ids(identity.entity_type_id, &metadata.data_values)"));',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let debug_body = native_input',
            '            .split("fn debug_push_wither_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_wandering_trader_additional_save_data").next())',
            '            .expect("Wither debug projection source");',
            '        assert!(debug_body.contains("vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type"));',
            '        for removed in [',
            '            "vanilla_is_wither",',
            '            "WITHER_INVULNERABLE_TICKS_DATA_ID",',
            '            "WITHER_DEFAULT_INVULNERABLE_TICKS",',
            '        ] {',
            '            assert!(!source.contains(removed), "restored {removed}");',
            '            assert!(!debug_body.contains(removed), "native debug restored {removed}");',
            "        }",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        assert!(!dimensions.contains("fn vanilla_is_wither"));',
            '        let animations = include_str!("../animations.rs");',
            '        assert!(animations.contains("vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type"));',
            '        let world_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        assert!(world_tests.contains("fn wither_invulnerability_data("));',
            '        assert!(world_tests.contains("vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type"));',
            '        assert!(world_tests.contains("fn wither_side_head_target_data("));',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        assert!(native_scene_tests.contains("fn protocol_wither_target_b_data("));',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        assert!(native_input_tests.contains("fn wither_invulnerability_data("));',
            '        assert!(native_input_tests.contains("vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type"));',
            '        for source in [world_tests, native_input, native_input_tests] {',
            '            for removed in ["WITHER_INVULNERABLE_TICKS_DATA_ID", "WITHER_INV_DATA_ID"] {',
            '                assert!(!source.contains(removed), "Wither fixture restored {removed}");',
            "            }",
            "        }",
            '        assert!(native_scene_tests.contains("vanilla_entity_metadata_accessor_for_entity_type"));',
            '        for removed in ["WITHER_TARGET_B_DATA_ID", "WITHER_TARGET_C_DATA_ID", "WITHER_DEFAULT_TARGET_B", "WITHER_DEFAULT_TARGET_C"] {',
            '            assert!(!animations.contains(removed), "animations restored {removed}");',
            '            assert!(!world_tests.contains(removed), "world fixtures restored {removed}");',
            '            assert!(!native_scene_tests.contains(removed), "native scene fixtures restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_enderman_projection_does_not_restore_type_or_accessor_constants() {",
            '        let source = include_str!("../store.rs");',
            '        assert!(source.contains("vanilla_entity_metadata_enderman_render_accessors_for_entity_type"));',
            '        let world_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_item_model_tests = include_str!("../../../../bbb-native/src/item_models/tests.rs");',
            '        for fixture in [world_tests, native_scene_tests, native_item_model_tests] {',
            '            assert!(fixture.contains("vanilla_entity_metadata_enderman_render_accessors_for_entity_type"));',
            "        }",
            '        for removed in [',
            '            "vanilla_is_enderman",',
            '            "ENDERMAN_CARRY_STATE_DATA_ID",',
            '            "ENDERMAN_CREEPY_DATA_ID",',
            '            "ENDERMAN_DEFAULT_CARRY_STATE",',
            '            "ENDERMAN_DEFAULT_CREEPY",',
            '            "CARRY_STATE_DATA_ID",',
            '            "CREEPY_DATA_ID",',
            '        ] {',
            '            assert!(!source.contains(removed), "restored {removed}");',
            '            for fixture in [world_tests, native_scene_tests, native_item_model_tests] {',
            '                assert!(!fixture.contains(removed), "EnderMan fixture restored {removed}");',
            "            }",
            "        }",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        assert!(!dimensions.contains("fn vanilla_is_enderman"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_bat_resting_does_not_restore_type_or_accessor_constants() {",
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [store, world_tests, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_bat_flags_accessor_for_entity_type"));',
            "        }",
            '        for removed in [',
            '            "vanilla_is_bat",',
            '            "BAT_FLAGS_DATA_ID",',
            '            "BAT_DEFAULT_FLAGS",',
            '            "BAT_FLAG_RESTING",',
            '        ] {',
            '            for source in [store, world_tests, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Bat consumer restored {removed}");',
            "            }",
            "        }",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        assert!(!dimensions.contains("fn vanilla_is_bat"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_bee_accessors_do_not_restore_fixed_metadata_facts() {",
            '        let store = include_str!("../store.rs");',
            '        let animations = include_str!("../animations.rs");',
            '        let world_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        let animation_tests = include_str!("../tests/small_mob_and_walk_animation.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [',
            '            store, animations, world_tests, animation_tests, native_scene, native_variants,',
            '            native_scene_tests, native_input, native_input_tests,',
            "        ] {",
            '            assert!(source.contains("vanilla_entity_metadata_bee_accessors_for_entity_type"));',
            "        }",
            '        assert!(!store.contains("vanilla_is_bee"));',
            '        for source in [',
            '            store, animations, world_tests, animation_tests, native_scene, native_variants,',
            '            native_scene_tests, native_input, native_input_tests,',
            "        ] {",
            '            for removed in [',
            '                "vanilla_entity_metadata_bee_flags_accessor_for_entity_type",',
            '                "BEE_FLAGS_DATA_ID",',
            '                "BEE_DEFAULT_FLAGS",',
            '                "BEE_FLAG_ROLL",',
            '                "BEE_FLAG_HAS_STUNG",',
            '                "BEE_FLAG_HAS_NECTAR",',
            '                "BEE_ANGER_END_TIME_DATA_ID",',
            '                "BEE_DEFAULT_ANGER_END_TIME",',
            "            ] {",
            '                assert!(!source.contains(removed), "Bee projection restored {removed}");',
            "            }",
            "        }",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        assert!(!dimensions.contains("fn vanilla_is_bee"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_dolphin_debug_does_not_restore_fixed_metadata_facts() {",
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_dolphin_accessors_for_entity_type"));',
            '            for removed in [',
            '                "DOLPHIN_GOT_FISH_DATA_ID",',
            '                "DOLPHIN_DEFAULT_GOT_FISH",',
            '                "DOLPHIN_MOISTNESS_DATA_ID",',
            '                "DOLPHIN_DEFAULT_MOISTNESS",',
            "            ] {",
            '                assert!(!source.contains(removed), "Dolphin projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_fox_bundle_does_not_restore_fixed_metadata_facts() {",
            '        let store = include_str!("../store.rs");',
            '        let animations = include_str!("../animations.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let animation_tests = include_str!("../tests/small_mob_and_walk_animation.rs");',
            '        for source in [store, animations, native_scene, native_variants, native_scene_tests, native_input, native_input_tests, animation_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_fox_accessors_for_entity_type"));',
            "        }",
            '        assert!(!store.contains("vanilla_is_fox"));',
            '        for removed in [',
            '            "vanilla_entity_metadata_fox_flags_accessor_for_entity_type",',
            '            "vanilla_entity_metadata_fox_variant_accessor_for_entity_type",',
            '            "FOX_FLAGS_DATA_ID",',
            '            "FOX_DEFAULT_FLAGS",',
            '            "FOX_FLAG_",',
            '            "FOX_TYPE_DATA_ID",',
            '            "FOX_DEFAULT_TYPE",',
            '            "FOX_DEFAULT_VARIANT",',
            '            "FOX_TRUSTED_DATA_ID_",',
            "        ] {",
            '            for source in [store, animations, native_scene, native_variants, native_scene_tests, native_input, native_input_tests, animation_tests] {',
            '                assert!(!source.contains(removed), "Fox projection restored {removed}");',
            "            }",
            "        }",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        assert!(!dimensions.contains("fn vanilla_is_fox"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_vex_charging_does_not_restore_type_or_accessor_constants() {",
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        for source in [store, world_tests, native_scene, native_kind, native_scene_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_vex_flags_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["vanilla_is_vex", "VEX_FLAGS_DATA_ID", "VEX_DEFAULT_FLAGS", "VEX_FLAG_IS_CHARGING"] {',
            '            for source in [store, world_tests, native_scene, native_kind, native_scene_tests] {',
            '                assert!(!source.contains(removed), "Vex consumer restored {removed}");',
            "            }",
            "        }",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        assert!(!dimensions.contains("fn vanilla_is_vex"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_panda_bundle_does_not_restore_fixed_metadata_facts() {",
            '        let animations = include_str!("../animations.rs");',
            '        let world_tests = include_str!("../tests/small_mob_and_walk_animation.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [animations, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_panda_accessors_for_entity_type"));',
            "        }",
            '        for removed in [',
            '                "vanilla_entity_metadata_panda_flags_accessor_for_entity_type",',
            '                "PANDA_UNHAPPY_COUNTER_DATA_ID",',
            '                "PANDA_SNEEZE_COUNTER_DATA_ID",',
            '                "PANDA_EAT_COUNTER_DATA_ID",',
            '                "PANDA_MAIN_GENE_DATA_ID",',
            '                "PANDA_HIDDEN_GENE_DATA_ID",',
            '                "PANDA_DEFAULT_UNHAPPY_COUNTER",',
            '                "PANDA_DEFAULT_SNEEZE_COUNTER",',
            '                "PANDA_DEFAULT_EAT_COUNTER",',
            '                "PANDA_DEFAULT_MAIN_GENE",',
            '                "PANDA_DEFAULT_HIDDEN_GENE",',
            '                "PANDA_FLAGS_DATA_ID",',
            '                "PANDA_DEFAULT_FLAGS",',
            '                "PANDA_FLAG_",',
            '                "PANDA_SNEEZING_FLAG",',
            '                "PANDA_ROLLING_FLAG",',
            '                "PANDA_SITTING_FLAG",',
            '                "PANDA_ON_BACK_FLAG",',
            "        ] {",
            '            for source in [animations, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Panda projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_abstract_horse_bundle_does_not_restore_fixed_metadata_facts() {",
            '        let animations = include_str!("../animations.rs");',
            '        let world_tests = include_str!("../tests/visibility_equipment_and_mount_projection.rs");',
            '        let inventory = include_str!("../../client/inventory.rs");',
            '        let slot_ops = include_str!("../../client/inventory/slot_ops.rs");',
            '        let inventory_tests = include_str!("../../client/inventory/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let native_inventory_tests = include_str!("../../../../bbb-native/src/input/inventory/tests.rs");',
            '        for source in [animations, world_tests, inventory, slot_ops, inventory_tests, native_input, native_input_tests, native_inventory_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_abstract_horse_accessors_for_entity_type"));',
            "        }",
            '        for removed in [',
            '            "vanilla_entity_metadata_abstract_horse_flags_accessor_for_entity_type",',
            '            "ABSTRACT_HORSE_FLAGS_DATA_ID",',
            '            "ABSTRACT_HORSE_DEFAULT_FLAGS",',
            '            "ABSTRACT_HORSE_FLAG_",',
            '            "ABSTRACT_HORSE_TAME_FLAG",',
            "        ] {",
            '            for source in [animations, world_tests, inventory, slot_ops, inventory_tests, native_input, native_input_tests, native_inventory_tests] {',
            '                assert!(!source.contains(removed), "AbstractHorse projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_equine_accessors_do_not_restore_fixed_metadata_facts() {",
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let horse_body = variants',
            '            .split("pub(super) fn horse_color_variant")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn llama_model_kind").next())',
            '            .expect("Horse model projection source");',
            '        assert!(horse_body.contains("vanilla_entity_metadata_horse_variant_accessor_for_entity_type"));',
            '        for removed in ["HORSE_VARIANT_DATA_ID", "HORSE_DEFAULT_VARIANT"] {',
            '            assert!(!horse_body.contains(removed), "Horse projection restored {removed}");',
            "        }",
            '        let donkey_body = variants',
            '            .split("pub(super) fn donkey_model_kind")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn undead_horse_model_kind").next())',
            '            .expect("Donkey model projection source");',
            '        let llama_body = variants',
            '            .split("pub(super) fn llama_model_kind")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn goat_model_kind").next())',
            '            .expect("Llama model projection source");',
            '        let chested_helper = variants',
            '            .split("pub(super) fn chested_horse_has_chest")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn zombie_baby").next())',
            '            .expect("chested horse metadata projection source");',
            '        assert!(donkey_body.contains("chested_horse_has_chest(entity_type_id, values)"));',
            '        assert!(llama_body.contains("chested_horse_has_chest(entity_type_id, values)"));',
            '        assert!(chested_helper.contains("vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type"));',
            '        assert!(llama_body.contains("vanilla_entity_metadata_llama_accessors_for_entity_type"));',
            '        for removed in [',
            '            "ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID",',
            '            "ABSTRACT_CHESTED_HORSE_DEFAULT_CHEST",',
            '            "LLAMA_VARIANT_DATA_ID",',
            '            "LLAMA_DEFAULT_VARIANT",',
            "        ] {",
            '            assert!(!donkey_body.contains(removed), "Donkey projection restored {removed}");',
            '            assert!(!llama_body.contains(removed), "Llama projection restored {removed}");',
            '            assert!(!chested_helper.contains(removed), "chested horse projection restored {removed}");',
            "        }",
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let chested_debug = input',
            '            .split("fn debug_push_abstract_chested_horse_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_horse_additional_save_data").next())',
            '            .expect("chested horse debug projection source");',
            '        let horse_debug = input',
            '            .split("fn debug_push_horse_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_skeleton_horse_additional_save_data").next())',
            '            .expect("Horse debug projection source");',
            '        let llama_debug = input',
            '            .split("fn debug_push_llama_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_tamable_animal_additional_save_data").next())',
            '            .expect("Llama debug projection source");',
            '        assert!(chested_debug.contains("vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type"));',
            '        assert!(horse_debug.contains("vanilla_entity_metadata_horse_variant_accessor_for_entity_type"));',
            '        assert!(llama_debug.contains("vanilla_entity_metadata_llama_accessors_for_entity_type"));',
            '        for source in [chested_debug, horse_debug, llama_debug] {',
            '            for removed in [',
            '                "ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID",',
            '                "HORSE_VARIANT_DATA_ID",',
            '                "HORSE_DEFAULT_VARIANT",',
            '                "LLAMA_STRENGTH_DATA_ID",',
            '                "LLAMA_DEFAULT_STRENGTH",',
            '                "LLAMA_VARIANT_DATA_ID",',
            '                "LLAMA_DEFAULT_VARIANT",',
            "            ] {",
            '                assert!(!source.contains(removed), "equine debug projection restored {removed}");',
            "            }",
            "        }",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_horse_variant_accessor_for_entity_type"));',
            '            assert!(source.contains("vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type"));',
            '            assert!(source.contains("vanilla_entity_metadata_llama_accessors_for_entity_type"));',
            '            for removed in [',
            '                "ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID",',
            '                "HORSE_VARIANT_DATA_ID",',
            '                "LLAMA_STRENGTH_DATA_ID",',
            '                "LLAMA_VARIANT_DATA_ID",',
            '                "VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID",',
            '                "VANILLA_HORSE_VARIANT_DATA_ID",',
            '                "VANILLA_LLAMA_STRENGTH_DATA_ID",',
            '                "VANILLA_LLAMA_VARIANT_DATA_ID",',
            "            ] {",
            '                assert!(!source.contains(removed), "equine fixture restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_entity_accessors_do_not_restore_fixed_metadata_facts() {",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let local_player_collision = include_str!("../../client/local_player_collision.rs");',
            '        let play_apply = include_str!("../../play_apply.rs");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let instance = include_str!("../../../../bbb-native/src/entity_scene/instance.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let movement_tests = include_str!("../tests/movement_combat_and_swing_projection.rs");',
            '        let combat_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        let player_pose_tests = include_str!("../tests/player_pose_and_sleeping_projection.rs");',
            '        let creature_bounds_tests = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        let transient_tests = include_str!("../tests/transient_events_and_animation_triggers.rs");',
            '        let aquatic_tests = include_str!("../tests/aquatic_and_beam_creature_animation.rs");',
            '        let small_mob_tests = include_str!("../tests/small_mob_and_walk_animation.rs");',
            '        let visibility_tests = include_str!("../tests/visibility_equipment_and_mount_projection.rs");',
            '        let item_frame_tests = include_str!("../../../../bbb-native/src/item_frames.rs");',
            '        let audio_tests = include_str!("../../client/audio.rs");',
            '        let entity_world_tests = include_str!("../tests.rs");',
            '        let lifecycle_tests = include_str!("../tests/lifecycle_and_queries.rs");',
            '        let native_runtime_tests = include_str!("../../../../bbb-native/src/runtime/events/tests.rs");',
            '        let native_particle_tests = include_str!("../../../../bbb-native/src/runtime/events/tests/entity_particle_events.rs");',
            '        let native_counter_tests = include_str!("../../../../bbb-native/src/runtime/events/tests/entity_and_inventory_counters.rs");',
            '        let native_session_tests = include_str!("../../../../bbb-native/src/runtime/events/tests/session_hud_and_vehicle_events.rs");',
            '        let native_audio_tests = include_str!("../../../../bbb-native/src/runtime/events/tests/audio_and_death_sounds.rs");',
            '        let net_event_tests = include_str!("../../../../bbb-net/src/event_stream.rs");',
            '        let net_probe_tests = include_str!("../../../../bbb-net/src/probe/play.rs");',
            '        for source in [dimensions, animations, store, local_player_collision, play_apply, input, input_tests, variants, instance, native_scene, native_scene_tests, movement_tests, combat_tests, player_pose_tests, creature_bounds_tests, transient_tests, aquatic_tests, small_mob_tests, visibility_tests, item_frame_tests, audio_tests, entity_world_tests, native_runtime_tests, net_event_tests, net_probe_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_entity_accessors"));',
            '            for removed in [',
            '                "VANILLA_ENTITY_SHARED_FLAGS_DATA_ID",',
            '                "VANILLA_ENTITY_AIR_SUPPLY_DATA_ID",',
            '                "VANILLA_ENTITY_CUSTOM_NAME_DATA_ID",',
            '                "VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID",',
            '                "VANILLA_ENTITY_SILENT_DATA_ID",',
            '                "VANILLA_ENTITY_NO_GRAVITY_DATA_ID",',
            '                "VANILLA_ENTITY_POSE_DATA_ID",',
            '                "VANILLA_ENTITY_TICKS_FROZEN_DATA_ID",',
            '                "VANILLA_ENTITY_SHARED_FLAG_ON_FIRE",',
            '                "VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN",',
            '                "VANILLA_ENTITY_SHARED_FLAG_SPRINTING",',
            '                "VANILLA_ENTITY_SHARED_FLAG_SWIMMING",',
            '                "VANILLA_ENTITY_SHARED_FLAG_INVISIBLE",',
            '                "VANILLA_ENTITY_SHARED_FLAG_GLOWING",',
            '                "VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING",',
            '                "ENTITY_SHARED_FLAGS_DATA_ID",',
            '                "ENTITY_AIR_SUPPLY_DATA_ID",',
            '                "ENTITY_CUSTOM_NAME_DATA_ID",',
            '                "ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID",',
            '                "ENTITY_SILENT_DATA_ID",',
            '                "ENTITY_NO_GRAVITY_DATA_ID",',
            '                "ENTITY_POSE_DATA_ID",',
            '                "ENTITY_DATA_POSE_ID",',
            '                "ENTITY_TICKS_FROZEN_DATA_ID",',
            '                "ENTITY_SHARED_FLAG_ON_FIRE",',
            '                "ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN",',
            '                "ENTITY_SHARED_FLAG_SPRINTING",',
            '                "ENTITY_SHARED_FLAG_SWIMMING",',
            '                "ENTITY_SHARED_FLAG_INVISIBLE",',
            '                "ENTITY_SHARED_FLAG_GLOWING",',
            '                "ENTITY_SHARED_FLAG_FALL_FLYING",',
            "            ] {",
            '                assert!(!source.contains(removed), "Entity projection restored {removed}");',
            "            }",
            "        }",
            '        assert!(!movement_tests.contains("ENTITY_DATA_POSE_ID"));',
            '        assert!(!movement_tests.contains("const POSE_STANDING"));',
            '        assert!(!movement_tests.contains("const POSE_SWIMMING"));',
            '        assert!(!combat_tests.contains("const POSE_STANDING"));',
            '        assert!(!combat_tests.contains("const POSE_CROUCHING"));',
            '        assert!(!player_pose_tests.contains("const POSE_STANDING"));',
            '        assert!(!player_pose_tests.contains("const POSE_SLEEPING"));',
            '        assert!(!player_pose_tests.contains("const POSE_CROUCHING"));',
            '        assert!(!creature_bounds_tests.contains("ENTITY_DATA_POSE_ID"));',
            '        assert!(!creature_bounds_tests.contains("const POSE_"));',
            '        assert!(!transient_tests.contains("protocol_pose_data(6"));',
            '        assert!(!transient_tests.contains("const POSE_"));',
            '        assert!(!aquatic_tests.contains("protocol_pose_data(6"));',
            '        assert!(!small_mob_tests.contains("protocol_pose_data(6"));',
            '        assert!(!small_mob_tests.contains("const POSE_"));',
            '        assert!(!native_scene_tests.contains("const POSE_"));',
            '        assert_eq!(play_apply.matches("silent_entity_data(true)").count(), 11);',
            '        assert!(play_apply.contains("accessors.silent_data_id"));',
            '        assert!(play_apply.contains("assert!(!accessors.silent_default)"));',
            '        assert_eq!(input_tests.matches("entity_pose_data(").count(), 2);',
            '        assert_eq!(input_tests.matches("entity_invisible_data()").count(), 3);',
            '        assert!(input_tests.contains("entity_accessors.air_supply_data_id"));',
            '        assert!(input_tests.contains("entity_accessors.custom_name_data_id"));',
            '        assert!(input_tests.contains("entity_accessors.custom_name_visible_data_id"));',
            '        assert!(input_tests.contains("entity_accessors.silent_data_id"));',
            '        assert!(input_tests.contains("entity_accessors.no_gravity_data_id"));',
            '        assert!(input_tests.contains("entity_accessors.ticks_frozen_data_id"));',
            '        assert!(native_particle_tests.contains("entity_invisible_data()"));',
            '        assert!(native_counter_tests.contains("entity_invisible_data()"));',
            '        assert!(native_session_tests.contains("entity_shift_key_down_data()"));',
            '        assert!(native_audio_tests.contains("entity_silent_data()"));',
            '        assert!(audio_tests.contains("entity_silent_data(true)"));',
            '        assert!(local_player_collision.contains("entity_shift_key_down_data()"));',
            '        assert!(local_player_collision.contains("entity_pose_data(VANILLA_ENTITY_POSE_DIGGING_ID)"));',
            '        assert!(lifecycle_tests.contains("entity_invisible_data()"));',
            '        assert!(lifecycle_tests.contains("entity_air_supply_data(301)"));',
            '        assert!(net_event_tests.contains("entity_invisible_data()"));',
            '        assert!(net_event_tests.contains("entity_air_supply_data(301)"));',
            '        assert!(net_probe_tests.contains("entity_invisible_data()"));',
            '        assert!(net_probe_tests.contains("entity_air_supply_data(301)"));',
            '        assert!(net_probe_tests.contains("entity_shift_key_down_data()"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_living_entity_accessors_do_not_restore_fixed_metadata_facts() {",
            '        let metadata = include_str!("../metadata.rs");',
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let entity_data = include_str!("../../../../bbb-native/src/entity_scene/entity_data.rs");',
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let combat_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        let player_pose_tests = include_str!("../tests/player_pose_and_sleeping_projection.rs");',
            '        let small_mob_tests = include_str!("../tests/small_mob_and_walk_animation.rs");',
            '        let transient_tests = include_str!("../tests/transient_events_and_animation_triggers.rs");',
            '        let world_tests = include_str!("../tests.rs");',
            '        let lifecycle_tests = include_str!("../tests/lifecycle_and_queries.rs");',
            '        let runtime_event_tests = include_str!("../../../../bbb-native/src/runtime/events/tests/session_hud_and_vehicle_events.rs");',
            '        for source in [metadata, animations, store, entity_data, variants, native_scene, native_scene_tests, transient_tests, world_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_living_entity_accessors_for_entity_type"));',
            '            for removed in [',
            '                "VANILLA_LIVING_ENTITY_FLAGS_DATA_ID",',
            '                "VANILLA_LIVING_ENTITY_HEALTH_DATA_ID",',
            '                "VANILLA_ENTITY_HEALTH_DATA_ID",',
            '                "VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID",',
            '                "VANILLA_LIVING_ENTITY_FLAG_IS_USING",',
            '                "VANILLA_LIVING_ENTITY_FLAG_OFF_HAND",',
            '                "VANILLA_LIVING_ENTITY_FLAG_SPIN_ATTACK",',
            '                "LIVING_ENTITY_FLAGS_DATA_ID",',
            '                "LIVING_ENTITY_HEALTH_DATA_ID",',
            '                "LIVING_ENTITY_SLEEPING_POS_DATA_ID",',
            '                "LIVING_ENTITY_FLAG_IS_USING",',
            '                "LIVING_ENTITY_FLAG_OFF_HAND",',
            '                "LIVING_ENTITY_FLAG_SPIN_ATTACK",',
            "            ] {",
            '                assert!(!source.contains(removed), "LivingEntity projection restored {removed}");',
            "            }",
            "        }",
            '        for source in [combat_tests, player_pose_tests, small_mob_tests, runtime_event_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_living_entity_accessors_for_entity_type"));',
            '            for removed in ["VANILLA_LIVING_ENTITY_HEALTH_DATA_ID", "VANILLA_ENTITY_HEALTH_DATA_ID", "LIVING_HEALTH_DATA_ID"] {',
            '                assert!(!source.contains(removed), "LivingEntity health fixture restored {removed}");',
            "            }",
            "        }",
            '        assert!(!player_pose_tests.contains("SLEEPING_POS_DATA_ID"));',
            '        assert!(world_tests.contains("living_entity_metadata_accessors()"));',
            '        assert!(lifecycle_tests.contains("accessors.is_using_mask"));',
            '        assert!(lifecycle_tests.contains("accessors.off_hand_mask"));',
            '        assert!(combat_tests.contains("living_flags.spin_attack_mask"));',
            '        assert!(!combat_tests.contains("const LIVING_ENTITY_FLAG_"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_mob_accessors_do_not_restore_fixed_metadata_facts() {",
            '        let store = include_str!("../store.rs");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let combat_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let mob_debug = input',
            '            .split("fn debug_push_mob_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_leash_additional_save_data").next())',
            '            .expect("Mob debug projection source");',
            '        let native_input_mob_helper = native_input_tests',
            '            .split("fn mob_flags_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn sniffer_baby_data").next())',
            '            .expect("native input Mob flags helper source");',
            '        assert!(native_input_mob_helper.contains("entity_type_id(entity_id)"));',
            '        assert!(native_input_mob_helper.contains("no_ai_mask"));',
            '        assert!(native_input_mob_helper.contains("left_handed_mask"));',
            '        assert_eq!(native_input_tests.matches("mob_flags_data(&world").count(), 77);',
            '        for source in [store, input, mob_debug, combat_tests, native_scene_tests, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_mob_accessors_for_entity_type"));',
            '            for removed in [',
            '                "VANILLA_MOB_FLAGS_DATA_ID",',
            '                "MOB_FLAGS_DATA_ID",',
            '                "MOB_FLAG_NO_AI",',
            '                "MOB_FLAG_LEFTHANDED",',
            '                "MOB_FLAG_LEFT_HANDED",',
            '                "MOB_FLAG_AGGRESSIVE",',
            "            ] {",
            '                assert!(!source.contains(removed), "Mob projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_frog_variant_accessor_does_not_restore_fixed_metadata_facts() {",
            '        let entity_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let ready_projection = variants',
            '            .split("pub(super) fn ready_frog_variant_texture")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn ready_wolf_variant_texture").next())',
            '            .expect("Frog ready-texture projection source");',
            '        let variant_projection = variants',
            '            .split("pub(super) fn frog_model_variant")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn frog_variant_from_registry_id").next())',
            '            .expect("Frog variant projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let debug_projection = input',
            '            .split("fn debug_push_frog_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_panda_additional_save_data").next())',
            '            .expect("Frog debug projection source");',
            '        let variant_textures = include_str!("../../../../bbb-native/src/entity_variant_textures.rs");',
            '        for source in [ready_projection, variant_projection, debug_projection] {',
            '            assert!(source.contains("vanilla_entity_metadata_frog_variant_accessor_for_entity_type"));',
            "        }",
            '        for source in [entity_scene, variants, input, variant_textures] {',
            '            for removed in [',
            '                "VANILLA_FROG_VARIANT_DATA_ID",',
            '                "FROG_VARIANT_DATA_ID",',
            '                "VANILLA_FROG_DEFAULT_VARIANT_RESOURCE_ID",',
            '                "FROG_DEFAULT_VARIANT_RESOURCE_ID",',
            "            ] {",
            '                assert!(!source.contains(removed), "Frog projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_zombie_nautilus_variant_accessor_does_not_restore_fixed_metadata_facts() {",
            '        let entity_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let variant_projection = variants',
            '            .split("pub(super) fn zombie_nautilus_model_variant")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn zombie_nautilus_variant_from_registry_id").next())',
            '            .expect("ZombieNautilus variant projection source");',
            '        let ready_projection = variants',
            '            .split("pub(super) fn apply_ready_zombie_nautilus_variant_presentation")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn entity_data_registry_holder_id").next())',
            '            .expect("ZombieNautilus ready-texture projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let debug_projection = input',
            '            .split("fn debug_push_zombie_nautilus_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_dye_color_legacy_save_id").next())',
            '            .expect("ZombieNautilus debug projection source");',
            '        let variant_textures = include_str!("../../../../bbb-native/src/entity_variant_textures.rs");',
            '        for source in [variant_projection, ready_projection, debug_projection] {',
            '            assert!(source.contains("vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type"));',
            "        }",
            '        for source in [entity_scene, variants, input, variant_textures] {',
            '            for removed in [',
            '                "VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID",',
            '                "ZOMBIE_NAUTILUS_VARIANT_DATA_ID",',
            '                "VANILLA_ZOMBIE_NAUTILUS_DEFAULT_VARIANT_RESOURCE_ID",',
            '                "ZOMBIE_NAUTILUS_DEFAULT_VARIANT_RESOURCE_ID",',
            "            ] {",
            '                assert!(!source.contains(removed), "ZombieNautilus projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_companion_variant_accessors_do_not_restore_fixed_metadata_facts() {",
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let cat_model = variants',
            '            .split("pub(super) fn feline_model_kind")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn fox_model_kind").next())',
            '            .expect("Cat model projection source");',
            '        let ready_textures = variants',
            '            .split("pub(super) fn ready_cat_variant_texture")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn apply_ready_zombie_nautilus_variant_presentation").next())',
            '            .expect("companion ready-texture projection source");',
            '        let wolf_model = variants',
            '            .split("pub(super) fn wolf_model_kind")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn entity_invisible").next())',
            '            .expect("Wolf model projection source");',
            '        let audio = include_str!("../../client/audio.rs");',
            '        let audio_body = audio',
            '            .split("pub fn living_death_sound_for_entity")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn sound_variant_death_sound").next())',
            '            .expect("companion death-sound projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let cat_debug = input',
            '            .split("fn debug_push_cat_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_fox_additional_save_data").next())',
            '            .expect("Cat debug projection source");',
            '        let wolf_debug = input',
            '            .split("fn debug_push_wolf_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_zombie_nautilus_additional_save_data").next())',
            '            .expect("Wolf debug projection source");',
            '        for source in [cat_model, ready_textures, wolf_model, audio_body, cat_debug, wolf_debug] {',
            '            assert!(source.contains("vanilla_entity_metadata_companion_variant_accessors_for_entity_type"));',
            '            for removed in [',
            '                "CAT_VARIANT_DATA_ID",',
            '                "CAT_DEFAULT_VARIANT_RESOURCE_ID",',
            '                "CAT_SOUND_VARIANT_DATA_ID",',
            '                "CAT_DEFAULT_SOUND_VARIANT_RESOURCE_ID",',
            '                "WOLF_VARIANT_DATA_ID",',
            '                "WOLF_DEFAULT_VARIANT_RESOURCE_ID",',
            '                "WOLF_SOUND_VARIANT_DATA_ID",',
            '                "WOLF_DEFAULT_SOUND_VARIANT_RESOURCE_ID",',
            "            ] {",
            '                assert!(!source.contains(removed), "companion projection restored {removed}");',
            "            }",
            "        }",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        for source in [native_scene, native_scene_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_companion_variant_accessors_for_entity_type"));',
            '            for removed in ["CAT_VARIANT_DATA_ID", "WOLF_VARIANT_DATA_ID"] {',
            '                assert!(!source.contains(removed), "companion fixture restored {removed}");',
            "            }",
            "        }",
            '        let variant_textures = include_str!("../../../../bbb-native/src/entity_variant_textures.rs");',
            '        for source in [audio, variant_textures] {',
            '            assert!(source.contains("vanilla_entity_metadata_companion_variant_accessors_for_entity_type"));',
            '            for removed in ["CAT_VARIANT_DATA_ID", "CAT_SOUND_VARIANT_DATA_ID", "WOLF_VARIANT_DATA_ID", "WOLF_SOUND_VARIANT_DATA_ID"] {',
            '                assert!(!source.contains(removed), "companion audio/texture fixture restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_farm_animal_variant_accessors_do_not_restore_fixed_metadata_facts() {",
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let model_body = variants',
            '            .split("pub(super) fn chicken_model_kind")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn goat_model_kind").next())',
            '            .expect("farm-animal model projection source");',
            '        let variant_body = variants',
            '            .split("pub(super) fn chicken_model_variant")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn chested_horse_has_chest").next())',
            '            .expect("farm-animal variant projection source");',
            '        let audio = include_str!("../../client/audio.rs");',
            '        let audio_body = audio',
            '            .split("pub fn living_death_sound_for_entity")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn sound_variant_death_sound").next())',
            '            .expect("farm-animal death-sound projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let chicken_debug = input',
            '            .split("fn debug_push_chicken_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_cow_additional_save_data").next())',
            '            .expect("Chicken debug projection source");',
            '        let cow_debug = input',
            '            .split("fn debug_push_cow_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_mooshroom_additional_save_data").next())',
            '            .expect("Cow debug projection source");',
            '        let pig_debug = input',
            '            .split("fn debug_push_pig_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_pufferfish_additional_save_data").next())',
            '            .expect("Pig debug projection source");',
            '        for source in [model_body, variant_body, audio_body, chicken_debug, cow_debug, pig_debug] {',
            '            assert!(source.contains("vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type"));',
            '            for removed in [',
            '                "CHICKEN_VARIANT_DATA_ID",',
            '                "CHICKEN_DEFAULT_VARIANT_RESOURCE_ID",',
            '                "CHICKEN_SOUND_VARIANT_DATA_ID",',
            '                "CHICKEN_DEFAULT_SOUND_VARIANT_RESOURCE_ID",',
            '                "COW_VARIANT_DATA_ID",',
            '                "COW_DEFAULT_VARIANT_RESOURCE_ID",',
            '                "COW_SOUND_VARIANT_DATA_ID",',
            '                "COW_DEFAULT_SOUND_VARIANT_RESOURCE_ID",',
            '                "PIG_VARIANT_DATA_ID",',
            '                "PIG_DEFAULT_VARIANT_RESOURCE_ID",',
            '                "PIG_SOUND_VARIANT_DATA_ID",',
            '                "PIG_DEFAULT_SOUND_VARIANT_RESOURCE_ID",',
            "            ] {",
            '                assert!(!source.contains(removed), "farm-animal projection restored {removed}");',
            "            }",
            "        }",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let play_apply = include_str!("../../play_apply.rs");',
            '        for source in [native_scene, native_scene_tests, native_input, native_input_tests, play_apply] {',
            '            assert!(source.contains("vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type"));',
            '            for removed in [',
            '                "CHICKEN_VARIANT_DATA_ID",',
            '                "CHICKEN_SOUND_VARIANT_DATA_ID",',
            '                "COW_VARIANT_DATA_ID",',
            '                "COW_SOUND_VARIANT_DATA_ID",',
            '                "PIG_VARIANT_DATA_ID",',
            '                "PIG_SOUND_VARIANT_DATA_ID",',
            '                "VANILLA_CHICKEN_VARIANT_DATA_ID",',
            '                "VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID",',
            '                "VANILLA_COW_VARIANT_DATA_ID",',
            '                "VANILLA_COW_SOUND_VARIANT_DATA_ID",',
            '                "VANILLA_PIG_VARIANT_DATA_ID",',
            '                "VANILLA_PIG_SOUND_VARIANT_DATA_ID",',
            "            ] {",
            '                assert!(!source.contains(removed), "farm-animal fixture restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn tamable_animal_consumers_do_not_restore_fixed_metadata_facts() {",
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/combat_state_and_mob_flag_projection.rs");',
            '        let inventory = include_str!("../../client/inventory.rs");',
            '        let slot_ops = include_str!("../../client/inventory/slot_ops.rs");',
            '        let inventory_tests = include_str!("../../client/inventory/tests.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let native_inventory_tests = include_str!("../../../../bbb-native/src/input/inventory/tests.rs");',
            '        for source in [store, world_tests, inventory, slot_ops, inventory_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests, native_inventory_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_tamable_animal_accessors_for_entity_type"));',
            "        }",
            '        for removed in [',
            '            "vanilla_entity_metadata_tamable_animal_flags_accessor_for_entity_type",',
            '            "TAMABLE_ANIMAL_FLAGS_DATA_ID",',
            '            "TAMABLE_ANIMAL_OWNER_DATA_ID",',
            '            "TAMABLE_ANIMAL_DEFAULT_FLAGS",',
            '            "TAMABLE_ANIMAL_FLAG_",',
            '            "TAMABLE_ANIMAL_SITTING_FLAG",',
            '            "TAMABLE_ANIMAL_TAME_FLAG",',
            '            "TEST_TAMABLE_ANIMAL",',
            '            "TEST_MOUNT_TAME_FLAGS_DATA_ID",',
            "        ] {",
            '            for source in [store, world_tests, inventory, slot_ops, inventory_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests, native_inventory_tests] {',
            '                assert!(!source.contains(removed), "TamableAnimal projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn production_projectile_family_accessors_do_not_restore_fixed_metadata_facts() {",
            '        let animations = include_str!("../animations.rs");',
            '        let projectile_source = include_str!("../projectiles.rs");',
            '        let projectiles = projectile_source.split("#[cfg(test)]\\nmod tests").next().unwrap();',
            '        let world_entities = include_str!("../../entities.rs");',
            '        let world_consumers = world_entities',
            '            .split("pub fn take_item_entity_pickup_particle_state")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("pub fn animal_love_particle_state"))',
            "            .map(|(body, _)| body)",
            '            .expect("projectile pickup and arrow particle consumers");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_data = include_str!("../../../../bbb-native/src/entity_scene/entity_data.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let native_runtime_tests = include_str!("../../../../bbb-native/src/runtime/events/tests.rs");',
            '        for source in [animations, projectiles, native_input] {',
            '            assert!(source.contains("vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type"));',
            "        }",
            '        for source in [world_consumers, native_scene, native_kind, native_data] {',
            '            assert!(source.contains("vanilla_entity_metadata_arrow_accessors_for_entity_type") || source.contains("vanilla_entity_metadata_thrown_trident_accessors_for_entity_type"));',
            "        }",
            '        assert!(world_consumers.contains("vanilla_entity_metadata_arrow_accessors_for_entity_type"));',
            '        assert!(world_consumers.contains("vanilla_entity_metadata_thrown_trident_accessors_for_entity_type"));',
            '        assert!(native_kind.contains("vanilla_entity_metadata_arrow_accessors_for_entity_type"));',
            '        assert!(native_data.contains("vanilla_entity_metadata_thrown_trident_accessors_for_entity_type"));',
            '        assert!(native_runtime_tests.contains("vanilla_entity_metadata_arrow_accessors_for_entity_type"));',
            '        assert!(native_runtime_tests.contains("vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer"));',
            '        assert!(!animations.contains("fn is_vanilla_arrow_type"));',
            '        for removed in [',
            '            "ABSTRACT_ARROW_FLAGS_DATA_ID",',
            '            "ABSTRACT_ARROW_DEFAULT_FLAGS",',
            '            "ABSTRACT_ARROW_PIERCE_LEVEL_DATA_ID",',
            '            "ABSTRACT_ARROW_DEFAULT_PIERCE_LEVEL",',
            '            "ABSTRACT_ARROW_IN_GROUND_DATA_ID",',
            '            "ABSTRACT_ARROW_DEFAULT_IN_GROUND",',
            '            "ABSTRACT_ARROW_FLAG_CRIT",',
            '            "ABSTRACT_ARROW_FLAG_NO_PHYSICS",',
            '            "VANILLA_ABSTRACT_ARROW_FLAG_CRIT",',
            '            "VANILLA_ABSTRACT_ARROW_FLAG_NO_PHYSICS",',
            '            "ARROW_EFFECT_COLOR_DATA_ID",',
            '            "ARROW_DEFAULT_EFFECT_COLOR",',
            '            "TRIDENT_FOIL_DATA_ID",',
            '            "TRIDENT_DEFAULT_FOIL",',
            "        ] {",
            '            assert!(!animations.contains(removed), "animations restored {removed}");',
            '            assert!(!projectiles.contains(removed), "projectiles restored {removed}");',
            '            assert!(!native_input.contains(removed), "native input restored {removed}");',
            '            assert!(!world_consumers.contains(removed), "world consumers restored {removed}");',
            '            assert!(!native_scene.contains(removed), "native scene restored {removed}");',
            '            assert!(!native_kind.contains(removed), "native kind restored {removed}");',
            '            assert!(!native_data.contains(removed), "native data restored {removed}");',
            '            assert!(!projectile_source.contains(removed), "projectile fixtures restored {removed}");',
            '            assert!(!native_input_tests.contains(removed), "native input fixtures restored {removed}");',
            '            assert!(!native_runtime_tests.contains(removed), "native runtime fixtures restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn guardian_consumers_do_not_restore_fixed_metadata_facts() {",
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/aquatic_and_beam_creature_animation.rs");',
            '        for source in [animations, world_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_guardian_accessors_for_entity_type"));',
            "        }",
            '        assert!(store.contains("guardian_is_moving(entity_type_id"));',
            '        assert!(store.contains("guardian_attack_target_id(entity_type_id"));',
            '        for removed in [',
            '            "GUARDIAN_MOVING_DATA_ID",',
            '            "GUARDIAN_DEFAULT_MOVING",',
            '            "GUARDIAN_ATTACK_TARGET_DATA_ID",',
            '            "GUARDIAN_DEFAULT_ATTACK_TARGET",',
            "        ] {",
            '            for source in [animations, world_tests] {',
            '                assert!(!source.contains(removed), "Guardian projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn shulker_consumers_do_not_restore_fixed_metadata_facts() {",
            '        let animations = include_str!("../animations.rs");',
            '        let dimensions = include_str!("../dimensions.rs");',
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests.rs");',
            '        let world_bounds_tests = include_str!("../tests/specialized_pick_bounds_and_render_scale.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [animations, dimensions, store, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_shulker_accessors_for_entity_type"));',
            "        }",
            '        for removed in [',
            '            "SHULKER_ATTACH_FACE_DATA_ID",',
            '            "SHULKER_DEFAULT_ATTACH_FACE_ID",',
            '            "SHULKER_DEFAULT_ATTACH_FACE",',
            '            "SHULKER_PEEK_DATA_ID",',
            '            "SHULKER_DEFAULT_PEEK",',
            '            "SHULKER_COLOR_DATA_ID",',
            '            "SHULKER_DEFAULT_COLOR",',
            '            "SHULKER_TYPE_ID",',
            "        ] {",
            '            for source in [animations, dimensions, store, world_tests, world_bounds_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Shulker projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn creeper_consumers_do_not_restore_fixed_metadata_facts() {",
            '        let animations = include_str!("../animations.rs");',
            '        let world_tests = include_str!("../tests/aquatic_and_beam_creature_animation.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_entity_data = include_str!("../../../../bbb-native/src/entity_scene/entity_data.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [animations, world_tests, native_scene, native_entity_data, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_creeper_accessors_for_entity_type"));',
            "        }",
            '        for removed in [',
            '            "CREEPER_SWELL_DIR_DATA_ID",',
            '            "CREEPER_DEFAULT_SWELL_DIR",',
            '            "CREEPER_POWERED_DATA_ID",',
            '            "CREEPER_IS_POWERED_DATA_ID",',
            '            "CREEPER_DEFAULT_POWERED",',
            '            "CREEPER_IGNITED_DATA_ID",',
            '            "CREEPER_DEFAULT_IGNITED",',
            "        ] {",
            '            for source in [animations, world_tests, native_scene, native_entity_data, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Creeper projection restored {removed}");',
            "            }",
            "        }",
            '        assert!(!native_entity_data.contains("entity_type_id == VANILLA_ENTITY_TYPE_CREEPER_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn vehicle_consumers_do_not_restore_fixed_metadata_facts() {",
            '        let animations = include_str!("../animations.rs");',
            '        let vehicle_tests = include_str!("../tests/vehicle_and_block_entity_projection.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        for source in [animations, vehicle_tests, native_scene_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_vehicle_accessors_for_entity_type"));',
            "        }",
            '        for removed in [',
            '            "VEHICLE_HURT_TIME_DATA_ID",',
            '            "VEHICLE_DEFAULT_HURT_TIME",',
            '            "VEHICLE_HURT_DIR_DATA_ID",',
            '            "VEHICLE_DEFAULT_HURT_DIR",',
            '            "VEHICLE_DAMAGE_DATA_ID",',
            '            "VEHICLE_DEFAULT_DAMAGE",',
            "        ] {",
            '            for source in [animations, vehicle_tests, native_scene_tests] {',
            '                assert!(!source.contains(removed), "VehicleEntity projection restored {removed}");',
            "            }",
            "        }",
            '        let entities = include_str!("../../entities.rs");',
            '        assert!(entities.contains("vanilla_entity_metadata_vehicle_accessors_for_entity_type"));',
            '        assert!(!entities.contains("is_vanilla_boat_type(entity_type_id) || is_vanilla_minecart_type(entity_type_id)"));',
            "    }",
            "",
            "    #[test]",
            "    fn abstract_boat_consumers_do_not_restore_fixed_metadata_facts() {",
            '        let animations = include_str!("../animations.rs");',
            '        let visibility_tests = include_str!("../tests/visibility_equipment_and_mount_projection.rs");',
            '        let vehicle_tests = include_str!("../tests/vehicle_and_block_entity_projection.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        for source in [animations, visibility_tests, vehicle_tests, native_scene_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_abstract_boat_accessors_for_entity_type"));',
            "        }",
            '        for removed in [',
            '            "ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID",',
            '            "ABSTRACT_BOAT_DEFAULT_PADDLE_LEFT",',
            '            "ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID",',
            '            "ABSTRACT_BOAT_DEFAULT_PADDLE_RIGHT",',
            '            "ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID",',
            '            "ABSTRACT_BOAT_DEFAULT_BUBBLE_TIME",',
            '            "BOAT_PADDLE_LEFT_DATA_ID",',
            '            "BOAT_PADDLE_RIGHT_DATA_ID",',
            '            "BOAT_BUBBLE_TIME_DATA_ID",',
            "        ] {",
            '            for source in [animations, visibility_tests, vehicle_tests, native_scene_tests] {',
            '                assert!(!source.contains(removed), "AbstractBoat projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn armor_stand_consumers_do_not_restore_fixed_metadata_facts() {",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        let dimensions_tests = include_str!("../tests/specialized_pick_bounds_and_render_scale.rs");',
            '        let projectiles = include_str!("../projectiles.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let native_crosshair = include_str!("../../../../bbb-native/src/crosshair.rs");',
            '        for source in [dimensions, dimensions_tests, projectiles, native_scene, native_variants, native_scene_tests, native_input, native_input_tests, native_crosshair] {',
            '            assert!(source.contains("vanilla_entity_metadata_armor_stand_accessors_for_entity_type"));',
            "        }",
            '        for removed in [',
            '            "ARMOR_STAND_CLIENT_FLAGS_DATA_ID",',
            '            "ARMOR_STAND_DEFAULT_CLIENT_FLAGS",',
            '            "ARMOR_STAND_HEAD_POSE_DATA_ID",',
            '            "ARMOR_STAND_BODY_POSE_DATA_ID",',
            '            "ARMOR_STAND_LEFT_ARM_POSE_DATA_ID",',
            '            "ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID",',
            '            "ARMOR_STAND_LEFT_LEG_POSE_DATA_ID",',
            '            "ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID",',
            '            "ARMOR_STAND_DEFAULT_HEAD_POSE",',
            '            "ARMOR_STAND_DEFAULT_BODY_POSE",',
            '            "ARMOR_STAND_DEFAULT_LEFT_ARM_POSE",',
            '            "ARMOR_STAND_DEFAULT_RIGHT_ARM_POSE",',
            '            "ARMOR_STAND_DEFAULT_LEFT_LEG_POSE",',
            '            "ARMOR_STAND_DEFAULT_RIGHT_LEG_POSE",',
            '            "ARMOR_STAND_CLIENT_FLAG_",',
            '            "DEFAULT_ARMOR_STAND_MODEL_POSE",',
            "        ] {",
            '            for source in [dimensions, dimensions_tests, projectiles, native_scene, native_variants, native_scene_tests, native_input, native_input_tests, native_crosshair] {',
            '                assert!(!source.contains(removed), "ArmorStand projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn interaction_consumers_do_not_restore_fixed_metadata_facts() {",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        let world_tests = include_str!("../tests/lifecycle_and_queries.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let native_crosshair = include_str!("../../../../bbb-native/src/crosshair.rs");',
            '        for source in [dimensions, world_tests, native_input, native_input_tests, native_crosshair] {',
            '            assert!(source.contains("vanilla_entity_metadata_interaction_accessors_for_entity_type"));',
            "        }",
            '        for removed in [',
            '            "INTERACTION_WIDTH_DATA_ID",',
            '            "INTERACTION_DEFAULT_WIDTH",',
            '            "INTERACTION_HEIGHT_DATA_ID",',
            '            "INTERACTION_DEFAULT_HEIGHT",',
            '            "INTERACTION_RESPONSE_DATA_ID",',
            '            "INTERACTION_DEFAULT_RESPONSE",',
            "        ] {",
            '            for source in [dimensions, world_tests, native_input, native_input_tests, native_crosshair] {',
            '                assert!(!source.contains(removed), "Interaction projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn iron_golem_flags_consumers_do_not_restore_accessor_constants() {",
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_iron_golem_flags_accessor_for_entity_type"));',
            "        }",
            '        let fixture = native_input_tests',
            '            .split("fn shift_f3_i_with_permission_copies_local_iron_golem_save_nbt_to_clipboard")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("IronGolem F3+I fixture source");',
            '        assert!(fixture.contains("iron_golem_flags_data("));',
            '        for removed in ["IRON_GOLEM_FLAGS_DATA_ID", "IRON_GOLEM_DEFAULT_FLAGS", "IRON_GOLEM_PLAYER_CREATED_FLAG", "VANILLA_IRON_GOLEM_FLAG_PLAYER_CREATED"] {',
            '            for source in [native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "IronGolem projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn snow_golem_consumers_do_not_restore_accessor_constants() {",
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/visibility_equipment_and_mount_projection.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [store, world_tests, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["SNOW_GOLEM_PUMPKIN_DATA_ID", "SNOW_GOLEM_DEFAULT_PUMPKIN_FLAGS", "SNOW_GOLEM_PUMPKIN_FLAG"] {',
            '            for source in [store, world_tests, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "SnowGolem projection restored {removed}");',
            "            }",
            "        }",
            '        assert!(!store.contains("identity.entity_type_id == VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID"));',
            "    }",
            "",
            "    #[test]",
            "    fn sheep_consumers_do_not_restore_accessor_constants() {",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_sheep_wool_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["SHEEP_WOOL_DATA_ID", "SHEEP_DEFAULT_WOOL", "SHEEP_FLAG_SHEARED", "SHEEP_WOOL_SHEARED_FLAG", "SHEEP_WOOL_COLOR_MASK"] {',
            '            for source in [native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Sheep projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn rabbit_consumers_do_not_restore_accessor_constants() {",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_rabbit_type_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["RABBIT_TYPE_DATA_ID", "RABBIT_DEFAULT_TYPE"] {',
            '            for source in [native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Rabbit projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn pufferfish_consumers_do_not_restore_accessor_constants() {",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        let world_tests = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [dimensions, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["PUFFERFISH_PUFF_STATE_DATA_ID", "PUFFERFISH_DEFAULT_PUFF_STATE"] {',
            '            for source in [dimensions, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Pufferfish projection restored {removed}");',
            "            }",
            "        }",
            '        assert!(!dimensions.contains("entity_type_id == VANILLA_ENTITY_TYPE_PUFFERFISH_ID"));',
            '        let world_fixture = world_tests',
            '            .split("fn pufferfish_pick_bounds_scale_with_vanilla_puff_state_metadata")',
            "            .nth(1)",
            '            .and_then(|source| source.split("\\n#[test]").next())',
            '            .expect("Pufferfish pick-bounds fixture source");',
            '        assert!(world_fixture.contains("VANILLA_ENTITY_TYPE_PUFFERFISH_ID"));',
            '        assert!(!world_fixture.contains("107"));',
            "    }",
            "",
            "    #[test]",
            "    fn slime_consumers_do_not_restore_accessor_constants() {",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        let world_tests = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [dimensions, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_slime_size_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["SLIME_SIZE_DATA_ID", "SLIME_DEFAULT_SIZE"] {',
            '            for source in [dimensions, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Slime projection restored {removed}");',
            "            }",
            "        }",
            '        assert!(!dimensions.contains("vanilla_entity_metadata_type_is_slime"));',
            '        let world_fixture = world_tests',
            '            .split("fn slime_pick_bounds_scale_with_vanilla_size_metadata")',
            "            .nth(1)",
            '            .and_then(|source| source.split("\\n#[test]").next())',
            '            .expect("Slime pick-bounds fixture source");',
            '        assert!(world_fixture.contains("VANILLA_ENTITY_TYPE_SLIME_ID"));',
            '        assert!(world_fixture.contains("VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID"));',
            '        assert!(!world_fixture.contains("117"));',
            '        assert!(!world_fixture.contains("80"));',
            "    }",
            "",
            "    #[test]",
            "    fn phantom_consumers_do_not_restore_accessor_constants() {",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        let world_tests = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [dimensions, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_phantom_size_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["PHANTOM_SIZE_DATA_ID", "PHANTOM_DEFAULT_SIZE"] {',
            '            for source in [dimensions, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Phantom projection restored {removed}");',
            "            }",
            "        }",
            '        assert!(!dimensions.contains("entity_type_id == VANILLA_ENTITY_TYPE_PHANTOM_ID"));',
            '        let world_fixture = world_tests',
            '            .split("fn phantom_pick_bounds_scale_with_vanilla_size_metadata")',
            "            .nth(1)",
            '            .and_then(|source| source.split("\\n#[test]").next())',
            '            .expect("Phantom pick-bounds fixture source");',
            '        assert!(world_fixture.contains("VANILLA_ENTITY_TYPE_PHANTOM_ID"));',
            '        assert!(!world_fixture.contains("99"));',
            "    }",
            "",
            "    #[test]",
            "    fn salmon_consumers_do_not_restore_accessor_constants() {",
            '        let dimensions = include_str!("../dimensions.rs");',
            '        let world_tests = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [dimensions, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_salmon_variant_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["SALMON_VARIANT_DATA_ID", "SALMON_DEFAULT_VARIANT"] {',
            '            for source in [dimensions, world_tests, native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Salmon projection restored {removed}");',
            "            }",
            "        }",
            '        assert!(!dimensions.contains("entity_type_id == VANILLA_ENTITY_TYPE_SALMON_ID"));',
            '        let world_fixture = world_tests',
            '            .split("fn salmon_pick_bounds_scale_with_vanilla_variant_metadata")',
            "            .nth(1)",
            '            .and_then(|source| source.split("\\n#[test]").next())',
            '            .expect("Salmon pick-bounds fixture source");',
            '        assert!(world_fixture.contains("VANILLA_ENTITY_TYPE_SALMON_ID"));',
            '        assert!(!world_fixture.contains("110"));',
            "    }",
            "",
            "    #[test]",
            "    fn tropical_fish_consumers_do_not_restore_accessor_constants() {",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type"));',
            "        }",
            '        assert!(native_kind.contains("tropical_fish_model_kind(entity_type_id, data_values)"));',
            '        for removed in ["TROPICAL_FISH_VARIANT_DATA_ID", "TROPICAL_FISH_DEFAULT_VARIANT", "TROPICAL_FISH_BASE_COLOR_MASK", "TROPICAL_FISH_BASE_COLOR_SHIFT", "TROPICAL_FISH_PATTERN_COLOR_MASK", "TROPICAL_FISH_PATTERN_COLOR_SHIFT", "TROPICAL_FISH_PATTERN_MASK"] {',
            '            for source in [native_scene, native_kind, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "TropicalFish projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn parrot_consumers_do_not_restore_accessor_constants() {",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_parrot_variant_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["PARROT_VARIANT_DATA_ID", "PARROT_DEFAULT_VARIANT", "PARROT_MAX_VARIANT"] {',
            '            for source in [native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Parrot projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn axolotl_variant_consumers_do_not_restore_accessor_constants() {",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_kind, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["AXOLOTL_VARIANT_DATA_ID", "AXOLOTL_DEFAULT_VARIANT", "debug_axolotl_variant_save_id"] {',
            '            for source in [native_scene, native_kind, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Axolotl variant projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn axolotl_from_bucket_consumers_do_not_restore_accessor_constants() {",
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_axolotl_from_bucket_accessor_for_entity_type"));',
            '            for removed in ["AXOLOTL_FROM_BUCKET_DATA_ID", "AXOLOTL_DEFAULT_FROM_BUCKET", "VANILLA_AXOLOTL_FROM_BUCKET_DATA_ID", "VANILLA_AXOLOTL_DEFAULT_FROM_BUCKET"] {',
            '                assert!(!source.contains(removed), "Axolotl from-bucket projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn abstract_fish_from_bucket_consumers_do_not_restore_accessor_constants() {",
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_abstract_fish_from_bucket_accessor_for_entity_type"));',
            '            for removed in ["ABSTRACT_FISH_FROM_BUCKET_DATA_ID", "ABSTRACT_FISH_DEFAULT_FROM_BUCKET", "VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID", "VANILLA_ABSTRACT_FISH_DEFAULT_FROM_BUCKET"] {',
            '                assert!(!source.contains(removed), "AbstractFish from-bucket projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn ocelot_trusting_consumers_do_not_restore_accessor_constants() {",
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_ocelot_trusting_accessor_for_entity_type"));',
            '            for removed in ["OCELOT_TRUSTING_DATA_ID", "OCELOT_DEFAULT_TRUSTING", "VANILLA_OCELOT_TRUSTING_DATA_ID", "VANILLA_OCELOT_DEFAULT_TRUSTING"] {',
            '                assert!(!source.contains(removed), "Ocelot trusting projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn mushroom_cow_variant_consumers_do_not_restore_accessor_constants() {",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type"));',
            "        }",
            '        assert!(native_kind.contains("mooshroom_model_kind(entity_type_id, data_values)"));',
            '        for removed in ["MUSHROOM_COW_TYPE_DATA_ID", "MUSHROOM_COW_DEFAULT_TYPE", "MOOSHROOM_TYPE_DATA_ID", "MOOSHROOM_DEFAULT_TYPE_ID", "debug_mooshroom_variant_name"] {',
            '            for source in [native_scene, native_kind, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "MushroomCow variant projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn bogged_sheared_consumers_do_not_restore_accessor_constants() {",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_scene, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type"));',
            "        }",
            '        assert!(native_kind.contains("bogged_sheared(entity_type_id, data_values)"));',
            '        for removed in ["BOGGED_SHEARED_DATA_ID", "BOGGED_DEFAULT_SHEARED"] {',
            '            for source in [native_scene, native_kind, native_variants, native_scene_tests, native_input, native_input_tests] {',
            '                assert!(!source.contains(removed), "Bogged sheared projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn allay_dancing_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/transient_events_and_animation_triggers.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        assert!(animations.contains("vanilla_entity_metadata_allay_dancing_accessor_for_entity_type"));',
            '        assert!(store.contains("allay_is_dancing(entity_type_id, &metadata.data_values)"));',
            '        for source in [world_tests, native_scene_tests, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_allay_dancing_accessor_for_entity_type"));',
            "        }",
            '        assert!(native_input_tests.contains("vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type"));',
            '        for removed in ["ALLAY_DANCING_DATA_ID", "ALLAY_DEFAULT_DANCING", "ALLAY_CAN_DUPLICATE_DATA_ID", "ALLAY_DEFAULT_CAN_DUPLICATE"] {',
            '            for source in [animations, store, world_tests, native_scene_tests, native_input_tests] {',
            '                assert!(!source.contains(removed), "Allay dancing projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn axolotl_playing_dead_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/transient_events_and_animation_triggers.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        assert!(animations.contains("vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type"));',
            '        assert!(store.contains("axolotl_is_playing_dead(entity_type_id, &metadata.data_values)"));',
            '        for source in [world_tests, native_scene_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["AXOLOTL_PLAYING_DEAD_DATA_ID", "AXOLOTL_DEFAULT_PLAYING_DEAD"] {',
            '            for source in [animations, store, world_tests, native_scene_tests] {',
            '                assert!(!source.contains(removed), "Axolotl playing-dead projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn camel_metadata_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let dimensions = include_str!("../dimensions.rs");',
            '        let walk_tests = include_str!("../tests/small_mob_and_walk_animation.rs");',
            '        let transient_tests = include_str!("../tests/transient_events_and_animation_triggers.rs");',
            '        let passenger_tests = include_str!("../tests/passenger_and_vehicle_movement.rs");',
            '        let bounds_tests = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_instance = include_str!("../../../../bbb-native/src/entity_scene/instance.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let sit_body = variants',
            '            .split("pub(super) fn camel_sit_state")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn chicken_model_kind").next())',
            '            .expect("Camel sit projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let debug_body = input',
            '            .split("fn debug_push_camel_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_llama_additional_save_data").next())',
            '            .expect("Camel debug projection source");',
            '        assert!(animations.contains("vanilla_entity_metadata_camel_dash_accessor_for_entity_type"));',
            '        assert!(store.contains("camel_is_dashing(entity_type_id, &metadata.data_values)"));',
            '        for source in [walk_tests, transient_tests, passenger_tests, native_scene_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_camel_dash_accessor_for_entity_type"));',
            "        }",
            '        assert_eq!(dimensions.matches("vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type").count(), 4);',
            '        assert!(dimensions.contains("pub(crate) fn vanilla_camel_leash_offset_for_entity_data"));',
            '        assert!(sit_body.contains("vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type"));',
            '        assert!(debug_body.contains("vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type"));',
            '        for source in [bounds_tests, native_scene_tests, input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type"));',
            "        }",
            '        assert!(!native_instance.contains("LAST_POSE_CHANGE_TICK (id 20)"));',
            '        assert!(!native_scene_tests.contains("LAST_POSE_CHANGE_TICK (20"));',
            '        assert!(dimensions.contains("vanilla_is_baby(entity_type_id, data_values)"));',
            '        for removed in ["CAMEL_DASH_DATA_ID", "CAMEL_DEFAULT_DASH"] {',
            '            for source in [animations, store, walk_tests, transient_tests, passenger_tests, native_scene_tests] {',
            '                assert!(!source.contains(removed), "Camel dash projection restored {removed}");',
            "            }",
            "        }",
            '        for removed in ["CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID", "CAMEL_DEFAULT_LAST_POSE_CHANGE_TICK"] {',
            '            for source in [dimensions, sit_body, debug_body, input, bounds_tests, native_scene_tests, input_tests] {',
            '                assert!(!source.contains(removed), "Camel pose projection restored {removed}");',
            "            }",
            "        }",
            '        let camel_pick_bounds_test = bounds_tests',
            '            .split("fn camel_pick_bounds_follow_vanilla_sitting_pose_and_age_scale")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn camel_body_anchor_y_offset_matches_vanilla_sit_stand_curve").next())',
            '            .expect("Camel pick-bounds fixture source");',
            '        let camel_anchor_test = bounds_tests',
            '            .split("fn camel_body_anchor_y_offset_matches_vanilla_sit_stand_curve")',
            "            .nth(1)",
            '            .and_then(|source| source.split("\n#[test]").next())',
            '            .expect("Camel body-anchor fixture source");',
            '        assert!(bounds_tests.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        for source in [camel_pick_bounds_test, camel_anchor_test] {',
            '            assert!(source.contains("camel_baby_data"));',
            '            assert!(!source.contains("AGEABLE_BABY_DATA_ID"));',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn polar_bear_standing_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        let world_tests = include_str!("../tests/specialized_pick_bounds_and_render_scale.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [animations, world_tests, native_scene_tests, input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type"));',
            "        }",
            '        for removed in ["POLAR_BEAR_STANDING_DATA_ID", "POLAR_BEAR_DEFAULT_STANDING"] {',
            '            for source in [animations, world_tests, native_scene_tests, input_tests] {',
            '                assert!(!source.contains(removed), "PolarBear standing projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn wolf_accessor_consumers_do_not_restore_fixed_metadata_facts() {",
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/movement_combat_and_swing_projection.rs");',
            '        let entity_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let wolf_model = variants',
            '            .split("pub(super) fn wolf_model_kind")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn entity_invisible").next())',
            '            .expect("Wolf model projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let wolf_debug = input',
            '            .split("fn debug_push_wolf_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_zombie_nautilus_additional_save_data").next())',
            '            .expect("Wolf debug projection source");',
            '        assert!(animations.contains("vanilla_entity_metadata_wolf_interested_accessor_for_entity_type"));',
            '        assert!(store.contains("wolf_is_interested(entity_type_id, &metadata.data_values)"));',
            '        for source in [wolf_model, wolf_debug, world_tests, native_scene_tests, input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_wolf_accessors_for_entity_type"));',
            "        }",
            '        for source in [animations, store, world_tests, entity_scene, variants, native_scene_tests, input, input_tests] {',
            '            for removed in [',
            '                "WOLF_INTERESTED_DATA_ID",',
            '                "WOLF_DEFAULT_INTERESTED",',
            '                "WOLF_COLLAR_COLOR_DATA_ID",',
            '                "WOLF_DEFAULT_COLLAR_COLOR",',
            '                "WOLF_ANGER_END_TIME_DATA_ID",',
            '                "WOLF_DEFAULT_ANGER_END_TIME",',
            "            ] {",
            '                assert!(!source.contains(removed), "Wolf projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn cat_accessor_consumers_do_not_restore_fixed_metadata_facts() {",
            '        let entities = include_str!("../../entities.rs");',
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/small_mob_and_walk_animation.rs");',
            '        let entity_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let feline_model = variants',
            '            .split("pub(super) fn feline_model_kind")',
            "            .nth(1)",
            '            .and_then(|source| source.split("pub(super) fn fox_model_kind").next())',
            '            .expect("Cat model projection source");',
            '        let input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let cat_debug = input',
            '            .split("fn debug_push_cat_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_push_fox_additional_save_data").next())',
            '            .expect("Cat debug projection source");',
            '        let sleeping_player_body = entities',
            '            .split("fn cat_is_lying_on_top_of_sleeping_player")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n    fn entity_outline_color"))',
            '            .map(|(body, _)| body)',
            '            .expect("cat sleeping-player projection source");',
            '        assert!(sleeping_player_body.contains("cat_is_lying(source.entity_type_id, &source.data_values)"));',
            '        assert!(!sleeping_player_body.contains("vanilla_is_cat"));',
            '        assert!(animations.contains("cat_is_lying(entity_type_id, data_values)"));',
            '        assert!(animations.contains("cat_relax_state_one(entity_type_id, data_values)"));',
            '        assert!(store.contains("cat_is_lying(entity_type_id, &metadata.data_values)"));',
            '        assert!(store.contains("cat_relax_state_one(entity_type_id, &metadata.data_values)"));',
            '        for source in [feline_model, cat_debug, world_tests, native_scene_tests, input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_cat_accessors_for_entity_type"));',
            "        }",
            '        for source in [sleeping_player_body, animations, store, world_tests, entity_scene, variants, native_scene_tests, input, input_tests] {',
            '            for removed in [',
            '                "CAT_IS_LYING_DATA_ID",',
            '                "CAT_DEFAULT_IS_LYING",',
            '                "CAT_RELAX_STATE_ONE_DATA_ID",',
            '                "CAT_DEFAULT_RELAX_STATE_ONE",',
            '                "CAT_COLLAR_COLOR_DATA_ID",',
            '                "CAT_DEFAULT_COLLAR_COLOR",',
            "            ] {",
            '                assert!(!source.contains(removed), "Cat projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn creaking_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/transient_events_and_animation_triggers.rs");',
            '        let collision = include_str!("../../client/local_player_collision.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        assert!(animations.contains("vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type"));',
            '        assert!(animations.contains("vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type"));',
            '        assert!(store.contains("creaking_can_move(identity.entity_type_id, &metadata.data_values)"));',
            '        assert!(store.contains("creaking_is_tearing_down(entity_type_id, &metadata.data_values)"));',
            '        assert!(!store.contains("identity.entity_type_id != VANILLA_ENTITY_TYPE_CREAKING_ID"));',
            '        assert!(collision.contains("vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type"));',
            '        assert!(native_kind.contains("vanilla_entity_metadata_creaking_active_accessor_for_entity_type"));',
            '        assert!(native_input.contains("vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type"));',
            '        assert!(world_tests.contains("vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type"));',
            '        assert!(world_tests.contains("vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type"));',
            '        assert!(native_scene_tests.contains("vanilla_entity_metadata_creaking_active_accessor_for_entity_type"));',
            '        assert!(native_input_tests.contains("vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type"));',
            '        for removed in ["CREAKING_CAN_MOVE_DATA_ID", "CREAKING_DEFAULT_CAN_MOVE", "VANILLA_CREAKING_CAN_MOVE_DATA_ID", "VANILLA_CREAKING_DEFAULT_CAN_MOVE", "CREAKING_IS_TEARING_DOWN_DATA_ID", "CREAKING_DEFAULT_TEARING_DOWN", "VANILLA_CREAKING_TEARING_DOWN_DATA_ID", "VANILLA_CREAKING_DEFAULT_TEARING_DOWN", "CREAKING_IS_ACTIVE_DATA_ID", "CREAKING_DEFAULT_ACTIVE", "VANILLA_CREAKING_ACTIVE_DATA_ID", "VANILLA_CREAKING_DEFAULT_ACTIVE", "CREAKING_HOME_POS_DATA_ID", "VANILLA_CREAKING_HOME_POS_DATA_ID"] {',
            '            assert!(!animations.contains(removed), "animations restored {removed}");',
            '            assert!(!store.contains(removed), "store restored {removed}");',
            '            assert!(!collision.contains(removed), "collision restored {removed}");',
            '            assert!(!native_scene.contains(removed), "native scene restored {removed}");',
            '            assert!(!native_kind.contains(removed), "native kind restored {removed}");',
            '            assert!(!native_input.contains(removed), "native input restored {removed}");',
            '            assert!(!world_tests.contains(removed), "world fixtures restored {removed}");',
            '            assert!(!native_scene_tests.contains(removed), "native scene fixtures restored {removed}");',
            '            assert!(!native_input_tests.contains(removed), "native input fixtures restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn crossbow_charging_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/transient_events_and_animation_triggers.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_hands = include_str!("../../../../bbb-native/src/entity_scene/hands.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let baby_tests = include_str!("../tests/specialized_pick_bounds_and_render_scale.rs");',
            '        assert!(animations.contains("vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type"));',
            '        assert!(animations.contains("vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type"));',
            '        assert!(store.contains("pillager_is_charging_crossbow(entity_type_id, &metadata.data_values)"));',
            '        assert!(store.contains("piglin_is_charging_crossbow(entity_type_id, &metadata.data_values)"));',
            '        assert!(native_hands.contains("vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type"));',
            '        assert!(native_hands.contains("vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type"));',
            '        assert!(world_tests.contains("vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type"));',
            '        assert!(world_tests.contains("vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type"));',
            '        assert!(native_scene_tests.contains("vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type"));',
            '        assert!(native_scene_tests.contains("vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type"));',
            '        let baby_fixture = baby_tests',
            '            .split("fn generated_baby_accessor_and_is_baby_dispatch_preserve_overrides")',
            '            .nth(1)',
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("Pillager non-baby fixture source");',
            '        assert!(baby_fixture.contains("vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type"));',
            '        assert!(baby_fixture.contains("vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type"));',
            '        assert!(!baby_fixture.contains("protocol_bool_data(16"));',
            '        assert!(!baby_fixture.contains("protocol_bool_data(17"));',
            '        assert!(!world_tests.contains("id (18 vs 17)"));',
            '        assert!(!native_scene_tests.contains("BOOLEAN id 18"));',
            '        assert!(!native_scene_tests.contains("slot 18"));',
            '        let pillager_fixture = native_scene_tests',
            '            .split("fn entity_model_instances_project_pillager_charging_crossbow")',
            '            .nth(1)',
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("pillager charging fixture source");',
            '        assert!(pillager_fixture.contains("vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type"));',
            '        assert!(!pillager_fixture.contains("const PILLAGER"));',
            '        for removed in ["PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID", "PILLAGER_CHARGING_CROSSBOW_DATA_ID", "PILLAGER_DEFAULT_CHARGING_CROSSBOW", "PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID", "PIGLIN_CHARGING_CROSSBOW_DATA_ID", "PIGLIN_DEFAULT_CHARGING_CROSSBOW"] {',
            '            assert!(!animations.contains(removed), "animations restored {removed}");',
            '            assert!(!store.contains(removed), "store restored {removed}");',
            '            assert!(!native_scene.contains(removed), "native scene restored {removed}");',
            '            assert!(!native_hands.contains(removed), "native hands restored {removed}");',
            '            assert!(!world_tests.contains(removed), "world fixtures restored {removed}");',
            '            assert!(!native_scene_tests.contains(removed), "native scene fixtures restored {removed}");',
            '            assert!(!baby_tests.contains(removed), "baby fixtures restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn zoglin_baby_consumers_do_not_restore_accessor_constants() {",
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for source in [native_input, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '            for removed in ["ZOGLIN_BABY_DATA_ID", "ZOGLIN_DEFAULT_BABY", "VANILLA_ZOGLIN_BABY_DATA_ID", "VANILLA_ZOGLIN_DEFAULT_BABY"] {',
            '                assert!(!source.contains(removed), "Zoglin baby projection restored {removed}");',
            "            }",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn piglin_metadata_consumers_do_not_restore_accessor_constants() {",
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        assert!(native_variants.contains("vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type"));',
            '        assert!(native_variants.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(native_kind.contains("piglin_baby(entity_type_id, data_values)"));',
            '        assert!(native_input.contains("vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type"));',
            '        assert!(native_input.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(native_scene_tests.contains("vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type"));',
            '        assert!(native_scene_tests.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(native_scene_tests.contains("vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type"));',
            '        assert!(native_input_tests.contains("vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type"));',
            '        assert!(native_input_tests.contains("vanilla_entity_metadata_baby_accessor_for_entity_type"));',
            '        assert!(!native_scene_tests.contains("DATA_IS_DANCING (data id"));',
            '        for removed in ["PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID", "ABSTRACT_PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID", "ABSTRACT_PIGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION", "PIGLIN_BABY_DATA_ID", "PIGLIN_DEFAULT_BABY", "PIGLIN_IS_DANCING_DATA_ID", "PIGLIN_DANCING_DATA_ID", "PIGLIN_DEFAULT_DANCING"] {',
            '            assert!(!native_scene.contains(removed), "native scene restored {removed}");',
            '            assert!(!native_variants.contains(removed), "native variants restored {removed}");',
            '            assert!(!native_kind.contains(removed), "native kind restored {removed}");',
            '            assert!(!native_input.contains(removed), "native input restored {removed}");',
            '            assert!(!native_scene_tests.contains(removed), "native scene fixtures restored {removed}");',
            '            assert!(!native_input_tests.contains(removed), "native input fixtures restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn copper_golem_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        let world_tests = include_str!("../tests/transient_events_and_animation_triggers.rs");',
            '        let native_scene = include_str!("../../../../bbb-native/src/entity_scene.rs");',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        let native_data = include_str!("../../../../bbb-native/src/entity_scene/entity_data.rs");',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        assert!(animations.contains("vanilla_entity_metadata_copper_golem_accessors_for_entity_type"));',
            '        assert!(animations.contains("fn copper_golem_animation_flags"));',
            '        assert!(store.contains("copper_golem_animation_flags(entity_type_id, &metadata.data_values)"));',
            '        assert!(native_data.contains("vanilla_entity_metadata_copper_golem_accessors_for_entity_type"));',
            '        assert!(native_kind.contains("copper_golem_weathering(entity_type_id, data_values)"));',
            '        assert!(native_input.contains("vanilla_entity_metadata_copper_golem_accessors_for_entity_type"));',
            '        for source in [world_tests, native_scene_tests, native_input_tests] {',
            '            assert!(source.contains("vanilla_entity_metadata_copper_golem_accessors_for_entity_type"));',
            "        }",
            '        for removed in ["COPPER_GOLEM_STATE_DATA_ID", "COPPER_GOLEM_DEFAULT_STATE", "COPPER_GOLEM_WEATHER_STATE_DATA_ID", "COPPER_GOLEM_DEFAULT_WEATHER_STATE"] {',
            '            assert!(!animations.contains(removed), "animations restored {removed}");',
            '            assert!(!store.contains(removed), "store restored {removed}");',
            '            assert!(!native_scene.contains(removed), "native scene restored {removed}");',
            '            assert!(!native_kind.contains(removed), "native kind restored {removed}");',
            '            assert!(!native_data.contains(removed), "native data restored {removed}");',
            '            assert!(!native_input.contains(removed), "native input restored {removed}");',
            '            assert!(!world_tests.contains(removed), "world fixtures restored {removed}");',
            '            assert!(!native_scene_tests.contains(removed), "native scene fixtures restored {removed}");',
            '            assert!(!native_input_tests.contains(removed), "native input fixtures restored {removed}");',
            "        }",
            '        for removed in ["copper_golem_is_idle(", "copper_golem_is_getting_item(", "copper_golem_is_getting_no_item(", "copper_golem_is_dropping_item(", "copper_golem_is_dropping_no_item("] {',
            '            assert!(!animations.contains(removed), "animations restored repeated state read {removed}");',
            '            assert!(!store.contains(removed), "store restored repeated state read {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn sniffer_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        assert!(animations.contains("vanilla_entity_metadata_sniffer_state_accessor_for_entity_type"));',
            '        assert!(animations.contains("sniffer_state_id(entity_type_id, data_values)"));',
            '        assert!(!animations.contains("synced state (id 18)"));',
            '        let dimensions = include_str!("../dimensions.rs");',
            '        assert!(dimensions.contains("vanilla_entity_metadata_sniffer_state_accessor_for_entity_type"));',
            '        assert!(dimensions.contains("vanilla_is_baby(entity_type_id, data_values)"));',
            '        let bounds_tests = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        let bounds_fixture = bounds_tests',
            '            .split("fn sniffer_pick_bounds_follow_vanilla_state_and_age_scale")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("Sniffer bounds fixture source");',
            '        assert!(bounds_fixture.contains("sniffer_state_data("));',
            '        assert!(bounds_fixture.contains("sniffer_baby_data("));',
            '        let animation_tests = include_str!("../tests/small_mob_and_walk_animation.rs");',
            '        let animation_fixture = animation_tests',
            '            .split("fn entity_model_sources_project_sniffer_state_animation")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("Sniffer animation fixture source");',
            '        assert!(animation_fixture.contains("sniffer_state_data("));',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let native_input_fixture = native_input_tests',
            '            .split("fn shift_f3_i_with_permission_copies_local_sniffer_save_nbt_to_clipboard")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("Sniffer F3+I fixture source");',
            '        assert!(native_input_fixture.contains("sniffer_state_data("));',
            '        assert!(native_input_fixture.contains("sniffer_drop_seed_at_tick_data("));',
            '        assert!(native_input_tests.contains("vanilla_entity_metadata_sniffer_drop_seed_at_tick_accessor_for_entity_type"));',
            '        for removed in ["SNIFFER_STATE_DATA_ID", "SNIFFER_DEFAULT_STATE", "SNIFFER_DROP_SEED_AT_TICK_DATA_ID", "SNIFFER_DEFAULT_DROP_SEED_AT_TICK"] {',
            '            assert!(!animations.contains(removed), "animations restored {removed}");',
            '            assert!(!dimensions.contains(removed), "dimensions restored {removed}");',
            '            assert!(!bounds_fixture.contains(removed), "bounds fixture restored {removed}");',
            '            assert!(!animation_fixture.contains(removed), "animation fixture restored {removed}");',
            '            assert!(!native_input_fixture.contains(removed), "native input fixture restored {removed}");',
            "        }",
            '        for removed in ["SNIFFER_DROP_SEED_AT_TICK_DATA_ID", "SNIFFER_DEFAULT_DROP_SEED_AT_TICK"] {',
            '            assert!(!native_input_tests.contains(removed), "native input tests restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn armadillo_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        assert_eq!(animations.matches("vanilla_entity_metadata_armadillo_state_accessor_for_entity_type").count(), 3);',
            '        assert!(!animations.contains("ArmadilloState` id (id 18)"));',
            '        let world_tests = include_str!("../tests/small_mob_and_walk_animation.rs");',
            '        let world_fixture = world_tests',
            '            .split("fn entity_model_sources_project_armadillo_state_animation")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("Armadillo animation fixture source");',
            '        assert!(world_fixture.contains("armadillo_state_data("));',
            '        let native_variants = include_str!("../../../../bbb-native/src/entity_scene/variants.rs");',
            '        assert!(native_variants.contains("vanilla_entity_metadata_armadillo_state_accessor_for_entity_type"));',
            '        let native_kind = include_str!("../../../../bbb-native/src/entity_scene/kind.rs");',
            '        assert!(native_kind.contains("armadillo_rolled_up(entity_type_id, data_values)"));',
            '        let native_scene_tests = include_str!("../../../../bbb-native/src/entity_scene/tests.rs");',
            '        assert!(native_scene_tests.contains("vanilla_entity_metadata_armadillo_state_accessor_for_entity_type"));',
            '        let native_input = include_str!("../../../../bbb-native/src/input.rs");',
            '        let debug_body = native_input',
            '            .split("fn debug_push_armadillo_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split("fn debug_armadillo_state_name").next())',
            '            .expect("armadillo debug projection source");',
            '        assert!(debug_body.contains("vanilla_entity_metadata_armadillo_state_accessor_for_entity_type"));',
            '        let native_input_tests = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        let native_input_fixture = native_input_tests',
            '            .split("fn shift_f3_i_with_permission_copies_local_armadillo_save_nbt_to_clipboard")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("Armadillo F3+I fixture source");',
            '        assert!(native_input_fixture.contains("armadillo_state_data("));',
            '        for removed in ["ARMADILLO_STATE_DATA_ID", "ARMADILLO_DEFAULT_STATE"] {',
            '            assert!(!animations.contains(removed), "animations restored {removed}");',
            '            assert!(!world_fixture.contains(removed), "world fixture restored {removed}");',
            '            assert!(!native_variants.contains(removed), "native variants restored {removed}");',
            '            assert!(!native_kind.contains(removed), "native kind restored {removed}");',
            '            assert!(!native_scene_tests.contains(removed), "native scene fixtures restored {removed}");',
            '            assert!(!debug_body.contains(removed), "native debug restored {removed}");',
            '            assert!(!native_input_fixture.contains(removed), "native input fixture restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn warden_anger_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        let store = include_str!("../store.rs");',
            '        assert!(animations.contains("vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type"));',
            '        assert!(store.contains("warden_heartbeat_delay(entity_type_id, &metadata.data_values)"));',
            '        assert!(!animations.contains("first own synced accessor"));',
            '        let world_tests = include_str!("../tests/transient_events_and_animation_triggers.rs");',
            '        let heartbeat_fixture = world_tests',
            '            .split("fn warden_client_anger_drives_heartbeat_period")',
            "            .nth(1)",
            '            .and_then(|source| source.split("#[test]").next())',
            '            .expect("Warden heartbeat fixture source");',
            '        assert!(heartbeat_fixture.contains("warden_anger_data("));',
            '        for removed in ["WARDEN_ANGER_LEVEL_DATA_ID", "WARDEN_DEFAULT_ANGER_LEVEL"] {',
            '            assert!(!animations.contains(removed), "animations restored {removed}");',
            '            assert!(!store.contains(removed), "store restored {removed}");',
            '            assert!(!heartbeat_fixture.contains(removed), "heartbeat fixture restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn ender_dragon_phase_consumers_do_not_restore_accessor_constants() {",
            '        let animations = include_str!("../animations.rs");',
            '        assert!(animations.contains("vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type"));',
            '        let dragon = include_str!("../dragon.rs");',
            '        assert!(dragon.contains("vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type"));',
            '        assert!(dragon.contains("fn default_ender_dragon_phase_id()"));',
            '        let world_tests = include_str!("../tests/creature_pick_bounds_projection.rs");',
            '        assert!(world_tests.contains("fn ender_dragon_phase_data("));',
            '        let native_crosshair = include_str!("../../../../bbb-native/src/crosshair.rs");',
            '        assert!(native_crosshair.contains("fn ender_dragon_phase_data("));',
            '        for removed in ["ENDER_DRAGON_PHASE_DATA_ID", "ENDER_DRAGON_DEFAULT_PHASE"] {',
            '            assert!(!animations.contains(removed), "animations restored {removed}");',
            '            assert!(!dragon.contains(removed), "dragon state restored {removed}");',
            '            assert!(!world_tests.contains(removed), "world fixtures restored {removed}");',
            '            assert!(!native_crosshair.contains(removed), "crosshair fixtures restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn semantic_consumers_do_not_restore_numeric_data_ids_or_local_aliases() {",
            "        fn visit(path: &std::path::Path, violations: &mut Vec<String>) {",
            "            if path.is_dir() {",
            "                for entry in std::fs::read_dir(path).unwrap() { visit(&entry.unwrap().path(), violations); }",
            "                return;",
            "            }",
            "            if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { return; }",
            "            let display = path.to_string_lossy().replace('\\\\', \"/\");",
            "            if display.ends_with(\"/bbb-world/src/entities/schema.rs\") || display.contains(\"/bbb-world/src/entities/schema/\") { return; }",
            "            let source = std::fs::read_to_string(path).unwrap();",
            "            for (line_number, line) in source.lines().enumerate() {",
            '                if let Some((_, value)) = line.split_once("data_id:") {',
            '                    let value = value.split_once(",").map_or(value, |(value, _)| value).trim();',
            "                    if !value.is_empty() && value.chars().all(|character| character.is_ascii_digit() || character == '_') {",
            '                        violations.push(format!("{}:{}: numeric data_id: {line}", path.display(), line_number + 1));',
            "                    }",
            "                }",
            '                let declaration = line.trim().strip_prefix("const ")',
            '                    .or_else(|| line.trim().strip_prefix("pub const "))',
            '                    .or_else(|| line.trim().strip_prefix("pub(crate) const "));',
            "                if let Some((name, value)) = declaration.and_then(|declaration| declaration.split_once(':')) {",
            "                    let numeric_u8 = value.split_once('=').is_some_and(|(type_name, value)| {",
            "                        let value = value.trim().trim_end_matches(';');",
            "                        type_name.trim() == \"u8\" && !value.is_empty() && value.chars().all(|character| character.is_ascii_digit() || character == '_')",
            "                    });",
            "                    if name.trim().ends_with(\"_DATA_ID\") && numeric_u8 {",
            '                        violations.push(format!("{}:{}: local data-id alias: {line}", path.display(), line_number + 1));',
            "                    }",
            "                }",
            "            }",
            "        }",
            "        let crates = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let mut violations = Vec::new();",
            "        for path in [crates.join(\"bbb-native/src\"), crates.join(\"bbb-world/src\"), crates.join(\"bbb-net/src\")] { visit(&path, &mut violations); }",
            '        assert!(violations.is_empty(), "semantic consumers restored fixed entity metadata ids:\\n{}", violations.join("\\n"));',
            "    }",
            "",
            "    #[test]",
            "    fn production_debug_save_data_does_not_restore_class_type_memberships() {",
            '        let source = include_str!("../../../../bbb-native/src/input.rs");',
            '        let body = source',
            '            .split("fn debug_push_entity_additional_save_data")',
            "            .nth(1)",
            '            .and_then(|source| source.split_once("\\n}"))',
            "            .map(|(body, _)| body)",
            '            .expect("debug entity save-data dispatch source");',
            '        for query in [',
            '            "bbb_world::vanilla_entity_metadata_type_is_abstract_chested_horse",',
            '            "bbb_world::vanilla_entity_metadata_type_is_llama",',
            '            "bbb_world::vanilla_entity_metadata_type_is_camel",',
            '            "bbb_world::vanilla_entity_metadata_type_is_guardian",',
            '            "bbb_world::vanilla_entity_metadata_type_is_slime",',
            '        ] {',
            '            assert!(body.contains(query), "missing {query}");',
            "        }",
            '        for removed in [',
            '            "VANILLA_ENTITY_TYPE_DONKEY_ID",',
            '            "VANILLA_ENTITY_TYPE_MULE_ID",',
            '            "VANILLA_ENTITY_TYPE_LLAMA_ID",',
            '            "VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID",',
            '            "VANILLA_ENTITY_TYPE_CAMEL_ID",',
            '            "VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID",',
            '            "VANILLA_ENTITY_TYPE_GUARDIAN_ID",',
            '            "VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID",',
            '            "VANILLA_ENTITY_TYPE_SLIME_ID",',
            '            "VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID",',
            '        ] {',
            '            assert!(!body.contains(removed), "restored {removed}");',
            "        }",
            '        assert!(!source.contains("fn debug_push_trader_llama_additional_save_data"));',
            '        let fixtures = include_str!("../../../../bbb-native/src/input/tests.rs");',
            '        for (fixture_name, query, removed) in [',
            '            (',
            '                "debug_save_data_abstract_chested_horse_class_dispatch_uses_generated_membership",',
            '                "bbb_world::vanilla_entity_metadata_type_is_abstract_chested_horse",',
            '                &[',
            '                    "VANILLA_ENTITY_TYPE_DONKEY_ID",',
            '                    "VANILLA_ENTITY_TYPE_MULE_ID",',
            '                    "VANILLA_ENTITY_TYPE_LLAMA_ID",',
            '                    "VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID",',
            '                ][..],',
            '            ),',
            '            (',
            '                "shift_f3_i_with_permission_copies_local_camel_family_save_nbt_to_clipboard",',
            '                "bbb_world::vanilla_entity_metadata_type_is_camel",',
            '                &["VANILLA_ENTITY_TYPE_CAMEL_ID", "VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID"][..],',
            '            ),',
            '            (',
            '                "shift_f3_i_with_permission_copies_local_guardian_family_save_nbt_to_clipboard",',
            '                "bbb_world::vanilla_entity_metadata_type_is_guardian",',
            '                &["VANILLA_ENTITY_TYPE_GUARDIAN_ID", "VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID"][..],',
            '            ),',
            '            (',
            '                "shift_f3_i_with_permission_copies_local_slime_family_save_nbt_to_clipboard",',
            '                "bbb_world::vanilla_entity_metadata_type_is_slime",',
            '                &["VANILLA_ENTITY_TYPE_SLIME_ID", "VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID"][..],',
            '            ),',
            '        ] {',
            '            let fixture = fixtures',
            '                .split(&format!("fn {fixture_name}"))',
            '                .nth(1)',
            '                .and_then(|source| source.split_once("\\n}"))',
            '                .map(|(body, _)| body)',
            '                .unwrap_or_else(|| panic!("missing {fixture_name}"));',
            '            assert!(fixture.contains("VANILLA_ENTITY_TYPES"));',
            '            assert!(fixture.contains(query), "{fixture_name} missing {query}");',
            '            for &member in removed {',
            '                assert!(!fixture.contains(member), "{fixture_name} restored {member}");',
            '            }',
            '        }',
            '        let entity_source = include_str!("../../entities.rs");',
            '        assert!(!entity_source.contains("fn is_vanilla_slime_type"));',
            '        let animation_source = include_str!("../animations.rs");',
            '        assert!(!animation_source.contains("fn is_guardian_entity_type"));',
            "    }",
            "}",
            "",
        ]
    )
    return format_rust("\n".join(lines))


def main() -> int:
    args = parse_args()
    generated = render(extract(args.source_root))
    current = args.output.read_text() if args.output.exists() else ""
    if args.command == "check":
        if current == generated:
            return 0
        sys.stderr.writelines(
            difflib.unified_diff(
                current.splitlines(keepends=True),
                generated.splitlines(keepends=True),
                fromfile=str(args.output),
                tofile="generated",
            )
        )
        return 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
