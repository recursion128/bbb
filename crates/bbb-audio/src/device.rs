use std::{
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait},
    Device,
};

const DEVICE_QUERY_INTERVAL: Duration = Duration::from_millis(1_000);
const OPEN_AL_SOFT_PREFIX: &str = "OpenAL Soft on ";

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
    } else if let Some(preferred_backend_name) =
        matching_preferred_device_name(preferred_device, &current.all_devices)
    {
        if current_device != Some(preferred_backend_name) {
            return Some(AudioDeviceReloadReason::PreferredDeviceAvailable);
        }
    }
    None
}

pub(crate) struct AudioDeviceInventory {
    pub(crate) snapshot: AudioDeviceSnapshot,
    devices: Vec<NamedDevice>,
}

struct NamedDevice {
    name: String,
    device: Device,
}

impl AudioDeviceInventory {
    pub(crate) fn empty() -> Self {
        Self {
            snapshot: AudioDeviceSnapshot::default(),
            devices: Vec::new(),
        }
    }

    pub(crate) fn preferred_device(&self, preferred: &str) -> Option<(Device, String)> {
        let name = matching_preferred_device_name(preferred, &self.snapshot.all_devices)?;
        self.devices
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| (entry.device.clone(), entry.name.clone()))
    }
}

/// Vanilla persists the OpenAL enumeration string but hides this stable prefix
/// in its options UI. CPAL exposes the underlying display name without that
/// transport prefix on common desktop backends, so exact matching wins first
/// and the UI-equivalent stripped form is the only adapter fallback.
fn matching_preferred_device_name<'a>(
    preferred: &'a str,
    available: &'a [String],
) -> Option<&'a str> {
    available
        .iter()
        .find(|name| name.as_str() == preferred)
        .map(String::as_str)
        .or_else(|| {
            let stripped = preferred.strip_prefix(OPEN_AL_SOFT_PREFIX)?;
            available
                .iter()
                .find(|name| name.as_str() == stripped)
                .map(String::as_str)
        })
}

fn query_audio_devices() -> Result<AudioDeviceInventory> {
    let host = cpal::default_host();
    let default_entry = host.default_output_device().and_then(|device| {
        let name = device.description().ok()?.name().to_owned();
        Some(NamedDevice { name, device })
    });
    let default_device = default_entry.as_ref().map(|entry| entry.name.clone());
    let mut devices = Vec::new();
    for device in host
        .output_devices()
        .context("enumerate CPAL output devices")?
    {
        let Ok(description) = device.description() else {
            continue;
        };
        devices.push(NamedDevice {
            name: description.name().to_owned(),
            device,
        });
    }
    if let Some(default_entry) = default_entry {
        if !devices.iter().any(|entry| entry.name == default_entry.name) {
            devices.push(default_entry);
        }
    }
    let all_devices = devices.iter().map(|entry| entry.name.clone()).collect();
    Ok(AudioDeviceInventory {
        snapshot: AudioDeviceSnapshot {
            default_device,
            all_devices,
        },
        devices,
    })
}

pub(crate) fn initial_audio_devices() -> (AudioDeviceInventory, Option<String>) {
    match query_audio_devices() {
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
}

impl AudioDeviceTracker {
    pub(crate) fn spawn() -> Result<Self> {
        let (update_sender, updates) = mpsc::sync_channel(1);
        let (shutdown, shutdown_receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("bbb-audio-device-tracker".to_owned())
            .spawn(move || loop {
                match shutdown_receiver.recv_timeout(DEVICE_QUERY_INTERVAL) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let update = query_audio_devices().map_err(|error| format!("{error:#}"));
                match update_sender.try_send(update) {
                    Ok(()) | Err(TrySendError::Full(_)) => {}
                    Err(TrySendError::Disconnected(_)) => break,
                }
            })
            .context("spawn bounded audio device tracker")?;
        Ok(Self {
            updates,
            shutdown,
            worker: Some(worker),
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
    fn vanilla_openal_display_prefix_maps_to_the_cpal_device_name() {
        let before = snapshot(Some("Built-in Audio"), &["Built-in Audio"]);
        let after = snapshot(Some("Built-in Audio"), &["Built-in Audio", "USB Headset"]);
        assert_eq!(
            audio_device_reload_reason(
                false,
                "OpenAL Soft on USB Headset",
                Some("Built-in Audio"),
                &before,
                &after,
            ),
            Some(AudioDeviceReloadReason::PreferredDeviceAvailable)
        );
        assert_eq!(
            matching_preferred_device_name("OpenAL Soft on USB Headset", &after.all_devices),
            Some("USB Headset")
        );
    }
}
