//! Decisions `Options` computes from stored values rather than reading back.
//!
//! Most of them already have an owner elsewhere and are deliberately not
//! repeated here: `getFinalSoundSourceVolume` is
//! `AudioVolumeSettings::final_source_volume`, and `getBackgroundOpacity` is
//! the text-backdrop branch the HUD applies while laying elements out. Only a
//! rule with no existing owner belongs in this module.

/// `Options.getEffectiveRenderDistance`: the server's radius wins when it sent
/// one, so a client configured beyond what the server streams does not render
/// past it. `handleSetChunkCacheRadius` is what supplies the server value, and
/// `serverRenderDistance` stays 0 until then.
pub(crate) fn effective_render_distance(client_chunks: u32, server_chunks: Option<i32>) -> u32 {
    match server_chunks {
        Some(server) if server > 0 => client_chunks.min(server as u32),
        _ => client_chunks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_server_radius_only_clamps_when_it_is_set_and_smaller() {
        assert_eq!(effective_render_distance(12, None), 12);
        // 0 is "the server has not said", not "render nothing".
        assert_eq!(effective_render_distance(12, Some(0)), 12);
        assert_eq!(effective_render_distance(12, Some(8)), 8);
        assert_eq!(effective_render_distance(12, Some(32)), 12);
        assert_eq!(effective_render_distance(12, Some(-1)), 12);
    }
}
