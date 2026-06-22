use super::*;

mod armor_stand;
mod auto_spin;
mod blaze;
mod boat;
mod camel;
mod chicken;
mod cow;
mod creeper;
mod death;
mod enderman;
mod endermite;
mod equine;
mod ghast;
mod goat;
mod golem;
mod happy_ghast;
mod head_look;
mod hoglin;
mod illager;
mod limb_swing;
mod llama;
mod minecart;
mod phantom;
mod pig;
mod piglin;
mod player;
mod polar_bear;
mod pufferfish;
mod ravager;
mod scale;
mod sheep;
mod silverfish;
mod skeleton;
mod sleeping;
mod slime;
mod spider;
mod upside_down;
mod villager;
mod witch;
mod wolf;
mod zombie;

#[test]
fn runtime_colored_mesh_excludes_texture_backed_entities() {
    let chicken = EntityModelInstance::chicken(303, [-2.0, 64.0, 0.0], 0.0, false);
    let sheep = EntityModelInstance::sheep(304, [0.0, 64.0, 0.0], 0.0, false);
    let wolf = EntityModelInstance::wolf(305, [2.0, 64.0, 0.0], 0.0, false);
    let boat = EntityModelInstance::boat(306, [4.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, true);
    let pig = EntityModelInstance::pig(
        307,
        [6.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    );
    let cow =
        EntityModelInstance::cow_variant(308, [8.0, 64.0, 0.0], 0.0, CowModelVariant::Warm, false);
    let player = EntityModelInstance::player(309, [10.0, 64.0, 0.0], 0.0, false);
    let creeper = EntityModelInstance::new(310, EntityModelKind::Creeper, [12.0, 64.0, 0.0], 0.0);
    let spider = EntityModelInstance::spider(311, [14.0, 64.0, 0.0], 0.0);
    let cave_spider = EntityModelInstance::cave_spider(312, [16.0, 64.0, 0.0], 0.0);
    let enderman = EntityModelInstance::enderman(313, [18.0, 64.0, 0.0], 0.0);
    let slime = EntityModelInstance::slime(314, [20.0, 64.0, 0.0], 0.0, 1);
    let magma_cube = EntityModelInstance::magma_cube(315, [22.0, 64.0, 0.0], 0.0, 3);
    let goat = EntityModelInstance::goat(316, [24.0, 64.0, 0.0], 0.0, false, true, false);
    let polar_bear = EntityModelInstance::polar_bear(317, [26.0, 64.0, 0.0], 0.0, false);
    let hoglin = EntityModelInstance::hoglin(
        318,
        [28.0, 64.0, 0.0],
        0.0,
        HoglinModelFamily::Hoglin,
        false,
    );
    let zoglin_baby =
        EntityModelInstance::hoglin(319, [30.0, 64.0, 0.0], 0.0, HoglinModelFamily::Zoglin, true);
    let ravager = EntityModelInstance::ravager(320, [32.0, 64.0, 0.0], 0.0);
    let villager = EntityModelInstance::villager(321, [34.0, 64.0, 0.0], 0.0, false);
    let wandering_trader = EntityModelInstance::wandering_trader(322, [36.0, 64.0, 0.0], 0.0);
    let iron_golem = EntityModelInstance::iron_golem(323, [38.0, 64.0, 0.0], 0.0);
    let snow_golem = EntityModelInstance::snow_golem(324, [40.0, 64.0, 0.0], 0.0);
    let witch = EntityModelInstance::witch(325, [42.0, 64.0, 0.0], 0.0);
    let colored = entity_model_colored_runtime_mesh(&[
        chicken,
        sheep,
        wolf,
        boat,
        pig,
        cow,
        player,
        creeper,
        spider,
        cave_spider,
        enderman,
        slime,
        magma_cube,
        goat,
        polar_bear,
        hoglin,
        zoglin_baby,
        ravager,
        villager,
        wandering_trader,
        iron_golem,
        snow_golem,
        witch,
    ]);
    assert!(colored.vertices.is_empty());
    assert!(colored.indices.is_empty());
    let legacy_chicken_geometry_guard = entity_model_mesh(&[chicken]);
    assert!(!legacy_chicken_geometry_guard.vertices.is_empty());
    let legacy_geometry_guard = entity_model_mesh(&[sheep]);
    assert!(!legacy_geometry_guard.vertices.is_empty());
    let legacy_wolf_geometry_guard = entity_model_mesh(&[wolf]);
    assert!(!legacy_wolf_geometry_guard.vertices.is_empty());
    let legacy_boat_geometry_guard = entity_model_mesh(&[boat]);
    assert!(!legacy_boat_geometry_guard.vertices.is_empty());
    let legacy_pig_geometry_guard = entity_model_mesh(&[pig]);
    assert!(!legacy_pig_geometry_guard.vertices.is_empty());
    let legacy_cow_geometry_guard = entity_model_mesh(&[cow]);
    assert!(!legacy_cow_geometry_guard.vertices.is_empty());
    let legacy_player_geometry_guard = entity_model_mesh(&[player]);
    assert!(!legacy_player_geometry_guard.vertices.is_empty());
    let legacy_creeper_geometry_guard = entity_model_mesh(&[creeper]);
    assert!(!legacy_creeper_geometry_guard.vertices.is_empty());
    let legacy_spider_geometry_guard = entity_model_mesh(&[spider]);
    assert!(!legacy_spider_geometry_guard.vertices.is_empty());
    let legacy_cave_spider_geometry_guard = entity_model_mesh(&[cave_spider]);
    assert!(!legacy_cave_spider_geometry_guard.vertices.is_empty());
    let legacy_enderman_geometry_guard = entity_model_mesh(&[enderman]);
    assert!(!legacy_enderman_geometry_guard.vertices.is_empty());
    let legacy_slime_geometry_guard = entity_model_mesh(&[slime]);
    assert!(!legacy_slime_geometry_guard.vertices.is_empty());
    let legacy_magma_cube_geometry_guard = entity_model_mesh(&[magma_cube]);
    assert!(!legacy_magma_cube_geometry_guard.vertices.is_empty());
    let legacy_goat_geometry_guard = entity_model_mesh(&[goat]);
    assert!(!legacy_goat_geometry_guard.vertices.is_empty());
    let legacy_polar_bear_geometry_guard = entity_model_mesh(&[polar_bear]);
    assert!(!legacy_polar_bear_geometry_guard.vertices.is_empty());
    let legacy_hoglin_geometry_guard = entity_model_mesh(&[hoglin]);
    assert!(!legacy_hoglin_geometry_guard.vertices.is_empty());
    let legacy_zoglin_baby_geometry_guard = entity_model_mesh(&[zoglin_baby]);
    assert!(!legacy_zoglin_baby_geometry_guard.vertices.is_empty());
    let legacy_ravager_geometry_guard = entity_model_mesh(&[ravager]);
    assert!(!legacy_ravager_geometry_guard.vertices.is_empty());
    let legacy_villager_geometry_guard = entity_model_mesh(&[villager]);
    assert!(!legacy_villager_geometry_guard.vertices.is_empty());
    let legacy_wandering_trader_geometry_guard = entity_model_mesh(&[wandering_trader]);
    assert!(!legacy_wandering_trader_geometry_guard.vertices.is_empty());
    let legacy_iron_golem_geometry_guard = entity_model_mesh(&[iron_golem]);
    assert!(!legacy_iron_golem_geometry_guard.vertices.is_empty());
    let legacy_snow_golem_geometry_guard = entity_model_mesh(&[snow_golem]);
    assert!(!legacy_snow_golem_geometry_guard.vertices.is_empty());
    let legacy_witch_geometry_guard = entity_model_mesh(&[witch]);
    assert!(!legacy_witch_geometry_guard.vertices.is_empty());
}

#[test]
fn entity_textured_shader_samples_bound_texture_and_discards_alpha() {
    assert!(ENTITY_MODEL_TEXTURED_SHADER
        .contains("textureSample(entity_texture_atlas, entity_sampler, input.uv)"));
    assert!(ENTITY_MODEL_TEXTURED_SHADER.contains("discard"));
    assert_eq!(
        ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2, 4 => Float32x2]
    );
}

#[test]
fn entity_textured_shader_applies_packed_light_lightmap() {
    // Mirrors the terrain lightmap: max(block, sky * 0.95) scaled into
    // 0.16..=1.0 and multiplied into the texel rgb (alpha untouched).
    assert!(ENTITY_MODEL_TEXTURED_SHADER.contains("max(input.light.x, input.light.y * 0.95)"));
    assert!(ENTITY_MODEL_TEXTURED_SHADER.contains("0.16 + light_level * 0.84"));
    assert!(ENTITY_MODEL_TEXTURED_SHADER.contains("rgb * shade"));
}

#[test]
fn entity_colored_shader_applies_packed_light_lightmap() {
    assert!(ENTITY_MODEL_SHADER.contains("max(input.light.x, input.light.y * 0.95)"));
    assert!(ENTITY_MODEL_SHADER.contains("0.16 + light_level * 0.84"));
    assert!(ENTITY_MODEL_SHADER.contains("input.color.rgb"));
}

#[test]
fn entity_shaders_apply_vanilla_overlay_texture_mix() {
    // OverlayTexture: hurt row (v < 8) mixes toward red at alpha 179/255; white
    // rows mix toward white at alpha 1 - u/15 * 0.75. Applied before the
    // lightmap, matching the vanilla entity fragment shader order.
    for shader in [ENTITY_MODEL_SHADER, ENTITY_MODEL_TEXTURED_SHADER] {
        assert!(shader.contains("input.overlay.y < 8.0"));
        assert!(shader.contains("mix(vec3<f32>(1.0, 0.0, 0.0), rgb, 179.0 / 255.0)"));
        assert!(shader.contains("1.0 - input.overlay.x / 15.0 * 0.75"));
    }
    // Eyes stay emissive and unaffected by the overlay.
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("overlay"));
}

