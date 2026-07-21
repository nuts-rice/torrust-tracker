#[derive(Debug, Clone, Copy)]
pub struct Step {
    pub name: &'static str,
    pub command: &'static str,
}

#[must_use]
pub fn sanitize_name_for_log(raw_name: &str) -> String {
    let mut normalized = String::with_capacity(raw_name.len());
    for char in raw_name.chars() {
        if char.is_ascii_alphanumeric() {
            normalized.push(char.to_ascii_lowercase());
        } else if !normalized.ends_with('-') {
            normalized.push('-');
        }
    }
    let trimmed = normalized.trim_matches('-');
    if trimmed.is_empty() {
        "step".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[must_use]
pub fn strip_ansi(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut characters = line.chars();
    while let Some(character) = characters.next() {
        if character != '\u{1b}' {
            result.push(character);
            continue;
        }

        // Consume the escape sequence: `[` then parameter bytes, ended by a letter.
        if characters.next() != Some('[') {
            continue;
        }

        for sequence_character in characters.by_ref() {
            if sequence_character.is_ascii_alphabetic() {
                break;
            }
        }
    }

    result
}
