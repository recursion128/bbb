#!/usr/bin/env python3
"""Generate/check the checked-in vanilla 26.1 entity metadata Rust catalog."""

from __future__ import annotations

import argparse
import difflib
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from tree_sitter import Node

from entity_metadata_extractor import (
    Accessor,
    ExtractionError,
    ExtractionResult,
    evaluate_integer_expression,
    extract,
)
from vanilla_java_source import JavaField, JavaMethod, JavaSource, JavaType, descendants


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


FAMILIES = (
    FamilySpec("Entity", "net.minecraft.world.entity.Entity", None),
    FamilySpec("LivingEntity", "net.minecraft.world.entity.LivingEntity", "Entity"),
    FamilySpec("Mob", "net.minecraft.world.entity.Mob", "LivingEntity"),
    FamilySpec("Slime", "net.minecraft.world.entity.monster.Slime", "Mob"),
    FamilySpec("Shulker", "net.minecraft.world.entity.monster.Shulker", "Mob"),
    FamilySpec("Zombie", "net.minecraft.world.entity.monster.zombie.Zombie", "Mob"),
    FamilySpec("AgeableMob", "net.minecraft.world.entity.AgeableMob", "Mob"),
    FamilySpec(
        "AbstractHorse",
        "net.minecraft.world.entity.animal.equine.AbstractHorse",
        "AgeableMob",
    ),
    FamilySpec("TamableAnimal", "net.minecraft.world.entity.TamableAnimal", "AgeableMob"),
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
    FamilySpec("HangingEntity", "net.minecraft.world.entity.decoration.HangingEntity", "Entity"),
    FamilySpec("ItemFrame", "net.minecraft.world.entity.decoration.ItemFrame", "HangingEntity"),
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
    "VANILLA_SLIME_SIZE_DATA_ID": ("net.minecraft.world.entity.monster.Slime", "ID_SIZE"),
    "VANILLA_SHULKER_ATTACH_FACE_DATA_ID": ("net.minecraft.world.entity.monster.Shulker", "DATA_ATTACH_FACE_ID"),
    "VANILLA_SHULKER_PEEK_DATA_ID": ("net.minecraft.world.entity.monster.Shulker", "DATA_PEEK_ID"),
    "VANILLA_SHULKER_COLOR_DATA_ID": ("net.minecraft.world.entity.monster.Shulker", "DATA_COLOR_ID"),
    "VANILLA_ZOMBIE_BABY_DATA_ID": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_BABY_ID"),
    "VANILLA_ZOMBIE_SPECIAL_TYPE_DATA_ID": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_SPECIAL_TYPE_ID"),
    "VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_DROWNED_CONVERSION_ID"),
    "VANILLA_AGEABLE_MOB_BABY_DATA_ID": ("net.minecraft.world.entity.AgeableMob", "DATA_BABY_ID"),
    "VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID": ("net.minecraft.world.entity.AgeableMob", "AGE_LOCKED"),
    "VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID": ("net.minecraft.world.entity.animal.equine.AbstractHorse", "DATA_ID_FLAGS"),
    "VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID": ("net.minecraft.world.entity.TamableAnimal", "DATA_FLAGS_ID"),
    "VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID": ("net.minecraft.world.entity.TamableAnimal", "DATA_OWNERUUID_ID"),
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
    "VANILLA_VEHICLE_HURT_TIME_DATA_ID": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_HURT"),
    "VANILLA_VEHICLE_HURT_DIR_DATA_ID": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_HURTDIR"),
    "VANILLA_VEHICLE_DAMAGE_DATA_ID": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_DAMAGE"),
    "VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_PADDLE_LEFT"),
    "VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_PADDLE_RIGHT"),
    "VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_BUBBLE_TIME"),
    "VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID": ("net.minecraft.world.entity.vehicle.minecart.AbstractMinecart", "DATA_ID_CUSTOM_DISPLAY_BLOCK"),
    "VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID": ("net.minecraft.world.entity.vehicle.minecart.AbstractMinecart", "DATA_ID_DISPLAY_OFFSET"),
    "VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID": ("net.minecraft.world.entity.decoration.HangingEntity", "DATA_DIRECTION"),
    "VANILLA_ITEM_FRAME_ITEM_DATA_ID": ("net.minecraft.world.entity.decoration.ItemFrame", "DATA_ITEM"),
    "VANILLA_ITEM_FRAME_ROTATION_DATA_ID": ("net.minecraft.world.entity.decoration.ItemFrame", "DATA_ROTATION"),
    "VANILLA_AVATAR_MAIN_HAND_DATA_ID": ("net.minecraft.world.entity.Avatar", "DATA_PLAYER_MAIN_HAND"),
    "VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID": ("net.minecraft.world.entity.Avatar", "DATA_PLAYER_MODE_CUSTOMISATION"),
    "VANILLA_PLAYER_ABSORPTION_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_PLAYER_ABSORPTION_ID"),
    "VANILLA_PLAYER_SCORE_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_SCORE_ID"),
    "VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_SHOULDER_PARROT_LEFT"),
    "VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID": ("net.minecraft.world.entity.player.Player", "DATA_SHOULDER_PARROT_RIGHT"),
}


