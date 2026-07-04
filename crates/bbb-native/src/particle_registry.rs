#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParticleTypeInfo {
    pub(crate) id: i32,
    pub(crate) name: &'static str,
    pub(crate) override_limiter: bool,
}

pub(crate) fn vanilla_particle_type(id: i32) -> Option<ParticleTypeInfo> {
    let index = usize::try_from(id).ok()?;
    let (name, override_limiter) = PARTICLE_TYPES_26_1.get(index).copied()?;
    let particle_type = ParticleTypeInfo {
        id,
        name,
        override_limiter,
    };
    (particle_type.id == id).then_some(particle_type)
}

#[cfg(test)]
pub(crate) fn vanilla_particle_type_count() -> usize {
    PARTICLE_TYPES_26_1.len()
}

const PARTICLE_TYPES_26_1: &[(&str, bool)] = &[
    ("minecraft:angry_villager", false),
    ("minecraft:block", false),
    ("minecraft:block_marker", true),
    ("minecraft:bubble", false),
    ("minecraft:cloud", false),
    ("minecraft:copper_fire_flame", false),
    ("minecraft:crit", false),
    ("minecraft:damage_indicator", true),
    ("minecraft:dragon_breath", false),
    ("minecraft:dripping_lava", false),
    ("minecraft:falling_lava", false),
    ("minecraft:landing_lava", false),
    ("minecraft:dripping_water", false),
    ("minecraft:falling_water", false),
    ("minecraft:dust", false),
    ("minecraft:dust_color_transition", false),
    ("minecraft:effect", false),
    ("minecraft:elder_guardian", true),
    ("minecraft:enchanted_hit", false),
    ("minecraft:enchant", false),
    ("minecraft:end_rod", false),
    ("minecraft:entity_effect", false),
    ("minecraft:explosion_emitter", true),
    ("minecraft:explosion", true),
    ("minecraft:gust", true),
    ("minecraft:small_gust", false),
    ("minecraft:gust_emitter_large", true),
    ("minecraft:gust_emitter_small", true),
    ("minecraft:sonic_boom", true),
    ("minecraft:falling_dust", false),
    ("minecraft:firework", false),
    ("minecraft:fishing", false),
    ("minecraft:flame", false),
    ("minecraft:infested", false),
    ("minecraft:cherry_leaves", false),
    ("minecraft:pale_oak_leaves", false),
    ("minecraft:tinted_leaves", false),
    ("minecraft:sculk_soul", false),
    ("minecraft:sculk_charge", true),
    ("minecraft:sculk_charge_pop", true),
    ("minecraft:soul_fire_flame", false),
    ("minecraft:soul", false),
    ("minecraft:flash", false),
    ("minecraft:happy_villager", false),
    ("minecraft:composter", false),
    ("minecraft:heart", false),
    ("minecraft:instant_effect", false),
    ("minecraft:item", false),
    ("minecraft:vibration", true),
    ("minecraft:trail", false),
    ("minecraft:pause_mob_growth", false),
    ("minecraft:reset_mob_growth", false),
    ("minecraft:item_slime", false),
    ("minecraft:item_cobweb", false),
    ("minecraft:item_snowball", false),
    ("minecraft:large_smoke", false),
    ("minecraft:lava", false),
    ("minecraft:mycelium", false),
    ("minecraft:note", false),
    ("minecraft:poof", true),
    ("minecraft:portal", false),
    ("minecraft:rain", false),
    ("minecraft:smoke", false),
    ("minecraft:white_smoke", false),
    ("minecraft:sneeze", false),
    ("minecraft:spit", true),
    ("minecraft:squid_ink", true),
    ("minecraft:sweep_attack", true),
    ("minecraft:totem_of_undying", false),
    ("minecraft:underwater", false),
    ("minecraft:splash", false),
    ("minecraft:witch", false),
    ("minecraft:bubble_pop", false),
    ("minecraft:current_down", false),
    ("minecraft:bubble_column_up", false),
    ("minecraft:nautilus", false),
    ("minecraft:dolphin", false),
    ("minecraft:campfire_cosy_smoke", true),
    ("minecraft:campfire_signal_smoke", true),
    ("minecraft:dripping_honey", false),
    ("minecraft:falling_honey", false),
    ("minecraft:landing_honey", false),
    ("minecraft:falling_nectar", false),
    ("minecraft:falling_spore_blossom", false),
    ("minecraft:ash", false),
    ("minecraft:crimson_spore", false),
    ("minecraft:warped_spore", false),
    ("minecraft:spore_blossom_air", false),
    ("minecraft:dripping_obsidian_tear", false),
    ("minecraft:falling_obsidian_tear", false),
    ("minecraft:landing_obsidian_tear", false),
    ("minecraft:reverse_portal", false),
    ("minecraft:white_ash", false),
    ("minecraft:small_flame", false),
    ("minecraft:snowflake", false),
    ("minecraft:dripping_dripstone_lava", false),
    ("minecraft:falling_dripstone_lava", false),
    ("minecraft:dripping_dripstone_water", false),
    ("minecraft:falling_dripstone_water", false),
    ("minecraft:glow_squid_ink", true),
    ("minecraft:glow", true),
    ("minecraft:wax_on", true),
    ("minecraft:wax_off", true),
    ("minecraft:electric_spark", true),
    ("minecraft:scrape", true),
    ("minecraft:shriek", false),
    ("minecraft:egg_crack", false),
    ("minecraft:dust_plume", false),
    ("minecraft:trial_spawner_detection", true),
    ("minecraft:trial_spawner_detection_ominous", true),
    ("minecraft:vault_connection", true),
    ("minecraft:dust_pillar", false),
    ("minecraft:ominous_spawning", true),
    ("minecraft:raid_omen", false),
    ("minecraft:trial_omen", false),
    ("minecraft:block_crumble", false),
    ("minecraft:firefly", false),
];

