//! Formula tokenizer for the array calculator
//!
//! Converts formula strings like "=SUM(price) * 1.1" into a sequence of tokens
//! that can be parsed into an AST.

use std::iter::Peekable;
use std::str::Chars;

/// A token in a formula expression
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A numeric literal (e.g., 123, 45.67, -89, 1.5e10)
    Number(f64),
    /// A string literal (e.g., "hello" or 'world')
    Text(String),
    /// A boolean literal (TRUE or FALSE)
    Boolean(bool),
    /// An identifier - could be a function name, variable, or table.column reference
    Identifier(String),
    /// Binary/comparison operators: + - * / ^ = <> >= <= < > &
    Operator(String),
    /// Opening parenthesis
    OpenParen,
    /// Closing parenthesis
    CloseParen,
    /// Opening bracket for array indexing
    OpenBracket,
    /// Closing bracket
    CloseBracket,
    /// Comma separator for function arguments
    Comma,
    /// Colon for ranges (A1:B2)
    Colon,
}

/// Error during tokenization
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizeError {
    pub message: String,
    pub position: usize,
}

impl TokenizeError {
    fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

impl std::fmt::Display for TokenizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tokenize error at position {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for TokenizeError {}

/// Tokenizer for formula expressions
pub struct Tokenizer<'a> {
    chars: Peekable<Chars<'a>>,
    position: usize,
}

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer for the given formula string
    #[must_use]
    pub fn new(formula: &'a str) -> Self {
        // Strip leading '=' if present (formulas start with =)
        let formula = formula.strip_prefix('=').unwrap_or(formula);
        Self {
            chars: formula.chars().peekable(),
            position: 0,
        }
    }

    /// Tokenize the entire formula into a vector of tokens
    ///
    /// # Errors
    ///
    /// Returns `TokenizeError` for invalid tokens (e.g., unterminated strings,
    /// invalid number literals).
    pub fn tokenize(mut self) -> Result<Vec<Token>, TokenizeError> {
        let mut tokens = Vec::new();

        while let Some(token) = self.next_token()? {
            tokens.push(token);
        }

        Ok(tokens)
    }

    /// Get the next token, or None if at end of input
    fn next_token(&mut self) -> Result<Option<Token>, TokenizeError> {
        self.skip_whitespace();

        match self.peek() {
            None => Ok(None),
            Some(c) => {
                let token = match c {
                    // String literals
                    '"' | '\'' => self.read_string()?,

                    // Parentheses and brackets
                    '(' => {
                        self.advance();
                        Token::OpenParen
                    },
                    ')' => {
                        self.advance();
                        Token::CloseParen
                    },
                    '[' => {
                        self.advance();
                        Token::OpenBracket
                    },
                    ']' => {
                        self.advance();
                        Token::CloseBracket
                    },

                    // Punctuation
                    ',' => {
                        self.advance();
                        Token::Comma
                    },
                    ':' => {
                        self.advance();
                        Token::Colon
                    },

                    // Operators (need to handle multi-char operators)
                    '+' | '*' | '/' | '^' | '&' => {
                        let op = self.advance().unwrap().to_string();
                        Token::Operator(op)
                    },

                    // Minus could be operator or negative number
                    '-' => self.read_minus_or_negative()?,

                    // Comparison operators
                    '<' => self.read_less_than_operator()?,
                    '>' => self.read_greater_than_operator()?,
                    '=' => {
                        self.advance();
                        Token::Operator("=".to_string())
                    },

                    // Numbers
                    c if c.is_ascii_digit() => self.read_number()?,

                    // Identifiers (function names, variable names, table.column)
                    c if c.is_alphabetic() || c == '_' => self.read_identifier()?,

                    // Unknown character
                    c => {
                        return Err(TokenizeError::new(
                            format!("Unexpected character: '{c}'"),
                            self.position,
                        ));
                    },
                };
                Ok(Some(token))
            },
        }
    }

