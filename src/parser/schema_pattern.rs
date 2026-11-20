//! The `pattern`s of the model schemas, matched without a regex engine
//! (ADR-069): the `regex` crate would be a quarter of a megabyte of the
//! browser module for six short patterns.
//!
//! The syntax read is what the patterns use, and a little more: literal
//! characters, `.`, `^` and `$`, classes (`[0-9]`, `[^a-z]`, `[12]`), groups
//! with alternatives (`(a|b)`), the quantifiers `?`, `*`, `+`, `{n}`,
//! `{n,}` and `{n,m}`, `\d` and `\D`, and a backslash before a punctuation
//! character. As `jsonschema` read them: `\d` is `[0-9]` (ECMA-262), `.` is
//! any character but `\n` (Rust's regex), `^` and `$` are the start and end
//! of the text, and the pattern is searched for anywhere in it. Anything
//! else is not read, and the schema that has it is not either.
//!
//! A model is text from anyone, so matching never goes deeper with the
//! length of the text: one character repeated is matched in a loop, and a
//! group may only be repeated a bounded number of times (`(ab){1,3}`, not
//! `(ab)+`), nor may `^` or `$`.

/// A pattern, read.
#[derive(Debug)]
pub(super) struct Pattern {
    alternatives: Vec<Vec<Piece>>,
}

#[derive(Debug)]
struct Piece {
    item: Item,
    min: usize,
    max: Option<usize>,
}

#[derive(Debug)]
enum Item {
    One(One),
    Start,
    End,
    Group(Vec<Vec<Piece>>),
}

/// An item that is one character.
#[derive(Debug)]
enum One {
    Char(char),
    /// Any character but `\n`
    Any,
    Class {
        negated: bool,
        ranges: Vec<(char, char)>,
    },
}

impl One {
    fn takes(&self, c: char) -> bool {
        match self {
            Self::Char(x) => c == *x,
            Self::Any => c != '\n',
            Self::Class { negated, ranges } => {
                ranges.iter().any(|(a, b)| (*a..=*b).contains(&c)) != *negated
            },
        }
    }
}

const DIGITS: (char, char) = ('0', '9');

struct Parser<'p> {
    chars: std::iter::Peekable<std::str::Chars<'p>>,
}

impl Parser<'_> {
    fn alternatives(&mut self) -> Option<Vec<Vec<Piece>>> {
        let mut out = vec![self.sequence()?];
        while self.chars.next_if_eq(&'|').is_some() {
            out.push(self.sequence()?);
        }
        Some(out)
    }

    fn sequence(&mut self) -> Option<Vec<Piece>> {
        let mut pieces = Vec::new();
        while let Some(&c) = self.chars.peek() {
            if c == '|' || c == ')' {
                break;
            }
            self.chars.next();
            let item = match c {
                '(' => {
                    let group = self.alternatives()?;
                    self.chars.next_if_eq(&')')?;
                    Item::Group(group)
                },
                '[' => Item::One(self.class()?),
                '.' => Item::One(One::Any),
                '^' => Item::Start,
                '$' => Item::End,
                '\\' => match self.chars.next()? {
                    'd' => Item::One(One::Class {
                        negated: false,
                        ranges: vec![DIGITS],
                    }),
                    'D' => Item::One(One::Class {
                        negated: true,
                        ranges: vec![DIGITS],
                    }),
                    p if p.is_ascii_punctuation() => Item::One(One::Char(p)),
                    _ => return None,
                },
                '?' | '*' | '+' | '{' | ']' | '}' => return None,
                literal => Item::One(One::Char(literal)),
            };
            let (min, max) = self.quantifier()?;
            // Only one character may repeat without a bound, and an anchor
            // not at all
            let bounded = match item {
                Item::One(_) => true,
                Item::Group(_) => max.is_some(),
                Item::Start | Item::End => (min, max) == (1, Some(1)),
            };
            if !bounded {
                return None;
            }
            pieces.push(Piece { item, min, max });
        }
        Some(pieces)
    }

    /// `[...]` after its `[`.
    fn class(&mut self) -> Option<One> {
        let negated = self.chars.next_if_eq(&'^').is_some();
        let mut ranges = Vec::new();
        loop {
            let c = self.chars.next()?;
            let from = match c {
                ']' if !ranges.is_empty() => {
                    return Some(One::Class { negated, ranges });
                },
                '\\' => match self.chars.next()? {
                    'd' => {
                        ranges.push(DIGITS);
                        continue;
                    },
                    p if p.is_ascii_punctuation() => p,
                    _ => return None,
                },
                '[' | ']' => return None,
                c => c,
            };
            let to = if self.chars.next_if_eq(&'-').is_some() {
                match self.chars.peek() {
                    Some(']') | None => {
                        ranges.push((from, from));
                        ('-', '-')
                    },
                    Some(&to) if to >= from && to != '\\' && to != '[' => {
                        self.chars.next();
                        (from, to)
                    },
                    Some(_) => return None,
                }
            } else {
                (from, from)
            };
            ranges.push(to);
        }
    }

    fn number(&mut self) -> Option<usize> {
        let mut digits = String::new();
        while let Some(d) = self.chars.next_if(char::is_ascii_digit) {
            digits.push(d);
        }
        digits.parse().ok()
    }

    /// The quantifier after an item, as `(min, max)`; `(1, Some(1))` if none.
    fn quantifier(&mut self) -> Option<(usize, Option<usize>)> {
        let simple = match self.chars.peek() {
            Some('?') => (0, Some(1)),
            Some('*') => (0, None),
            Some('+') => (1, None),
            Some('{') => return self.braces(),
            _ => return Some((1, Some(1))),
        };
        self.chars.next();
        Some(simple)
    }

    /// `{n}`, `{n,}` or `{n,m}`.
    fn braces(&mut self) -> Option<(usize, Option<usize>)> {
        self.chars.next();
        let min = self.number()?;
        let max = if self.chars.next_if_eq(&',').is_none() {
            Some(min)
        } else if self.chars.peek() == Some(&'}') {
            None
        } else {
            Some(self.number()?)
        };
        self.chars.next_if_eq(&'}')?;
        max.is_none_or(|max| max >= min).then_some((min, max))
    }
}

