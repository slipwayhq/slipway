fn extract_text_from_strings(strings: &[String]) -> Vec<String> {
    let mut extracted_text = Vec::new();

    for string in strings {
        if let Some(text) = string.strip_prefix("$.rigging.") {
            let placeholder = text.strip_suffix(".output");
            if let Some(placeholder) = placeholder {
                extracted_text.push(placeholder.to_string());
            }
        } else if let Some(text) = string.strip_prefix("$$") {
            extracted_text.push(text.to_string());
        }
    }

    extracted_text
}
