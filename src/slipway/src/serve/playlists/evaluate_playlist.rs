use std::{collections::HashSet, sync::Arc};

use chrono::{DateTime, Datelike, TimeZone, Utc, Weekday};
use chrono_tz::Tz;

use crate::{
    primitives::{PlaylistName, RigName},
    serve::responses::ServeError,
};

use super::{
    super::{
        ServeState,
        repository::{Playlist, PlaylistItem, PlaylistTimeSpan},
    },
    evaluate_refresh_time::get_next_refresh_time,
};

pub(super) struct PlaylistResult {
    pub refresh_rate_seconds: u32,
    pub rig: RigName,
}

pub(super) async fn evaluate_playlist(
    state: Arc<ServeState>,
    playlist_name: &PlaylistName,
) -> Result<Option<PlaylistResult>, ServeError> {
    let playlist = state.repository.get_playlist(playlist_name).await?;
    let timezone = state.config.timezone.unwrap_or_default();
    evaluate_playlist_and_refresh(playlist, timezone).map_err(ServeError::Internal)
}

fn evaluate_playlist_and_refresh(
    playlist: Playlist,
    timezone: Tz,
) -> anyhow::Result<Option<PlaylistResult>> {
    let now = timezone.from_utc_datetime(&Utc::now().naive_utc());

    let playlist_item = find_active_playlist_item(&playlist, now);

    let playlist_item = match playlist_item {
        Some(playlist_item) => playlist_item,
        None => return Ok(None),
    };

    let next = get_next_refresh_time(now, &playlist_item.refresh, &playlist)?;

    let duration = next - now;
    let refresh_rate_seconds = duration.num_seconds() as u32;

    let rig = playlist_item.rig.clone();
    Ok(Some(PlaylistResult {
        refresh_rate_seconds,
        rig,
    }))
}

fn find_active_playlist_item(playlist: &Playlist, now: DateTime<Tz>) -> Option<&PlaylistItem> {
    let playlist_item = playlist.schedule.iter().find(|item| {
        let days = &item.days;
        if let Some(days) = days {
            if !is_today_in_days(days, now) {
                return false;
            }
        }

        let span = &item.time;
        if let Some(span) = span {
            if !is_now_in_timespan(span, now) {
                return false;
            }
        }

        true
    });

    playlist_item
}

fn is_today_in_days(days: &HashSet<Weekday>, now: DateTime<Tz>) -> bool {
    if days.is_empty() {
        return false;
    }
    let today = now.weekday();
    days.contains(&today)
}

