//! `FeatureSize` (`FeatureSize.java:9-32`): the clearance profile
//! `TreeFeature.getMaxFreeTreeHeight` checks (`TreeFeature.java:96-113`).

pub use bbb_server_datapack::worldgen::tree::{FeatureSize, FeatureSizeKind};

pub trait FeatureSizeExt {
    fn get_size_at_height(&self, tree_height: i32, yo: i32) -> i32;
}

impl FeatureSizeExt for FeatureSize {
    /// `FeatureSize.getSizeAtHeight(treeHeight, yo)`.
    fn get_size_at_height(&self, tree_height: i32, yo: i32) -> i32 {
        match self.kind {
            // `TwoLayersFeatureSize.getSizeAtHeight`
            // (`TwoLayersFeatureSize.java:39-41`).
            FeatureSizeKind::TwoLayers {
                limit,
                lower_size,
                upper_size,
            } => {
                if yo < limit {
                    lower_size
                } else {
                    upper_size
                }
            }
            // `ThreeLayersFeatureSize.getSizeAtHeight`
            // (`ThreeLayersFeatureSize.java:43-49`).
            FeatureSizeKind::ThreeLayers {
                limit,
                upper_limit,
                lower_size,
                middle_size,
                upper_size,
            } => {
                if yo < limit {
                    lower_size
                } else if yo >= tree_height - upper_limit {
                    upper_size
                } else {
                    middle_size
                }
            }
        }
    }
}
