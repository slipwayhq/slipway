use tracing::info;

pub(crate) fn get_system_timezone() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| {
        info!(
            "Failed to detect system timezone, defaulting to {}",
            crate::DEFAULT_TIMEZONE
        );
        crate::DEFAULT_TIMEZONE.to_string()
    })
}

pub(crate) fn get_system_locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| {
        info!(
            "Failed to detect system locale, defaulting to {}",
            crate::DEFAULT_LOCALE
        );
        crate::DEFAULT_LOCALE.to_string()
    })
}
