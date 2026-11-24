//! Block-owned actions reached by `ServerExplosion`'s TRIGGER_BLOCK pass.

use super::LevelCtx;
use bbb_protocol::packets::BlockPos;

use super::{button, candle, door, fence_gate, footing, lever, trapdoor};

pub(crate) fn trigger_block(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> bool {
    if button::is_button(state) {
        if super::bool_property(state, "powered") == Some(false) {
            button::press(level, pos, state, None);
        }
        return true;
    }
    if lever::is_lever(state) {
        lever::pull(level, pos, state);
        return true;
    }
    if door::is_door(state) {
        if super::bool_property(state, "powered") == Some(false)
            && door::half(state) == Some(door::DoorHalf::Lower)
            && door::can_open_by_wind_charge(state) == Some(true)
        {
            let open = super::bool_property(state, "open") == Some(true);
            door::set_open(level, pos, state, None, !open);
        }
        return true;
    }
    if trapdoor::is_trapdoor(state) {
        if super::bool_property(state, "powered") == Some(false)
            && trapdoor::can_open_by_wind_charge(state) == Some(true)
        {
            trapdoor::toggle(level, pos, state, None);
        }
        return true;
    }
    if fence_gate::is_fence_gate(state) {
        if super::bool_property(state, "powered") == Some(false) {
            fence_gate::toggle_from_explosion(level, pos, state);
        }
        return true;
    }
    if (candle::is_candle(state) || footing::is_candle_cake(state)) && candle::is_lit(state) {
        candle::extinguish(level, pos, state, None);
        return true;
    }
    false
}