const fn particle_name_bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut index = 0;
    while index < a.len() {
        if a[index] != b[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Resolves the vanilla 26.1 particle type id (the `PARTICLE_TYPES_26_1` index)
/// for `name`. This makes the registry the single source of truth for the
/// `*_PARTICLE_TYPE_ID` constants declared via [`particle_type_ids!`]: reordering
/// the registry re-derives every id, and renaming/removing an entry that still
/// backs a constant fails the build here instead of drifting silently.
pub(crate) const fn particle_type_id(name: &str) -> i32 {
    let mut index = 0;
    while index < PARTICLE_TYPES_26_1.len() {
        if particle_name_bytes_eq(PARTICLE_TYPES_26_1[index].0.as_bytes(), name.as_bytes()) {
            return index as i32;
        }
        index += 1;
    }
    panic!("particle type name not present in PARTICLE_TYPES_26_1");
}

/// Declares `*_PARTICLE_TYPE_ID` constants paired with their canonical
/// `minecraft:` registry name in a single place. Each id is derived at compile
/// time from `PARTICLE_TYPES_26_1` via [`particle_type_id`], so the array is the
/// only source of truth. A generated test additionally reasserts every
/// `(id, name)` pairing against the registry and guards its length against
/// truncation.
macro_rules! particle_type_ids {
    (
        expect_registry_len = $len:literal;
        $( $vis:vis const $name:ident = $lit:literal; )+
    ) => {
        $(
            $vis const $name: i32 = $crate::particle_registry::particle_type_id($lit);
        )+

        #[cfg(test)]
        const PARTICLE_TYPE_ID_NAME_PAIRS: &[(i32, &str)] = &[
            $( ($name, $lit), )+
        ];

        #[cfg(test)]
        #[test]
        fn particle_type_id_constants_match_registry() {
            assert_eq!(
                $crate::particle_registry::vanilla_particle_type_count(),
                $len,
                "PARTICLE_TYPES_26_1 length changed (possible truncation); \
                 re-verify the particle type id constants and update expect_registry_len",
            );
            for &(value, name) in PARTICLE_TYPE_ID_NAME_PAIRS {
                let info = $crate::particle_registry::vanilla_particle_type(value)
                    .expect("particle type id constant out of registry range");
                assert_eq!(
                    info.name, name,
                    "constant declared for {name:?} resolved to id {value} = {:?}",
                    info.name,
                );
            }
        }
    };
}
pub(crate) use particle_type_ids;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_particle_registry_matches_26_1_order() {
        assert_eq!(vanilla_particle_type_count(), 117);
        assert_particle(0, "minecraft:angry_villager", false);
        assert_particle(1, "minecraft:block", false);
        assert_particle(2, "minecraft:block_marker", true);
        assert_particle(14, "minecraft:dust", false);
        assert_particle(32, "minecraft:flame", false);
        assert_particle(47, "minecraft:item", false);
        assert_particle(48, "minecraft:vibration", true);
        assert_particle(105, "minecraft:shriek", false);
        assert_particle(111, "minecraft:dust_pillar", false);
        assert_particle(115, "minecraft:block_crumble", false);
        assert_particle(116, "minecraft:firefly", false);
        assert!(vanilla_particle_type(117).is_none());
    }

    fn assert_particle(id: i32, name: &'static str, override_limiter: bool) {
        assert_eq!(
            vanilla_particle_type(id),
            Some(ParticleTypeInfo {
                id,
                name,
                override_limiter,
            })
        );
    }
}