#[test]
fn entity_eyes_shader_samples_bound_texture_without_alpha_cutout() {
    assert!(ENTITY_MODEL_EYES_SHADER
        .contains("textureSample(entity_texture_atlas, entity_sampler, input.uv)"));
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("discard"));
    assert_eq!(
        ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2, 4 => Float32x2]
    );
    // Eyes stay emissive: the lightmap shade must not dim them.
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("light_level"));
}

#[test]
fn entity_model_root_transform_rotates_instances_by_body_yaw() {
    let mesh = entity_model_mesh(&[EntityModelInstance::chicken(
        26,
        [10.0, 64.0, -3.0],
        90.0,
        false,
    )]);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [9.5, 64.001, -3.25]);
    assert_close3(max, [10.25, 64.9385, -2.75]);
}

#[test]
fn humanoid_model_families_emit_deterministic_non_empty_meshes() {
    for family in [
        HumanoidModelFamily::Player,
        HumanoidModelFamily::Zombie,
        HumanoidModelFamily::Skeleton,
        HumanoidModelFamily::Villager,
        HumanoidModelFamily::Illager,
        HumanoidModelFamily::ArmorStand,
    ] {
        let instance = EntityModelInstance::humanoid(1, [0.0, 64.0, 0.0], 0.0, family, false);
        let mesh = entity_model_mesh(&[instance]);
        let repeat = entity_model_mesh(&[instance]);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices, repeat.vertices);
        assert_eq!(mesh.indices, repeat.indices);
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn quadruped_model_families_emit_deterministic_non_empty_meshes() {
    for family in [
        QuadrupedModelFamily::Pig,
        QuadrupedModelFamily::Cow,
        QuadrupedModelFamily::Sheep,
        QuadrupedModelFamily::Horse,
        QuadrupedModelFamily::Wolf,
    ] {
        let instance = EntityModelInstance::quadruped(1, [0.0, 64.0, 0.0], 0.0, family, false);
        let mesh = entity_model_mesh(&[instance]);
        let repeat = entity_model_mesh(&[instance]);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices, repeat.vertices);
        assert_eq!(mesh.indices, repeat.indices);
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn vehicle_and_placeholder_models_emit_sane_bounds() {
    let cases = [
        EntityModelInstance::new(1, EntityModelKind::Minecart, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::new(
            2,
            EntityModelKind::Boat {
                family: BoatModelFamily::Oak,
                chest: true,
            },
            [3.0, 64.0, 0.0],
            0.0,
        ),
        EntityModelInstance::placeholder(
            3,
            [6.0, 64.0, 0.0],
            0.0,
            "todo_test_bounds",
            1.0,
            2.0,
            0.5,
        ),
    ];

    for instance in cases {
        let mesh = entity_model_mesh(&[instance]);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn entity_model_kind_exposes_stable_model_keys() {
    assert_eq!(
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
        .model_key(),
        "chicken_temperate"
    );
    assert_eq!(
        EntityModelKind::Pig {
            variant: PigModelVariant::Cold,
            baby: false
        }
        .model_key(),
        "pig_cold"
    );
    assert_eq!(
        EntityModelKind::Pig {
            variant: PigModelVariant::Warm,
            baby: true
        }
        .model_key(),
        "pig_warm_baby"
    );
    assert_eq!(
        EntityModelKind::Humanoid {
            family: HumanoidModelFamily::Zombie,
            baby: true
        }
        .model_key(),
        "humanoid_zombie_baby"
    );
    assert_eq!(
        EntityModelKind::ArmorStand {
            small: true,
            show_arms: true,
            show_base_plate: false,
            pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
        }
        .model_key(),
        "armor_stand_small"
    );
    assert_eq!(EntityModelKind::Slime { size: 4 }.model_key(), "slime");
    assert_eq!(
        EntityModelKind::MagmaCube { size: 3 }.model_key(),
        "magma_cube"
    );
    assert_eq!(
        EntityModelKind::Zombie { baby: true }.model_key(),
        "zombie_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: false
        }
        .model_key(),
        "husk"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: true
        }
        .model_key(),
        "husk_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: false
        }
        .model_key(),
        "drowned"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: true
        }
        .model_key(),
        "drowned_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: false
        }
        .model_key(),
        "zombie_villager"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: true
        }
        .model_key(),
        "zombie_villager_baby"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: false
        }
        .model_key(),
        "piglin"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: true
        }
        .model_key(),
        "piglin_baby"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: false
        }
        .model_key(),
        "piglin_brute"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: false
        }
        .model_key(),
        "zombified_piglin"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: true
        }
        .model_key(),
        "zombified_piglin_baby"
    );
    assert_eq!(EntityModelKind::Skeleton.model_key(), "skeleton");
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Stray
        }
        .model_key(),
        "stray"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Parched
        }
        .model_key(),
        "parched"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::WitherSkeleton
        }
        .model_key(),
        "wither_skeleton"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: true }
        }
        .model_key(),
        "bogged"
    );
    assert_eq!(
        EntityModelKind::Cow {
            variant: CowModelVariant::Warm,
            baby: false
        }
        .model_key(),
        "cow_warm"
    );
    assert_eq!(
        EntityModelKind::Cow {
            variant: CowModelVariant::Cold,
            baby: true
        }
        .model_key(),
        "cow_cold_baby"
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: true,
            sheared: false,
            wool_color: SheepWoolColor::White,
            invisible: false,
            jeb: false,
            age_ticks: 0.0,
        }
        .model_key(),
        "sheep_baby"
    );
    assert_eq!(
        EntityModelKind::Villager { baby: true }.model_key(),
        "villager_baby"
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.model_key(),
        "wandering_trader"
    );
    assert_eq!(
        EntityModelInstance::wolf(0, [0.0, 0.0, 0.0], 0.0, true)
            .kind
            .model_key(),
        "wolf_baby"
    );
    assert_eq!(
        EntityModelKind::Horse { baby: true }.model_key(),
        "horse_baby"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "donkey"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "donkey_baby"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "mule"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "mule_baby"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: false
        }
        .model_key(),
        "skeleton_horse"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: true
        }
        .model_key(),
        "skeleton_horse_baby"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: false
        }
        .model_key(),
        "zombie_horse"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: true
        }
        .model_key(),
        "zombie_horse_baby"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: false
        }
        .model_key(),
        "camel"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: true
        }
        .model_key(),
        "camel_baby"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            baby: true
        }
        .model_key(),
        "camel_husk"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::Creamy,
            baby: false,
            has_chest: true
        }
        .model_key(),
        "llama_creamy"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::White,
            baby: true,
            has_chest: false
        }
        .model_key(),
        "llama_white_baby"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Brown,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "trader_llama_brown"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Gray,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "trader_llama_gray_baby"
    );
    assert_eq!(
        EntityModelKind::Goat {
            baby: true,
            left_horn: false,
            right_horn: true
        }
        .model_key(),
        "goat_baby"
    );
    assert_eq!(EntityModelKind::Spider.model_key(), "spider");
    assert_eq!(EntityModelKind::CaveSpider.model_key(), "cave_spider");
    assert_eq!(EntityModelKind::Enderman.model_key(), "enderman");
    assert_eq!(EntityModelKind::IronGolem.model_key(), "iron_golem");
    assert_eq!(EntityModelKind::SnowGolem.model_key(), "snow_golem");
    assert_eq!(EntityModelKind::Witch.model_key(), "witch");
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Evoker
        }
        .model_key(),
        "evoker"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Illusioner
        }
        .model_key(),
        "illusioner"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Pillager
        }
        .model_key(),
        "pillager"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Vindicator
        }
        .model_key(),
        "vindicator"
    );
    assert_eq!(
        EntityModelKind::Placeholder {
            name: "todo_test_bounds",
            bounds: EntityModelBounds {
                width: 1.0,
                height: 1.0,
                depth: 1.0
            }
        }
        .model_key(),
        "todo_test_bounds"
    );
}

