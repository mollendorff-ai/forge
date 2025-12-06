//! Formula parser for the array calculator
//!
//! Converts a sequence of tokens into an Abstract Syntax Tree (AST).
//! Uses recursive descent parsing with operator precedence.

use super::tokenizer::Token;

/// A reference to a variable or table column
#[derive(Debug, Clone, PartialEq)]
pub enum Reference {
    /// A scalar variable (e.g., "price")
    Scalar(String),
    /// A table column reference (e.g., "sales.revenue")
    TableColumn { table: String, column: String },
}

/// Abstract Syntax Tree node for formula expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A numeric literal
    Number(f64),
    /// A string literal
    Text(String),
    /// A variable or table.column reference
    Reference(Reference),
    /// Array indexing: expr[index]
    ArrayIndex { array: Box<Expr>, index: Box<Expr> },
    /// Function call: NAME(arg1, arg2, ...)
    FunctionCall { name: String, args: Vec<Expr> },
    /// Calling the result of an expression: (expr)(args)
    /// Used for LAMBDA immediate invocation: LAMBDA(x, x*2)(5)
    CallResult {
        callable: Box<Expr>,
        args: Vec<Expr>,
    },
    /// Binary operation: left op right
    BinaryOp {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Unary operation: -expr
    UnaryOp { op: String, operand: Box<Expr> },
    /// Range expression: A1:B10 (for INDIRECT, etc.)
    Range { start: Box<Expr>, end: Box<Expr> },
}