DEFAULT_ALIASES = {
    "VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION": ("net.minecraft.world.entity.Avatar", "DATA_PLAYER_MODE_CUSTOMISATION"),
    "VANILLA_SLIME_DEFAULT_SIZE": ("net.minecraft.world.entity.monster.Slime", "ID_SIZE"),
    "VANILLA_SHULKER_DEFAULT_ATTACH_FACE_ID": ("net.minecraft.world.entity.monster.Shulker", "DATA_ATTACH_FACE_ID"),
    "VANILLA_SHULKER_DEFAULT_PEEK": ("net.minecraft.world.entity.monster.Shulker", "DATA_PEEK_ID"),
    "VANILLA_SHULKER_DEFAULT_COLOR": ("net.minecraft.world.entity.monster.Shulker", "DATA_COLOR_ID"),
    "VANILLA_ZOMBIE_DEFAULT_BABY": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_BABY_ID"),
    "VANILLA_ZOMBIE_DEFAULT_SPECIAL_TYPE": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_SPECIAL_TYPE_ID"),
    "VANILLA_ZOMBIE_DEFAULT_DROWNED_CONVERSION": ("net.minecraft.world.entity.monster.zombie.Zombie", "DATA_DROWNED_CONVERSION_ID"),
    "VANILLA_AGEABLE_MOB_DEFAULT_BABY": ("net.minecraft.world.entity.AgeableMob", "DATA_BABY_ID"),
    "VANILLA_AGEABLE_MOB_DEFAULT_AGE_LOCKED": ("net.minecraft.world.entity.AgeableMob", "AGE_LOCKED"),
    "VANILLA_ABSTRACT_HORSE_DEFAULT_FLAGS": ("net.minecraft.world.entity.animal.equine.AbstractHorse", "DATA_ID_FLAGS"),
    "VANILLA_TAMABLE_ANIMAL_DEFAULT_FLAGS": ("net.minecraft.world.entity.TamableAnimal", "DATA_FLAGS_ID"),
    "VANILLA_ABSTRACT_ARROW_DEFAULT_FLAGS": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "ID_FLAGS"),
    "VANILLA_ABSTRACT_ARROW_DEFAULT_PIERCE_LEVEL": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "PIERCE_LEVEL"),
    "VANILLA_ABSTRACT_ARROW_DEFAULT_IN_GROUND": ("net.minecraft.world.entity.projectile.arrow.AbstractArrow", "IN_GROUND"),
    "VANILLA_ARROW_DEFAULT_EFFECT_COLOR": ("net.minecraft.world.entity.projectile.arrow.Arrow", "ID_EFFECT_COLOR"),
    "VANILLA_TRIDENT_DEFAULT_LOYALTY": ("net.minecraft.world.entity.projectile.arrow.ThrownTrident", "ID_LOYALTY"),
    "VANILLA_TRIDENT_DEFAULT_FOIL": ("net.minecraft.world.entity.projectile.arrow.ThrownTrident", "ID_FOIL"),
    "VANILLA_VEHICLE_DEFAULT_HURT_TIME": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_HURT"),
    "VANILLA_VEHICLE_DEFAULT_HURT_DIR": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_HURTDIR"),
    "VANILLA_VEHICLE_DEFAULT_DAMAGE": ("net.minecraft.world.entity.vehicle.VehicleEntity", "DATA_ID_DAMAGE"),
    "VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_LEFT": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_PADDLE_LEFT"),
    "VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_RIGHT": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_PADDLE_RIGHT"),
    "VANILLA_ABSTRACT_BOAT_DEFAULT_BUBBLE_TIME": ("net.minecraft.world.entity.vehicle.boat.AbstractBoat", "DATA_ID_BUBBLE_TIME"),
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


NAMED_FLAG_ALIASES = {
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


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("update", "check"))
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def source_for_owner(result: ExtractionResult, owner: str) -> JavaSource:
    java_type = next(item for item in result.classes if item.fqcn == owner)
    return result.sources[java_type.source_path]


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


