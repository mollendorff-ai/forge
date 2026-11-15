//! Array constants (ADR-064) between Forge and Excel: `{1,2,3}` (a row) and
//! `{1;2;3}` (a column), with numbers, texts and TRUE or FALSE in them.
//! Excel writes them the same way in a workbook, so a constant goes out and
//! comes back as it is. One with both separators has rows and columns, which
//! Forge cannot hold, and an error value such as `#N/A` inside the braces is
//! not read either: such a formula is imported as values.

use crate::core::array_calculator::tokenizer::Token;

/// The Excel text of the constant whose `{` is token `open`, and the index
/// of the token after its `}`.
pub(super) fn to_excel(tokens: &[Token], open: usize) -> Result<(String, usize), String> {
    let mut out = String::from("{");
    let mut i = open + 1;
    loop {
        match tokens.get(i) {
            Some(Token::CloseBrace) => {
                out.push('}');
                return Ok((out, i + 1));
            },
            Some(Token::Number(n)) => out.push_str(&super::number_text(*n)),
            Some(Token::Operator(op)) if op == "-" || op == "+" => {
                if op == "-" {
                    out.push('-');
                }
            },
            Some(Token::Text(t)) => {
                out.push('"');
                out.push_str(&t.replace('"', "\"\""));
                out.push('"');
            },
            Some(Token::Boolean(b)) => out.push_str(if *b { "TRUE" } else { "FALSE" }),
            Some(Token::Comma) => out.push(','),
            Some(Token::Semicolon) => out.push(';'),
            Some(other) => return Err(format!("{other:?} in an array constant")),
            None => return Err("array constant without }".to_string()),
        }
        i += 1;
    }
}

/// Read the Excel constant whose `{` is at `chars[*i]`, and give it as
/// Forge text. `*i` ends after the `}`.
pub(super) fn from_excel(chars: &[char], i: &mut usize) -> Result<String, String> {
    let mut out = String::from("{");
    let (mut commas, mut semicolons) = (false, false);
    *i += 1;
    loop {
        let Some(&c) = chars.get(*i) else {
            return Err("array constant without }".to_string());
        };
        match c {
            '}' => {
                *i += 1;
                break;
            },
            '"' => {
                // A text, with "" for a quote inside
                let mut text = String::new();
                *i += 1;
                loop {
                    match (chars.get(*i), chars.get(*i + 1)) {
                        (Some('"'), Some('"')) => {
                            text.push('"');
                            *i += 2;
                        },
                        (Some('"'), _) => {
                            *i += 1;
                            break;
                        },
                        (Some(&ch), _) => {
                            text.push(ch);
                            *i += 1;
                        },
                        (None, _) => return Err("unterminated text".to_string()),
                    }
                }
                if text.contains('"') {
                    return Err("text with a double quote".to_string());
                }
                out.push('"');
                out.push_str(&text);
                out.push('"');
            },
            ',' => {
                commas = true;
                out.push_str(", ");
                *i += 1;
            },
            ';' => {
                semicolons = true;
                out.push_str("; ");
                *i += 1;
            },
            ' ' => *i += 1,
            '#' => return Err("an error value inside an array constant".to_string()),
            _ => {
                // A number with its sign, or TRUE or FALSE
                let start = *i;
                while chars
                    .get(*i)
                    .is_some_and(|ch| !matches!(ch, ',' | ';' | '}' | ' '))
                {
                    *i += 1;
                }
                let item: String = chars[start..*i].iter().collect();
                let upper = item.to_uppercase();
                if upper == "TRUE" || upper == "FALSE" {
                    out.push_str(&upper);
                } else if item.parse::<f64>().is_ok_and(f64::is_finite) {
                    out.push_str(&item);
                } else {
                    return Err(format!("'{item}' in an array constant"));
                }
            },
        }
    }
    if commas && semicolons {
        return Err("an array constant with rows and columns".to_string());
    }
    out.push('}');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::array_calculator::tokenizer::tokenize;

    fn excel(forge: &str) -> String {
        let tokens = tokenize(forge).unwrap();
        to_excel(&tokens, 0).unwrap().0
    }

    fn forge(excel: &str) -> Result<String, String> {
        let chars: Vec<char> = excel.chars().collect();
        let mut i = 0;
        from_excel(&chars, &mut i)
    }

    #[test]
    fn constants_go_out_as_excel_writes_them() {
        assert_eq!(excel("{1, -2.5, 3}"), "{1,-2.5,3}");
        assert_eq!(excel("{\"a\"; \"b\"\"c\"}"), "{\"a\";\"b\"\"c\"}");
        assert_eq!(excel("{TRUE, FALSE}"), "{TRUE,FALSE}");
    }

    #[test]
    fn constants_come_back_as_forge_text() {
        assert_eq!(forge("{1,-2.5,3}").unwrap(), "{1, -2.5, 3}");
        assert_eq!(forge("{\"a\";\"b\"}").unwrap(), "{\"a\"; \"b\"}");
        assert_eq!(forge("{true,FALSE}").unwrap(), "{TRUE, FALSE}");
        assert_eq!(forge("{1E+15}").unwrap(), "{1E+15}");
    }

    #[test]
    fn what_forge_cannot_hold_is_refused() {
        assert!(forge("{1,2;3,4}").unwrap_err().contains("rows and columns"));
        assert!(forge("{1,#N/A}").unwrap_err().contains("error value"));
        assert!(forge("{1,x}").is_err());
        assert!(forge("{1,2").is_err());
    }
}
