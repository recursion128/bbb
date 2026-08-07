use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{Context, Result};
const DEVICE_QUERY_INTERVAL: Duration = Duration::from_millis(1_000);
const CALLBACK_QUERY_INTERVAL: Duration = Duration::from_millis(50);

/// The device-name state `DeviceList` exposes to `SoundEngine`. Names and
/// ordering deliberately remain observable: Vanilla compares the complete
/// record before applying its reload decision.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioDeviceSnapshot {
    pub default_device: Option<String>,
    pub all_devices: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioDeviceReloadReason {
    CurrentDeviceDisconnected,
    CurrentDeviceDisappeared,
    SystemDefaultChanged,
    PreferredDeviceAvailable,
}

impl AudioDeviceReloadReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CurrentDeviceDisconnected => "current-device-disconnected",
            Self::CurrentDeviceDisappeared => "current-device-disappeared",
            Self::SystemDefaultChanged => "system-default-changed",
            Self::PreferredDeviceAvailable => "preferred-device-available",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioDeviceObservation {
    pub preferred_device: Option<String>,
    pub current_device: Option<String>,
    pub default_device: Option<String>,
    pub available_device_count: usize,
    pub reloads: u64,
    pub reload_failures: u64,
    pub query_failures: u64,
    pub preferred_fallbacks: u64,
    pub last_reload_reason: Option<String>,
    pub last_error: Option<String>,
    pub requested_directional_audio: bool,
    pub hrtf_extension: bool,
    pub hrtf_specifiers: i32,
    pub hrtf_status: i32,
    pub output_limiter: bool,
    pub supports_disconnection: bool,
    pub tracking_mode: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioDevicePoll {
    pub reload_reason: Option<AudioDeviceReloadReason>,
    pub error: Option<String>,
    pub observation_changed: bool,
}

/// `Options.isSoundDeviceDefault`: only the empty persisted string means the
/// system default. Whitespace and display labels are real preferred names.
pub fn is_sound_device_default(device_name: &str) -> bool {
    device_name.is_empty()
}

/// `SoundEngine.shouldChangeDevice`, preserving its branch order. A backend
/// disconnect is handled before device-list polling. The remaining branches
/// only run when the full device snapshot changed.
pub fn audio_device_reload_reason(
    disconnected: bool,
    preferred_device: &str,
    current_device: Option<&str>,
    previous: &AudioDeviceSnapshot,
    current: &AudioDeviceSnapshot,
) -> Option<AudioDeviceReloadReason> {
    if disconnected {
        return Some(AudioDeviceReloadReason::CurrentDeviceDisconnected);
    }
    if previous == current {
        return None;
    }
    if current_device.is_some_and(|name| !current.all_devices.iter().any(|item| item == name)) {
        return Some(AudioDeviceReloadReason::CurrentDeviceDisappeared);
    }
    if is_sound_device_default(preferred_device) {
        if current_device != current.default_device.as_deref() {
            return Some(AudioDeviceReloadReason::SystemDefaultChanged);
        }
    } else if current
        .all_devices
        .iter()
        .any(|name| name == preferred_device)
    {
        if current_device != Some(preferred_device) {
            return Some(AudioDeviceReloadReason::PreferredDeviceAvailable);
        }
    }
    None
}

pub(crate) struct AudioDeviceInventory {
    pub(crate) snapshot: AudioDeviceSnapshot,
}

impl AudioDeviceInventory {
    pub(crate) fn empty() -> Self {
        Self {
            snapshot: AudioDeviceSnapshot::default(),
        }
    }
}

fn query_audio_devices(openal_library_path: &Path) -> Result<AudioDeviceInventory> {
    Ok(AudioDeviceInventory {
        snapshot: crate::openal::query_device_snapshot(openal_library_path)?,
    })
}

pub(crate) fn initial_audio_devices(
    openal_library_path: &Path,
) -> (AudioDeviceInventory, Option<String>) {
    match query_audio_devices(openal_library_path) {
        Ok(inventory) => (inventory, None),
        Err(error) => (AudioDeviceInventory::empty(), Some(format!("{error:#}"))),
    }
}

/// A single bounded worker mirrors `PollingDeviceTracker`: the potentially
/// blocking host query never runs on the application/render thread, at most
/// one unconsumed snapshot is retained, and dropping the owner wakes and joins
/// the worker instead of leaking per-poll threads.
pub(crate) struct AudioDeviceTracker {
    updates: Receiver<std::result::Result<AudioDeviceInventory, String>>,
    shutdown: SyncSender<()>,
    worker: Option<JoinHandle<()>>,
    openal_library_path: PathBuf,
    tracking_mode: &'static str,
}

impl AudioDeviceTracker {
    pub(crate) fn spawn(openal_library_path: PathBuf) -> Result<Self> {
        let (update_sender, updates) = mpsc::sync_channel(1);
        let (shutdown, shutdown_receiver) = mpsc::sync_channel(1);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let worker_library_path = openal_library_path.clone();
        let worker = thread::Builder::new()
            .name("bbb-audio-device-tracker".to_owned())
            .spawn(move || {
                let enumerator = crate::openal::OpenAlDeviceEnumerator::open(&worker_library_path)
                    .map_err(|error| format!("{error:#}"));
                let system_events = enumerator
                    .as_ref()
                    .ok()
                    .and_then(|enumerator| enumerator.install_system_event_tracker());
                let tracking_mode = if system_events.is_some() {
                    "openal-soft-system-events"
                } else {
                    "openal-polling-1000ms"
                };
                if ready_sender.send(tracking_mode).is_err() {
                    return;
                }
                loop {
                    let interval = if system_events.is_some() {
                        CALLBACK_QUERY_INTERVAL
                    } else {
                        DEVICE_QUERY_INTERVAL
                    };
                    match shutdown_receiver.recv_timeout(interval) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    if system_events
                        .as_ref()
                        .is_some_and(|events| !events.take_update_request())
                    {
                        continue;
                    }
                    let update = match &enumerator {
                        Ok(enumerator) => enumerator
                            .snapshot()
                            .map(|snapshot| AudioDeviceInventory { snapshot })
                            .map_err(|error| format!("{error:#}")),
                        Err(error) => Err(error.clone()),
                    };
                    match update_sender.try_send(update) {
                        Ok(()) | Err(TrySendError::Full(_)) => {}
                        Err(TrySendError::Disconnected(_)) => break,
                    }
                }
            })
            .context("spawn bounded audio device tracker")?;
        let tracking_mode = ready_receiver
            .recv()
            .context("initialize OpenAL audio device tracker")?;
        Ok(Self {
            updates,
            shutdown,
            worker: Some(worker),
            openal_library_path,
            tracking_mode,
        })
    }

    pub(crate) fn latest(&self) -> Option<std::result::Result<AudioDeviceInventory, String>> {
        let mut latest = None;
        loop {
            match self.updates.try_recv() {
                Ok(update) => latest = Some(update),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return latest,
            }
        }
    }

    pub(crate) fn force_refresh(&self) -> std::result::Result<AudioDeviceInventory, String> {
        let _ = self.latest();
        query_audio_devices(&self.openal_library_path).map_err(|error| format!("{error:#}"))
    }

    pub(crate) fn tracking_mode(&self) -> &'static str {
        self.tracking_mode
    }
}

