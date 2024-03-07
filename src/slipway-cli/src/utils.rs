pub(crate) fn format_bytes(bytes: usize) -> String {
    let all_units = ["bytes", "kb", "mb", "gb", "tb"];
    let mut size = bytes as f64;
    let mut i = 0;

    while size >= 1024.0 && i < all_units.len() - 1 {
        size /= 1024.0;
        i += 1;
    }

    let units = all_units[i];
    if i == 0 && size == 1. {
        "1 byte".to_string()
    } else if size.fract() == 0.0 {
        // If the number is effectively an integer, don't show any decimal places.
        format!("{} {units}", size.trunc())
    } else {
        // Format with up to 2 decimal places, removing trailing zeros.
        format!("{:.2} {units}", size)
    }
}

#[cfg(test)]
mod format_bytes_tests {
    use super::*;

    #[test]
    fn it_should_convert_bytes_to_human_readable_string() {
        assert_eq!(format_bytes(0), "0 bytes");
        assert_eq!(format_bytes(1), "1 byte");
        assert_eq!(format_bytes(10), "10 bytes");
        assert_eq!(format_bytes(256), "256 bytes");
        assert_eq!(format_bytes(1024), "1 kb");
        assert_eq!(format_bytes(5632), "5.50 kb");
        assert_eq!(format_bytes(1024 * 1024), "1 mb");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1 gb");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1 tb");
    }
}

pub(crate) fn skip_first_n_chars(s: &str, n: usize) -> &str {
    let char_pos = s
        .char_indices()
        .nth(n)
        .map(|(pos, _)| pos)
        .unwrap_or(s.len());
    &s[char_pos..]
}

#[cfg(test)]
mod skip_first_n_chars_tests {
    use super::*;

    #[test]
    fn it_should_return_end_of_string() {
        assert_eq!(skip_first_n_chars("abcdef", 0), "abcdef");
        assert_eq!(skip_first_n_chars("abcdef", 3), "def");
        assert_eq!(skip_first_n_chars("abcdef", 6), "");

        assert_eq!(skip_first_n_chars("Löwe老虎", 0), "Löwe老虎");
        assert_eq!(skip_first_n_chars("Löwe老虎", 3), "e老虎");
        assert_eq!(skip_first_n_chars("Löwe老虎", 5), "虎");
        assert_eq!(skip_first_n_chars("Löwe老虎", 6), "");
    }
}

pub(crate) fn get_first_n_chars(s: &str, n: usize) -> &str {
    let char_pos = s
        .char_indices()
        .nth(n)
        .map(|(pos, _)| pos)
        .unwrap_or(s.len());
    &s[..char_pos]
}

#[cfg(test)]
mod get_first_n_chars_tests {
    use super::*;

    #[test]
    fn it_should_return_end_of_string() {
        assert_eq!(get_first_n_chars("abcdef", 0), "");
        assert_eq!(get_first_n_chars("abcdef", 3), "abc");
        assert_eq!(get_first_n_chars("abcdef", 6), "abcdef");

        assert_eq!(get_first_n_chars("Löwe老虎", 0), "");
        assert_eq!(get_first_n_chars("Löwe老虎", 3), "Löw");
        assert_eq!(get_first_n_chars("Löwe老虎", 5), "Löwe老");
        assert_eq!(get_first_n_chars("Löwe老虎", 6), "Löwe老虎");
    }
}
