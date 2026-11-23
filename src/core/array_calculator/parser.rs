//! Formula parser for the array calculator
//!
//! Converts a sequence of tokens into an Abstract Syntax Tree (AST).
//! Uses recursive descent parsing with operator precedence.

use super::tokenizer::Token;

/// A reference to a variable or table column
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// A boolean literal (TRUE or FALSE)
    Boolean(bool),
    /// A variable or table.column reference
    Reference(Reference),
    /// Array indexing: expr[index]
    ArrayIndex { array: Box<Self>, index: Box<Self> },
    /// Function call: NAME(arg1, arg2, ...)
    FunctionCall { name: String, args: Vec<Self> },
    /// Calling the result of an expression: (expr)(args)
    /// Used for LAMBDA immediate invocation: LAMBDA(x, x*2)(5)
    CallResult {
        callable: Box<Self>,
        args: Vec<Self>,
    },
    /// Binary operation: left op right
    BinaryOp {
        op: String,
        left: Box<Self>,
        right: Box<Self>,
    },
    /// Unary operation: -expr
    UnaryOp { op: String, operand: Box<Self> },
    /// Range expression: A1:B10 (for INDIRECT, etc.)
    Range { start: Box<Self>, end: Box<Self> },
    /// An array constant (ADR-064): `{1,2,3}` (a row, `column` false) or
    /// `{1;2;3}` (a column). Each item is a number, a text or a boolean.
    Array { items: Vec<Self>, column: bool },
}

/// Error during parsing
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// How deep a formula may nest, the formula itself being the first level.
///
/// Parentheses, function arguments, `[ ]`, a chain of operators (`a + b + c`
/// nests to the left in the tree) and a run of `-` or `^` each count one
/// level. The evaluator and the drop of the tree recurse to the depth of the
/// tree, so without a limit a formula of 10,000 parentheses ends the process
/// with a stack overflow (roadmap T1-DEPTH). No real model comes near 128,
/// and a debug build on a 2 MiB thread, the size of a test thread, overflows
/// between 160 and 192 levels of function calls.
pub const MAX_DEPTH: usize = 128;

/// Parser for formula tokens
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    /// Levels of nesting entered and not yet left
    depth: usize,
}

