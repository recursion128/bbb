use bbb_pack::{PackRoots, SpriteGuiScaling, SpriteNineSliceBorder};

#[test]
#[ignore = "requires repo-managed Vanilla 26.1 resources"]
fn vanilla_now_playing_gui_assets_preserve_animation_and_nine_slice_metadata() {
    let roots = PackRoots::from_environment().unwrap();
    let gui_sources = roots.load_atlas_texture_sources("gui").unwrap();

    let music_notes = gui_sources
        .iter()
        .find(|source| source.id == "minecraft:icon/music_notes")
        .unwrap();
    assert_eq!((music_notes.width, music_notes.height), (16, 16));
    let animation = music_notes.animation.as_ref().unwrap();
    assert_eq!(animation.frame_count, 8);
    assert_eq!(animation.default_frame_time, 2);
    assert!(!animation.interpolate);
    assert_eq!(animation.frames.len(), 8);

    let now_playing = gui_sources
        .iter()
        .find(|source| source.id == "minecraft:toast/now_playing")
        .unwrap();
    assert_eq!((now_playing.width, now_playing.height), (160, 32));
    assert_eq!(
        now_playing.gui_metadata.scaling,
        SpriteGuiScaling::NineSlice {
            width: 160,
            height: 32,
            border: SpriteNineSliceBorder::uniform(4),
            stretch_inner: false,
        }
    );
}
