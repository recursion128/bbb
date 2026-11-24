//! Scene layer between `bbb-client-world` and the renderer: terrain
//! upload/meshing, biome tinting, sky/weather/cloud/world-border passes, and
//! block/entity/debug scene overlays. Nothing here may depend on the
//! application runtime in `bbb-client`.

pub mod biome_tint;
pub(crate) mod block_destroy;
pub(crate) mod clouds;
pub(crate) mod debug_filled_boxes;
pub(crate) mod debug_text_billboards;
pub(crate) mod item_entities;
pub(crate) mod lightmap;
pub(crate) mod lod_terrain;
pub(crate) mod outline;
pub(crate) mod panorama;
pub(crate) mod screen_blur;
pub(crate) mod screen_effects;
pub(crate) mod selection;
pub(crate) mod sky;
#[doc(hidden)]
pub mod synchronized_registry_test_fixture;
pub(crate) mod terrain_animation;
pub mod terrain_textures;
pub mod vanilla_color;
pub(crate) mod vignette;
pub(crate) mod weather;
pub(crate) mod world_border;