#[test]
fn sanitize_entity_model_instances_drops_non_finite_instances() {
    assert_eq!(
        sanitize_entity_model_instances(vec![
            EntityModelInstance::chicken(1, [0.0, 0.0, 0.0], 0.0, false),
            EntityModelInstance::chicken(2, [0.0, f32::NAN, 0.0], 0.0, false),
            EntityModelInstance::chicken(3, [0.0, 0.0, 0.0], f32::INFINITY, false),
        ]),
        vec![EntityModelInstance::chicken(1, [0.0, 0.0, 0.0], 0.0, false)]
    );
}

#[test]
fn entity_mesh_fills_per_instance_packed_light() {
    // pack(block 10, sky 0) -> shader light [10/15, 0].
    let dim = EntityModelInstance::placeholder(1, [0.0, 0.0, 0.0], 0.0, "dim", 1.0, 1.0, 1.0)
        .with_light_coords(10 << 4);
    // pack(block 0, sky 15) -> shader light [0, 1].
    let lit = EntityModelInstance::placeholder(2, [4.0, 0.0, 0.0], 0.0, "lit", 1.0, 1.0, 1.0)
        .with_light_coords(15 << 20);

    let mesh = entity_model_mesh(&[dim, lit]);
    assert!(!mesh.vertices.is_empty());
    // Every vertex carries one of the two per-instance lights, and both appear:
    // the post-pass assigned each entity's geometry its own sampled light.
    let dim_light = [10.0 / 15.0, 0.0];
    let lit_light = [0.0, 1.0];
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.light == dim_light || vertex.light == lit_light));
    assert!(mesh.vertices.iter().any(|vertex| vertex.light == dim_light));
    assert!(mesh.vertices.iter().any(|vertex| vertex.light == lit_light));
}

