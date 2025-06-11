use anyhow::Context;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone};
use chrono_tz::Tz;
use tracing::debug;

use super::super::repository::{Playlist, PlaylistTimeSpan, Refresh};

pub(super) fn get_next_refresh_time(
    now: DateTime<Tz>,
    refresh: &Refresh,
    playlist: &Playlist,
) -> anyhow::Result<DateTime<Tz>> {
    let specified_next = get_next_specified_refresh_time(now, refresh)?;

    // If the specified refresh is zero or in the past, just return a sensible future value.
    if specified_next <= now {
        debug!("Specified refresh is zero or in the past, returning a minute from now.");
        return Ok(now + chrono::Duration::minutes(1));
    }

    let maybe_boundary = get_next_playlist_item_boundary_before_end(now, specified_next, playlist)?;

    // The earliest boundary or specified refresh is our new earliest refresh because we don't
    // want to skip over playlist items.
    if let Some(earliest_boundary) = maybe_boundary {
        let result = earliest_boundary.min(specified_next);
        debug!(
            "Specified refresh is {}. Earliest playlist boundary is at {}. Using {}.",
            specified_next, earliest_boundary, result
        );
        Ok(result)
    } else {
        debug!("Using specified refresh {}.", specified_next);
        Ok(specified_next)
    }
}

fn get_next_specified_refresh_time(
    now: DateTime<Tz>,
    refresh: &Refresh,
) -> anyhow::Result<DateTime<Tz>> {
    match refresh {
        Refresh::Seconds { seconds } => Ok(now + chrono::Duration::seconds(*seconds as i64)),
        Refresh::Minutes { minutes } => Ok(now + chrono::Duration::minutes(*minutes as i64)),
        Refresh::Hours { hours } => Ok(now + chrono::Duration::hours(*hours as i64)),
        Refresh::Cron { cron } => {
            let cron_evaluator = croner::Cron::new(cron)
                .with_seconds_optional()
                .parse()
                .with_context(|| format!("Failed to parse cron schedule: {cron}"))?;
            let next = cron_evaluator
                .find_next_occurrence(&now, false)
                .with_context(|| {
                    format!("Failed to evaluate next occurrence with cron schedule: {cron}")
                })?;
            Ok(next)
        }
    }
}

fn get_next_playlist_item_boundary_before_end(
    now: DateTime<Tz>,
    end: DateTime<Tz>,
    playlist: &Playlist,
) -> anyhow::Result<Option<DateTime<Tz>>> {
    let mut boundaries = Vec::new();
    for item in &playlist.schedule {
        // Only consider days that might apply between now and the normal_next day
        for day in days_in_range_inclusive(now, end) {
            // If `days` is Some(..) then check if this day of week is included
            if let Some(valid_days) = &item.days {
                if !valid_days.contains(&day.weekday()) {
                    continue;
                }
            }
            // Pull from/to boundaries (if any) for this day
            if let Some(span) = &item.time {
                match span {
                    PlaylistTimeSpan::From { from } => {
                        if let Some(boundary) = make_boundary(now, end, day, *from) {
                            boundaries.push(boundary);
                        }
                    }
                    PlaylistTimeSpan::To { to } => {
                        if let Some(boundary) = make_boundary(now, end, day, *to) {
                            boundaries.push(boundary);
                        }
                    }
                    PlaylistTimeSpan::Between { from, to } => {
                        if let Some(b) = make_boundary(now, end, day, *from) {
                            boundaries.push(b);
                        }
                        if let Some(b) = make_boundary(now, end, day, *to) {
                            boundaries.push(b);
                        }
                    }
                }
            }
        }
    }

    Ok(boundaries.into_iter().min())
}

/// Returns each local calendar date from `start` to `end` inclusive.
/// Internally, we just compare the two local dates, then iterate over them.
///
/// For convenience, this takes `DateTime<Tz>` rather than `NaiveDate`, so that
/// the caller is free to supply fully-specified instants in a timezone.
fn days_in_range_inclusive(
    start: DateTime<Tz>,
    end: DateTime<Tz>,
) -> impl Iterator<Item = chrono::NaiveDate> {
    let start_day = start.date_naive();
    let end_day = end.date_naive();

    let direction = if start_day <= end_day { 1 } else { -1 };
    let distance = (end_day - start_day).num_days().abs();

    (0..=distance).map(move |offset| start_day + Duration::days(offset * direction))
}