    /// Peek at the next character without consuming it
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    /// Advance to the next character
    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if c.is_some() {
            self.position += 1;
        }
        c
    }

    /// Skip whitespace characters
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Read a string literal (double or single quoted)
    fn read_string(&mut self) -> Result<Token, TokenizeError> {
        let quote = self.advance().unwrap(); // consume opening quote
        let start_pos = self.position;
        let mut value = String::new();

        loop {
            match self.advance() {
                None => {
                    return Err(TokenizeError::new("Unterminated string literal", start_pos));
                },
                Some(c) if c == quote => {
                    // Check for escaped quote (doubled)
                    if self.peek() == Some(quote) {
                        value.push(quote);
                        self.advance();
                    } else {
                        break;
                    }
                },
                Some(c) => {
                    value.push(c);
                },
            }
        }

        Ok(Token::Text(value))
    }

    /// Read a number (integer, decimal, or scientific notation)
    fn read_number(&mut self) -> Result<Token, TokenizeError> {
        let start_pos = self.position;
        let mut num_str = String::new();

        // Read integer part
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        // Read decimal part
        if self.peek() == Some('.') {
            num_str.push(self.advance().unwrap());
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    num_str.push(self.advance().unwrap());
                } else {
                    break;
                }
            }
        }

        // Read exponent part (e.g., 1.5e10, 2E-5)
        if let Some(c) = self.peek() {
            if c == 'e' || c == 'E' {
                num_str.push(self.advance().unwrap());
                // Optional sign
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        num_str.push(self.advance().unwrap());
                    }
                }
                // Exponent digits
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        num_str.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }
            }
        }

        num_str
            .parse::<f64>()
            .map(Token::Number)
            .map_err(|_| TokenizeError::new(format!("Invalid number: {num_str}"), start_pos))
    }

    /// Read an identifier (function name, variable, or table.column reference)
    /// Also recognizes TRUE/FALSE as boolean literals
    // Returns Result for uniform tokenizer method signature.
    #[allow(clippy::unnecessary_wraps)]
    fn read_identifier(&mut self) -> Result<Token, TokenizeError> {
        let mut ident = String::new();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '.' {
                ident.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        // Check for boolean literals (case-insensitive)
        // Only treat as literal if NOT followed by '(' (function call)
        self.skip_whitespace();
        let is_function_call = self.peek() == Some('(');

        if !is_function_call {
            let upper = ident.to_uppercase();
            if upper == "TRUE" {
                return Ok(Token::Boolean(true));
            }
            if upper == "FALSE" {
                return Ok(Token::Boolean(false));
            }
        }

        Ok(Token::Identifier(ident))
    }

    /// Handle minus sign - could be operator or start of negative number
    // Returns Result for uniform tokenizer method signature.
    #[allow(clippy::unnecessary_wraps)]
    fn read_minus_or_negative(&mut self) -> Result<Token, TokenizeError> {
        self.advance(); // consume '-'

        // Check if followed by a digit (negative number)
        // But we need context: if previous token was a value, this is an operator
        // For simplicity in tokenizer, we'll treat '-' as always an operator
        // The parser will handle unary minus
        Ok(Token::Operator("-".to_string()))
    }

    /// Read operators starting with '<'
    // Returns Result for uniform tokenizer method signature.
    #[allow(clippy::unnecessary_wraps)]
    fn read_less_than_operator(&mut self) -> Result<Token, TokenizeError> {
        self.advance(); // consume '<'

        match self.peek() {
            Some('=') => {
                self.advance();
                Ok(Token::Operator("<=".to_string()))
            },
            Some('>') => {
                self.advance();
                Ok(Token::Operator("<>".to_string()))
            },
            _ => Ok(Token::Operator("<".to_string())),
        }
    }

    /// Read operators starting with '>'
    // Returns Result for uniform tokenizer method signature.
    #[allow(clippy::unnecessary_wraps)]
    fn read_greater_than_operator(&mut self) -> Result<Token, TokenizeError> {
        self.advance(); // consume '>'

        match self.peek() {
            Some('=') => {
                self.advance();
                Ok(Token::Operator(">=".to_string()))
            },
            _ => Ok(Token::Operator(">".to_string())),
        }
    }
}

