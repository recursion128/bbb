//! `FoliagePlacer` (`FoliagePlacer.java:20-218`) and the eleven 26.1
//! foliage placers.

use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;
pub use bbb_server_datapack::worldgen::tree::{FoliagePlacer, FoliagePlacerKind};

use super::super::state_providers::BlockStateProviderExt;
use super::super::value_providers::IntProviderExt;
use super::super::{
    block_has_property, block_set_bool, dist_manhattan, mth_floor_f32, WorldGenLevel,
};
use super::{
    above, below, clock_wise, is_positive, offset, relative, tree_valid_pos, TreeConfiguration,
    TreeLevel, HORIZONTAL,
};

/// `FoliagePlacer.FoliageAttachment` (`FoliagePlacer.java:189-211`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoliageAttachment {
    pub pos: BlockPos,
    pub radius_offset: i32,
    pub double_trunk: bool,
}

impl FoliageAttachment {
    pub fn new(pos: BlockPos, radius_offset: i32, double_trunk: bool) -> Self {
        Self {
            pos,
            radius_offset,
            double_trunk,
        }
    }
}

pub trait FoliagePlacerExt {
    fn foliage_height(
        &self,
        random: &mut dyn RandomSource,
        tree_height: i32,
        config: &TreeConfiguration,
    ) -> i32;
    fn foliage_radius(&self, random: &mut dyn RandomSource, trunk_height: i32) -> i32;
    #[allow(clippy::too_many_arguments)]
    fn create_foliage<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        config: &TreeConfiguration,
        tree_height: i32,
        attachment: &FoliageAttachment,
        foliage_height: i32,
        leaf_radius: i32,
    ) -> Option<()>;
    fn should_skip_location(
        &self,
        random: &mut dyn RandomSource,
        dx: i32,
        y: i32,
        dz: i32,
        current_radius: i32,
        double_trunk: bool,
    ) -> bool;
    fn should_skip_location_signed(
        &self,
        random: &mut dyn RandomSource,
        dx: i32,
        y: i32,
        dz: i32,
        current_radius: i32,
        double_trunk: bool,
    ) -> bool;
    #[allow(clippy::too_many_arguments)]
    fn place_leaves_row<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        config: &TreeConfiguration,
        origin: BlockPos,
        current_radius: i32,
        y: i32,
        double_trunk: bool,
    ) -> Option<()>;
    #[allow(clippy::too_many_arguments)]
    fn place_leaves_row_with_hanging_leaves_below<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        config: &TreeConfiguration,
        origin: BlockPos,
        current_radius: i32,
        y: i32,
        double_trunk: bool,
        hanging_leaves_chance: f32,
        hanging_leaves_extension_chance: f32,
    ) -> Option<()>;
    fn try_place_extension<L: WorldGenLevel>(
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        config: &TreeConfiguration,
        chance: f32,
        log_pos: BlockPos,
        pos: BlockPos,
    ) -> bool;
    fn try_place_leaf<L: WorldGenLevel>(
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        config: &TreeConfiguration,
        pos: BlockPos,
    ) -> Option<bool>;
}

