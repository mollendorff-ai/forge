//! Excel's wildcards: `*` any run of characters, `?` one character, `~`
//! before `*`, `?` or `~` that character itself. Used by SEARCH, MATCH and
//! VLOOKUP with an exact match, XMATCH and XLOOKUP with match mode 2, and the
//! text criteria of COUNTIF, SUMIF and the like. Matching ignores case, as in
//! Excel, and counts characters, not bytes.

#[derive(Clone, Copy, PartialEq, Eq)]
enum Piece {
    Any,
    One,
    Char(char),
}

fn pieces(pattern: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '~' if matches!(chars.peek(), Some('*' | '?' | '~')) => {
                out.push(Piece::Char(chars.next().unwrap_or('~')));
            },
            '*' => out.push(Piece::Any),
            '?' => out.push(Piece::One),
            c => out.push(Piece::Char(c)),
        }
    }
    out
}

fn lower(text: &str) -> Vec<char> {
    text.chars().flat_map(char::to_lowercase).collect()
}

fn lower_piece(piece: Piece) -> Piece {
    match piece {
        Piece::Char(c) => Piece::Char(c.to_lowercase().next().unwrap_or(c)),
        other => other,
    }
}

/// Whether `pattern` has a wildcard or a `~` escape in it
pub fn has_wildcards(pattern: &str) -> bool {
    pattern.contains(['*', '?', '~'])
}

/// Whether the pieces match all of `text`
fn matches_all(pattern: &[Piece], text: &[char]) -> bool {
    // The usual glob match with one backtrack point for the last `*`
    let (mut p, mut t) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        match pattern.get(p) {
            Some(Piece::Any) => {
                star = Some((p, t));
                p += 1;
            },
            Some(Piece::One) => {
                p += 1;
                t += 1;
            },
            Some(Piece::Char(c)) if *c == text[t] => {
                p += 1;
                t += 1;
            },
            _ => match star {
                Some((sp, st)) => {
                    p = sp + 1;
                    t = st + 1;
                    star = Some((sp, st + 1));
                },
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|piece| *piece == Piece::Any)
}

/// Whether all of `text` matches `pattern`, ignoring case
pub fn matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<Piece> = pieces(pattern).into_iter().map(lower_piece).collect();
    matches_all(&pattern, &lower(text))
}

/// The first character position, from `start` (0-based), where `pattern`
/// matches the start of the rest of `text`, ignoring case: SEARCH.
pub fn find(pattern: &str, text: &str, start: usize) -> Option<usize> {
    let mut pattern: Vec<Piece> = pieces(pattern).into_iter().map(lower_piece).collect();
    pattern.push(Piece::Any);
    let text = lower(text);
    (start..=text.len()).find(|&i| matches_all(&pattern, &text[i..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stars_questions_and_escapes() {
        assert!(matches("b*", "Banana"));
        assert!(matches("*e", "apple"));
        assert!(matches("?pple", "Apple"));
        assert!(matches("c?erry", "cherry"));
        assert!(!matches("ban*", "apple"));
        assert!(matches("b~*", "b*"));
        assert!(!matches("b~*", "bx"));
        assert!(matches("~?", "?"));
        assert!(matches("a~~", "a~"));
        assert!(matches("*", ""));
        assert!(!matches("?", ""));
        assert!(matches("STRA?E", "straße"));
    }

    #[test]
    fn search_positions_count_characters() {
        assert_eq!(find("?", "hello", 0), Some(0));
        assert_eq!(find("*", "hello", 3), Some(3));
        assert_eq!(find("~*", "a*b?c", 0), Some(1));
        assert_eq!(find("?", "a*b?c", 0), Some(0));
        assert_eq!(find("γ", "αβγ", 0), Some(2));
        assert_eq!(find("b?c", "a*b?c", 0), Some(2));
        assert_eq!(find("z", "hello", 0), None);
    }
}