impl Pattern {
    /// Read a pattern; `None` if it has syntax this module does not read.
    pub(super) fn new(pattern: &str) -> Option<Self> {
        let mut parser = Parser {
            chars: pattern.chars().peekable(),
        };
        let alternatives = parser.alternatives()?;
        parser
            .chars
            .next()
            .is_none()
            .then_some(Self { alternatives })
    }

    /// Whether the pattern is found anywhere in `text`.
    pub(super) fn is_match(&self, text: &str) -> bool {
        let chars: Vec<char> = text.chars().collect();
        let matcher = Matcher { text: &chars };
        (0..=chars.len()).any(|start| {
            self.alternatives
                .iter()
                .any(|a| matcher.sequence(a, start, &mut |_| true))
        })
    }
}

struct Matcher<'t> {
    text: &'t [char],
}

impl Matcher<'_> {
    /// Whether `pieces` match from `at`, with `then` matching what follows.
    fn sequence(&self, pieces: &[Piece], at: usize, then: &mut dyn FnMut(usize) -> bool) -> bool {
        let Some((piece, rest)) = pieces.split_first() else {
            return then(at);
        };
        let then = &mut |next| self.sequence(rest, next, then);
        match &piece.item {
            // As many characters as it takes, then fewer: a loop, not a
            // level of recursion for each
            Item::One(one) => {
                let most = piece.max.unwrap_or(usize::MAX);
                let taken = self.text[at.min(self.text.len())..]
                    .iter()
                    .take(most)
                    .take_while(|c| one.takes(**c))
                    .count();
                (piece.min..=taken).rev().any(|n| then(at + n))
            },
            Item::Start => at == 0 && then(at),
            Item::End => at == self.text.len() && then(at),
            Item::Group(alternatives) => self.group(alternatives, piece, 0, at, then),
        }
    }

    /// One more repetition of a group if it may have one (as many as it
    /// can, then fewer), else what follows. A repetition that matches no
    /// text counts only until the group has its minimum. Groups repeat a
    /// bounded number of times, so this goes no deeper than the pattern.
    fn group(
        &self,
        alternatives: &[Vec<Piece>],
        piece: &Piece,
        count: usize,
        at: usize,
        then: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        let more = piece.max.is_some_and(|max| count < max)
            && alternatives.iter().any(|a| {
                self.sequence(a, at, &mut |next| {
                    (next != at || count < piece.min)
                        && self.group(alternatives, piece, count + 1, next, then)
                })
            });
        more || (count >= piece.min && then(at))
    }
}

#[cfg(test)]
mod tests {
    use super::Pattern;

    /// The six patterns of the schema files.
    const THE_SIX: [&str; 6] = [
        "^=.+",
        r"^=(SUM|AVERAGE|MAX|MIN|COUNT|PRODUCT|SUMIF|AVERAGEIF|COUNTIF)\(.+\)",
        r"^\d{4}-\d{2}(-\d{2})?$",
        "^[0-9]{4}-(0[1-9]|1[0-2])(-(0[1-9]|[12][0-9]|3[01]))?$",
        "^[0-9]{4}-[0-9]{2}-[0-9]{2}$",
        "^[0-9]{4}-[0-9]{1,2}(-[0-9]{1,2})?$",
    ];