// Make the local DateTime for the given day/time. Then see if it's after `now`
// and before `end`.
fn make_boundary(
    now: DateTime<Tz>,
    end: DateTime<Tz>,
    day: NaiveDate,
    time: NaiveTime,
) -> Option<DateTime<Tz>> {
    let local_candidate = now
        .timezone()
        .from_local_datetime(&day.and_time(time))
        .earliest()?;
    if local_candidate > now && local_candidate < end {
        Some(local_candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{primitives::RigName, serve::repository::PlaylistItem};

    use super::*;
    use chrono::{NaiveDateTime, TimeZone, Weekday};
    use std::{collections::HashSet, str::FromStr};

    fn rig() -> RigName {
        RigName::from_str("test_rig").unwrap()
    }

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

    #[test]
    fn days_in_range() {
        // Pick times close to midnight to reveal any timezone issues.
        let start = dt("2025-01-05 00:15:01");
        let end = dt("2025-01-07 23:57:16");

        let expected_start = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let expected_end = NaiveDate::from_ymd_opt(2025, 1, 7).unwrap();
        let days: Vec<_> = days_in_range_inclusive(start, end).collect();
        assert_eq!(
            days,
            vec![
                expected_start,
                expected_start.succ_opt().unwrap(),
                expected_end
            ]
        );
    }

    #[test]
    fn no_boundaries_earlier_than_normal_refresh() {
        // If no item boundaries fall before the normal refresh, we get the normal refresh back.
        let now = dt("2025-01-05 14:15:16");
        let refresh = Refresh::Hours { hours: 1 };
        let playlist = Playlist { schedule: vec![] };

        let next = get_next_refresh_time(now, &refresh, &playlist).unwrap();
        assert_eq!(next, dt("2025-01-05 15:15:16"));
    }

    fn boundary_before_normal_refresh_inner(days: Option<Vec<Weekday>>) -> DateTime<Tz> {
        let now = dt("2025-01-05 14:00:00");
        let refresh = Refresh::Minutes { minutes: 30 };
        let days = days.map(HashSet::from_iter);

        let item = PlaylistItem {
            time: Some(PlaylistTimeSpan::From {
                from: NaiveTime::from_hms_opt(14, 10, 0).unwrap(),
            }),
            days,
            refresh: Refresh::Hours { hours: 10 },
            rig: rig(),
        };
        let playlist = Playlist {
            schedule: vec![item],
        };

        get_next_refresh_time(now, &refresh, &playlist).unwrap()
    }

    #[test]
    fn boundary_before_normal_refresh_day_specified() {
        let next = boundary_before_normal_refresh_inner(
            Some(vec![Weekday::Sun]), // "2025-01-05" is a Sunday
        );
        assert_eq!(next, dt("2025-01-05 14:10:00"));
    }

    #[test]
    fn boundary_before_normal_refresh_other_day_specified() {
        let next = boundary_before_normal_refresh_inner(
            Some(vec![Weekday::Mon]), // "2025-01-05" is a Sunday
        );

        // Day didn't match.
        assert_eq!(next, dt("2025-01-05 14:30:00"));
    }

    #[test]
    fn boundary_before_normal_refresh_no_day_specified() {
        let next = boundary_before_normal_refresh_inner(None);
        assert_eq!(next, dt("2025-01-05 14:10:00"));
    }

    #[test]
    fn boundary_after_normal_refresh() {
        // The boundary is at 14:50, but our normal refresh is 14:30.
        // We should get 14:30 back.
        let now = dt("2025-01-05 14:00:00");
        let refresh = Refresh::Minutes { minutes: 30 };
        let mut days = HashSet::new();
        days.insert(Weekday::Sun);

        let item = PlaylistItem {
            time: Some(PlaylistTimeSpan::From {
                from: NaiveTime::from_hms_opt(14, 50, 0).unwrap(),
            }),
            days: Some(days),
            refresh: Refresh::Hours { hours: 10 },
            rig: rig(),
        };
        let playlist = Playlist {
            schedule: vec![item],
        };

        let next = get_next_refresh_time(now, &refresh, &playlist).unwrap();
        assert_eq!(next, dt("2025-01-05 14:30:00"));
    }

    #[test]
    fn boundary_between_span() {
        // The item is active between 14:05 and 14:15, and we want to detect that
        // it "turns on" at 14:05, which is earlier than the normal 14:30 refresh.
        let now = dt("2025-01-05 14:00:00");
        let refresh = Refresh::Minutes { minutes: 30 };
        let mut days = HashSet::new();
        days.insert(Weekday::Sun);

        let item = PlaylistItem {
            time: Some(PlaylistTimeSpan::Between {
                from: NaiveTime::from_hms_opt(14, 5, 0).unwrap(),
                to: NaiveTime::from_hms_opt(14, 15, 0).unwrap(),
            }),
            days: Some(days),
            refresh: Refresh::Hours { hours: 10 },
            rig: rig(),
        };
        let playlist = Playlist {
            schedule: vec![item],
        };

        let next = get_next_refresh_time(now, &refresh, &playlist).unwrap();
        assert_eq!(next, dt("2025-01-05 14:05:00"));
    }

    #[test]
    fn cron_no_boundary() {
        let now = dt("2025-01-05 14:00:00");
        let refresh = Refresh::Cron {
            cron: "0 * * * *".to_string(),
        };

        let item = PlaylistItem {
            time: Some(PlaylistTimeSpan::From {
                from: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            }),
            days: None,
            refresh: Refresh::Hours { hours: 10 },
            rig: rig(),
        };
        let playlist = Playlist {
            schedule: vec![item],
        };

        let next = get_next_refresh_time(now, &refresh, &playlist).unwrap();
        assert_eq!(next, dt("2025-01-05 15:00:00"));
    }
}
