//! Feature-gated validation faults for the deterministic lifecycle soak.

use std::{
    env, fs,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use bbb_renderer::{LifecycleSoakSurfaceError, Renderer};
use serde::Deserialize;
use winit::{dpi::PhysicalSize, window::Window};

const MANIFEST_ENV: &str = "BBB_LIFECYCLE_SOAK_MANIFEST";

#[derive(Debug, Deserialize)]
struct Manifest {
    schema: u32,
    vanilla_version: String,
    fault_plan: FaultPlan,
}

#[derive(Debug, Deserialize)]
struct FaultPlan {
    force_no_audio_device: bool,
    actions: Vec<FaultAction>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct FaultAction {
    after_millis: u64,
    action: FaultActionKind,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum FaultActionKind {
    Resize,
    Minimize,
    Restore,
    SurfaceLost,
    SurfaceOutdated,
    SurfaceTimeout,
}

pub(crate) struct LifecycleSoakRuntime {
    force_no_audio_device: bool,
    actions: Vec<FaultAction>,
    next_action: usize,
    restore_in_progress: bool,
    started_at: Option<Instant>,
}

impl LifecycleSoakRuntime {
    pub(crate) fn from_env() -> Result<Option<Self>> {
        let Some(path) = env::var_os(MANIFEST_ENV) else {
            return Ok(None);
        };
        Self::from_path(Path::new(&path)).map(Some)
    }

    fn from_path(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("read lifecycle-soak manifest {}", path.display()))?;
        let manifest: Manifest = serde_json::from_slice(&bytes)
            .with_context(|| format!("decode lifecycle-soak manifest {}", path.display()))?;
        if manifest.schema != 1 || manifest.vanilla_version != "26.1" {
            bail!("lifecycle-soak manifest must use schema 1 and vanilla_version 26.1");
        }
        validate_actions(&manifest.fault_plan.actions)?;
        Ok(Self {
            force_no_audio_device: manifest.fault_plan.force_no_audio_device,
            actions: manifest.fault_plan.actions,
            next_action: 0,
            restore_in_progress: false,
            started_at: None,
        })
    }

    pub(crate) fn force_no_audio_device(&self) -> bool {
        self.force_no_audio_device
    }

    pub(crate) fn apply_due(&mut self, now: Instant, window: &Window, renderer: &mut Renderer) {
        let started_at = *self.started_at.get_or_insert(now);
        let elapsed = now.saturating_duration_since(started_at);
        while let Some(action) = self.actions.get(self.next_action) {
            if elapsed < Duration::from_millis(action.after_millis) {
                break;
            }
            match action.action {
                FaultActionKind::Resize => {
                    renderer.record_lifecycle_soak_window_resize_request();
                    let _ = window.request_inner_size(PhysicalSize::new(
                        action.width.expect("validated resize width"),
                        action.height.expect("validated resize height"),
                    ));
                }
                FaultActionKind::Minimize => {
                    renderer.record_lifecycle_soak_window_minimize_request();
                    window.set_minimized(true);
                }
                FaultActionKind::Restore => {
                    if !self.restore_in_progress {
                        renderer.record_lifecycle_soak_window_restore_request();
                        self.restore_in_progress = true;
                    }
                    window.set_minimized(false);
                    window.request_redraw();
                    if window.is_minimized() == Some(true) {
                        // AppKit's miniaturize/deminiaturize transitions are
                        // asynchronous. A restore requested while the minimize
                        // animation is still active can leave CAMetalLayer
                        // occluded indefinitely unless the validation driver
                        // retries after AppKit observes the minimized state.
                        break;
                    }
                    self.restore_in_progress = false;
                    window.focus_window();
                }
                FaultActionKind::SurfaceLost => {
                    renderer.inject_lifecycle_soak_surface_error(LifecycleSoakSurfaceError::Lost)
                }
                FaultActionKind::SurfaceOutdated => renderer
                    .inject_lifecycle_soak_surface_error(LifecycleSoakSurfaceError::Outdated),
                FaultActionKind::SurfaceTimeout => {
                    renderer.inject_lifecycle_soak_surface_error(LifecycleSoakSurfaceError::Timeout)
                }
            }
            tracing::info!(
                action = ?action.action,
                after_millis = action.after_millis,
                "applied lifecycle-soak validation fault"
            );
            self.next_action += 1;
            window.request_redraw();
        }
    }
}

fn validate_actions(actions: &[FaultAction]) -> Result<()> {
    if actions.is_empty() {
        bail!("lifecycle-soak fault plan must contain actions");
    }
    let mut previous = None;
    for action in actions {
        if previous.is_some_and(|value| action.after_millis <= value) {
            bail!("lifecycle-soak actions must have strictly increasing after_millis");
        }
        previous = Some(action.after_millis);
        match action.action {
            FaultActionKind::Resize => {
                if action.width == Some(0)
                    || action.height == Some(0)
                    || action.width.is_none()
                    || action.height.is_none()
                {
                    bail!("lifecycle-soak resize actions require positive dimensions");
                }
            }
            _ if action.width.is_some() || action.height.is_some() => {
                bail!("only lifecycle-soak resize actions may declare dimensions");
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_manifest(source: &str) -> Result<LifecycleSoakRuntime> {
        let manifest: Manifest = serde_json::from_str(source)?;
        if manifest.schema != 1 || manifest.vanilla_version != "26.1" {
            bail!("invalid test manifest header");
        }
        validate_actions(&manifest.fault_plan.actions)?;
        Ok(LifecycleSoakRuntime {
            force_no_audio_device: manifest.fault_plan.force_no_audio_device,
            actions: manifest.fault_plan.actions,
            next_action: 0,
            restore_in_progress: false,
            started_at: None,
        })
    }

    #[test]
    fn checked_in_manifest_projects_the_validation_fault_contract() {
        let source = include_str!("../../../scripts/fixtures/lifecycle-soak/manifest-26.1.json");
        let runtime = parse_manifest(source).unwrap();
        assert!(runtime.force_no_audio_device());
        assert_eq!(runtime.actions.len(), 7);
        assert_eq!(runtime.actions[0].action, FaultActionKind::Resize);
        assert!(!runtime.restore_in_progress);
        assert_eq!(
            runtime.actions[runtime.actions.len() - 1].action,
            FaultActionKind::Resize
        );
    }

    #[test]
    fn malformed_resize_is_rejected_before_the_event_loop() {
        let source = r#"{
            "schema": 1,
            "vanilla_version": "26.1",
            "fault_plan": {
                "force_no_audio_device": true,
                "actions": [{
                    "after_millis": 1,
                    "action": "resize",
                    "width": 0,
                    "height": 720
                }]
            }
        }"#;
        let error = parse_manifest(source).err().unwrap().to_string();
        assert!(error.contains("positive dimensions"));
    }
}