/// Convenience function to tokenize a formula string
///
/// # Errors
///
/// Returns `TokenizeError` for invalid tokens.
pub fn tokenize(formula: &str) -> Result<Vec<Token>, TokenizeError> {
    Tokenizer::new(formula).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_number() {
        let tokens = tokenize("42").unwrap();
        assert_eq!(tokens, vec![Token::Number(42.0)]);
    }

    #[test]
    fn test_tokenize_decimal_number() {
        let tokens = tokenize("3.567").unwrap();
        assert_eq!(tokens, vec![Token::Number(3.567)]);
    }

    #[test]
    fn test_tokenize_scientific_notation() {
        let tokens = tokenize("1.5e10").unwrap();
        assert_eq!(tokens, vec![Token::Number(1.5e10)]);

        let tokens = tokenize("2E-5").unwrap();
        assert_eq!(tokens, vec![Token::Number(2e-5)]);
    }

    #[test]
    fn test_tokenize_string_double_quotes() {
        let tokens = tokenize("\"hello world\"").unwrap();
        assert_eq!(tokens, vec![Token::Text("hello world".to_string())]);
    }

    #[test]
    fn test_tokenize_string_single_quotes() {
        let tokens = tokenize("'hello'").unwrap();
        assert_eq!(tokens, vec![Token::Text("hello".to_string())]);
    }

    #[test]
    fn test_tokenize_string_escaped_quotes() {
        let tokens = tokenize("\"hello \"\"world\"\"\"").unwrap();
        assert_eq!(tokens, vec![Token::Text("hello \"world\"".to_string())]);
    }

    #[test]
    fn test_tokenize_identifier() {
        let tokens = tokenize("price").unwrap();
        assert_eq!(tokens, vec![Token::Identifier("price".to_string())]);
    }

    #[test]
    fn test_tokenize_identifier_with_underscore() {
        let tokens = tokenize("tax_rate").unwrap();
        assert_eq!(tokens, vec![Token::Identifier("tax_rate".to_string())]);
    }

    #[test]
    fn test_tokenize_table_column_reference() {
        let tokens = tokenize("sales.revenue").unwrap();
        assert_eq!(tokens, vec![Token::Identifier("sales.revenue".to_string())]);
    }

    #[test]
    fn test_tokenize_function_call() {
        let tokens = tokenize("SUM(price)").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("SUM".to_string()),
                Token::OpenParen,
                Token::Identifier("price".to_string()),
                Token::CloseParen,
            ]
        );
    }

    #[test]
    fn test_tokenize_binary_expression() {
        let tokens = tokenize("a + b").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".to_string()),
                Token::Operator("+".to_string()),
                Token::Identifier("b".to_string()),
            ]
        );
    }

    #[test]
    fn test_tokenize_all_operators() {
        let tokens = tokenize("+ - * / ^").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Operator("+".to_string()),
                Token::Operator("-".to_string()),
                Token::Operator("*".to_string()),
                Token::Operator("/".to_string()),
                Token::Operator("^".to_string()),
            ]
        );
    }

    #[test]
    fn test_tokenize_comparison_operators() {
        // Note: leading = is stripped as formula prefix, so use "a = b" to test =
        let tokens = tokenize("a = b < c > d <= e >= f <> g").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".to_string()),
                Token::Operator("=".to_string()),
                Token::Identifier("b".to_string()),
                Token::Operator("<".to_string()),
                Token::Identifier("c".to_string()),
                Token::Operator(">".to_string()),
                Token::Identifier("d".to_string()),
                Token::Operator("<=".to_string()),
                Token::Identifier("e".to_string()),
                Token::Operator(">=".to_string()),
                Token::Identifier("f".to_string()),
                Token::Operator("<>".to_string()),
                Token::Identifier("g".to_string()),
            ]
        );
    }

    #[test]
    fn test_tokenize_formula_with_equals_prefix() {
        let tokens = tokenize("=price * 1.1").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("price".to_string()),
                Token::Operator("*".to_string()),
                Token::Number(1.1),
            ]
        );
    }

    #[test]
    fn test_tokenize_complex_formula() {
        let tokens = tokenize("=SUM(table.column) * (1 + tax_rate)").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("SUM".to_string()),
                Token::OpenParen,
                Token::Identifier("table.column".to_string()),
                Token::CloseParen,
                Token::Operator("*".to_string()),
                Token::OpenParen,
                Token::Number(1.0),
                Token::Operator("+".to_string()),
                Token::Identifier("tax_rate".to_string()),
                Token::CloseParen,
            ]
        );
    }

    #[test]
    fn test_tokenize_nested_function() {
        let tokens = tokenize("ROUND(SUM(values), 2)").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("ROUND".to_string()),
                Token::OpenParen,
                Token::Identifier("SUM".to_string()),
                Token::OpenParen,
                Token::Identifier("values".to_string()),
                Token::CloseParen,
                Token::Comma,
                Token::Number(2.0),
                Token::CloseParen,
            ]
        );
    }

    #[test]
    fn test_tokenize_if_condition() {
        let tokens = tokenize("IF(x > 10, \"yes\", \"no\")").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("IF".to_string()),
                Token::OpenParen,
                Token::Identifier("x".to_string()),
                Token::Operator(">".to_string()),
                Token::Number(10.0),
                Token::Comma,
                Token::Text("yes".to_string()),
                Token::Comma,
                Token::Text("no".to_string()),
                Token::CloseParen,
            ]
        );
    }

    #[test]
    fn test_tokenize_array_index() {
        let tokens = tokenize("table.column[0]").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("table.column".to_string()),
                Token::OpenBracket,
                Token::Number(0.0),
                Token::CloseBracket,
            ]
        );
    }

    #[test]
    fn test_tokenize_negative_number_as_operator() {
        // In tokenizer, minus is always an operator - parser handles unary minus
        let tokens = tokenize("-5").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Operator("-".to_string()), Token::Number(5.0),]
        );
    }

    #[test]
    fn test_tokenize_colon_for_range() {
        let tokens = tokenize("A1:B10").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("A1".to_string()),
                Token::Colon,
                Token::Identifier("B10".to_string()),
            ]
        );
    }

    #[test]
    fn test_tokenize_empty_string() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_tokenize_whitespace_only() {
        let tokens = tokenize("   ").unwrap();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_tokenize_error_unterminated_string() {
        let result = tokenize("\"hello");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unterminated"));
    }

    #[test]
    fn test_tokenize_error_unexpected_char() {
        let result = tokenize("@invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unexpected"));
    }

    #[test]
    fn test_tokenize_boolean_true() {
        let tokens = tokenize("TRUE").unwrap();
        assert_eq!(tokens, vec![Token::Boolean(true)]);

        // Case insensitive
        let tokens = tokenize("true").unwrap();
        assert_eq!(tokens, vec![Token::Boolean(true)]);

        let tokens = tokenize("True").unwrap();
        assert_eq!(tokens, vec![Token::Boolean(true)]);
    }

    #[test]
    fn test_tokenize_boolean_false() {
        let tokens = tokenize("FALSE").unwrap();
        assert_eq!(tokens, vec![Token::Boolean(false)]);

        // Case insensitive
        let tokens = tokenize("false").unwrap();
        assert_eq!(tokens, vec![Token::Boolean(false)]);
    }

    #[test]
    fn test_tokenize_boolean_in_expression() {
        let tokens = tokenize("TRUE + 1").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Boolean(true),
                Token::Operator("+".to_string()),
                Token::Number(1.0),
            ]
        );
    }

    #[test]
    fn test_tokenize_ampersand_operator() {
        let tokens = tokenize("\"Hello\" & \" World\"").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Text("Hello".to_string()),
                Token::Operator("&".to_string()),
                Token::Text(" World".to_string()),
            ]
        );
    }
}