impl FoliagePlacerExt for FoliagePlacer {
    /// `FoliagePlacer.foliageHeight(random, treeHeight, config)` per family.
    fn foliage_height(
        &self,
        random: &mut dyn RandomSource,
        tree_height: i32,
        config: &TreeConfiguration,
    ) -> i32 {
        let _ = config;
        match &self.kind {
            // `BlobFoliagePlacer.foliageHeight` (`BlobFoliagePlacer.java:51-53`),
            // inherited by `BushFoliagePlacer` and `FancyFoliagePlacer`.
            FoliagePlacerKind::Blob { height }
            | FoliagePlacerKind::Bush { height }
            | FoliagePlacerKind::Fancy { height } => *height,
            // `SpruceFoliagePlacer.foliageHeight` (`SpruceFoliagePlacer.java:58-60`).
            FoliagePlacerKind::Spruce { trunk_height } => {
                4.max(tree_height - trunk_height.sample(random))
            }
            // `PineFoliagePlacer.foliageHeight` (`PineFoliagePlacer.java:57-59`).
            FoliagePlacerKind::Pine { height } => height.sample(random),
            // `AcaciaFoliagePlacer.foliageHeight` (`AcaciaFoliagePlacer.java:43-45`).
            FoliagePlacerKind::Acacia => 0,
            // `DarkOakFoliagePlacer.foliageHeight` (`DarkOakFoliagePlacer.java:51-53`).
            FoliagePlacerKind::DarkOak => 4,
            // `MegaJungleFoliagePlacer.foliageHeight`
            // (`MegaJungleFoliagePlacer.java:48-50`).
            FoliagePlacerKind::MegaJungle { height } => *height,
            // `MegaPineFoliagePlacer.foliageHeight` (`MegaPineFoliagePlacer.java:62-64`).
            FoliagePlacerKind::MegaPine { crown_height } => crown_height.sample(random),
            // `RandomSpreadFoliagePlacer.foliageHeight`
            // (`RandomSpreadFoliagePlacer.java:65-67`).
            FoliagePlacerKind::RandomSpread { foliage_height, .. } => foliage_height.sample(random),
            // `CherryFoliagePlacer.foliageHeight` (`CherryFoliagePlacer.java:86-88`).
            FoliagePlacerKind::Cherry { height, .. } => height.sample(random),
        }
    }

    /// `FoliagePlacer.foliageRadius` (`FoliagePlacer.java:65-67`), plus
    /// `PineFoliagePlacer.foliageRadius` (`PineFoliagePlacer.java:52-54`).
    fn foliage_radius(&self, random: &mut dyn RandomSource, trunk_height: i32) -> i32 {
        let radius = self.radius.sample(random);
        if let FoliagePlacerKind::Pine { .. } = self.kind {
            return radius + random.next_int((trunk_height + 1).max(1));
        }
        radius
    }