def normalize_default(result: ExtractionResult, accessor: Accessor) -> NormalizedDefault | None:
    source, node = resolve_default_node(result, accessor)
    canonical = source.expression(node).canonical
    compact = canonical.replace(" ", "")
    serializer = accessor.serializer
    if serializer == "BOOLEAN" and node.type in {"true", "false"}:
        return NormalizedDefault(f"VanillaEntityMetadataDefault::Boolean({compact})", "bool", compact)
    if serializer in {"BYTE", "INT"}:
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
        return NormalizedDefault(f"VanillaEntityMetadataDefault::Int({value})", "i32", str(value))
    if serializer == "FLOAT" and node.type in {"decimal_floating_point_literal", "decimal_integer_literal"}:
        value = float(source.text(node).rstrip("fFdD"))
        rust = f"{value:.1f}"
        return NormalizedDefault(f"VanillaEntityMetadataDefault::Float({rust})", "f32", rust)
    optional = {
        "OPTIONAL_COMPONENT": "OptionalComponentNone",
        "OPTIONAL_BLOCK_POS": "OptionalBlockPosNone",
        "OPTIONAL_BLOCK_STATE": "OptionalBlockStateNone",
        "OPTIONAL_UNSIGNED_INT": "OptionalUnsignedIntNone",
        "OPTIONAL_LIVING_ENTITY_REFERENCE": "OptionalLivingEntityReferenceNone",
    }
    if serializer in optional and compact in {"Optional.empty()", "OptionalInt.empty()"}:
        return NormalizedDefault(f"VanillaEntityMetadataDefault::{optional[serializer]}")
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


def method_for(result: ExtractionResult, owner: str, name: str) -> tuple[JavaSource, JavaMethod]:
    source = source_for_owner(result, owner)
    matches = [item for item in source.methods_for(owner) if item.name == name]
    if len(matches) != 1:
        raise ExtractionError(f"{owner}.{name}: expected one method, found {len(matches)}")
    return source, matches[0]


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
    normalized = {
        key: normalize_default(result, item) for key, item in accessors.items()
    }
    for family in FAMILIES:
        for accessor in result.accessors:
            if accessor.owner == family.java_class and normalized[(accessor.owner, accessor.java_name)] is None:
                raise ExtractionError(f"compatibility accessor default unresolved: {accessor.owner}.{accessor.java_name}")
    flags = resolve_flags(result)
    flag_by_owner: dict[str, list[ResolvedFlag]] = {}
    for flag in flags:
        flag_by_owner.setdefault(flag.owner, []).append(flag)

    class_map = {item.fqcn: item for item in result.classes}
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

    lines = [
        "// @generated by scripts/generate-entity-metadata-schema.py; do not edit manually.",
        "// Vanilla version: 26.1.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-entity-metadata-schema.py update",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-entity-metadata-schema.py check",
        f"// Stable source manifest SHA-256: {result.manifest_digest}",
        "",
        "use bbb_protocol::entity_data_serializers::*;",
        "",
        "use super::*;",
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

    pose_ordinals = enum_ordinals(result, "net.minecraft.world.entity.HumanoidArm")
    for name in ("LEFT", "RIGHT"):
        public_name = f"VANILLA_HUMANOID_ARM_{name}_ID"
        lines.append(f"const GENERATED_{public_name}: i32 = {pose_ordinals[name]};")
        lines.append(f"pub const {public_name}: i32 = GENERATED_{public_name};")
    lines.append("")

    for public_name, key in DEFAULT_ALIASES.items():
        value = normalized[key]
        if value is None or value.scalar_type is None or value.scalar_rust is None:
            raise ExtractionError(f"{public_name}: default has no scalar compatibility value")
        lines.append(f"const GENERATED_{public_name}: {value.scalar_type} = {value.scalar_rust};")
        lines.append(f"pub const {public_name}: {value.scalar_type} = GENERATED_{public_name};")

    virtual_values = {
        "VANILLA_ENTITY_DEFAULT_MAX_AIR_SUPPLY_TICKS": integer_constants(result, "net.minecraft.world.entity.Entity")["TOTAL_AIR_SUPPLY"],
        "VANILLA_ABSTRACT_MINECART_DEFAULT_DISPLAY_OFFSET": method_return_integer(result, "net.minecraft.world.entity.vehicle.minecart.AbstractMinecart", "getDefaultDisplayOffset"),
        "VANILLA_CHEST_MINECART_DEFAULT_DISPLAY_OFFSET": method_return_integer(result, "net.minecraft.world.entity.vehicle.minecart.MinecartChest", "getDefaultDisplayOffset"),
        "VANILLA_HOPPER_MINECART_DEFAULT_DISPLAY_OFFSET": method_return_integer(result, "net.minecraft.world.entity.vehicle.minecart.MinecartHopper", "getDefaultDisplayOffset"),
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
                f"        diagnostic: {option_string(item.diagnostic)},",
                "    },",
            ]
        )
    lines.append("];")
    lines.append("")

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
