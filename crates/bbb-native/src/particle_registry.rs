#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParticleTypeInfo {
    pub(crate) id: i32,
    pub(crate) name: &'static str,
    pub(crate) override_limiter: bool,
}

pub(crate) fn vanilla_particle_type(id: i32) -> Option<ParticleTypeInfo> {
    let particle_type = bbb_protocol::particle_types::vanilla_particle_type(id)?;
    Some(ParticleTypeInfo {
        id: particle_type.id,
        name: particle_type.resource_id,
        override_limiter: particle_type.override_limiter,
    })
}

#[cfg(test)]
pub(crate) fn vanilla_particle_type_count() -> usize {
    bbb_protocol::particle_types::VANILLA_PARTICLE_TYPES.len()
}

/// Resolves the vanilla 26.1 particle type id for `name` through the generated
/// protocol catalog. This makes vanilla registration metadata the single source
/// of truth for the `*_PARTICLE_TYPE_ID` constants declared via
/// [`particle_type_ids!`]: reordering the registry re-derives every id, and
/// renaming/removing an entry that still backs a constant fails the build here.
pub(crate) const fn particle_type_id(name: &str) -> i32 {
    match bbb_protocol::particle_types::vanilla_particle_type_id_for_resource_id(name) {
        Some(id) => id,
        None => panic!("particle type name not present in generated catalog"),
    }
}

/// Declares `*_PARTICLE_TYPE_ID` constants paired with their canonical
/// `minecraft:` registry name in a single place. Each id is derived at compile
/// time from the protocol catalog via [`particle_type_id`]. A generated test
/// reasserts every `(id, name)` pairing and guards its length against truncation.
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
                "generated particle catalog length changed (possible truncation); \
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