impl Drop for AudioDeviceTracker {
    fn drop(&mut self) {
        let _ = self.shutdown.try_send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(default: Option<&str>, devices: &[&str]) -> AudioDeviceSnapshot {
        AudioDeviceSnapshot {
            default_device: default.map(str::to_owned),
            all_devices: devices.iter().map(|name| (*name).to_owned()).collect(),
        }
    }

    #[test]
    fn only_the_empty_name_selects_the_default_device() {
        assert!(is_sound_device_default(""));
        assert!(!is_sound_device_default(" "));
        assert!(!is_sound_device_default("default"));
    }

    #[test]
    fn reload_decision_preserves_vanilla_branch_order() {
        let before = snapshot(Some("speakers"), &["speakers", "headset"]);
        let after = snapshot(Some("headset"), &["headset", "speakers"]);

        assert_eq!(
            audio_device_reload_reason(true, "headset", Some("speakers"), &before, &after),
            Some(AudioDeviceReloadReason::CurrentDeviceDisconnected)
        );

        let disappeared = snapshot(Some("headset"), &["headset"]);
        assert_eq!(
            audio_device_reload_reason(false, "headset", Some("speakers"), &before, &disappeared),
            Some(AudioDeviceReloadReason::CurrentDeviceDisappeared)
        );

        assert_eq!(
            audio_device_reload_reason(false, "", Some("speakers"), &before, &after),
            Some(AudioDeviceReloadReason::SystemDefaultChanged)
        );

        let preferred_appeared = snapshot(Some("speakers"), &["speakers", "usb"]);
        assert_eq!(
            audio_device_reload_reason(
                false,
                "usb",
                Some("speakers"),
                &before,
                &preferred_appeared
            ),
            Some(AudioDeviceReloadReason::PreferredDeviceAvailable)
        );
    }

    #[test]
    fn unchanged_snapshot_never_runs_the_name_branches() {
        let devices = snapshot(Some("speakers"), &["speakers"]);
        assert_eq!(
            audio_device_reload_reason(false, "usb", Some("missing"), &devices, &devices),
            None
        );
    }

    #[test]
    fn a_changed_irrelevant_device_does_not_reload_the_current_output() {
        let before = snapshot(Some("speakers"), &["speakers", "hdmi"]);
        let after = snapshot(Some("speakers"), &["speakers", "hdmi", "capture-monitor"]);
        assert_eq!(
            audio_device_reload_reason(false, "usb", Some("speakers"), &before, &after),
            None
        );
    }

    #[test]
    fn preferred_device_matching_uses_the_exact_openal_enumeration_name() {
        let before = snapshot(
            Some("OpenAL Soft on Built-in Audio"),
            &["OpenAL Soft on Built-in Audio"],
        );
        let after = snapshot(
            Some("OpenAL Soft on Built-in Audio"),
            &[
                "OpenAL Soft on Built-in Audio",
                "OpenAL Soft on USB Headset",
            ],
        );
        assert_eq!(
            audio_device_reload_reason(
                false,
                "OpenAL Soft on USB Headset",
                Some("OpenAL Soft on Built-in Audio"),
                &before,
                &after,
            ),
            Some(AudioDeviceReloadReason::PreferredDeviceAvailable)
        );
    }

    #[test]
    #[ignore = "requires BBB_TEST_OPENAL_LIBRARY and an explicitly selected OpenAL driver"]
    fn verified_device_tracker_runs_selected_mode_and_forced_refresh() {
        let _receipt = crate::openal::OPENAL_RECEIPT_LOCK.lock().unwrap();
        let library = std::env::var_os("BBB_TEST_OPENAL_LIBRARY")
            .expect("BBB_TEST_OPENAL_LIBRARY must name the verified OpenAL library");
        let tracker = AudioDeviceTracker::spawn(PathBuf::from(library)).unwrap();
        assert!(matches!(
            tracker.tracking_mode(),
            "openal-soft-system-events" | "openal-polling-1000ms"
        ));
        let snapshot = tracker.force_refresh().unwrap().snapshot;
        assert!(snapshot.all_devices.iter().all(|name| !name.is_empty()));
    }
}