#[test]
fn entity_mesh_fills_per_instance_hurt_overlay() {
    let calm = EntityModelInstance::placeholder(1, [0.0, 0.0, 0.0], 0.0, "calm", 1.0, 1.0, 1.0);
    let hurt = EntityModelInstance::placeholder(2, [4.0, 0.0, 0.0], 0.0, "hurt", 1.0, 1.0, 1.0)
        .with_has_red_overlay(true);

    let mesh = entity_model_mesh(&[calm, hurt]);
    assert!(!mesh.vertices.is_empty());
    // Calm entities carry OverlayTexture NO_OVERLAY = [0, 10]; hurt entities the
    // red row [0, 3]. Both appear: each entity got its own overlay coords.
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.overlay == [0.0, 10.0] || vertex.overlay == [0.0, 3.0]));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == [0.0, 10.0]));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == [0.0, 3.0]));
}

#[test]
fn entity_model_vertex_layout_matches_shader_inputs() {
    let layout = entity_model_vertex_layout();

    assert_eq!(
        layout.array_stride,
        std::mem::size_of::<EntityModelVertex>() as wgpu::BufferAddress
    );
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES.len(), 4);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[0].shader_location, 0);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[1].shader_location, 1);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[2].shader_location, 2);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[3].shader_location, 3);
}