    /// `FoliagePlacer.createFoliage` (`FoliagePlacer.java:38-49`): sample
    /// `offset` (`FoliagePlacer.java:69-71`), then the family's body.
    #[allow(clippy::too_many_arguments)]
    fn create_foliage<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        config: &TreeConfiguration,
        tree_height: i32,
        attachment: &FoliageAttachment,
        foliage_height: i32,
        leaf_radius: i32,
    ) -> Option<()> {
        let _ = tree_height;
        let offset_value = self.offset.sample(random);
        let double_trunk = attachment.double_trunk;
        match &self.kind {
            // `BlobFoliagePlacer.createFoliage` (`BlobFoliagePlacer.java:33-48`).
            FoliagePlacerKind::Blob { .. } => {
                let mut yo = offset_value;
                while yo >= offset_value - foliage_height {
                    let current_radius =
                        (leaf_radius + attachment.radius_offset - 1 - yo / 2).max(0);
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        attachment.pos,
                        current_radius,
                        yo,
                        double_trunk,
                    )?;
                    yo -= 1;
                }
            }
            // `BushFoliagePlacer.createFoliage` (`BushFoliagePlacer.java:23-38`).
            FoliagePlacerKind::Bush { .. } => {
                let mut yo = offset_value;
                while yo >= offset_value - foliage_height {
                    let current_radius = leaf_radius + attachment.radius_offset - 1 - yo;
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        attachment.pos,
                        current_radius,
                        yo,
                        double_trunk,
                    )?;
                    yo -= 1;
                }
            }
            // `FancyFoliagePlacer.createFoliage` (`FancyFoliagePlacer.java:24-39`).
            FoliagePlacerKind::Fancy { .. } => {
                let mut yo = offset_value;
                while yo >= offset_value - foliage_height {
                    let current_radius = leaf_radius
                        + if yo != offset_value && yo != offset_value - foliage_height {
                            1
                        } else {
                            0
                        };
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        attachment.pos,
                        current_radius,
                        yo,
                        double_trunk,
                    )?;
                    yo -= 1;
                }
            }
            // `SpruceFoliagePlacer.createFoliage` (`SpruceFoliagePlacer.java:29-55`).
            FoliagePlacerKind::Spruce { .. } => {
                let foliage_pos = attachment.pos;
                let mut current_radius = random.next_int(2);
                let mut max_radius = 1;
                let mut min_radius = 0;
                let mut yo = offset_value;
                while yo >= -foliage_height {
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        foliage_pos,
                        current_radius,
                        yo,
                        double_trunk,
                    )?;
                    if current_radius >= max_radius {
                        current_radius = min_radius;
                        min_radius = 1;
                        max_radius = (max_radius + 1).min(leaf_radius + attachment.radius_offset);
                    } else {
                        current_radius += 1;
                    }
                    yo -= 1;
                }
            }
            // `PineFoliagePlacer.createFoliage` (`PineFoliagePlacer.java:28-49`).
            FoliagePlacerKind::Pine { .. } => {
                let mut current_radius = 0;
                let mut yo = offset_value;
                while yo >= offset_value - foliage_height {
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        attachment.pos,
                        current_radius,
                        yo,
                        double_trunk,
                    )?;
                    if current_radius >= 1 && yo == offset_value - foliage_height + 1 {
                        current_radius -= 1;
                    } else if current_radius < leaf_radius + attachment.radius_offset {
                        current_radius += 1;
                    }
                    yo -= 1;
                }
            }
            // `AcaciaFoliagePlacer.createFoliage` (`AcaciaFoliagePlacer.java:24-40`).
            FoliagePlacerKind::Acacia => {
                let foliage_pos = above(attachment.pos, offset_value);
                self.place_leaves_row(
                    tree,
                    random,
                    config,
                    foliage_pos,
                    leaf_radius + attachment.radius_offset,
                    -1 - foliage_height,
                    double_trunk,
                )?;
                self.place_leaves_row(
                    tree,
                    random,
                    config,
                    foliage_pos,
                    leaf_radius - 1,
                    -foliage_height,
                    double_trunk,
                )?;
                self.place_leaves_row(
                    tree,
                    random,
                    config,
                    foliage_pos,
                    leaf_radius + attachment.radius_offset - 1,
                    0,
                    double_trunk,
                )?;
            }
            // `DarkOakFoliagePlacer.createFoliage` (`DarkOakFoliagePlacer.java:24-48`).
            FoliagePlacerKind::DarkOak => {
                let pos = above(attachment.pos, offset_value);
                if double_trunk {
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        pos,
                        leaf_radius + 2,
                        -1,
                        double_trunk,
                    )?;
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        pos,
                        leaf_radius + 3,
                        0,
                        double_trunk,
                    )?;
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        pos,
                        leaf_radius + 2,
                        1,
                        double_trunk,
                    )?;
                    if random.next_boolean() {
                        self.place_leaves_row(
                            tree,
                            random,
                            config,
                            pos,
                            leaf_radius,
                            2,
                            double_trunk,
                        )?;
                    }
                } else {
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        pos,
                        leaf_radius + 2,
                        -1,
                        double_trunk,
                    )?;
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        pos,
                        leaf_radius + 1,
                        0,
                        double_trunk,
                    )?;
                }
            }
            // `MegaJungleFoliagePlacer.createFoliage`
            // (`MegaJungleFoliagePlacer.java:28-45`).
            FoliagePlacerKind::MegaJungle { .. } => {
                let leaf_height = if double_trunk {
                    foliage_height
                } else {
                    1 + random.next_int(2)
                };
                let mut yo = offset_value;
                while yo >= offset_value - leaf_height {
                    let current_radius = leaf_radius + attachment.radius_offset + 1 - yo;
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        attachment.pos,
                        current_radius,
                        yo,
                        double_trunk,
                    )?;
                    yo -= 1;
                }
            }
            // `MegaPineFoliagePlacer.createFoliage` (`MegaPineFoliagePlacer.java:30-59`).
            FoliagePlacerKind::MegaPine { .. } => {
                let foliage_pos = attachment.pos;
                let mut prev_radius = 0;
                let mut yy = foliage_pos.y - foliage_height + offset_value;
                while yy <= foliage_pos.y + offset_value {
                    let yo = foliage_pos.y - yy;
                    let smooth_radius = leaf_radius
                        + attachment.radius_offset
                        + mth_floor_f32(yo as f32 / foliage_height as f32 * 3.5);
                    let jagged_radius = if yo > 0 && smooth_radius == prev_radius && (yy & 1) == 0 {
                        smooth_radius + 1
                    } else {
                        smooth_radius
                    };
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        BlockPos {
                            x: foliage_pos.x,
                            y: yy,
                            z: foliage_pos.z,
                        },
                        jagged_radius,
                        0,
                        double_trunk,
                    )?;
                    prev_radius = smooth_radius;
                    yy += 1;
                }
            }
            // `RandomSpreadFoliagePlacer.createFoliage`
            // (`RandomSpreadFoliagePlacer.java:39-62`).
            FoliagePlacerKind::RandomSpread {
                leaf_placement_attempts,
                ..
            } => {
                let origin = attachment.pos;
                for _ in 0..*leaf_placement_attempts {
                    let dx = random.next_int(leaf_radius) - random.next_int(leaf_radius);
                    let dy = random.next_int(foliage_height) - random.next_int(foliage_height);
                    let dz = random.next_int(leaf_radius) - random.next_int(leaf_radius);
                    let pos = offset(origin, dx, dy, dz);
                    Self::try_place_leaf(tree, random, config, pos)?;
                }
            }
            // `CherryFoliagePlacer.createFoliage` (`CherryFoliagePlacer.java:56-83`).
            FoliagePlacerKind::Cherry {
                hanging_leaves_chance,
                hanging_leaves_extension_chance,
                ..
            } => {
                let foliage_pos = above(attachment.pos, offset_value);
                let current_radius = leaf_radius + attachment.radius_offset - 1;
                self.place_leaves_row(
                    tree,
                    random,
                    config,
                    foliage_pos,
                    current_radius - 2,
                    foliage_height - 3,
                    double_trunk,
                )?;
                self.place_leaves_row(
                    tree,
                    random,
                    config,
                    foliage_pos,
                    current_radius - 1,
                    foliage_height - 4,
                    double_trunk,
                )?;
                let mut y = foliage_height - 5;
                while y >= 0 {
                    self.place_leaves_row(
                        tree,
                        random,
                        config,
                        foliage_pos,
                        current_radius,
                        y,
                        double_trunk,
                    )?;
                    y -= 1;
                }
                self.place_leaves_row_with_hanging_leaves_below(
                    tree,
                    random,
                    config,
                    foliage_pos,
                    current_radius,
                    -1,
                    double_trunk,
                    *hanging_leaves_chance,
                    *hanging_leaves_extension_chance,
                )?;
                self.place_leaves_row_with_hanging_leaves_below(
                    tree,
                    random,
                    config,
                    foliage_pos,
                    current_radius - 1,
                    -2,
                    double_trunk,
                    *hanging_leaves_chance,
                    *hanging_leaves_extension_chance,
                )?;
            }
        }
        Some(())
    }

    /// `FoliagePlacer.shouldSkipLocation` per family.
    fn should_skip_location(
        &self,
        random: &mut dyn RandomSource,
        dx: i32,
        y: i32,
        dz: i32,
        current_radius: i32,
        double_trunk: bool,
    ) -> bool {
        let _ = double_trunk;
        match &self.kind {
            // `BlobFoliagePlacer.shouldSkipLocation` (`BlobFoliagePlacer.java:56-58`).
            FoliagePlacerKind::Blob { .. } => {
                dx == current_radius && dz == current_radius && (random.next_int(2) == 0 || y == 0)
            }
            // `BushFoliagePlacer.shouldSkipLocation` (`BushFoliagePlacer.java:41-43`).
            FoliagePlacerKind::Bush { .. } => {
                dx == current_radius && dz == current_radius && random.next_int(2) == 0
            }
            // `FancyFoliagePlacer.shouldSkipLocation` (`FancyFoliagePlacer.java:42-44`):
            // `Mth.square(float)` (`Mth.java:616-618`).
            FoliagePlacerKind::Fancy { .. } => {
                let fx = dx as f32 + 0.5;
                let fz = dz as f32 + 0.5;
                fx * fx + fz * fz > (current_radius * current_radius) as f32
            }
            // `SpruceFoliagePlacer.shouldSkipLocation`
            // (`SpruceFoliagePlacer.java:63-65`) and
            // `PineFoliagePlacer.shouldSkipLocation` (`PineFoliagePlacer.java:62-64`).
            FoliagePlacerKind::Spruce { .. } | FoliagePlacerKind::Pine { .. } => {
                dx == current_radius && dz == current_radius && current_radius > 0
            }
            // `AcaciaFoliagePlacer.shouldSkipLocation` (`AcaciaFoliagePlacer.java:48-50`).
            FoliagePlacerKind::Acacia => {
                if y == 0 {
                    (dx > 1 || dz > 1) && dx != 0 && dz != 0
                } else {
                    dx == current_radius && dz == current_radius && current_radius > 0
                }
            }
            // `DarkOakFoliagePlacer.shouldSkipLocation` (`DarkOakFoliagePlacer.java:65-71`).
            FoliagePlacerKind::DarkOak => {
                if y == -1 && !double_trunk {
                    dx == current_radius && dz == current_radius
                } else {
                    y == 1 && dx + dz > current_radius * 2 - 2
                }
            }
            // `MegaJungleFoliagePlacer.shouldSkipLocation`
            // (`MegaJungleFoliagePlacer.java:53-55`) and
            // `MegaPineFoliagePlacer.shouldSkipLocation`
            // (`MegaPineFoliagePlacer.java:67-69`).
            FoliagePlacerKind::MegaJungle { .. } | FoliagePlacerKind::MegaPine { .. } => {
                if dx + dz >= 7 {
                    true
                } else {
                    dx * dx + dz * dz > current_radius * current_radius
                }
            }
            // `RandomSpreadFoliagePlacer.shouldSkipLocation`
            // (`RandomSpreadFoliagePlacer.java:70-72`).
            FoliagePlacerKind::RandomSpread { .. } => false,
            // `CherryFoliagePlacer.shouldSkipLocation` (`CherryFoliagePlacer.java:91-101`).
            FoliagePlacerKind::Cherry {
                wide_bottom_layer_hole_chance,
                corner_hole_chance,
                ..
            } => {
                if y == -1
                    && (dx == current_radius || dz == current_radius)
                    && random.next_float() < *wide_bottom_layer_hole_chance
                {
                    return true;
                }
                let corner = dx == current_radius && dz == current_radius;
                let wide_layer = current_radius > 2;
                if wide_layer {
                    corner
                        || dx + dz > current_radius * 2 - 2
                            && random.next_float() < *corner_hole_chance
                } else {
                    corner && random.next_float() < *corner_hole_chance
                }
            }
        }
    }

    /// `FoliagePlacer.shouldSkipLocationSigned` (`FoliagePlacer.java:77-91`),
    /// with `DarkOakFoliagePlacer.shouldSkipLocationSigned`
    /// (`DarkOakFoliagePlacer.java:56-62`) in front of it.
    fn should_skip_location_signed(
        &self,
        random: &mut dyn RandomSource,
        dx: i32,
        y: i32,
        dz: i32,
        current_radius: i32,
        double_trunk: bool,
    ) -> bool {
        if let FoliagePlacerKind::DarkOak = self.kind {
            if !(y != 0
                || !double_trunk
                || dx != -current_radius && dx < current_radius
                || dz != -current_radius && dz < current_radius)
            {
                return true;
            }
        }
        let (min_dx, min_dz) = if double_trunk {
            (dx.abs().min((dx - 1).abs()), dz.abs().min((dz - 1).abs()))
        } else {
            (dx.abs(), dz.abs())
        };
        self.should_skip_location(random, min_dx, y, min_dz, current_radius, double_trunk)
    }

    /// `FoliagePlacer.placeLeavesRow` (`FoliagePlacer.java:93-114`).
    #[allow(clippy::too_many_arguments)]
    fn place_leaves_row<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        config: &TreeConfiguration,
        origin: BlockPos,
        current_radius: i32,
        y: i32,
        double_trunk: bool,
    ) -> Option<()> {
        let offset_value = if double_trunk { 1 } else { 0 };
        for dx in -current_radius..=current_radius + offset_value {
            for dz in -current_radius..=current_radius + offset_value {
                if !self.should_skip_location_signed(
                    random,
                    dx,
                    y,
                    dz,
                    current_radius,
                    double_trunk,
                ) {
                    let pos = offset(origin, dx, y, dz);
                    Self::try_place_leaf(tree, random, config, pos)?;
                }
            }
        }
        Some(())
    }

    /// `FoliagePlacer.placeLeavesRowWithHangingLeavesBelow`
    /// (`FoliagePlacer.java:116-152`).
    #[allow(clippy::too_many_arguments)]
    fn place_leaves_row_with_hanging_leaves_below<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        config: &TreeConfiguration,
        origin: BlockPos,
        current_radius: i32,
        y: i32,
        double_trunk: bool,
        hanging_leaves_chance: f32,
        hanging_leaves_extension_chance: f32,
    ) -> Option<()> {
        self.place_leaves_row(
            tree,
            random,
            config,
            origin,
            current_radius,
            y,
            double_trunk,
        )?;
        let offset_value = if double_trunk { 1 } else { 0 };
        let log_pos = below(origin, 1);
        for along_edge in HORIZONTAL {
            let to_edge = clock_wise(along_edge);
            let offset_to_edge = if is_positive(to_edge) {
                current_radius + offset_value
            } else {
                current_radius
            };
            let mut pos = relative(
                relative(offset(origin, 0, y - 1, 0), to_edge, offset_to_edge),
                along_edge,
                -current_radius,
            );
            let mut offset_along_edge = -current_radius;
            while offset_along_edge < current_radius + offset_value {
                // `foliageSetter.isSet(pos.move(UP))` then `pos.move(DOWN)`.
                let leaves_above = tree.foliage_is_set(above(pos, 1));
                if leaves_above
                    && Self::try_place_extension(
                        tree,
                        random,
                        config,
                        hanging_leaves_chance,
                        log_pos,
                        pos,
                    )
                {
                    pos = below(pos, 1);
                    Self::try_place_extension(
                        tree,
                        random,
                        config,
                        hanging_leaves_extension_chance,
                        log_pos,
                        pos,
                    );
                    pos = above(pos, 1);
                }
                offset_along_edge += 1;
                pos = relative(pos, along_edge, 1);
            }
        }
        Some(())
    }

    /// `FoliagePlacer.tryPlaceExtension` (`FoliagePlacer.java:154-168`).
    fn try_place_extension<L: WorldGenLevel>(
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        config: &TreeConfiguration,
        chance: f32,
        log_pos: BlockPos,
        pos: BlockPos,
    ) -> bool {
        if dist_manhattan(pos, log_pos) >= 7 {
            return false;
        }
        if random.next_float() > chance {
            false
        } else {
            Self::try_place_leaf(tree, random, config, pos).unwrap_or(false)
        }
    }

    /// `FoliagePlacer.tryPlaceLeaf` (`FoliagePlacer.java:170-187`).
    fn try_place_leaf<L: WorldGenLevel>(
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        config: &TreeConfiguration,
        pos: BlockPos,
    ) -> Option<bool> {
        // `state.getValueOrElse(PERSISTENT, false)`.
        let is_persistent = persistent_value_or_false(tree.get_block(pos));
        if !is_persistent && tree_valid_pos(tree, pos) {
            let mut foliage_state = config
                .foliage_provider
                .get_state(&*tree.level, random, pos)?;
            if block_has_property(foliage_state, "waterlogged") {
                foliage_state =
                    block_set_bool(foliage_state, "waterlogged", tree.is_water_source_at(pos));
            }
            tree.set_foliage(pos, foliage_state);
            Some(true)
        } else {
            Some(false)
        }
    }
}

/// `BlockStateBase.getValueOrElse(BlockStateProperties.PERSISTENT, false)`
/// (`StateHolder.java:100-102`).
fn persistent_value_or_false(state: i32) -> bool {
    bbb_facts::block_state_types::vanilla_block_state_property(state, "persistent") == Some("true")
}
