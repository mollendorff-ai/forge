//! Text Functions (v1.1.0)
//! CONCAT, TRIM, UPPER, LOWER, LEN, MID

use super::ArrayCalculator;

#[allow(dead_code)]
impl ArrayCalculator {
    /// Evaluate CONCAT/CONCATENATE function: CONCAT(text1, text2, ...)
    pub(super) fn eval_concat(&self, texts: Vec<String>) -> String {
        texts.join("")
    }

    /// Evaluate TRIM function: TRIM(text)
    pub(super) fn eval_trim(&self, text: &str) -> String {
        text.trim().to_string()
    }

    /// Evaluate UPPER function: UPPER(text)
    pub(super) fn eval_upper(&self, text: &str) -> String {
        text.to_uppercase()
    }

    /// Evaluate LOWER function: LOWER(text)
    pub(super) fn eval_lower(&self, text: &str) -> String {
        text.to_lowercase()
    }

    /// Evaluate LEN function: LEN(text)
    pub(super) fn eval_len(&self, text: &str) -> f64 {
        text.len() as f64
    }

    /// Evaluate MID function: MID(text, start, length)
    pub(super) fn eval_mid(&self, text: &str, start: usize, length: usize) -> String {
        let chars: Vec<char> = text.chars().collect();
        // Excel uses 1-based indexing
        let start_idx = if start > 0 { start - 1 } else { 0 };
        let end_idx = (start_idx + length).min(chars.len());

        if start_idx >= chars.len() {
            return String::new();
        }

        chars[start_idx..end_idx].iter().collect()
    }
}
