from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import sys
import tempfile
import unittest
from collections import Counter
from dataclasses import replace
from pathlib import Path
from unittest.mock import patch

import pytest

pytestmark = pytest.mark.vanilla

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location(
    "generate_server_entity_runtime_facts",
    SCRIPTS / "generate-server-entity-runtime-facts.py",
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)


class GenerateServerEntityRuntimeFactsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.args = GENERATOR.parse_args([])
        cls.extracted = GENERATOR.extract(cls.args.source_root)
        cls.rows = GENERATOR.runtime_facts(cls.extracted)
        cls.parrot_sounds = GENERATOR.parrot_imitation_facts(cls.extracted, cls.args.source_root)
        cls.generated = GENERATOR.render_rows(cls.rows, cls.parrot_sounds)
        cls.by_resource = {row.resource_id: row for row in cls.rows}
        cls.pick_rows = GENERATOR.pick_facts(cls.extracted, cls.args.source_root)
        cls.picks = {row.resource_id: row for row in cls.pick_rows}
        cls.ambient_rows = GENERATOR.spawn_ambient_facts(cls.extracted, cls.args.source_root)

    def test_parrot_imitation_map_is_source_derived_including_shared_and_empty_sounds(self) -> None:
        by_id = {row.entity_type_id: row.resource_id for row in self.rows}
        sounds = {by_id[entity]: sound for entity, sound in self.parrot_sounds}
        self.assertEqual(len(sounds), 42)
        self.assertEqual(sounds["minecraft:cave_spider"], sounds["minecraft:spider"])
        self.assertEqual(sounds["minecraft:happy_ghast"], "minecraft:intentionally_empty")
        source = GENERATOR.source_for_owner(
            self.extracted, "net.minecraft.world.entity.animal.parrot.Parrot"
        )
        relative = source.relative_path
        sources = dict(self.extracted.sources)
        sources[relative] = GENERATOR.parse_source(
            relative,
            source.source.replace(
                b"map.put(EntityType.CAVE_SPIDER, SoundEvents.PARROT_IMITATE_SPIDER)",
                b"map.put(EntityType.CAVE_SPIDER, SoundEvents.PARROT_AMBIENT)",
            ),
        )
        changed = GENERATOR.parrot_imitation_facts(
            replace(self.extracted, sources=sources), self.args.source_root
        )
        self.assertEqual(
            dict(changed)[self.by_resource["minecraft:cave_spider"].entity_type_id],
            "minecraft:entity.parrot.ambient",
        )

    def test_session_pose_eye_constants_match_vanilla_fields(self) -> None:
        def initializer(owner: str, field_name: str) -> str:
            source = GENERATOR.source_for_owner(self.extracted, owner)
            field = next(f for f in source.fields_for(owner) if f.name == field_name)
            return "".join(source.text(field.initializer.node).split())

        avatar = "net.minecraft.world.entity.Avatar"
        poses = initializer(avatar, "POSES")
        for pose, width, height, eye in [
            ("CROUCHING", "0.6", "1.5", "1.27"),
            ("FALL_FLYING", "0.6", "0.6", "0.4"),
            ("SWIMMING", "0.6", "0.6", "0.4"),
            ("SPIN_ATTACK", "0.6", "0.6", "0.4"),
        ]:
            self.assertIn(
                f".put(Pose.{pose},EntityDimensions.scalable({width}F,{height}F)"
                f".withEyeHeight({eye}F)", poses
            )
        self.assertIn(".withEyeHeight(1.62F)", initializer(avatar, "STANDING_DIMENSIONS"))
        self.assertIn(
            ".put(Pose.DYING,EntityDimensions.fixed(0.2F,0.2F).withEyeHeight(1.62F))", poses
        )
        self.assertIn(".put(Pose.SLEEPING,SLEEPING_DIMENSIONS)", poses)
        self.assertEqual(
            initializer("net.minecraft.world.entity.LivingEntity", "SLEEPING_DIMENSIONS"),
            "EntityDimensions.fixed(0.2F,0.2F).withEyeHeight(0.2F)",
        )

    def test_sniffer_runtime_constants_match_vanilla_members(self) -> None:
        def member(source, owner: str, name: str) -> str:
            method, = [m for m in source.methods_for(owner) if m.name == name]
            return "".join(source.text(method.node).split())

        owner = "net.minecraft.world.entity.animal.sniffer.Sniffer"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        fields = {
            field.name: "".join(source.text(field.initializer.node).split())
            for field in source.fields_for(owner) if field.initializer is not None
        }
        self.assertEqual(fields["DIGGING_DROP_SEED_OFFSET_TICKS"], "120")
        self.assertEqual(fields["DIGGING_BB_HEIGHT_OFFSET"], "0.4F")
        self.assertIn("getHeight()-0.4F", fields["DIGGING_DIMENSIONS"])
        self.assertIn("withEyeHeight(0.81F)", fields["DIGGING_DIMENSIONS"])
        for name, expression in (
            ("onDiggingStart", "this.tickCount+120"),
            ("getHeadPosition", "this.getForward().scale(2.25)"),
            ("getHeadBlock", "this.getY()+0.2F"),
            ("storeExploredPosition", "this.getExploredPositions().limit(20L)"),
            ("calculateDigPosition", "LandRandomPos.getPos(this,10+2*idx,3)"),
        ):
            self.assertIn(expression, member(source, owner, name))

        owner = "net.minecraft.world.entity.animal.sniffer.SnifferAi"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        for expression in ("newSnifferAi.Digging(160,180)", "newSnifferAi.FinishedDigging(40)"):
            self.assertIn(expression, member(source, owner, "initDigActivity"))
        for expression in (
            "newSnifferAi.FeelingHappy(40,100)", "newSnifferAi.Scenting(40,80)",
            "newSnifferAi.Sniffing(40,80)", "sniffer->1.25F",
            "sniffer.isBaby()?2.5:3.5",
        ):
            self.assertIn(expression, member(source, owner, "initIdleActivity"))
        nested = {kind.simple_name: kind.fqcn for kind in source.types}
        self.assertIn(
            "setMemoryWithExpiry(MemoryModuleType.SNIFF_COOLDOWN,Unit.INSTANCE,9600L)",
            member(source, nested["Digging"], "stop"),
        )
        self.assertIn(
            "newWalkTarget(position,1.25F,0)", member(source, nested["Sniffing"], "stop")
        )
        self.assertIn(
            ",600);", member(source, nested["Searching"], "Searching")
        )

    def test_copper_golem_weathering_constants_match_vanilla_members(self) -> None:
        owner = "net.minecraft.world.entity.animal.golem.CopperGolem"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        fields = {
            field.name: "".join(source.text(field.initializer.node).split())
            for field in source.fields_for(owner) if field.initializer is not None
        }
        self.assertEqual(fields["TURN_TO_STATUE_CHANCE"], "0.0058F")
        methods = {
            method.name: "".join(source.text(method.node).split())
            for method in source.methods_for(owner)
        }
        for expression in (
            "this.nextWeatheringTick!=-2L", "this.nextWeatheringTick==-1L",
            "gameTime+random.nextIntBetweenInclusive(504000,552000)",
            "this.nextWeatheringTick+random.nextIntBetweenInclusive(504000,552000)",
            "isNewStateFullyOxidized?0L:",
        ):
            self.assertIn(expression, methods["updateWeathering"])
        self.assertIn("nextFloat()<=0.0058F", methods["canTurnToStatue"])

    def test_horizontal_facing_yaw_matches_vanilla_direction(self) -> None:
        owner = "net.minecraft.core.Direction"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        method, = [m for m in source.methods_for(owner) if m.name == "getYRot"]
        body = "".join(source.text(method.node).split())
        for expression in (
            "caseNORTH->180.0F;", "caseSOUTH->0.0F;",
            "caseWEST->90.0F;", "caseEAST->-90.0F;",
        ):
            self.assertIn(expression, body)

    def test_strider_goal_parameters_match_vanilla(self) -> None:
        owner = "net.minecraft.world.entity.monster.Strider"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        method, = [m for m in source.methods_for(owner) if m.name == "registerGoals"]
        goals = "".join(source.text(method.node).split())
        for expression in (
            "newPanicGoal(this,1.65)",
            "newTemptGoal(this,1.4,i->i.is(ItemTags.STRIDER_TEMPT_ITEMS),false)",
            "newStrider.StriderGoToLavaGoal(this,1.0)",
            "newFollowParentGoal(this,1.0)",
            "newRandomStrollGoal(this,1.0,60)",
        ):
            self.assertIn(expression, goals)
        owner = next(
            kind.fqcn for kind in source.types if kind.simple_name == "StriderGoToLavaGoal"
        )
        constructor, = [m for m in source.methods_for(owner) if m.name == "StriderGoToLavaGoal"]
        self.assertIn(
            "super(strider,speedModifier,8,2)", "".join(source.text(constructor.node).split())
        )

    def test_creaking_home_distance_matches_vanilla_node_evaluator(self) -> None:
        owner = "net.minecraft.world.entity.monster.creaking.Creaking"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        owner = next(kind.fqcn for kind in source.types if kind.simple_name == "HomeNodeEvaluator")
        field, = [f for f in source.fields_for(owner) if f.name == "MAX_DISTANCE_TO_HOME_SQ"]
        self.assertEqual("".join(source.text(field.initializer.node).split()), "1024")
        method, = [m for m in source.methods_for(owner) if m.name == "getPathType"]
        self.assertIn(
            "homeDistance>1024.0&&homeDistance>=homePos.distSqr(context.mobPosition())",
            "".join(source.text(method.node).split()),
        )

    def test_shulker_teleport_parameters_match_vanilla(self) -> None:
        owner = "net.minecraft.world.entity.monster.Shulker"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        methods = {
            method.name: "".join(source.text(method.node).split())
            for method in source.methods_for(owner)
        }
        self.assertIn(
            "this.getHealth()<this.getMaxHealth()*0.5&&this.random.nextInt(4)==0",
            methods["hurtServer"],
        )
        teleport = methods["teleportSomewhere"]
        for expression in (
            "attempt<5",
            "Mth.randomBetweenInclusive(this.random,-8,8)",
            "newAABB(target).deflate(1.0E-6)",
        ):
            self.assertIn(expression, teleport)
        self.assertIn(
            "0.5F-Mth.sin((0.5F+amount)*(float)Math.PI)*0.5F",
            methods["getPhysicalPeek"],
        )
        for expression in (
            "getRawPeekAmount()*0.01F", "currentPeekAmount-0.05F",
            "currentPeekAmount+0.05F",
        ):
            self.assertIn(expression, methods["updatePeekAmount"])
        self.assertIn("Math.min(scale,3.0F)", methods["sanitizeScale"])
        for expression in (
            "oldAabb.inflate(8.0)", "(shulkerCount-1)/5.0F",
            "EntitySpawnReason.BREEDING", "baby.setVariant(this.getVariant())",
        ):
            self.assertIn(expression, methods["hitByShulkerBullet"])

    def test_zombie_turtle_egg_parameters_match_vanilla(self) -> None:
        owner = "net.minecraft.world.entity.monster.zombie.Zombie"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        method, = [m for m in source.methods_for(owner) if m.name == "registerGoals"]
        self.assertIn(
            "addGoal(4,newZombie.ZombieAttackTurtleEggGoal(this,1.0,3))",
            "".join(source.text(method.node).split()),
        )
        owner = next(
            kind.fqcn for kind in source.types if kind.simple_name == "ZombieAttackTurtleEggGoal"
        )
        methods = {
            method.name: "".join(source.text(method.node).split())
            for method in source.methods_for(owner)
        }
        self.assertIn("return1.14", methods["acceptedDistance"])
        self.assertIn("SoundSource.HOSTILE,0.5F,0.9F+", methods["playDestroyProgressSound"])
        self.assertIn("SoundSource.BLOCKS,0.7F,0.9F+", methods["playBreakSound"])
        owner = "net.minecraft.world.entity.ai.goal.RemoveBlockGoal"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        methods = {
            method.name: "".join(source.text(method.node).split())
            for method in source.methods_for(owner)
        }
        self.assertIn("super(mob,speedModifier,24,verticalSearchRange)", methods["RemoveBlockGoal"])
        for expression in (
            "setDeltaMovement(movement.x,0.3,movement.z)",
            "setDeltaMovement(movement.x,-0.3,movement.z)",
            "ticksSinceReachedGoal>60", "level.removeBlock(eatPos,false)",
        ):
            self.assertIn(expression, methods["tick"])

    def test_drowned_navigation_and_trident_parameters_match_vanilla(self) -> None:
        owner = "net.minecraft.world.entity.monster.zombie.Drowned"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        methods = {
            method.name: "".join(source.text(method.node).split())
            for method in source.methods_for(owner)
        }
        self.assertIn("newAmphibiousPathNavigation(this,level)", methods["createNavigation"])
        self.assertIn("this.moveRelative(0.01F,input)", methods["travelInWater"])
        self.assertIn(".scale(0.9)", methods["travelInWater"])
        for expression in (
            "addGoal(1,newDrowned.DrownedGoToWaterGoal(this,1.0))",
            "addGoal(2,newDrowned.DrownedTridentAttackGoal(this,1.0,40,10.0F))",
            "addGoal(5,newDrowned.DrownedGoToBeachGoal(this,1.0))",
            "addGoal(6,newDrowned.DrownedSwimUpGoal(this,1.0,this.level().getSeaLevel()))",
            "addGoal(7,newRandomStrollGoal(this,1.0))",
        ):
            self.assertIn(expression, methods["addBehaviourGoals"])
        for expression in (
            "mainHandItem.is(Items.TRIDENT)?mainHandItem:newItemStack(Items.TRIDENT)",
            "newThrownTrident(this.level(),this,tridentItemStack)",
            "target.getY(0.3333333333333333)-trident.getY()",
            "yd+distanceToTarget*0.2F,zd,1.6F,14-this.level().getDifficulty().getId()*4",
            "SoundEvents.DROWNED_SHOOT,1.0F,1.0F/(this.getRandom().nextFloat()*0.4F+0.8F)",
        ):
            self.assertIn(expression, methods["performRangedAttack"])
        self.assertIn("sqrDistToNextPos<4.0", methods["closeToNextPos"])
        for name, method_name, expression in (
            (
                "DrownedGoToWaterGoal", "getWaterPos",
                "pos.offset(random.nextInt(20)-10,2-random.nextInt(8),random.nextInt(20)-10)",
            ),
            ("DrownedGoToWaterGoal", "getWaterPos", "i<10"),
            ("DrownedTridentAttackGoal", "canUse", "getMainHandItem().is(Items.TRIDENT)"),
            ("DrownedTridentAttackGoal", "start", "startUsingItem(InteractionHand.MAIN_HAND)"),
            ("DrownedTridentAttackGoal", "stop", "stopUsingItem()"),
            ("DrownedGoToBeachGoal", "DrownedGoToBeachGoal", "super(drowned,speedModifier,8,2)"),
            ("DrownedGoToBeachGoal", "canUse", "getSeaLevel()-3"),
            ("DrownedSwimUpGoal", "canUse", "getY()<this.seaLevel-2"),
            ("DrownedSwimUpGoal", "tick", "this.drowned,4,8,newVec3"),
            ("DrownedSwimUpGoal", "tick", "this.seaLevel-1"),
        ):
            inner = next(kind.fqcn for kind in source.types if kind.simple_name == name)
            method, = [m for m in source.methods_for(inner) if m.name == method_name]
            self.assertIn(expression, "".join(source.text(method.node).split()))
        control = next(
            kind.fqcn for kind in source.types if kind.simple_name == "DrownedMoveControl"
        )
        method, = [m for m in source.methods_for(control) if m.name == "tick"]
        tick = "".join(source.text(method.node).split())
        for expression in (
            ".add(0.0,0.002,0.0)", ".add(0.0,-0.008,0.0)",
            "Mth.lerp(0.125F", "newSpeed*xd*0.005", "newSpeed*yd*0.1",
        ):
            self.assertIn(expression, tick)
        owner = "net.minecraft.world.entity.LivingEntity"
        source = GENERATOR.source_for_owner(self.extracted, owner)
        method, = [m for m in source.methods_for(owner) if m.name == "travelInWater"]
        travel = "".join(source.text(method.node).split())
        for expression in (
            "?0.9F:", "speed=0.02F", "waterWalker*=0.5F", "0.54600006F",
            "slowDown=0.96F", "multiply(slowDown,0.8F,slowDown)",
        ):
            self.assertIn(expression, travel)
        method, = [
            m for m in source.methods_for(owner) if m.name == "getFluidFallingAdjustedMovement"
        ]
        self.assertIn("baseGravity/16.0", "".join(source.text(method.node).split()))
        method, = [m for m in source.methods_for(owner) if m.name == "jumpOutOfFluid"]
        self.assertIn("0.3F", source.text(method.node))

    def test_pick_dispatch_covers_all_registered_types_and_dynamic_fallbacks(self) -> None:
        self.assertEqual([r.entity_type_id for r in self.pick_rows], list(range(157)))
        self.assertEqual(
            Counter(r.kind for r in self.pick_rows),
            {"None": 35, "Mob": 89, "Item": 31, "ItemFrame": 2},
        )
        self.assertEqual(
            {r.resource_id for r in self.pick_rows if r.kind == "Mob" and r.item_id is None},
            {"minecraft:giant", "minecraft:illusioner"},
        )
        prototypes = json.loads(GENERATOR.ITEM_PROTOTYPES.read_text())
        resources = prototypes["item_resource_ids"]
        expected = {
            "minecraft:ender_dragon": "minecraft:ender_dragon_spawn_egg",
            "minecraft:spawner_minecart": "minecraft:minecart",
            "minecraft:painting": "minecraft:painting",
            "minecraft:glow_item_frame": "minecraft:glow_item_frame",
            "minecraft:item_frame": "minecraft:item_frame",
            "minecraft:acacia_chest_boat": "minecraft:acacia_chest_boat",
            "minecraft:bamboo_raft": "minecraft:bamboo_raft",
            "minecraft:leash_knot": "minecraft:lead",
        }
        for entity, item in expected.items():
            self.assertEqual(resources[self.picks[entity].item_id], item)
        for entity in [
            "minecraft:player",
            "minecraft:mannequin",
            "minecraft:item",
            "minecraft:falling_block",
        ]:
            self.assertEqual(self.picks[entity].kind, "None")

    def test_mob_pick_uses_component_membership_not_resource_name(self) -> None:
        prototype = json.loads(GENERATOR.ITEM_PROTOTYPES.read_text())
        component = next(
            c for c in prototype["component_values"] if c["resource_id"] == "minecraft:entity_data"
        )
        value = next(v for v in component["values"] if v["value"]["id"] == "minecraft:creeper")
        value["item_id_ranges"] = [[1, 1]]
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / "prototypes.json"
            artifact.write_text(json.dumps(prototype))
            with patch.object(GENERATOR, "ITEM_PROTOTYPES", artifact):
                rows = GENERATOR.pick_facts(self.extracted, self.args.source_root)
                self.assertEqual(
                    next(r.item_id for r in rows if r.resource_id == "minecraft:creeper"), 1
                )
                value["item_id_ranges"] = [[1, 2]]
                artifact.write_text(json.dumps(prototype))
                with self.assertRaisesRegex(ValueError, "ambiguous"):
                    GENERATOR.pick_facts(self.extracted, self.args.source_root)

    def test_boat_supplier_and_frame_fallback_are_source_derived(self) -> None:
        sources = dict(self.extracted.sources)
        replacements = {
            "net/minecraft/world/entity/EntityType.java": (
                b"boatFactory(() -> Items.ACACIA_BOAT)",
                b"boatFactory(() -> Items.BIRCH_BOAT)",
            ),
            "net/minecraft/world/entity/decoration/GlowItemFrame.java": (
                b"Items.GLOW_ITEM_FRAME",
                b"Items.LEAD",
            ),
        }
        for relative, (before, after) in replacements.items():
            source = sources[relative]
            self.assertIn(before, source.source)
            sources[relative] = GENERATOR.parse_source(
                relative, source.source.replace(before, after)
            )
        rows = GENERATOR.pick_facts(replace(self.extracted, sources=sources), self.args.source_root)
        picks = {r.resource_id: r for r in rows}
        self.assertEqual(
            picks["minecraft:acacia_boat"].item_id, picks["minecraft:birch_boat"].item_id
        )
        self.assertEqual(
            picks["minecraft:glow_item_frame"].item_id, picks["minecraft:leash_knot"].item_id
        )

    def test_changed_dynamic_pick_behavior_fails_closed(self) -> None:
        sources = dict(self.extracted.sources)
        relative = "net/minecraft/world/entity/decoration/ItemFrame.java"
        source = sources[relative]
        sources[relative] = GENERATOR.parse_source(
            relative, source.source.replace(b"framedStack.copy()", b"framedStack.copyWithCount(1)")
        )
        with self.assertRaisesRegex(ValueError, "unsupported pick behavior"):
            GENERATOR.pick_facts(replace(self.extracted, sources=sources), self.args.source_root)

    def test_default_attributes_membership_defines_90_general_living_rows(self) -> None:
        living = [row for row in self.rows if row.is_living]
        general = [
            row
            for row in living
            if row.resource_id not in GENERATOR.GENERAL_LIVING_EXCLUSIONS
        ]
        self.assertEqual(len(self.rows), 157)
        self.assertEqual(len(living), 92)
        self.assertEqual(len(general), 90)
        self.assertFalse(self.by_resource["minecraft:arrow"].is_living)
        self.assertTrue(self.by_resource["minecraft:player"].is_living)
        self.assertTrue(self.by_resource["minecraft:armor_stand"].is_living)

    def test_spawn_ambient_static_inheritance_and_unknown_selectors(self) -> None:
        rows = {row.resource_id: row for row in self.ambient_rows}
        self.assertEqual(
            (
                rows["minecraft:piglin_brute"].kind,
                rows["minecraft:piglin_brute"].sound,
                rows["minecraft:piglin_brute"].volume,
            ),
            ("Fixed", "minecraft:entity.piglin_brute.ambient", "1.0"),
        )
        self.assertEqual(
            (
                rows["minecraft:mooshroom"].kind,
                rows["minecraft:mooshroom"].sound,
                rows["minecraft:mooshroom"].volume,
            ),
            ("Fixed", "minecraft:entity.cow.ambient", "0.4"),
        )
        self.assertEqual(rows["minecraft:trader_llama"].volume, "0.8")
        self.assertEqual(rows["minecraft:trader_llama"].sound, "minecraft:entity.llama.ambient")
        self.assertEqual(rows["minecraft:creeper"].kind, "Silent")
        for resource in ["cow", "shulker", "happy_ghast", "zombie_villager"]:
            self.assertEqual(rows["minecraft:" + resource].kind, "Dynamic")
        self.assertNotIn("minecraft:minecart", rows)
        sources = dict(self.extracted.sources)
        relative = "net/minecraft/world/entity/monster/piglin/PiglinBrute.java"
        original = sources[relative]
        sources[relative] = GENERATOR.parse_source(
            relative,
            original.source.replace(
                b"return SoundEvents.PIGLIN_BRUTE_AMBIENT;", b"return chooseUnknownSound();"
            ),
        )
        relative = "net/minecraft/world/entity/animal/cow/CowSoundVariants.java"
        original = sources[relative]
        sources[relative] = GENERATOR.parse_source(
            relative,
            original.source.replace(
                b'CLASSIC("classic", "cow")', b'CLASSIC("classic", "changed_cow")'
            ),
        )
        changed = GENERATOR.spawn_ambient_facts(
            replace(self.extracted, sources=sources), self.args.source_root
        )
        self.assertEqual(
            next(r.kind for r in changed if r.resource_id == "minecraft:piglin_brute"), "Dynamic"
        )
        self.assertEqual(
            next(r.sound for r in changed if r.resource_id == "minecraft:mooshroom"),
            "minecraft:entity.changed_cow.ambient",
        )

    def test_checked_in_output_is_exact_and_check_fails_closed(self) -> None:
        self.assertEqual(GENERATOR.DEFAULT_OUTPUT.read_text(encoding="utf-8"), self.generated)
        self.assertEqual(
            GENERATOR.DEFAULT_PICK_OUTPUT.read_text(encoding="utf-8"),
            GENERATOR.render_pick_rows(self.pick_rows),
        )
        self.assertEqual(
            GENERATOR.DEFAULT_AMBIENT_OUTPUT.read_text(encoding="utf-8"),
            GENERATOR.render_spawn_ambient_rows(self.ambient_rows),
        )
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "facts.rs"
            output.write_text(self.generated, encoding="utf-8")
            arguments = [
                "--vanilla-root",
                str(self.args.source_root.parents[1]),
                "--output",
                str(output),
                "--check",
            ]
            self.assertEqual(GENERATOR.main(arguments), 0)
            output.write_text(
                self.generated.replace("minecraft:allay", "minecraft:fake", 1),
                encoding="utf-8",
            )
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(GENERATOR.main(arguments), 1)


if __name__ == "__main__":
    unittest.main()
