//! Pointer positions and buttons as simulation commands. Window events and the
//! control plane's `input.cursor`/`input.click` share these, so a scripted
//! click resolves its GUI position and screen hit targets exactly like a
//! player's click.

use bbb_client_render::Renderer;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta},
};

use crate::{
    control::{ControlWriteRequest, InputControlRequest},
    runtime::input_mouse_button,
    simulation::commands::SimulationCommand,
    window_surface::CursorPositionCache,
};

/// `MouseHandler.onMove` stores raw physical
/// coordinates; `getScaledXPos/getScaledYPos` project them onto the logical
/// GUI surface at the boundary. The cache stays physical so a later resize
/// never reuses a stale projection. Returns the command and the GUI position.
pub(crate) fn cursor_moved(
    position: PhysicalPosition<f64>,
    renderer: &Renderer,
    cursor: &mut CursorPositionCache,
) -> (SimulationCommand, PhysicalPosition<f64>) {
    cursor.on_cursor_moved(position);
    let gui = cursor
        .project(
            renderer.physical_surface_size(),
            renderer.gui_surface_size(),
        )
        .expect("cursor position cached by CursorMoved");
    let at = (gui.x, gui.y);
    let command = SimulationCommand::CursorMoved {
        position: gui,
        disconnected_screen_target: renderer.hud_disconnected_screen_hit_target_at(at),
        death_screen_target: renderer.hud_death_screen_hit_target_at(at),
        game_event_target: renderer.hud_game_event_hit_target_at(at),
        social_interactions_target: renderer.hud_social_interactions_hit_target_at(at),
    };
    (command, gui)
}

/// `MouseHandler.onButton`: recompute the scaled
/// position from the raw physical cache with the *current* physical/logical
/// sizes at click time.
pub(crate) fn mouse_button(
    button: MouseButton,
    state: ElementState,
    renderer: &Renderer,
    cursor: &CursorPositionCache,
) -> SimulationCommand {
    let gui = cursor.project(
        renderer.physical_surface_size(),
        renderer.gui_surface_size(),
    );
    let left_press = matches!((button, state), (MouseButton::Left, ElementState::Pressed));
    let press = gui.filter(|_| left_press);
    SimulationCommand::MouseButton {
        button,
        state,
        position: gui,
        chat_target: press.and_then(|at| renderer.hud_chat_hit_target_at((at.x, at.y))),
        book_target: press.and_then(|at| renderer.hud_book_hit_target_at((at.x, at.y))),
        disconnected_screen_target: press
            .and_then(|at| renderer.hud_disconnected_screen_hit_target_at((at.x, at.y))),
        command_confirmation_target: gui
            .and_then(|at| renderer.hud_command_confirmation_hit_target_at((at.x, at.y))),
        death_screen_target: press
            .and_then(|at| renderer.hud_death_screen_hit_target_at((at.x, at.y))),
        game_event_target: press.and_then(|at| renderer.hud_game_event_hit_target_at((at.x, at.y))),
        social_interactions_target: press
            .and_then(|at| renderer.hud_social_interactions_hit_target_at((at.x, at.y))),
    }
}

/// Replaces control-plane pointer requests with the commands the equivalent
/// window events would queue, resolved against the frame on screen.
pub(crate) fn resolve_control_pointer(
    commands: Vec<SimulationCommand>,
    renderer: &Renderer,
    cursor: &mut CursorPositionCache,
) -> Vec<SimulationCommand> {
    let mut resolved = Vec::with_capacity(commands.len());
    for command in commands {
        let (command, completion) = match command {
            SimulationCommand::WithCompletion {
                command,
                completion,
            } => (*command, Some(completion)),
            command => (command, None),
        };
        match command {
            SimulationCommand::Control(ControlWriteRequest::Input(
                InputControlRequest::Cursor { x, y },
            )) => resolved.push(cursor_moved(PhysicalPosition::new(x, y), renderer, cursor).0),
            SimulationCommand::Control(ControlWriteRequest::Input(
                InputControlRequest::Click { x, y, button },
            )) => {
                resolved.push(cursor_moved(PhysicalPosition::new(x, y), renderer, cursor).0);
                let button = input_mouse_button(button);
                for state in [ElementState::Pressed, ElementState::Released] {
                    resolved.push(mouse_button(button, state, renderer, cursor));
                }
            }
            SimulationCommand::Control(ControlWriteRequest::Input(
                InputControlRequest::Scroll { dx, dy },
            )) => {
                let position = cursor.project(
                    renderer.physical_surface_size(),
                    renderer.gui_surface_size(),
                );
                resolved.push(SimulationCommand::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(dx, dy),
                    position,
                });
            }
            command => resolved.push(command),
        }
        if let Some(completion) = completion {
            let command = resolved.pop().expect("a control input produces an event");
            resolved.push(SimulationCommand::WithCompletion {
                command: Box::new(command),
                completion,
            });
        }
    }
    resolved
}