    /// The regex crate, with `\d` as `[0-9]` as `jsonschema` translated it.
    fn regex(pattern: &str) -> regex::Regex {
        regex::Regex::new(&pattern.replace(r"\d", "[0-9]").replace(r"\D", "[^0-9]")).unwrap()
    }

    /// Texts near each pattern: dates and months right and wrong, formulas
    /// with and without their `=`, other scripts' digits, a newline.
    fn texts() -> Vec<String> {
        let mut out: Vec<String> = [
            "",
            "=",
            "=a",
            "=\n",
            "= ",
            "x=a",
            "SUM(a)",
            "=SUM(a)",
            "=SUM()",
            "=SUMIF(a, b)",
            "=SUMX(a)",
            "=AVERAGE(x",
            "=COUNTIF(a)b",
            "2025",
            "2025-01",
            "2025-1",
            "2025-1-1",
            "2025-01-1",
            "2025-01-01",
            "2025-12-31",
            "2025-13-01",
            "2025-00-01",
            "2025-01-32",
            "2025-01-00",
            "2025-02-30",
            "2025-01-01x",
            "x2025-01-01",
            "202-01-01",
            "20250-01-01",
            "٢٠٢٥-01-01",
            "2025-01-01\n",
            "2025/01/01",
            "2025-001",
            "abc",
            "a-b",
            "ABC",
            "a]b",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        // Every year-month-day shape of one and two digits
        for y in ["2025", "1900"] {
            for m in ["1", "01", "09", "10", "12", "13", "00", "1x"] {
                out.push(format!("{y}-{m}"));
                for d in ["1", "01", "09", "10", "29", "31", "32", "00", "3"] {
                    out.push(format!("{y}-{m}-{d}"));
                }
            }
        }
        out
    }

    #[track_caller]
    fn agree(pattern: &str) {
        let ours = Pattern::new(pattern).unwrap();
        let theirs = regex(pattern);
        for text in texts() {
            assert_eq!(
                ours.is_match(&text),
                theirs.is_match(&text),
                "{pattern} on {text:?}"
            );
        }
    }

    #[test]
    fn the_six_patterns_match_as_the_regex_crate_matches() {
        for pattern in THE_SIX {
            agree(pattern);
        }
    }

    #[test]
    fn the_rest_of_the_syntax_matches_as_the_regex_crate_matches() {
        for pattern in [
            "a",
            "^a$",
            "a.c",
            "[^0-9]",
            r"\D",
            r"[\d]",
            "[a-]",
            "[-a]",
            "[a-c-]",
            r"[\]]",
            r"\.",
            r"a\-b",
            "a?b",
            "a*",
            "^a+$",
            "^(ab){1,3}$",
            "^(a|){2,4}$",
            "^(a|b){0,3}c",
            "^a{2,}$",
            "^a{1,2}$",
            "^a{0}$",
            "(^|x)a",
            "a($|x)",
            "()",
            "^$",
            "|a",
            "^(|a)$",
            "x*$",
            "^x{3}",
        ] {
            agree(pattern);
        }
    }

    #[test]
    fn syntax_it_does_not_read_is_refused() {
        for pattern in [
            "(", "(a", "a)", "[", "[a", "[]", "[]a]", "[z-a]", r"[a-\d]", "[a-[]", "[[]", r"[\w]",
            "[a", r"\w", r"\s", r"\b", "\\", "?", "*a", "+", "{2}", "}", "]", "a{", "a{x}", "a{2",
            "a{3,2}", "a{2,x}", "a{,2}", "(ab)+", "(a)*", "(a){2,}", "^+", "$?", "^{2}", "a|(",
            "((", "[\\",
        ] {
            assert!(Pattern::new(pattern).is_none(), "{pattern}");
        }
        // As regex refuses them too, where it reads them at all
        for pattern in [
            "(", "(a", "a)", "[", "[]", "[z-a]", "?", "*a", "+", "a{3,2}",
        ] {
            assert!(regex::Regex::new(pattern).is_err(), "{pattern}");
        }
    }

    /// A model is anyone's text: a formula of a million characters is
    /// matched in a small stack, so the depth does not grow with the text.
    #[test]
    fn a_long_text_is_matched_without_going_deeper() {
        let long = format!("=SUM({})", "a".repeat(1_000_000));
        let matched = std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(move || {
                THE_SIX
                    .iter()
                    .map(|p| Pattern::new(p).unwrap().is_match(&long))
                    .collect::<Vec<_>>()
            })
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(matched, [true, true, false, false, false, false]);
    }
}