fn is_now_in_timespan(span: &PlaylistTimeSpan, now: DateTime<Tz>) -> bool {
    let now = now.time();

    match span {
        PlaylistTimeSpan::From { from } => now >= *from,
        PlaylistTimeSpan::To { to } => now <= *to,
        PlaylistTimeSpan::Between { from, to } => {
            // Handle overnight spans where from > to
            if from <= to {
                now >= *from && now <= *to
            } else {
                now >= *from || now <= *to
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDateTime, NaiveTime};

    use crate::serve::repository::Refresh;

    use super::*;

    fn tz() -> Tz {
        // Some timezone which is never UTC.
        Tz::Canada__Atlantic
    }

    fn dt(s: &str) -> DateTime<Tz> {
        // Helper for "2025-01-05 14:00:00"
        tz().from_local_datetime(
            &NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .expect("Should be valid date time"),
        )
        .single()
        .expect("Should be unambiguous local time")
    }

    fn midnight_refresh() -> Refresh {
        Refresh::Seconds { seconds: 20 }
    }

    fn standard_refresh() -> Refresh {
        Refresh::Seconds { seconds: 10 }
    }

    fn create_playlist() -> Playlist {
        Playlist {
            schedule: vec![
                PlaylistItem {
                    time: Some(PlaylistTimeSpan::Between {
                        from: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
                        to: NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
                    }),
                    days: Some(
                        vec![Weekday::Tue, Weekday::Wed, Weekday::Thu]
                            .into_iter()
                            .collect(),
                    ),
                    refresh: midnight_refresh(),
                    rig: RigName("rig20".to_string()),
                },
                PlaylistItem {
                    time: None,
                    days: None,
                    refresh: standard_refresh(),
                    rig: RigName("rig10".to_string()),
                },
            ],
        }
    }

    #[test]
    fn monday_during_midnight_refresh_time_should_return_standard() {
        let playlist = create_playlist();

        assert_eq!(
            &standard_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-06 23:30:00"))
                .unwrap()
                .refresh
        );
    }

    #[test]
    fn tuesday_morning_during_midnight_refresh_time_should_return_midnight() {
        let playlist = create_playlist();

        // This result isn't necessarily expected, but it is reasonable.
        // If it isn't the desired behavior then a more complicated playlist can be used.
        assert_eq!(
            &midnight_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-07 00:30:00"))
                .unwrap()
                .refresh
        );
    }

    #[test]
    fn tuesday_night_during_midnight_refresh_time_should_return_midnight() {
        let playlist = create_playlist();

        assert_eq!(
            &midnight_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-07 23:30:00"))
                .unwrap()
                .refresh
        );
    }

    #[test]
    fn wednesday_morning_during_midnight_refresh_time_should_return_midnight() {
        let playlist = create_playlist();

        assert_eq!(
            &midnight_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-08 00:30:00"))
                .unwrap()
                .refresh
        );
    }

    #[test]
    fn thursday_night_during_midnight_refresh_time_should_return_midnight() {
        let playlist = create_playlist();

        assert_eq!(
            &midnight_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-09 23:30:00"))
                .unwrap()
                .refresh
        );
    }

    #[test]
    fn friday_morning_during_midnight_refresh_time_should_return_standard() {
        let playlist = create_playlist();

        assert_eq!(
            &standard_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-10 00:30:00"))
                .unwrap()
                .refresh
        );
    }

    #[test]
    fn wednesday_outside_midnight_refresh_time_should_return_standard() {
        let playlist = create_playlist();

        assert_eq!(
            &standard_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-08 22:55:00"))
                .unwrap()
                .refresh
        );
    }

    #[test]
    fn wednesday_outside_midnight_refresh_time_should_return_standard_2() {
        let playlist = create_playlist();

        assert_eq!(
            &standard_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-08 01:15:00"))
                .unwrap()
                .refresh
        );
    }

    #[test]
    fn when_no_days_specified_should_apply_to_all_days() {
        let playlist = Playlist {
            schedule: vec![
                PlaylistItem {
                    time: Some(PlaylistTimeSpan::Between {
                        from: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
                        to: NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
                    }),
                    days: None,
                    refresh: midnight_refresh(),
                    rig: RigName("rig20".to_string()),
                },
                PlaylistItem {
                    time: None,
                    days: None,
                    refresh: standard_refresh(),
                    rig: RigName("rig10".to_string()),
                },
            ],
        };

        assert_eq!(
            &standard_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-05 22:30:00"))
                .unwrap()
                .refresh
        );

        assert_eq!(
            &midnight_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-05 23:30:00"))
                .unwrap()
                .refresh
        );

        assert_eq!(
            &midnight_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-05 00:30:00"))
                .unwrap()
                .refresh
        );

        assert_eq!(
            &standard_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-05 01:30:00"))
                .unwrap()
                .refresh
        );
    }

    #[test]
    fn when_no_times_specified_should_apply_to_all_times() {
        let playlist = Playlist {
            schedule: vec![
                PlaylistItem {
                    time: None,
                    days: Some(
                        vec![Weekday::Tue, Weekday::Wed, Weekday::Thu]
                            .into_iter()
                            .collect(),
                    ),
                    refresh: midnight_refresh(),
                    rig: RigName("rig20".to_string()),
                },
                PlaylistItem {
                    time: None,
                    days: None,
                    refresh: standard_refresh(),
                    rig: RigName("rig10".to_string()),
                },
            ],
        };

        assert_eq!(
            &standard_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-05 22:30:00"))
                .unwrap()
                .refresh
        );

        assert_eq!(
            &standard_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-06 22:30:00"))
                .unwrap()
                .refresh
        );

        assert_eq!(
            &midnight_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-07 22:30:00"))
                .unwrap()
                .refresh
        );

        assert_eq!(
            &midnight_refresh(),
            &find_active_playlist_item(&playlist, dt("2025-01-08 22:30:00"))
                .unwrap()
                .refresh
        );
    }
}
