use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use bbb_pack::{PeriodicNotification, PeriodicNotificationCatalog};
use isocountry::CountryCode;

const MINUTE: Duration = Duration::from_secs(60);
const MAX_CATCH_UP_TICKS_PER_FRAME: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PeriodicNotificationSchedule {
    generation: u64,
    notifications: Vec<PeriodicNotification>,
    initial_delay_minutes: u64,
    period_minutes: u64,
}

impl PeriodicNotificationSchedule {
    pub(super) fn prepare(
        catalog: &PeriodicNotificationCatalog,
        country_iso3: Option<&str>,
    ) -> Result<Option<Self>> {
        let Some(country_iso3) = country_iso3 else {
            return Ok(None);
        };
        prepare_selected_notifications(
            catalog.generation(),
            catalog.notifications_for_iso3(country_iso3),
        )
    }
}

fn prepare_selected_notifications(
    generation: u64,
    notifications: &[PeriodicNotification],
) -> Result<Option<PeriodicNotificationSchedule>> {
    if notifications.is_empty()
        || notifications
            .iter()
            .any(|notification| notification.period == 0)
    {
        return Ok(None);
    }
    let initial_delay = notifications
        .iter()
        .map(|notification| notification.delay)
        .min()
        .expect("non-empty periodic notification selection");
    if initial_delay < 0 {
        bail!("periodic notification initial delay cannot be negative: {initial_delay}");
    }
    let period = notifications
        .iter()
        .try_fold(0_i64, |period, notification| {
            if notification.period < 0 {
                bail!(
                    "periodic notification period cannot be negative: {}",
                    notification.period
                );
            }
            let relative_delay = notification
                .delay
                .checked_sub(initial_delay)
                .context("periodic notification relative delay overflow")?;
            Ok::<_, anyhow::Error>(greatest_common_divisor(
                period,
                greatest_common_divisor(relative_delay, notification.period),
            ))
        })?;
    if period <= 0 {
        bail!("periodic notification optimal period must be positive");
    }
    let initial_delay_minutes =
        u64::try_from(initial_delay).context("periodic notification initial delay conversion")?;
    let period_minutes =
        u64::try_from(period).context("periodic notification period conversion")?;
    minutes_duration(initial_delay_minutes)
        .context("periodic notification initial delay duration overflow")?;
    minutes_duration(period_minutes).context("periodic notification period duration overflow")?;
    Ok(Some(PeriodicNotificationSchedule {
        generation,
        notifications: notifications.to_vec(),
        initial_delay_minutes,
        period_minutes,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PeriodicNotificationToast {
    pub(super) title_key: String,
    pub(super) message_key: String,
    pub(super) elapsed_periods: i64,
    pub(super) sequence: usize,
}

#[derive(Debug, Default)]
pub(super) struct PeriodicNotificationRuntime {
    generation: u64,
    schedule: Option<ActiveSchedule>,
    toast: Option<PeriodicNotificationToast>,
    next_sequence: usize,
}

#[derive(Debug)]
struct ActiveSchedule {
    notifications: Vec<PeriodicNotification>,
    period_minutes: u64,
    elapsed_minutes: u64,
    next_tick_at: Instant,
}

impl PeriodicNotificationRuntime {
    pub(super) fn install(
        &mut self,
        generation: u64,
        schedule: Option<PeriodicNotificationSchedule>,
        now: Instant,
    ) {
        let previous_elapsed = self
            .schedule
            .as_ref()
            .map_or(0, |schedule| schedule.elapsed_minutes);
        let Some(schedule) = schedule else {
            self.generation = generation;
            self.schedule = None;
            self.toast = None;
            return;
        };
        debug_assert_eq!(generation, schedule.generation);
        self.generation = generation;
        let elapsed_minutes = if self.schedule.is_some() {
            previous_elapsed
        } else {
            schedule.initial_delay_minutes
        };
        let initial_delay = minutes_duration(schedule.initial_delay_minutes)
            .expect("prepared periodic initial delay fits Duration");
        let Some(next_tick_at) = now.checked_add(initial_delay) else {
            self.schedule = None;
            return;
        };
        self.schedule = Some(ActiveSchedule {
            notifications: schedule.notifications,
            period_minutes: schedule.period_minutes,
            elapsed_minutes,
            next_tick_at,
        });
        self.toast = None;
    }

    pub(super) fn update(&mut self, now: Instant) -> Option<&PeriodicNotificationToast> {
        let Some(schedule) = self.schedule.as_mut() else {
            return self.toast.as_ref();
        };
        let period_duration = minutes_duration(schedule.period_minutes)
            .expect("prepared periodic period fits Duration");
        let mut catch_up_ticks = 0_usize;
        while now >= schedule.next_tick_at && catch_up_ticks < MAX_CATCH_UP_TICKS_PER_FRAME {
            let current_minute = schedule.elapsed_minutes;
            let next_minute = current_minute.saturating_add(schedule.period_minutes);
            if let Some(notification) = schedule.notifications.iter().find(|notification| {
                let Ok(delay) = u64::try_from(notification.delay) else {
                    return false;
                };
                let Ok(period) = u64::try_from(notification.period) else {
                    return false;
                };
                current_minute >= delay && current_minute / period != next_minute / period
            }) {
                self.next_sequence = self.next_sequence.saturating_add(1);
                self.toast = Some(PeriodicNotificationToast {
                    title_key: notification.title.clone(),
                    message_key: notification.message.clone(),
                    elapsed_periods: i64::try_from(current_minute / notification.period as u64)
                        .unwrap_or(i64::MAX),
                    sequence: self.next_sequence,
                });
            }
            schedule.elapsed_minutes = next_minute;
            schedule.next_tick_at = schedule
                .next_tick_at
                .checked_add(period_duration)
                .unwrap_or(now);
            catch_up_ticks += 1;
        }
        if catch_up_ticks == MAX_CATCH_UP_TICKS_PER_FRAME && now >= schedule.next_tick_at {
            let overdue = now.duration_since(schedule.next_tick_at);
            let skipped_ticks = overdue
                .as_secs()
                .checked_div(period_duration.as_secs())
                .unwrap_or_default()
                .saturating_add(1);
            schedule.elapsed_minutes = schedule
                .elapsed_minutes
                .saturating_add(schedule.period_minutes.saturating_mul(skipped_ticks));
            schedule.next_tick_at = period_duration
                .as_secs()
                .checked_mul(skipped_ticks)
                .map(Duration::from_secs)
                .and_then(|skipped| schedule.next_tick_at.checked_add(skipped))
                .or_else(|| now.checked_add(period_duration))
                .unwrap_or(now);
        }
        self.toast.as_ref()
    }

    #[cfg(test)]
    pub(super) fn generation(&self) -> u64 {
        self.generation
    }
}

pub(super) fn host_country_iso3() -> Option<String> {
    country_iso3_from_locale(sys_locale::get_locale().as_deref())
}

fn country_iso3_from_locale(locale: Option<&str>) -> Option<String> {
    let locale = locale?
        .split(['.', '@'])
        .next()
        .unwrap_or_default()
        .replace('_', "-");
    locale
        .split('-')
        .skip(1)
        .find_map(|subtag| match subtag.len() {
            2 if subtag.bytes().all(|byte| byte.is_ascii_alphabetic()) => {
                CountryCode::for_alpha2_caseless(subtag).ok()
            }
            3 if subtag.bytes().all(|byte| byte.is_ascii_alphabetic()) => {
                CountryCode::for_alpha3_caseless(subtag).ok()
            }
            _ => None,
        })
        .map(|country| country.alpha3().to_string())
}

fn greatest_common_divisor(mut left: i64, mut right: i64) -> i64 {
    debug_assert!(left >= 0 && right >= 0);
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}

fn minutes_duration(minutes: u64) -> Option<Duration> {
    minutes
        .checked_mul(MINUTE.as_secs())
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_country_uses_iso_3166_alpha3_like_java_locale() {
        assert_eq!(
            country_iso3_from_locale(Some("ko_KR.UTF-8")),
            Some("KOR".to_string())
        );
        assert_eq!(
            country_iso3_from_locale(Some("zh-Hans-CN")),
            Some("CHN".to_string())
        );
        assert_eq!(
            country_iso3_from_locale(Some("en-US")),
            Some("USA".to_string())
        );
        assert_eq!(country_iso3_from_locale(Some("C.UTF-8")), None);
        assert_eq!(country_iso3_from_locale(Some("en")), None);
    }

    #[test]
    fn notification_tick_uses_vanilla_delay_period_and_first_match() {
        let start = Instant::now();
        let mut runtime = PeriodicNotificationRuntime::default();
        runtime.install(
            7,
            Some(PeriodicNotificationSchedule {
                generation: 7,
                notifications: vec![
                    PeriodicNotification {
                        delay: 2,
                        period: 4,
                        title: "first".to_string(),
                        message: "first.message".to_string(),
                    },
                    PeriodicNotification {
                        delay: 0,
                        period: 2,
                        title: "second".to_string(),
                        message: "second.message".to_string(),
                    },
                ],
                initial_delay_minutes: 0,
                period_minutes: 2,
            }),
            start,
        );
        let toast = runtime.update(start).expect("first scheduled task");
        assert_eq!(toast.title_key, "second");
        assert_eq!(toast.elapsed_periods, 0);
        let toast = runtime
            .update(start + Duration::from_secs(120))
            .expect("second scheduled task");
        assert_eq!(toast.title_key, "first");
        assert_eq!(toast.elapsed_periods, 0);
        assert_eq!(runtime.generation(), 7);
    }

    #[test]
    fn due_notification_projects_through_the_shared_system_toast_surface() {
        let start = Instant::now();
        let mut hud = super::super::HudRenderRuntime::default();
        hud.install_periodic_notifications(
            12,
            Some(PeriodicNotificationSchedule {
                generation: 12,
                notifications: vec![PeriodicNotification {
                    delay: 0,
                    period: 1,
                    title: "periodic.fixture.title".to_string(),
                    message: "periodic.fixture.message".to_string(),
                }],
                initial_delay_minutes: 0,
                period_minutes: 1,
            }),
            start,
        );
        let world = bbb_world::WorldStore::new();
        let (toast, sound) = super::super::hud_system_toast(&mut hud, &world, None, start);
        let toast = toast.expect("due periodic system toast");
        assert_eq!(toast.title, "periodic.fixture.title");
        assert_eq!(toast.message, "periodic.fixture.message");
        assert!(!toast.multiline);
        assert_eq!(
            sound.and_then(|sound| sound.sound.location),
            Some("minecraft:ui.toast.in".to_string())
        );
    }

    #[test]
    fn schedule_uses_gcd_and_rejects_invalid_signed_timer_values() {
        let notifications = [
            PeriodicNotification {
                delay: 2,
                period: 6,
                title: "first".to_string(),
                message: "first.message".to_string(),
            },
            PeriodicNotification {
                delay: 8,
                period: 10,
                title: "second".to_string(),
                message: "second.message".to_string(),
            },
        ];
        let schedule = prepare_selected_notifications(9, &notifications)
            .unwrap()
            .unwrap();
        assert_eq!(schedule.initial_delay_minutes, 2);
        assert_eq!(schedule.period_minutes, 2);

        let zero = [PeriodicNotification {
            period: 0,
            ..notifications[0].clone()
        }];
        assert!(prepare_selected_notifications(9, &zero).unwrap().is_none());
        for invalid in [
            PeriodicNotification {
                delay: -1,
                ..notifications[0].clone()
            },
            PeriodicNotification {
                period: -1,
                ..notifications[0].clone()
            },
        ] {
            assert!(prepare_selected_notifications(9, &[invalid]).is_err());
        }
    }

    #[test]
    fn reload_preserves_elapsed_but_restarts_selected_initial_delay() {
        let start = Instant::now();
        let mut runtime = PeriodicNotificationRuntime::default();
        let notification = PeriodicNotification {
            delay: 0,
            period: 2,
            title: "title".to_string(),
            message: "message".to_string(),
        };
        runtime.install(
            1,
            Some(PeriodicNotificationSchedule {
                generation: 1,
                notifications: vec![notification.clone()],
                initial_delay_minutes: 0,
                period_minutes: 2,
            }),
            start,
        );
        runtime.update(start);
        runtime.update(start + Duration::from_secs(120));
        runtime.install(
            2,
            Some(PeriodicNotificationSchedule {
                generation: 2,
                notifications: vec![notification],
                initial_delay_minutes: 3,
                period_minutes: 2,
            }),
            start + Duration::from_secs(121),
        );
        let before = runtime.toast.clone();
        assert_eq!(
            runtime.update(start + Duration::from_secs(300)),
            before.as_ref()
        );
        let toast = runtime
            .update(start + Duration::from_secs(301))
            .expect("reloaded task");
        assert_eq!(toast.elapsed_periods, 2);
        assert_eq!(runtime.generation(), 2);
    }

    #[test]
    fn bounded_catch_up_advances_elapsed_without_replaying_an_unbounded_backlog() {
        let start = Instant::now();
        let mut runtime = PeriodicNotificationRuntime::default();
        runtime.install(
            1,
            Some(PeriodicNotificationSchedule {
                generation: 1,
                notifications: vec![PeriodicNotification {
                    delay: 0,
                    period: 1,
                    title: "title".to_string(),
                    message: "message".to_string(),
                }],
                initial_delay_minutes: 0,
                period_minutes: 1,
            }),
            start,
        );
        let now = start + Duration::from_secs(60 * 1_030);
        runtime.update(now);

        let schedule = runtime.schedule.as_ref().expect("active schedule");
        assert_eq!(schedule.elapsed_minutes, 1_031);
        assert_eq!(
            schedule.next_tick_at,
            start + Duration::from_secs(60 * 1_031)
        );
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 resources"]
    fn local_kor_catalog_prepares_and_emits_real_notification_keys() {
        let catalog = bbb_pack::PeriodicNotificationCatalog::load(
            &bbb_pack::PackRoots::from_environment().expect("local vanilla roots"),
        )
        .expect("local periodic catalog");
        let schedule = PeriodicNotificationSchedule::prepare(&catalog, Some("KOR"))
            .expect("prepare KOR schedule")
            .expect("KOR notifications");
        assert_eq!(schedule.notifications.len(), 2);
        let start = Instant::now();
        let initial_delay = schedule.initial_delay_minutes;
        let mut runtime = PeriodicNotificationRuntime::default();
        runtime.install(catalog.generation(), Some(schedule), start);
        let toast = runtime
            .update(start + minutes_duration(initial_delay).unwrap())
            .expect("first KOR notification");
        assert!(!toast.title_key.is_empty());
        assert!(!toast.message_key.is_empty());
    }
}
