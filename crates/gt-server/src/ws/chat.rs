/// Sanitize a chat message: strip control characters, trim whitespace.
/// Returns None if the message is empty after sanitization.
pub(crate) fn sanitize_chat(message: &str) -> Option<String> {
    let cleaned: String = message
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Round a money value to an approximate range for intel level 1 (basic financials).
/// Returns the value rounded to the nearest "bucket" so the player gets a rough idea
/// but not exact numbers. Uses significant-digit rounding:
/// - Values < 10,000 -> round to nearest 1,000
/// - Values < 1,000,000 -> round to nearest 100,000
/// - Values >= 1,000,000 -> round to nearest 1,000,000
pub(super) fn approximate_money(value: i64) -> i64 {
    let abs = value.unsigned_abs();
    let bucket = if abs < 10_000 {
        1_000
    } else if abs < 1_000_000 {
        100_000
    } else {
        1_000_000
    };
    let rounded = ((abs + bucket / 2) / bucket) * bucket;
    if value >= 0 {
        rounded as i64
    } else {
        -(rounded as i64)
    }
}
