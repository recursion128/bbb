use super::catalog::*;
use super::SheepHeadEatPose;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelInstance {
    pub entity_id: i32,
    pub kind: EntityModelKind,
    pub position: [f32; 3],
    pub y_rot: f32,
    /// Per-frame sheep eat-grass head pose. [`SheepHeadEatPose::NONE`] for every
    /// other entity and for a sheep that is not currently eating.
    pub head_eat: SheepHeadEatPose,
}

impl EntityModelInstance {
    pub fn new(entity_id: i32, kind: EntityModelKind, position: [f32; 3], y_rot: f32) -> Self {
        Self {
            entity_id,
            kind,
            position,
            y_rot,
            head_eat: SheepHeadEatPose::NONE,
        }
    }

    pub fn with_head_eat(mut self, head_eat: SheepHeadEatPose) -> Self {
        self.head_eat = head_eat;
        self
    }

    pub fn chicken(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::chicken_variant(
            entity_id,
            position,
            y_rot,
            ChickenModelVariant::Temperate,
            baby,
        )
    }

    pub fn chicken_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: ChickenModelVariant,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Chicken { variant, baby },
            position,
            y_rot,
        )
    }

    pub fn pig(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: PigModelVariant,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Pig { variant, baby },
            position,
            y_rot,
        )
    }

    pub fn player(entity_id: i32, position: [f32; 3], y_rot: f32, slim: bool) -> Self {
        Self::player_with_parts(
            entity_id,
            position,
            y_rot,
            slim,
            PLAYER_MODEL_PARTS_ALL_VISIBLE,
        )
    }

    pub fn player_with_parts(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        slim: bool,
        parts: PlayerModelPartVisibility,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Player { slim, parts },
            position,
            y_rot,
        )
    }

    pub fn humanoid(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: HumanoidModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Humanoid { family, baby },
            position,
            y_rot,
        )
    }

    pub fn armor_stand(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        small: bool,
        show_arms: bool,
        show_base_plate: bool,
        pose: ArmorStandModelPose,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::ArmorStand {
                small,
                show_arms,
                show_base_plate,
                pose,
            },
            position,
            y_rot,
        )
    }

    pub fn slime(entity_id: i32, position: [f32; 3], y_rot: f32, size: i32) -> Self {
        Self::new(entity_id, EntityModelKind::Slime { size }, position, y_rot)
    }

    pub fn magma_cube(entity_id: i32, position: [f32; 3], y_rot: f32, size: i32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::MagmaCube { size },
            position,
            y_rot,
        )
    }

    pub fn zombie(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Zombie { baby }, position, y_rot)
    }

    pub fn zombie_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: ZombieVariantModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::ZombieVariant { family, baby },
            position,
            y_rot,
        )
    }

    pub fn piglin(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: PiglinModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Piglin { family, baby },
            position,
            y_rot,
        )
    }

    pub fn hoglin(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: HoglinModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Hoglin { family, baby },
            position,
            y_rot,
        )
    }

    pub fn ravager(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Ravager, position, y_rot)
    }

    pub fn boat(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: BoatModelFamily,
        chest: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Boat { family, chest },
            position,
            y_rot,
        )
    }

    pub fn skeleton(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Skeleton, position, y_rot)
    }

    pub fn skeleton_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: SkeletonModelFamily,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::SkeletonVariant { family },
            position,
            y_rot,
        )
    }

    pub fn cow(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::cow_variant(entity_id, position, y_rot, CowModelVariant::Temperate, baby)
    }

    pub fn cow_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: CowModelVariant,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Cow { variant, baby },
            position,
            y_rot,
        )
    }

    pub fn sheep(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::sheep_wool(
            entity_id,
            position,
            y_rot,
            baby,
            false,
            SheepWoolColor::White,
        )
    }

    pub fn sheep_wool(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        sheared: bool,
        wool_color: SheepWoolColor,
    ) -> Self {
        Self::sheep_render_state(
            entity_id, position, y_rot, baby, sheared, wool_color, false, false, 0.0,
        )
    }

    pub fn sheep_render_state(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        sheared: bool,
        wool_color: SheepWoolColor,
        invisible: bool,
        jeb: bool,
        age_ticks: f32,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Sheep {
                baby,
                sheared,
                wool_color,
                invisible,
                jeb,
                age_ticks,
            },
            position,
            y_rot,
        )
    }

    #[cfg(test)]
    pub fn sheep_eating(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        sheared: bool,
        wool_color: SheepWoolColor,
        eat_animation_tick: i32,
        partial_tick: f32,
    ) -> Self {
        Self::sheep_wool(entity_id, position, y_rot, baby, sheared, wool_color).with_head_eat(
            SheepHeadEatPose::from_eat_tick(eat_animation_tick, partial_tick),
        )
    }

    pub fn villager(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Villager { baby },
            position,
            y_rot,
        )
    }

    pub fn wandering_trader(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::WanderingTrader, position, y_rot)
    }

    pub fn wolf(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::wolf_state(entity_id, position, y_rot, baby, false, false, false, None)
    }

    pub fn wolf_state(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        tame: bool,
        angry: bool,
        invisible: bool,
        collar_color: Option<EntityDyeColor>,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Wolf {
                baby,
                tame,
                angry,
                invisible,
                collar_color: tame.then_some(collar_color).flatten(),
            },
            position,
            y_rot,
        )
    }

    pub fn horse(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Horse { baby }, position, y_rot)
    }

    pub fn donkey(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: DonkeyModelFamily,
        baby: bool,
        has_chest: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Donkey {
                family,
                baby,
                has_chest,
            },
            position,
            y_rot,
        )
    }

    pub fn undead_horse(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: UndeadHorseModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::UndeadHorse { family, baby },
            position,
            y_rot,
        )
    }

    pub fn camel(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: CamelModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Camel { family, baby },
            position,
            y_rot,
        )
    }

    pub fn llama(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: LlamaModelFamily,
        variant: LlamaVariant,
        baby: bool,
        has_chest: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Llama {
                family,
                variant,
                baby,
                has_chest,
            },
            position,
            y_rot,
        )
    }

    pub fn goat(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        left_horn: bool,
        right_horn: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Goat {
                baby,
                left_horn,
                right_horn,
            },
            position,
            y_rot,
        )
    }

    pub fn polar_bear(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::PolarBear { baby },
            position,
            y_rot,
        )
    }

    pub fn spider(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Spider, position, y_rot)
    }

    pub fn cave_spider(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::CaveSpider, position, y_rot)
    }

    pub fn enderman(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Enderman, position, y_rot)
    }

    pub fn iron_golem(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::IronGolem, position, y_rot)
    }

    pub fn snow_golem(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::SnowGolem, position, y_rot)
    }

    pub fn witch(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Witch, position, y_rot)
    }

    pub fn illager(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: IllagerModelFamily,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Illager { family },
            position,
            y_rot,
        )
    }

    pub fn quadruped(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: QuadrupedModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Quadruped { family, baby },
            position,
            y_rot,
        )
    }

    pub fn placeholder(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        name: &'static str,
        width: f32,
        height: f32,
        depth: f32,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Placeholder {
                name,
                bounds: EntityModelBounds {
                    width,
                    height,
                    depth,
                },
            },
            position,
            y_rot,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_model_instance_constructors_project_render_state() {
        assert_eq!(
            EntityModelInstance::chicken(10, [1.0, 2.0, 3.0], 45.0, true),
            EntityModelInstance::new(
                10,
                EntityModelKind::Chicken {
                    variant: ChickenModelVariant::Temperate,
                    baby: true,
                },
                [1.0, 2.0, 3.0],
                45.0,
            )
        );

        let wild = EntityModelInstance::wolf_state(
            11,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            false,
            false,
            Some(EntityDyeColor::Blue),
        );
        assert_eq!(
            wild.kind,
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                invisible: false,
                collar_color: None,
            }
        );

        let placeholder = EntityModelInstance::placeholder(
            12,
            [4.0, 5.0, 6.0],
            90.0,
            "custom_bounds",
            1.0,
            2.0,
            3.0,
        );
        assert_eq!(placeholder.entity_id, 12);
        assert_eq!(placeholder.position, [4.0, 5.0, 6.0]);
        assert_eq!(placeholder.y_rot, 90.0);
        assert_eq!(
            placeholder.kind,
            EntityModelKind::Placeholder {
                name: "custom_bounds",
                bounds: EntityModelBounds {
                    width: 1.0,
                    height: 2.0,
                    depth: 3.0,
                },
            }
        );
    }
}