/// Error during parsing
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl ParseError {
    fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Parse error at position {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// Parser for formula tokens
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Create a new parser for the given tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parse the tokens into an AST
    pub fn parse(mut self) -> Result<Expr, ParseError> {
        if self.tokens.is_empty() {
            return Err(ParseError::new("Empty expression", 0));
        }
        let expr = self.expression()?;

        if !self.is_at_end() {
            return Err(ParseError::new(
                format!("Unexpected token after expression: {:?}", self.peek()),
                self.position,
            ));
        }

        Ok(expr)
    }

    /// Check if we've consumed all tokens
    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }

    /// Peek at the current token
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    /// Advance to the next token and return the current
    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.tokens.get(self.position - 1)
    }

    /// Check if current token matches and consume it
    fn match_token(&mut self, expected: &Token) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Check if current token is an operator with given value
    fn match_operator(&mut self, op: &str) -> bool {
        if let Some(Token::Operator(s)) = self.peek() {
            if s == op {
                self.advance();
                return true;
            }
        }
        false
    }

    /// Check if current token is any of the given operators
    fn match_any_operator(&mut self, ops: &[&str]) -> Option<String> {
        if let Some(Token::Operator(s)) = self.peek() {
            if ops.contains(&s.as_str()) {
                let op = s.clone();
                self.advance();
                return Some(op);
            }
        }
        None
    }

    /// Expression: comparison
    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.comparison()
    }

    /// Comparison: term (( "=" | "<>" | "<" | ">" | "<=" | ">=" ) term)*
    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.term()?;

        while let Some(op) = self.match_any_operator(&["=", "<>", "<", ">", "<=", ">="]) {
            let right = self.term()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Term: factor (( "+" | "-" ) factor)*
    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.factor()?;

        while let Some(op) = self.match_any_operator(&["+", "-"]) {
            let right = self.factor()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Factor: power (( "*" | "/" ) power)*
    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.power()?;

        while let Some(op) = self.match_any_operator(&["*", "/"]) {
            let right = self.power()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Power: unary ( "^" power )?   (right-associative)
    fn power(&mut self) -> Result<Expr, ParseError> {
        let left = self.unary()?;

        if self.match_operator("^") {
            let right = self.power()?; // right-associative
            Ok(Expr::BinaryOp {
                op: "^".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    /// Unary: ( "-" ) unary | postfix
    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_operator("-") {
            let operand = self.unary()?;
            Ok(Expr::UnaryOp {
                op: "-".to_string(),
                operand: Box::new(operand),
            })
        } else {
            self.postfix()
        }
    }

    /// Postfix: primary ( "(" arguments? ")" | "[" expr "]" | ":" expr )*
    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(&Token::OpenParen) {
                // Function call or callable result invocation
                let args = self.arguments()?;
                if !self.match_token(&Token::CloseParen) {
                    return Err(ParseError::new(
                        "Expected ')' after function arguments",
                        self.position,
                    ));
                }

                expr = match &expr {
                    Expr::Reference(Reference::Scalar(name)) => {
                        // Normal function call: FN(args)
                        Expr::FunctionCall {
                            name: name.clone(),
                            args,
                        }
                    }
                    Expr::Reference(Reference::TableColumn { table, column }) => {
                        // Handle function names with dots like VAR.P, STDEV.S
                        Expr::FunctionCall {
                            name: format!("{}.{}", table, column),
                            args,
                        }
                    }
                    _ => {
                        // Calling the result of an expression: expr(args)
                        // Used for LAMBDA immediate invocation: LAMBDA(x, x*2)(5)
                        Expr::CallResult {
                            callable: Box::new(expr.clone()),
                            args,
                        }
                    }
                };
            } else if self.match_token(&Token::OpenBracket) {
                // Array indexing
                let index = self.expression()?;
                if !self.match_token(&Token::CloseBracket) {
                    return Err(ParseError::new(
                        "Expected ']' after array index",
                        self.position,
                    ));
                }
                expr = Expr::ArrayIndex {
                    array: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.match_token(&Token::Colon) {
                // Range expression (A1:B10)
                let end = self.primary()?;
                expr = Expr::Range {
                    start: Box::new(expr),
                    end: Box::new(end),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Arguments: ( expr ( "," expr )* )?
    fn arguments(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();

        // Check for empty argument list
        if let Some(Token::CloseParen) = self.peek() {
            return Ok(args);
        }

        // First argument
        args.push(self.expression()?);

        // Remaining arguments
        while self.match_token(&Token::Comma) {
            args.push(self.expression()?);
        }

        Ok(args)
    }

    /// Primary: NUMBER | STRING | IDENTIFIER | "(" expr ")"
    fn primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.peek().cloned();

        match token {
            Some(Token::Number(n)) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Some(Token::Text(s)) => {
                self.advance();
                Ok(Expr::Text(s))
            }
            Some(Token::Identifier(name)) => {
                self.advance();
                Ok(self.parse_identifier(name))
            }
            Some(Token::OpenParen) => {
                self.advance();
                let expr = self.expression()?;
                if !self.match_token(&Token::CloseParen) {
                    return Err(ParseError::new(
                        "Expected ')' after expression",
                        self.position,
                    ));
                }
                Ok(expr)
            }
            Some(token) => Err(ParseError::new(
                format!("Unexpected token: {:?}", token),
                self.position,
            )),
            None => Err(ParseError::new(
                "Unexpected end of expression",
                self.position,
            )),
        }
    }

    /// Parse an identifier - could be scalar or table.column
    fn parse_identifier(&self, name: String) -> Expr {
        if let Some((table, column)) = name.split_once('.') {
            Expr::Reference(Reference::TableColumn {
                table: table.to_string(),
                column: column.to_string(),
            })
        } else {
            Expr::Reference(Reference::Scalar(name))
        }
    }
}

/// Convenience function to parse tokens into an AST
pub fn parse(tokens: Vec<Token>) -> Result<Expr, ParseError> {
    Parser::new(tokens).parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::array_calculator::tokenizer::tokenize;

    /// Helper to parse a formula string directly
    fn parse_formula(formula: &str) -> Result<Expr, ParseError> {
        let tokens = tokenize(formula).map_err(|e| ParseError::new(e.message, e.position))?;
        parse(tokens)
    }

    #[test]
    fn test_parse_number() {
        let expr = parse_formula("42").unwrap();
        assert_eq!(expr, Expr::Number(42.0));
    }

    #[test]
    fn test_parse_negative_number() {
        let expr = parse_formula("-42").unwrap();
        assert_eq!(
            expr,
            Expr::UnaryOp {
                op: "-".to_string(),
                operand: Box::new(Expr::Number(42.0)),
            }
        );
    }

    #[test]
    fn test_parse_string() {
        let expr = parse_formula("\"hello\"").unwrap();
        assert_eq!(expr, Expr::Text("hello".to_string()));
    }

    #[test]
    fn test_parse_scalar_reference() {
        let expr = parse_formula("price").unwrap();
        assert_eq!(
            expr,
            Expr::Reference(Reference::Scalar("price".to_string()))
        );
    }

    #[test]
    fn test_parse_table_column_reference() {
        let expr = parse_formula("sales.revenue").unwrap();
        assert_eq!(
            expr,
            Expr::Reference(Reference::TableColumn {
                table: "sales".to_string(),
                column: "revenue".to_string(),
            })
        );
    }

    #[test]
    fn test_parse_simple_addition() {
        let expr = parse_formula("a + b").unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: "+".to_string(),
                left: Box::new(Expr::Reference(Reference::Scalar("a".to_string()))),
                right: Box::new(Expr::Reference(Reference::Scalar("b".to_string()))),
            }
        );
    }

    #[test]
    fn test_parse_operator_precedence_mul_over_add() {
        // a + b * c should be a + (b * c)
        let expr = parse_formula("a + b * c").unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: "+".to_string(),
                left: Box::new(Expr::Reference(Reference::Scalar("a".to_string()))),
                right: Box::new(Expr::BinaryOp {
                    op: "*".to_string(),
                    left: Box::new(Expr::Reference(Reference::Scalar("b".to_string()))),
                    right: Box::new(Expr::Reference(Reference::Scalar("c".to_string()))),
                }),
            }
        );
    }

    #[test]
    fn test_parse_operator_precedence_power() {
        // 2 ^ 3 ^ 2 should be 2 ^ (3 ^ 2) (right-associative)
        let expr = parse_formula("2 ^ 3 ^ 2").unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: "^".to_string(),
                left: Box::new(Expr::Number(2.0)),
                right: Box::new(Expr::BinaryOp {
                    op: "^".to_string(),
                    left: Box::new(Expr::Number(3.0)),
                    right: Box::new(Expr::Number(2.0)),
                }),
            }
        );
    }

    #[test]
    fn test_parse_parentheses() {
        // (a + b) * c
        let expr = parse_formula("(a + b) * c").unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: "*".to_string(),
                left: Box::new(Expr::BinaryOp {
                    op: "+".to_string(),
                    left: Box::new(Expr::Reference(Reference::Scalar("a".to_string()))),
                    right: Box::new(Expr::Reference(Reference::Scalar("b".to_string()))),
                }),
                right: Box::new(Expr::Reference(Reference::Scalar("c".to_string()))),
            }
        );
    }

    #[test]
    fn test_parse_function_call_no_args() {
        let expr = parse_formula("TODAY()").unwrap();
        assert_eq!(
            expr,
            Expr::FunctionCall {
                name: "TODAY".to_string(),
                args: vec![],
            }
        );
    }

    #[test]
    fn test_parse_function_call_one_arg() {
        let expr = parse_formula("SUM(values)").unwrap();
        assert_eq!(
            expr,
            Expr::FunctionCall {
                name: "SUM".to_string(),
                args: vec![Expr::Reference(Reference::Scalar("values".to_string()))],
            }
        );
    }

    #[test]
    fn test_parse_function_call_multiple_args() {
        let expr = parse_formula("ROUND(value, 2)").unwrap();
        assert_eq!(
            expr,
            Expr::FunctionCall {
                name: "ROUND".to_string(),
                args: vec![
                    Expr::Reference(Reference::Scalar("value".to_string())),
                    Expr::Number(2.0),
                ],
            }
        );
    }

    #[test]
    fn test_parse_nested_function_calls() {
        let expr = parse_formula("ROUND(SUM(values), 2)").unwrap();
        assert_eq!(
            expr,
            Expr::FunctionCall {
                name: "ROUND".to_string(),
                args: vec![
                    Expr::FunctionCall {
                        name: "SUM".to_string(),
                        args: vec![Expr::Reference(Reference::Scalar("values".to_string()))],
                    },
                    Expr::Number(2.0),
                ],
            }
        );
    }

    #[test]
    fn test_parse_if_expression() {
        let expr = parse_formula("IF(x > 10, \"yes\", \"no\")").unwrap();
        assert_eq!(
            expr,
            Expr::FunctionCall {
                name: "IF".to_string(),
                args: vec![
                    Expr::BinaryOp {
                        op: ">".to_string(),
                        left: Box::new(Expr::Reference(Reference::Scalar("x".to_string()))),
                        right: Box::new(Expr::Number(10.0)),
                    },
                    Expr::Text("yes".to_string()),
                    Expr::Text("no".to_string()),
                ],
            }
        );
    }

    #[test]
    fn test_parse_array_index() {
        let expr = parse_formula("table.column[0]").unwrap();
        assert_eq!(
            expr,
            Expr::ArrayIndex {
                array: Box::new(Expr::Reference(Reference::TableColumn {
                    table: "table".to_string(),
                    column: "column".to_string(),
                })),
                index: Box::new(Expr::Number(0.0)),
            }
        );
    }

    #[test]
    fn test_parse_complex_formula() {
        let expr = parse_formula("SUM(sales.revenue) * (1 + tax_rate)").unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: "*".to_string(),
                left: Box::new(Expr::FunctionCall {
                    name: "SUM".to_string(),
                    args: vec![Expr::Reference(Reference::TableColumn {
                        table: "sales".to_string(),
                        column: "revenue".to_string(),
                    })],
                }),
                right: Box::new(Expr::BinaryOp {
                    op: "+".to_string(),
                    left: Box::new(Expr::Number(1.0)),
                    right: Box::new(Expr::Reference(Reference::Scalar("tax_rate".to_string()))),
                }),
            }
        );
    }

    #[test]
    fn test_parse_comparison_operators() {
        let expr = parse_formula("a <= b").unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: "<=".to_string(),
                left: Box::new(Expr::Reference(Reference::Scalar("a".to_string()))),
                right: Box::new(Expr::Reference(Reference::Scalar("b".to_string()))),
            }
        );
    }

    #[test]
    fn test_parse_range_expression() {
        let expr = parse_formula("A1:B10").unwrap();
        assert_eq!(
            expr,
            Expr::Range {
                start: Box::new(Expr::Reference(Reference::Scalar("A1".to_string()))),
                end: Box::new(Expr::Reference(Reference::Scalar("B10".to_string()))),
            }
        );
    }

    #[test]
    fn test_parse_unary_minus_in_expression() {
        let expr = parse_formula("a + -b").unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: "+".to_string(),
                left: Box::new(Expr::Reference(Reference::Scalar("a".to_string()))),
                right: Box::new(Expr::UnaryOp {
                    op: "-".to_string(),
                    operand: Box::new(Expr::Reference(Reference::Scalar("b".to_string()))),
                }),
            }
        );
    }

    #[test]
    fn test_parse_error_empty() {
        let result = parse_formula("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_close_paren() {
        let result = parse_formula("SUM(a, b");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("')'"));
    }

    #[test]
    fn test_parse_error_missing_close_bracket() {
        let result = parse_formula("arr[0");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("']'"));
    }

    #[test]
    fn test_parse_with_formula_prefix() {
        // Leading = should be stripped by tokenizer
        let expr = parse_formula("=price * 1.1").unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: "*".to_string(),
                left: Box::new(Expr::Reference(Reference::Scalar("price".to_string()))),
                right: Box::new(Expr::Number(1.1)),
            }
        );
    }

    #[test]
    fn test_parse_multiple_comparison() {
        // a < b < c should be (a < b) < c (left-associative)
        let expr = parse_formula("a < b < c").unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: "<".to_string(),
                left: Box::new(Expr::BinaryOp {
                    op: "<".to_string(),
                    left: Box::new(Expr::Reference(Reference::Scalar("a".to_string()))),
                    right: Box::new(Expr::Reference(Reference::Scalar("b".to_string()))),
                }),
                right: Box::new(Expr::Reference(Reference::Scalar("c".to_string()))),
            }
        );
    }

    #[test]
    fn test_parse_function_with_expression_arg() {
        let expr = parse_formula("MAX(a + b, c * d)").unwrap();
        assert_eq!(
            expr,
            Expr::FunctionCall {
                name: "MAX".to_string(),
                args: vec![
                    Expr::BinaryOp {
                        op: "+".to_string(),
                        left: Box::new(Expr::Reference(Reference::Scalar("a".to_string()))),
                        right: Box::new(Expr::Reference(Reference::Scalar("b".to_string()))),
                    },
                    Expr::BinaryOp {
                        op: "*".to_string(),
                        left: Box::new(Expr::Reference(Reference::Scalar("c".to_string()))),
                        right: Box::new(Expr::Reference(Reference::Scalar("d".to_string()))),
                    },
                ],
            }
        );
    }
}
