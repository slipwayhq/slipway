use std::{collections::HashSet, sync::Arc};

use chrono::{DateTime, Datelike, TimeZone, Utc, Weekday};
use chrono_tz::Tz;

use crate::primitives::{PlaylistName, RigName};

use super::{
    super::{
        repository::{Playlist, PlaylistItem, PlaylistTimeSpan},
        ServeState,
    },
    get_next_refresh_time,
};

pub(super) struct PlaylistResult {
    pub refresh_rate_seconds: u32,
    pub rig: RigName,
}

pub(super) async fn evaluate_playlist(
    state: Arc<ServeState>,
    playlist_name: &PlaylistName,
) -> anyhow::Result<Option<PlaylistResult>> {
    let playlist = state.repository.get_playlist(playlist_name).await?;
    let timezone = state.config.timezone.unwrap_or_default();
    evaluate_playlist_and_refresh(playlist, timezone)
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
    let playlist_item = playlist.items.iter().find(|item| {
        let days = &item.days;
        if let Some(days) = days {
            if !is_today_in_days(days, now) {
                return false;
            }
        }

        let span = &item.span;
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