impl Parser {
    /// Create a new parser for the given tokens
    #[must_use]
    pub const fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            depth: 0,
        }
    }

    /// Parse the tokens into an AST
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the tokens do not form a valid expression.
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
    const fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }

    /// Enter one more level of nesting; the caller gives it back with
    /// `self.depth -= n` when the rule is done. The tree of a chain such as
    /// `a + b + c` is as deep as the chain is long, so each link enters a
    /// level too and the depth of the tree stays under twice `MAX_DEPTH`.
    fn enter(&mut self) -> Result<(), ParseError> {
        if self.depth >= MAX_DEPTH {
            return Err(ParseError::new(
                format!(
                    "The formula is nested more than {MAX_DEPTH} levels deep (parentheses, \
                     function arguments and chained operators each count one). Split it into \
                     several named values or columns"
                ),
                self.position,
            ));
        }
        self.depth += 1;
        Ok(())
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
        self.enter()?;
        let expr = self.comparison()?;
        self.depth -= 1;
        Ok(expr)
    }

    /// Comparison: concat (( "=" | "<>" | "<" | ">" | "<=" | ">=" ) concat)*
    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.concat()?;
        let mut links = 0;

        while let Some(op) = self.match_any_operator(&["=", "<>", "<", ">", "<=", ">="]) {
            self.enter()?;
            links += 1;
            let right = self.concat()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        self.depth -= links;
        Ok(left)
    }

    /// Concat: term ( "&" term )*, which binds less than `+` and more than a
    /// comparison, as in Excel: `a & b + 1 = c` is `(a & (b + 1)) = c`
    fn concat(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.term()?;
        let mut links = 0;

        while self.match_operator("&") {
            self.enter()?;
            links += 1;
            let right = self.term()?;
            left = Expr::BinaryOp {
                op: "&".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        self.depth -= links;
        Ok(left)
    }

    /// Term: factor (( "+" | "-" ) factor)*
    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.factor()?;
        let mut links = 0;

        while let Some(op) = self.match_any_operator(&["+", "-"]) {
            self.enter()?;
            links += 1;
            let right = self.factor()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        self.depth -= links;
        Ok(left)
    }

    /// Factor: power (( "*" | "/" ) power)*
    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.power()?;
        let mut links = 0;

        while let Some(op) = self.match_any_operator(&["*", "/"]) {
            self.enter()?;
            links += 1;
            let right = self.power()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        self.depth -= links;
        Ok(left)
    }

    /// Power: unary ( "^" power )?   (right-associative)
    fn power(&mut self) -> Result<Expr, ParseError> {
        let left = self.unary()?;

        if self.match_operator("^") {
            self.enter()?;
            let right = self.power()?; // right-associative
            self.depth -= 1;
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
            self.enter()?;
            let operand = self.unary()?;
            self.depth -= 1;
            Ok(Expr::UnaryOp {
                op: "-".to_string(),
                operand: Box::new(operand),
            })
        } else {
            self.postfix()
        }
    }

    /// Postfix: `primary ( "(" arguments? ")" | "[" expr "]" | ":" expr )*`
    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        let mut links = 0;

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
                        Self::refuse_constant_for_range(name, &args)?;
                        // Normal function call: FN(args)
                        Expr::FunctionCall {
                            name: name.clone(),
                            args,
                        }
                    },
                    Expr::Reference(Reference::TableColumn { table, column }) => {
                        // Handle function names with dots like VAR.P, STDEV.S
                        Expr::FunctionCall {
                            name: format!("{table}.{column}"),
                            args,
                        }
                    },
                    _ => {
                        // Calling the result of an expression: expr(args)
                        // Used for LAMBDA immediate invocation: LAMBDA(x, x*2)(5)
                        self.enter()?;
                        links += 1;
                        Expr::CallResult {
                            callable: Box::new(expr.clone()),
                            args,
                        }
                    },
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
                // The index was parsed one level down, so there is room
                // for this link: the limit was checked then
                self.depth += 1;
                links += 1;
                expr = Expr::ArrayIndex {
                    array: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.match_token(&Token::Colon) {
                // Range expression (A1:B10)
                self.enter()?;
                links += 1;
                let end = self.primary()?;
                expr = Expr::Range {
                    start: Box::new(expr),
                    end: Box::new(end),
                };
            } else {
                break;
            }
        }

        self.depth -= links;
        Ok(expr)
    }

    /// Arguments: ( expr ( "," expr )* )?
    fn arguments(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();

        // Check for empty argument list
        if matches!(self.peek(), Some(Token::CloseParen)) {
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
            },
            Some(Token::Text(s)) => {
                self.advance();
                Ok(Expr::Text(s))
            },
            Some(Token::Boolean(b)) => {
                self.advance();
                Ok(Expr::Boolean(b))
            },
            Some(Token::Identifier(name)) => {
                self.advance();
                Ok(Self::parse_identifier(name))
            },
            Some(Token::OpenBrace) => {
                self.advance();
                self.array_constant()
            },
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
            },
            Some(token) => Err(ParseError::new(
                format!("Unexpected token: {token:?}"),
                self.position,
            )),
            None => Err(ParseError::new(
                "Unexpected end of expression",
                self.position,
            )),
        }
    }

    /// Excel takes only a range, not an array constant, where `COUNTIF` and
    /// its family read a range: it answers "There's a problem with this
    /// formula" (checked in Excel for `COUNTIF` and `SUMIF`, 2026-11-15). A
    /// model that Excel refuses after `forge export` is refused here, with the
    /// reason, instead of being exported (ADR-064).
    fn refuse_constant_for_range(name: &str, args: &[Expr]) -> Result<(), ParseError> {
        let function = name.to_uppercase();
        let in_family = matches!(
            function.as_str(),
            "SUMIF"
                | "COUNTIF"
                | "AVERAGEIF"
                | "SUMIFS"
                | "COUNTIFS"
                | "AVERAGEIFS"
                | "MAXIFS"
                | "MINIFS"
        );
        if in_family
            && args.iter().enumerate().any(|(position, arg)| {
                matches!(arg, Expr::Array { .. })
                    && super::whole_column::is_whole_column(&function, args.len(), position)
            })
        {
            return Err(ParseError::new(
                format!(
                    "{function} reads a range, and Excel does not accept an array constant \
                     there (it says \"There's a problem with this formula\"). Put the \
                     values in a table column and give that column"
                ),
                0,
            ));
        }
        Ok(())
    }

    /// The rest of an array constant, after its `{` (ADR-064): items
    /// separated by `,` (a row) or by `;` (a column), as Excel writes them.
    /// An item is a number, with a sign if any, a text or TRUE or FALSE.
    fn array_constant(&mut self) -> Result<Expr, ParseError> {
        let start = self.position;
        let mut items = Vec::new();
        let mut separator: Option<Token> = None;
        loop {
            items.push(self.array_item()?);
            match self.peek().cloned() {
                Some(Token::CloseBrace) => {
                    self.advance();
                    break;
                },
                Some(sep @ (Token::Comma | Token::Semicolon)) => {
                    if separator.as_ref().is_some_and(|seen| *seen != sep) {
                        return Err(ParseError::new(
                            "An array constant with both , and ; (such as {1,2;3,4}) has rows \
                             and columns, and Forge has no two-dimensional values. Use a row \
                             {1,2,3}, a column {1;2;3}, or a table",
                            self.position,
                        ));
                    }
                    separator = Some(sep);
                    self.advance();
                },
                _ => return Err(Self::not_an_item(self.position.max(start))),
            }
        }
        Ok(Expr::Array {
            items,
            column: separator == Some(Token::Semicolon),
        })
    }

    /// One item of an array constant
    fn array_item(&mut self) -> Result<Expr, ParseError> {
        let sign = match self.peek() {
            Some(Token::Operator(op)) if op == "-" || op == "+" => {
                let negative = op == "-";
                self.advance();
                Some(negative)
            },
            _ => None,
        };
        match (self.peek().cloned(), sign) {
            (Some(Token::Number(n)), Some(true)) => {
                self.advance();
                Ok(Expr::Number(-n))
            },
            (Some(Token::Number(n)), _) => {
                self.advance();
                Ok(Expr::Number(n))
            },
            (Some(Token::Text(s)), None) => {
                self.advance();
                Ok(Expr::Text(s))
            },
            (Some(Token::Boolean(b)), None) => {
                self.advance();
                Ok(Expr::Boolean(b))
            },
            (Some(Token::CloseBrace), None) => Err(ParseError::new(
                "An array constant needs at least one item, and no empty item: {1,2,3}",
                self.position,
            )),
            _ => Err(Self::not_an_item(self.position)),
        }
    }

    fn not_an_item(position: usize) -> ParseError {
        ParseError::new(
            "An array constant holds numbers, texts and TRUE or FALSE only, as in Excel: \
             {1,2,3} or {\"a\";\"b\"}. A name, a formula or an error such as #N/A cannot go \
             inside the braces",
            position,
        )
    }

    /// Parse an identifier - could be scalar or table.column
    fn parse_identifier(name: String) -> Expr {
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
///
/// # Errors
///
/// Returns `ParseError` if the tokens do not form a valid expression.
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
    fn test_parse_array_constants() {
        assert_eq!(
            parse_formula("{1,-2,\"a\"}").unwrap(),
            Expr::Array {
                items: vec![
                    Expr::Number(1.0),
                    Expr::Number(-2.0),
                    Expr::Text("a".to_string())
                ],
                column: false,
            }
        );
        assert_eq!(
            parse_formula("{TRUE; 2}").unwrap(),
            Expr::Array {
                items: vec![Expr::Boolean(true), Expr::Number(2.0)],
                column: true,
            }
        );
        // One item is a row
        assert_eq!(
            parse_formula("{7}").unwrap(),
            Expr::Array {
                items: vec![Expr::Number(7.0)],
                column: false,
            }
        );
        for bad in ["{1,2;3}", "{}", "{1,}", "{x}", "{(1)}", "{-\"a\"}", "{1 2}"] {
            assert!(parse_formula(bad).is_err(), "{bad}");
        }
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
    // Nesting limit (roadmap T1-DEPTH): the formula itself is the first level

    fn refused(formula: &str) -> bool {
        match parse_formula(formula) {
            Ok(_) => false,
            Err(e) => {
                assert!(
                    e.message
                        .contains(&format!("nested more than {MAX_DEPTH} levels")),
                    "{}",
                    e.message
                );
                true
            },
        }
    }

    /// The largest `n` the formula built by `build` accepts, found by
    /// trying one past it
    fn at_the_limit(build: impl Fn(usize) -> String, n: usize) {
        assert!(!refused(&build(n)), "{n} should be accepted");
        let one_more = n + 1;
        assert!(refused(&build(one_more)), "{one_more} should be refused");
    }

    fn depth_of(expr: &Expr) -> usize {
        1 + match expr {
            Expr::Number(_) | Expr::Text(_) | Expr::Boolean(_) | Expr::Reference(_) => 0,
            Expr::Array { items, .. } => items.iter().map(depth_of).max().unwrap_or(0),
            Expr::ArrayIndex { array, index } => depth_of(array).max(depth_of(index)),
            Expr::FunctionCall { args, .. } => args.iter().map(depth_of).max().unwrap_or(0),
            Expr::CallResult { callable, args } => {
                depth_of(callable).max(args.iter().map(depth_of).max().unwrap_or(0))
            },
            Expr::BinaryOp { left, right, .. } => depth_of(left).max(depth_of(right)),
            Expr::UnaryOp { operand, .. } => depth_of(operand),
            Expr::Range { start, end } => depth_of(start).max(depth_of(end)),
        }
    }

    #[test]
    fn parentheses_stop_at_the_limit() {
        at_the_limit(
            |n| format!("{}1{}", "(".repeat(n), ")".repeat(n)),
            MAX_DEPTH - 1,
        );
        // The parentheses add no node: the tree is the number alone
        let expr = parse_formula(&format!(
            "{}1{}",
            "(".repeat(MAX_DEPTH - 1),
            ")".repeat(MAX_DEPTH - 1)
        ))
        .unwrap();
        assert_eq!(expr, Expr::Number(1.0));
    }

    #[test]
    fn function_arguments_stop_at_the_limit() {
        at_the_limit(
            |n| format!("{}1{}", "ABS(".repeat(n), ")".repeat(n)),
            MAX_DEPTH - 1,
        );
    }

    #[test]
    fn a_run_of_minus_signs_stops_at_the_limit() {
        at_the_limit(|n| format!("{}1", "-".repeat(n)), MAX_DEPTH - 1);
    }

    #[test]
    fn a_run_of_powers_stops_at_the_limit() {
        at_the_limit(|n| vec!["2"; n].join("^"), MAX_DEPTH);
    }

    #[test]
    fn a_chain_of_each_operator_stops_at_the_limit() {
        for op in ["+", "*", "&", "="] {
            at_the_limit(|n| vec!["1"; n].join(op), MAX_DEPTH);
        }
    }

    #[test]
    fn a_chain_of_indexes_stops_at_the_limit() {
        at_the_limit(|n| format!("a{}", "[0]".repeat(n)), MAX_DEPTH - 1);
    }

    #[test]
    fn a_chain_of_calls_of_a_result_stops_at_the_limit() {
        at_the_limit(
            |n| format!("LAMBDA(q, q){}", "(1)".repeat(n)),
            MAX_DEPTH - 1,
        );
    }

    #[test]
    fn a_chain_of_ranges_stops_at_the_limit() {
        at_the_limit(|n| vec!["a"; n].join(":"), MAX_DEPTH);
    }

    #[test]
    fn the_error_gives_the_limit_and_what_to_do() {
        let message = parse_formula(&"-".repeat(1_000)).unwrap_err().message;
        assert_eq!(
            message,
            "The formula is nested more than 128 levels deep (parentheses, function \
             arguments and chained operators each count one). Split it into several \
             named values or columns"
        );
    }

    #[test]
    fn a_hundred_thousand_levels_are_refused_not_built() {
        for formula in [
            format!("{}1{}", "(".repeat(100_000), ")".repeat(100_000)),
            format!("{}1{}", "ABS(".repeat(100_000), ")".repeat(100_000)),
            format!("{}1", "-".repeat(100_000)),
            vec!["2"; 100_000].join("^"),
            vec!["1"; 100_000].join("+"),
        ] {
            assert!(refused(&formula));
        }
    }

    #[test]
    fn levels_are_given_back_when_a_rule_is_done() {
        // A thousand siblings, none inside another, however deep the sum of them
        let siblings = vec!["(1)"; 1_000].join(", ");
        assert!(parse_formula(&format!("SUM({siblings})")).is_ok());
        let indexes = vec!["a[0]"; 1_000].join(", ");
        assert!(parse_formula(&format!("SUM({indexes})")).is_ok());
        let powers = vec!["2^3"; 1_000].join(", ");
        assert!(parse_formula(&format!("SUM({powers})")).is_ok());
        let signs = vec!["--1"; 1_000].join(", ");
        assert!(parse_formula(&format!("SUM({signs})")).is_ok());
        let calls = vec!["LAMBDA(q, q)(1)(2)"; 1_000].join(", ");
        assert!(parse_formula(&format!("SUM({calls})")).is_ok());
        let ranges = vec!["a:b:c"; 1_000].join(", ");
        assert!(parse_formula(&format!("SUM({ranges})")).is_ok());
        let sums = vec!["1+2+3"; 1_000].join(", ");
        assert!(parse_formula(&format!("SUM({sums})")).is_ok());
    }

    #[test]
    fn the_tree_stays_under_twice_the_limit() {
        // The worst case: a chain as long as is allowed inside parentheses,
        // as the first operand of another as long as is allowed. The first
        // operand ends at the bottom of the second chain's left side.
        let inner = (1..=MAX_DEPTH)
            .map(|n| vec!["1"; n].join("+"))
            .take_while(|chain| !refused(&format!("({chain})+1")))
            .last()
            .unwrap();
        let outer = (1..=MAX_DEPTH)
            .map(|n| format!("({inner}){}", "+1".repeat(n)))
            .take_while(|formula| !refused(formula))
            .last()
            .unwrap();
        let depth = depth_of(&parse_formula(&outer).unwrap());
        assert!(depth > MAX_DEPTH, "{depth}: the case is not the worst one");
        assert!(depth <= 2 * MAX_DEPTH, "{depth}");
    }

    mod array_constants {
        //! Array constants (ADR-064): `{1,2,3}` (a row) and `{1;2;3}` (a column) of
        //! numbers, texts and TRUE or FALSE, as in Excel. Before, `{` was a syntax
        //! error, and `forge import` turned a formula with a constant into values.
        //! The answers are checked against Gnumeric and `LibreOffice` in
        //! `validation/gnumeric/tests/functions/array_constants.yaml`; these tests
        //! cover what the engines cannot: the refusals and the import.

        use crate::cli::{calculate_core, export_core, import_core};
        use rust_xlsxwriter::{Formula, Workbook};

        fn calculate(
            yaml: &str,
        ) -> Result<std::collections::BTreeMap<String, Option<f64>>, String> {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            std::fs::write(&path, yaml).unwrap();
            calculate_core(&path, true, None)
                .map(|r| r.scalars.into_iter().collect())
                .map_err(|e| e.to_string())
        }

        fn one(formula: &str) -> Result<Option<f64>, String> {
            let yaml = format!(
                "_forge_version: \"5.4.0\"\ns:\n  a: {{formula: {}}}\n",
                serde_json::to_string(formula).unwrap()
            );
            calculate(&yaml).map(|s| s["s.a"])
        }

        #[test]
        fn constants_calculate() {
            assert_eq!(one("=SUM({1,2,3})").unwrap(), Some(6.0));
            assert_eq!(one("=SUM({1;2;3})").unwrap(), Some(6.0));
            assert_eq!(one("=SUM({-1, +2, -3.5E1})").unwrap(), Some(-34.0));
            assert_eq!(
                one("=MATCH(\"b\", {\"a\";\"b\";\"c\"}, 0)").unwrap(),
                Some(2.0)
            );
            assert_eq!(one("=INDEX({10,20,30}, 2)").unwrap(), Some(20.0));
            assert_eq!(one("=SUMPRODUCT({1,2,3}, {4,5,6})").unwrap(), Some(32.0));
            assert_eq!(one("=IF(AND({TRUE, FALSE}), 1, 0)").unwrap(), Some(0.0));
        }

        #[test]
        fn a_row_formula_reads_a_constant_whole() {
            let yaml =
                "_forge_version: \"5.4.0\"\nt:\n  x: [1, 2, 3]\n  y: \"=INDEX({10;20;30}, x)\"\n\
                s:\n  total: {formula: \"=SUM(t.y)\"}\n";
            assert_eq!(calculate(yaml).unwrap()["s.total"], Some(60.0));
        }

        #[test]
        fn a_constant_is_still_fine_where_excel_takes_one() {
            // Not in a range argument: the criterion, or a function that takes arrays
            assert_eq!(one("=SUMPRODUCT(({1,2,3,2}=2)*1)").unwrap(), Some(2.0));
            let yaml = "_forge_version: \"5.4.0\"\nt:\n  v: [1, 2, 3, 2]\ns:\n  n: {formula: \"=COUNTIF(t.v, 2)\"}\n";
            assert_eq!(calculate(yaml).unwrap()["s.n"], Some(2.0));
        }

        #[test]
        fn what_forge_cannot_hold_is_refused_with_a_reason() {
            for (formula, reason) in [
                ("=SUM({1,2;3,4})", "two-dimensional"),
                ("=SUM({1,#N/A})", "NA() gives #N/A"),
                ("=SUM({1,x})", "numbers, texts and TRUE or FALSE only"),
                ("=SUM({1,2+3})", "numbers, texts and TRUE or FALSE only"),
                ("=SUM({})", "at least one item"),
                ("=SUM({1,2", "numbers, texts and TRUE or FALSE only"),
                ("={1,2,3}", "returns 3 values"),
                // Excel refuses a constant where these read a range (checked in Excel,
                // 2026-11-15)
                (
                    "=COUNTIF({1,2,3,2}, 2)",
                    "Excel does not accept an array constant",
                ),
                (
                    "=SUMIF({1,2,3}, \">1\")",
                    "Excel does not accept an array constant",
                ),
                (
                    "=SUMIFS({1,2,3}, {1,2,3}, 2)",
                    "Excel does not accept an array constant",
                ),
            ] {
                let err = one(formula).unwrap_err();
                assert!(err.contains(reason), "{formula}: {err}");
            }
            let yaml = "_forge_version: \"5.4.0\"\nt:\n  x: [1, 2, 3]\n  y: \"=x * {1,2,3}\"\n";
            let err = calculate(yaml).unwrap_err();
            assert!(err.contains("has 3 values"), "{err}");
        }

        #[test]
        fn import_keeps_a_formula_with_a_constant() {
            let dir = tempfile::tempdir().unwrap();
            let xlsx = dir.path().join("book.xlsx");
            let mut workbook = Workbook::new();
            let sheet = workbook.add_worksheet().set_name("t").unwrap();
            sheet.write_string(0, 0, "a").unwrap();
            sheet.write_string(0, 1, "b").unwrap();
            sheet.write_string(0, 2, "c").unwrap();
            for row in 1..=2u32 {
                sheet.write_number(row, 0, f64::from(row)).unwrap();
                let b = Formula::new(format!("=SUM({{1,2,3}})+A{}", row + 1)).set_result("0");
                sheet.write_formula(row, 1, b).unwrap();
                let c = Formula::new("=SUM({1,2;3,4})").set_result("10");
                sheet.write_formula(row, 2, c).unwrap();
            }
            workbook.save(&xlsx).unwrap();

            let out = dir.path().join("book.yaml");
            let imported = import_core(&xlsx, &out, false, false).unwrap();
            let yaml = std::fs::read_to_string(&out).unwrap();
            assert!(yaml.contains("SUM({1, 2, 3}) + a"), "{yaml}");
            // The two-dimensional one is imported as values, with the reason
            assert_eq!(imported.warnings.len(), 1, "{:?}", imported.warnings);
            assert!(
                imported.warnings[0].contains("rows and columns"),
                "{:?}",
                imported.warnings
            );
        }

        #[test]
        fn export_then_import_gives_the_same_answers() {
            let dir = tempfile::tempdir().unwrap();
            let model = dir.path().join("m.yaml");
            std::fs::write(
                &model,
                "_forge_version: \"5.4.0\"\ns:\n  a: {formula: \"=SUM({1;2;3}) * 2\"}\n  \
         b: {formula: \"=MATCH(\\\"c\\\", {\\\"a\\\",\\\"b\\\",\\\"c\\\"}, 0)\"}\n  \
         c: {formula: \"=LARGE({5,-1,3}, 3)\"}\n",
            )
            .unwrap();
            let first = calculate_core(&model, true, None).unwrap().scalars;
            let xlsx = dir.path().join("m.xlsx");
            export_core(&model, &xlsx).unwrap();
            let back = dir.path().join("back.yaml");
            let imported = import_core(&xlsx, &back, false, false).unwrap();
            assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
            let text = std::fs::read_to_string(&back).unwrap();
            assert!(text.contains("{1; 2; 3}"), "{text}");
            let again = calculate_core(&back, true, None).unwrap().scalars;
            assert_eq!(first, again);
        }
    }

    /// A formula that does not parse says where and why, in one message.
    #[test]
    fn a_malformed_formula_is_an_error_with_its_position() {
        for (formula, message) in [
            (
                "1 2",
                "Parse error at position 1: Unexpected token after expression: Some(Number(2.0))",
            ),
            (
                "(1",
                "Parse error at position 2: Expected ')' after expression",
            ),
            (
                "1 +",
                "Parse error at position 2: Unexpected end of expression",
            ),
            (
                "MAX(1",
                "Parse error at position 3: Expected ')' after function arguments",
            ),
        ] {
            let error = parse_formula(formula).unwrap_err();
            assert_eq!(error.to_string(), message, "{formula}");
        }
    }

    /// `depth_of` counts the levels of each kind of node, so the limit tests
    /// above count what they mean to.
    #[test]
    fn the_depth_of_each_kind_of_node() {
        for (formula, depth) in [
            ("1", 1),
            ("{1, 2, 3}", 2),
            ("a[0]", 2),
            ("a[b[0]]", 3),
            ("SUM(1, 2)", 2),
            ("LAMBDA(q, q)(1)", 3),
            ("1 + -2", 3),
            ("t.a:t.b", 2),
        ] {
            assert_eq!(
                depth_of(&parse_formula(formula).unwrap()),
                depth,
                "{formula}"
            );
        }
    }

    /// An operand that is missing or broken is the error of the operator
    /// before it, wherever it is.
    #[test]
    fn a_missing_operand_is_an_error_at_each_level_of_the_grammar() {
        for (formula, message) in [
            (
                "1 &",
                "Parse error at position 2: Unexpected end of expression",
            ),
            (
                "1 +",
                "Parse error at position 2: Unexpected end of expression",
            ),
            (
                "2 ^",
                "Parse error at position 2: Unexpected end of expression",
            ),
            (
                "1 =",
                "Parse error at position 2: Unexpected end of expression",
            ),
            (
                "2 *",
                "Parse error at position 2: Unexpected end of expression",
            ),
            (
                "a:",
                "Parse error at position 2: Unexpected end of expression",
            ),
            (
                "SUM(1, )",
                "Parse error at position 4: Unexpected token: CloseParen",
            ),
        ] {
            let error = parse_formula(formula).unwrap_err();
            assert_eq!(error.to_string(), message, "{formula}");
        }
    }

    #[test]
    fn a_formula_the_tokenizer_refuses_is_refused_with_its_position() {
        let error = parse_formula("\"abc").unwrap_err();
        assert!(
            error.to_string().starts_with("Parse error at position "),
            "{error}"
        );
    }

    #[test]
    fn advance_at_the_end_stays_on_the_last_token() {
        let mut parser = Parser::new(vec![Token::Number(1.0)]);
        assert_eq!(parser.advance(), Some(&Token::Number(1.0)));
        assert_eq!(parser.advance(), Some(&Token::Number(1.0)));
        assert_eq!(parser.position, 1);
    }

    #[test]
    fn links_with_nothing_inside_stop_at_the_limit() {
        at_the_limit(|n| format!("LAMBDA(q, q){}", "()".repeat(n)), MAX_DEPTH - 1);
        at_the_limit(|n| format!("a{}", "[0]".repeat(n)), MAX_DEPTH - 1);
        at_the_limit(|n| vec!["a"; n + 1].join(":"), MAX_DEPTH - 1);
    }
}