fn mesh_extents(mesh: &EntityModelMesh) -> ([f32; 3], [f32; 3]) {
    let mut vertices = mesh.vertices.iter();
    let first = vertices.next().expect("mesh has vertices").position;
    let mut min = Vec3::from_array(first);
    let mut max = Vec3::from_array(first);
    for vertex in vertices {
        let position = Vec3::from_array(vertex.position);
        min = min.min(position);
        max = max.max(position);
    }
    (min.to_array(), max.to_array())
}

fn textured_mesh_extents(mesh: &EntityModelTexturedMesh) -> ([f32; 3], [f32; 3]) {
    let mut vertices = mesh.vertices.iter();
    let first = vertices.next().expect("mesh has vertices").position;
    let mut min = Vec3::from_array(first);
    let mut max = Vec3::from_array(first);
    for vertex in vertices {
        let position = Vec3::from_array(vertex.position);
        min = min.min(position);
        max = max.max(position);
    }
    (min.to_array(), max.to_array())
}

fn assert_close3(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1.0e-4,
            "expected {expected}, got {actual}"
        );
    }
}

fn assert_close2(actual: [f32; 2], expected: [f32; 2]) {
    for (actual, expected) in actual.iter().copied().zip(expected.iter().copied()) {
        assert!(
            (actual - expected).abs() < 1.0e-4,
            "expected {expected}, got {actual}"
        );
    }
}

fn spider_texture_images() -> Vec<EntityModelTextureImage> {
    spider_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_same_geometry(actual: &EntityModelMesh, expected: &EntityModelMesh) {
    assert_eq!(actual.opaque_faces, expected.opaque_faces);
    assert_eq!(actual.indices, expected.indices);
    assert_eq!(actual.vertices.len(), expected.vertices.len());
    for (actual, expected) in actual.vertices.iter().zip(expected.vertices.iter()) {
        assert_eq!(actual.position, expected.position);
    }
}

fn assert_part(
    part: &ModelPartDesc,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &[ModelCubeDesc],
) {
    assert_eq!(part.pose.offset, offset);
    assert_eq!(part.pose.rotation, rotation);
    assert_eq!(part.cubes, cubes);
    assert!(part.children.is_empty());
}

fn assert_part_tree(
    part: &ModelPartDesc,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &[ModelCubeDesc],
    children: &[ModelPartDesc],
) {
    assert_eq!(part.pose.offset, offset);
    assert_eq!(part.pose.rotation, rotation);
    assert_eq!(part.cubes, cubes);
    assert_eq!(part.children, children);
}
