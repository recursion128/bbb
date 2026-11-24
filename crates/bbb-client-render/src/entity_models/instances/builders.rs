use super::*;
use crate::entity_models::{
    CatModelVariant, ChickenModelVariant, CowModelVariant, DecoratedPotPattern, FrogModelVariant,
    PigModelVariant, WolfModelVariant,
};
use crate::item_models::SpecialItemCopperGolemPose;

impl EntityModelInstance {
    pub fn new(entity_id: i32, kind: EntityModelKind, position: [f32; 3], y_rot: f32) -> Self {
        Self {
            entity_id,
            kind,
            position,
            dynamic_texture: None,
            render_state: EntityRenderState {
                body_rot: y_rot,
                ..EntityRenderState::default()
            },
            block_entity: BlockEntityRenderState::default(),
        }
    }

    /// Applies `set` to this instance's entity render state.
    pub fn with(mut self, set: impl FnOnce(&mut EntityRenderState)) -> Self {
        set(&mut self.render_state);
        self
    }

    /// Applies `set` to this instance's block-entity render inputs.
    pub fn with_block_entity(mut self, set: impl FnOnce(&mut BlockEntityRenderState)) -> Self {
        set(&mut self.block_entity);
        self
    }

    pub fn cod(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Cod, position, y_rot)
    }

    /// A chest block-entity model instance at the chest block's min corner.
    /// `y_rot` carries vanilla `-facing.toYRot()` degrees (the
    /// `ChestRenderer.createModelTransformation` yaw about the block centre).
    pub fn chest(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        texture: ChestModelTexture,
        half: ChestModelHalf,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Chest { texture, half },
            position,
            y_rot,
        )
    }

    /// A sign block-entity model instance at the sign block's min corner.
    /// `y_rot` carries vanilla `-angle` degrees (the `-RotationSegment`
    /// / `-FACING.toYRot()` yaw of `StandingSignRenderer.baseTransformation`
    /// / `HangingSignRenderer.baseTransformation`).
    pub fn sign(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        wood: SignModelWood,
        attachment: SignModelAttachment,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Sign { wood, attachment },
            position,
            y_rot,
        )
    }

    /// A bed block-entity model instance at the bed block's min corner.
    /// `y_rot` carries the vanilla `180 + facing.toYRot()` degrees of
    /// `BedRenderer.createModelTransform`'s
    /// `rotateAround(Axis.ZP.rotationDegrees(180 + toYRot), 0.5, 0.5, 0.5)` —
    /// a Z rotation in the already X-rotated bed model frame, not an entity
    /// yaw.
    pub fn bed(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        color: EntityDyeColor,
        part: BedModelPart,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Bed { color, part },
            position,
            y_rot,
        )
    }

    /// A bell block-entity model instance at the bell block's min corner.
    /// `BellRenderer.submit` applies no facing/attachment transform — the bell
    /// body renders identically for all four attachments — so the yaw is `0`;
    /// the shake angle rides `bell_ticks` / `bell_shake_direction`.
    pub fn bell(entity_id: i32, position: [f32; 3]) -> Self {
        Self::new(entity_id, EntityModelKind::Bell, position, 0.0)
    }

    /// One beacon beam section at the beacon block's min corner.
    pub fn beacon_beam(entity_id: i32, position: [f32; 3]) -> Self {
        Self::new(entity_id, EntityModelKind::BeaconBeam, position, 0.0)
    }

    /// A copper golem statue block-entity model at the statue block's min
    /// corner, facing `body_rot` (`FACING.toYRot()`).
    pub fn copper_golem_statue_block(
        entity_id: i32,
        position: [f32; 3],
        facing_y_rot: f32,
        pose: SpecialItemCopperGolemPose,
        weathering: CopperGolemWeathering,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::CopperGolemStatueBlock { pose, weathering },
            position,
            facing_y_rot,
        )
    }

    /// A conduit block-entity model part at the conduit block's min corner.
    /// Active conduits expand into four such instances because vanilla submits
    /// cage, wind shell, inner wind, and eye with distinct transforms and
    /// textures; inactive conduits use only [`ConduitModelPart::Shell`].
    pub fn conduit(entity_id: i32, position: [f32; 3], part: ConduitModelPart) -> Self {
        Self::new(entity_id, EntityModelKind::Conduit { part }, position, 0.0)
    }

    /// A skull/head block-entity model instance at the block min corner.
    /// `y_rot` carries the ground skull's vanilla `-RotationSegment` yaw; wall
    /// skull yaw/offset rides `attachment`.
    pub fn skull_block(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        skull: EntityCustomHeadSkull,
        attachment: SkullBlockModelAttachment,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::SkullBlock { skull, attachment },
            position,
            y_rot,
        )
    }

    /// A shulker box block-entity model instance at the box block's min
    /// corner. The six-way `facing` rides the kind (the
    /// `ShulkerBoxRenderer.createModelTransform` rotation is a full
    /// `Direction.getRotation()` quaternion, not a yaw), so the yaw is `0`;
    /// the lid animation rides `shulker_box_progress`.
    pub fn shulker_box(
        entity_id: i32,
        position: [f32; 3],
        color: Option<EntityDyeColor>,
        facing: EntityAttachmentFace,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::ShulkerBox { color, facing },
            position,
            0.0,
        )
    }

    /// A decorated pot block-entity model instance at the pot block's min
    /// corner. `y_rot` carries the vanilla `180 - facing.toYRot()` degrees of
    /// `DecoratedPotRenderer.createModelTransformation`'s
    /// `rotateAround(Axis.YP.rotationDegrees(180 - toYRot), 0.5, 0.5, 0.5)`;
    /// the four sides carry their sherd patterns (`None` = the plain
    /// `decorated_pot_side`); the wobble rides `decorated_pot_wobble`.
    #[allow(clippy::too_many_arguments)]
    pub fn decorated_pot(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        back: Option<DecoratedPotPattern>,
        left: Option<DecoratedPotPattern>,
        right: Option<DecoratedPotPattern>,
        front: Option<DecoratedPotPattern>,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::DecoratedPot {
                back,
                left,
                right,
                front,
            },
            position,
            y_rot,
        )
    }

    /// A banner block-entity model instance at the banner block's min corner.
    /// `y_rot` carries the vanilla `-angle` degrees of
    /// `BannerRenderer.modelTransformation` (`Axis.YP.rotationDegrees(-angle)`
    /// — ground `RotationSegment.convertToDegrees(ROTATION)`, wall
    /// `FACING.toYRot()`); the pattern layers carry their dyes for the
    /// per-pass tints; the flag swing phase rides `banner_flag_phase`.
    pub fn banner(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        wall: bool,
        base_color: EntityDyeColor,
        layers: [Option<BannerPatternLayer>; 16],
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Banner {
                wall,
                flag_only: false,
                base_color,
                layers,
            },
            position,
            y_rot,
        )
    }

    /// Vanilla `GuiBannerResultRenderer`: the standing flag model without its
    /// pole/bar frame, centered and scaled by the owning GUI PIP descriptor.
    pub fn banner_flag(
        entity_id: i32,
        base_color: EntityDyeColor,
        layers: [Option<BannerPatternLayer>; 16],
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Banner {
                wall: false,
                flag_only: true,
                base_color,
                layers,
            },
            [0.0, 0.0, 0.0],
            0.0,
        )
    }

    /// One 1x1 tile of a painting entity. A complete painting is represented by one instance per
    /// `segment_x`/`segment_y` pair so the scene can attach vanilla's per-block light sample to
    /// each tile independently.
    #[allow(clippy::too_many_arguments)]
    pub fn painting_tile(
        entity_id: i32,
        position: [f32; 3],
        width: i32,
        height: i32,
        segment_x: i32,
        segment_y: i32,
        facing: EntityAttachmentFace,
        front: EntityDynamicTexture,
        back: EntityDynamicTexture,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::PaintingTile {
                width,
                height,
                segment_x,
                segment_y,
                facing,
                front,
                back,
            },
            position,
            0.0,
        )
    }

    /// An enchanting-table hovering book instance at the table block's min
    /// corner. `y_rot` carries the lerped `EnchantTableRenderer` book yaw in
    /// degrees (`state.yRot`, converted from the radian `oRot + or·partialTick`
    /// the projection interpolates); the book animation rides `book_openness` /
    /// `book_page_flip_1` / `book_page_flip_2`, and the hover offset rides
    /// `book_float_y`.
    pub fn enchanting_book(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::EnchantingBook, position, y_rot)
    }

    /// A lectern open-book instance at the lectern block's min corner. `y_rot`
    /// carries the vanilla `FACING.getClockWise().toYRot()` degrees the book
    /// renders at (`LecternRenderer.extractRenderState`); the fixed
    /// `BookModel.State.forAnimation(0, 0.1, 0.9, 1.2)` openness / page flips
    /// ride the shared book render state.
    pub fn lectern_book(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::LecternBook, position, y_rot)
    }

    pub fn salmon(entity_id: i32, position: [f32; 3], y_rot: f32, size: SalmonModelSize) -> Self {
        Self::new(entity_id, EntityModelKind::Salmon { size }, position, y_rot)
    }

    pub fn tropical_fish(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        shape: TropicalFishModelShape,
        base_color: EntityDyeColor,
        pattern: TropicalFishPattern,
        pattern_color: EntityDyeColor,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::TropicalFish {
                shape,
                base_color,
                pattern,
                pattern_color,
            },
            position,
            y_rot,
        )
    }

    pub fn squid(entity_id: i32, position: [f32; 3], y_rot: f32, glow: bool, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Squid { glow, baby },
            position,
            y_rot,
        )
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
        Self::player_with_skin(
            entity_id,
            position,
            y_rot,
            EntityPlayerSkin::default_for_model(slim),
            parts,
        )
    }

    pub fn player_with_skin(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        skin: EntityPlayerSkin,
        parts: PlayerModelPartVisibility,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Player { skin, parts },
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
        Self::armor_stand_with_marker(
            entity_id,
            position,
            y_rot,
            small,
            false,
            show_arms,
            show_base_plate,
            pose,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn armor_stand_with_marker(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        small: bool,
        marker: bool,
        show_arms: bool,
        show_base_plate: bool,
        pose: ArmorStandModelPose,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::ArmorStand {
                small,
                marker,
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

    pub fn ghast(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Ghast { charging: false },
            position,
            y_rot,
        )
    }

    pub fn happy_ghast(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::HappyGhast { baby: false },
            position,
            y_rot,
        )
    }

    pub fn minecart(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Minecart, position, y_rot)
    }

    pub fn blaze(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Blaze, position, y_rot)
    }

    pub fn endermite(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Endermite, position, y_rot)
    }

    pub fn silverfish(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Silverfish, position, y_rot)
    }

    pub fn vex(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Vex { charging: false },
            position,
            y_rot,
        )
    }

    pub fn allay(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Allay, position, y_rot)
    }

    pub fn strider(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool, cold: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Strider { baby, cold },
            position,
            y_rot,
        )
    }

    pub fn turtle(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Turtle { baby }, position, y_rot)
    }

    pub fn bat(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Bat, position, y_rot)
    }

    pub fn bee(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Bee {
                baby,
                angry: false,
                has_nectar: false,
            },
            position,
            y_rot,
        )
    }

    pub fn breeze(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Breeze, position, y_rot)
    }

    pub fn dolphin(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Dolphin { baby },
            position,
            y_rot,
        )
    }

    pub fn guardian(entity_id: i32, position: [f32; 3], y_rot: f32, elder: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Guardian { elder },
            position,
            y_rot,
        )
    }

    pub fn frog(entity_id: i32, position: [f32; 3], y_rot: f32, variant: FrogModelVariant) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Frog { variant },
            position,
            y_rot,
        )
    }

    pub fn creaking(entity_id: i32, position: [f32; 3], y_rot: f32, eyes_glowing: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Creaking { eyes_glowing },
            position,
            y_rot,
        )
    }

    pub fn sniffer(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Sniffer { baby: false },
            position,
            y_rot,
        )
    }

    pub fn snifflet(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Sniffer { baby: true },
            position,
            y_rot,
        )
    }

    pub fn warden(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Warden, position, y_rot)
    }

    pub fn armadillo(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        rolled_up: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Armadillo { baby, rolled_up },
            position,
            y_rot,
        )
    }

    pub fn axolotl(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: AxolotlModelVariant,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Axolotl { baby, variant },
            position,
            y_rot,
        )
    }

    pub fn tadpole(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Tadpole, position, y_rot)
    }

    pub fn parrot(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: ParrotModelVariant,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Parrot { variant },
            position,
            y_rot,
        )
    }

    pub fn shulker(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        color: Option<EntityDyeColor>,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Shulker { color },
            position,
            y_rot,
        )
    }

    pub fn wither(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Wither, position, y_rot)
    }

    pub fn giant(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Giant, position, y_rot)
    }

    pub fn end_crystal(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::EndCrystal, position, y_rot)
    }

    pub fn evoker_fangs(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::EvokerFangs, position, y_rot)
    }

    pub fn leash_knot(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::LeashKnot, position, y_rot)
    }

    pub fn arrow(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        texture: ArrowModelTexture,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Arrow { texture },
            position,
            y_rot,
        )
    }

    pub fn trident(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Trident, position, y_rot)
    }

    pub fn wither_skull(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::wither_skull_with_dangerous(entity_id, position, y_rot, false)
    }

    pub fn wither_skull_with_dangerous(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        dangerous: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::WitherSkull { dangerous },
            position,
            y_rot,
        )
    }

    pub fn llama_spit(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::LlamaSpit, position, y_rot)
    }

    pub fn shulker_bullet(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::ShulkerBullet, position, y_rot)
    }

    pub fn wind_charge(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::WindCharge, position, y_rot)
    }

    pub fn ender_dragon(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::EnderDragon, position, y_rot)
    }

    pub fn no_render(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::NoRender, position, y_rot)
    }

    pub fn phantom(entity_id: i32, position: [f32; 3], y_rot: f32, size: i32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Phantom { size },
            position,
            y_rot,
        )
    }

    pub fn pufferfish(entity_id: i32, position: [f32; 3], y_rot: f32, puff_state: i32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Pufferfish { puff_state },
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

    pub fn mooshroom(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Mooshroom {
                baby,
                variant: MooshroomVariant::Red,
            },
            position,
            y_rot,
        )
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

    #[allow(clippy::too_many_arguments)]
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
                jeb,
                age_ticks,
            },
            position,
            y_rot,
        )
        .with_invisible(invisible)
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
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
        Self::sheep_wool(entity_id, position, y_rot, baby, sheared, wool_color).with(|s| {
            s.kind.sheep_mut().head_eat = crate::entity_models::SheepHeadEatPose::from_eat_tick(
                eat_animation_tick,
                partial_tick,
            )
        })
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

    #[allow(clippy::too_many_arguments)]
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
                collar_color: tame.then_some(collar_color).flatten(),
                variant: WolfModelVariant::Pale,
            },
            position,
            y_rot,
        )
        .with_invisible(invisible)
    }

    pub fn horse(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::horse_with_variant(entity_id, position, y_rot, baby, HorseColorVariant::White)
    }

    pub fn horse_with_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: HorseColorVariant,
    ) -> Self {
        Self::horse_full(
            entity_id,
            position,
            y_rot,
            baby,
            variant,
            HorseMarkings::None,
        )
    }

    pub fn horse_full(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: HorseColorVariant,
        markings: HorseMarkings,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Horse {
                baby,
                variant,
                markings,
            },
            position,
            y_rot,
        )
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

    pub fn rabbit(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: RabbitModelVariant,
        toast: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Rabbit {
                baby,
                variant,
                toast,
            },
            position,
            y_rot,
        )
    }

    pub fn panda(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: PandaModelVariant,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Panda { baby, variant },
            position,
            y_rot,
        )
    }

    pub fn feline(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        cat: bool,
        baby: bool,
        cat_variant: CatModelVariant,
        collar: Option<EntityDyeColor>,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Feline {
                cat,
                baby,
                cat_variant,
                collar,
            },
            position,
            y_rot,
        )
    }

    pub fn fox(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: FoxModelVariant,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Fox { baby, variant },
            position,
            y_rot,
        )
    }

    pub fn nautilus(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Nautilus { baby },
            position,
            y_rot,
        )
    }

    #[cfg(test)]
    pub fn polar_bear_standing(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        stand_scale: f32,
    ) -> Self {
        Self::polar_bear(entity_id, position, y_rot, baby)
            .with(|s| s.kind.polar_bear_mut().stand_scale = stand_scale)
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
        Self::new(
            entity_id,
            EntityModelKind::IronGolem {
                crackiness: IronGolemCrackiness::None,
            },
            position,
            y_rot,
        )
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
}
