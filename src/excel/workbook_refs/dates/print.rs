//! A formula's syntax tree written back as formula text, with only the
//! parentheses its operators need.

use super::rewrite::Dates;
use crate::core::array_calculator::parser::Expr;
use crate::core::array_calculator::tokenizer::Token;

pub(super) fn precedence(expr: &Expr) -> u8 {
    match expr {
        Expr::BinaryOp { op, .. } => match op.as_str() {
            "&" => 2,
            "+" | "-" => 3,
            "*" | "/" => 4,
            "^" => 5,
            _ => 1,
        },
        Expr::UnaryOp { .. } => 6,
        _ => 7,
    }
}

pub(super) fn push(expr: &Expr, tokens: &mut Vec<Token>) {
    let wrapped = |inner: &Expr, needs: bool, tokens: &mut Vec<Token>| {
        if needs {
            tokens.push(Token::OpenParen);
            push(inner, tokens);
            tokens.push(Token::CloseParen);
        } else {
            push(inner, tokens);
        }
    };
    let arguments = |args: &[Expr], tokens: &mut Vec<Token>| {
        tokens.push(Token::OpenParen);
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                tokens.push(Token::Comma);
            }
            push(arg, tokens);
        }
        tokens.push(Token::CloseParen);
    };
    match expr {
        Expr::Number(n) => tokens.push(Token::Number(*n)),
        Expr::Text(text) => tokens.push(Token::Text(text.clone())),
        Expr::Boolean(b) => tokens.push(Token::Boolean(*b)),
        Expr::Array { items, column } => {
            tokens.push(Token::OpenBrace);
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    tokens.push(if *column {
                        Token::Semicolon
                    } else {
                        Token::Comma
                    });
                }
                push(item, tokens);
            }
            tokens.push(Token::CloseBrace);
        },
        Expr::Reference(reference) => tokens.push(Token::Identifier(Dates::word(reference))),
        Expr::FunctionCall { name, args } => {
            tokens.push(Token::Identifier(name.clone()));
            arguments(args, tokens);
        },
        Expr::CallResult { callable, args } => {
            push(callable, tokens);
            arguments(args, tokens);
        },
        Expr::ArrayIndex { array, index } => {
            wrapped(array, precedence(array) < 7, tokens);
            tokens.push(Token::OpenBracket);
            push(index, tokens);
            tokens.push(Token::CloseBracket);
        },
        Expr::Range { start, end } => {
            push(start, tokens);
            tokens.push(Token::Colon);
            push(end, tokens);
        },
        Expr::UnaryOp { op, operand } => {
            tokens.push(Token::Operator(op.clone()));
            wrapped(operand, precedence(operand) < 6, tokens);
        },
        Expr::BinaryOp { op, left, right } => {
            let own = precedence(expr);
            // `^` takes a unary on its left and groups to the right; the
            // others group to the left
            let (left_needs, right_needs) = if op == "^" {
                (precedence(left) < 6, precedence(right) < 5)
            } else {
                (precedence(left) < own, precedence(right) <= own)
            };
            wrapped(left, left_needs, tokens);
            tokens.push(Token::Operator(op.clone()));
            wrapped(right, right_needs, tokens);
        },
    }
}

/// A Forge formula as text, from its tokens.
pub(super) fn formula_text(tokens: &[Token]) -> String {
    let mut out = String::from("=");
    let mut after_value = false;
    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::Number(n) => out.push_str(&super::super::number_text(*n)),
            Token::Text(text) => {
                out.push('"');
                out.push_str(text);
                out.push('"');
            },
            Token::Boolean(b) => out.push_str(if *b { "TRUE" } else { "FALSE" }),
            Token::Identifier(word) => out.push_str(word),
            Token::Operator(op) => {
                if after_value {
                    out.push(' ');
                    out.push_str(op);
                    out.push(' ');
                } else {
                    out.push_str(op);
                }
            },
            Token::OpenParen => out.push('('),
            Token::CloseParen => out.push(')'),
            Token::OpenBracket => out.push('['),
            Token::CloseBracket => out.push(']'),
            Token::Comma => out.push_str(", "),
            Token::Colon => out.push(':'),
            Token::OpenBrace => out.push('{'),
            Token::CloseBrace => out.push('}'),
            Token::Semicolon => out.push_str("; "),
        }
        after_value = match token {
            Token::Number(_)
            | Token::Text(_)
            | Token::Boolean(_)
            | Token::CloseParen
            | Token::CloseBracket
            | Token::CloseBrace => true,
            Token::Identifier(_) => !matches!(tokens.get(i + 1), Some(Token::OpenParen)),
            _ => false,
        };
    }
    out
}

/// A part of a formula as Forge text, without the `=`.
#[must_use]
pub fn formula_of(expr: &Expr) -> String {
    let mut tokens = Vec::new();
    push(expr, &mut tokens);
    formula_text(&tokens)[1..].to_string()
}
