use crate::lexer_test;

use super::{
    helpers::take_while,
    lexer_token::{Token, TokenKind},
};
use anyhow::{bail, Result};
use std::{io::ErrorKind, str};

pub fn tokenize_ident(input: &str) -> Result<(TokenKind, usize)> {
    match input.chars().next() {
        Some(ch) if ch.is_digit(10) => bail!("Identifiers cannot start with a digit"),
        None => bail!(ErrorKind::UnexpectedEof),
        _ => {}
    }

    let (got, len_read) = take_while(input, |ch| ch.is_alphanumeric() || ch == '_')?;

    let tok = TokenKind::Identifier(got.to_string());

    Ok((tok, len_read))
}

lexer_test!(tokenize_a_single_letter, tokenize_ident, "F" => "F");
lexer_test!(tokenize_an_identifer, tokenize_ident, "Foo" => "Foo");
lexer_test!(tokenize_ident_containing_an_underscore, tokenize_ident, "Foo_bar" => "Foo_bar");
lexer_test!(FAIL: tokenize_ident_cant_start_with_number, tokenize_ident, "7Foo_bar");
lexer_test!(FAIL: tokenize_ident_cant_start_with_dot, tokenize_ident, ".Foo_bar");

pub fn tokenize_number(input: &str) -> Result<(TokenKind, usize)> {
    let mut dot_seen = false;
    let (got, len_read) = take_while(input, |ch| match ch {
        c if c.is_digit(10) => true,
        c if c == '.' && !dot_seen => {
            dot_seen = true;
            true
        }
        _ => false,
    })?;

    let number: f64 = got.parse()?;
    let token = TokenKind::Number(number);

    Ok((token, len_read))
}

lexer_test!(tokenize_a_single_digit_integer, tokenize_number, "1" => 1.0);
lexer_test!(tokenize_a_longer_integer, tokenize_number, "1234567890" => 1234567890.0);
lexer_test!(tokenize_basic_decimal, tokenize_number, "12.3" => 12.3);
lexer_test!(tokenize_string_with_multiple_decimal_points, tokenize_number, "12.3.456" => 12.3);
lexer_test!(FAIL: cant_tokenize_a_string_as_a_decimal, tokenize_number, "asdfghj");
lexer_test!(tokenizing_decimal_stops_at_alpha, tokenize_number, "123.4asdfghj" => 123.4);

trait CharExtension {
    fn is_ws_without_nl(&self) -> bool;
}

impl CharExtension for char {
    fn is_ws_without_nl(&self) -> bool {
        self.is_whitespace() && *self != '\n'
    }
}

pub fn skip_whitespace(input: &str) -> usize {
    let first_char = match input.chars().next() {
        Some(ch) => ch,
        _ => return 0,
    };

    if !first_char.is_ws_without_nl() {
        return 0;
    }

    match take_while(input, |ch| ch != '\n' && ch.is_whitespace()) {
        Ok((_, len_skipped)) => len_skipped,
        _ => 0,
    }
}

pub fn capture_indentation(input: &str) -> Result<(TokenKind, usize)> {
    let length = match take_while(input, |ch| ch.is_whitespace()) {
        Ok((_, len_skipped)) => len_skipped,
        _ => 0,
    };

    let whitespace_size = u8::try_from(length)?;

    Ok((TokenKind::Indentation(whitespace_size), length))
}

#[test]
fn testws() {
    assert!('\n'.is_whitespace());
}

#[test]
fn skip_past_several_whitespace_chars() {
    let src = " \t\n\r123";
    let should_be = 4;

    let num_skipped = skip_whitespace(src);
    assert_eq!(num_skipped, should_be);
}

#[test]
fn skipping_whitespace_when_first_is_a_letter_returns_zero() {
    let src = "Hello World";
    let should_be = 0;

    let num_skipped = skip_whitespace(src);
    assert_eq!(num_skipped, should_be);
}

// // I will not skip whitespace due to indentation
// fn skip(input: &str) -> usize {
//     let mut remaining = input;

//     loop {
//         let ws = skip_whitespace(remaining);
//         remaining = &remaining[ws..];

//         if ws == 0 {
//             return input.len() - remaining.len();
//         }
//     }
// }

pub fn tokenize_single_token(input: &str) -> Result<(TokenKind, usize)> {
    let next = match input.chars().next() {
        Some(c) => c,
        _ => bail!(ErrorKind::UnexpectedEof),
    };

    let (token_got, length) = match next {
        '*' => (TokenKind::Asterisk, 1),
        '=' => (TokenKind::Equals, 1),
        '+' => (TokenKind::Plus, 1),
        '/' => (TokenKind::Slash, 1),
        '<' => (TokenKind::LessThan, 1),
        '>' => (TokenKind::GreaterThan, 1),
        '-' => (TokenKind::Minus, 1),
        ':' => (TokenKind::Colon, 1),
        '@' => (TokenKind::At, 1),
        '.' => (TokenKind::Dot, 1),
        ')' => (TokenKind::CloseParen, 1),
        ']' => (TokenKind::CloseSquare, 1),
        '(' => (TokenKind::OpenParen, 1),
        '[' => (TokenKind::OpenSquare, 1),
        ';' => (TokenKind::Semicolon, 1),
        '0'..='9' => tokenize_number(input)?,
        '"' => {
            let (got, len_read) = take_while(&input[1..], |ch| ch != '"')?;
            let token = TokenKind::QuotedString(got.to_string());
            (token, len_read + 2)
        }
        c @ '_' | c if c.is_alphabetic() => tokenize_ident(input)?,
        // c if c.is_whitespace() => (_, skip_whitespace(input)),
        '\n' => capture_indentation(input)?,
        _ => bail!(ErrorKind::InvalidData), // ErrorKind::UnknownCharacter(other)
    };

    Ok((token_got, length))
}

pub fn lex(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut remaining = input;
    let mut row = 1;
    let mut col_start = 1;
    let mut col_end = 1;
    let mut is_line_start = true;

    loop {
        if !is_line_start {
            let ws = skip_whitespace(remaining);
            col_start += ws;
            remaining = &remaining[ws..]
        } else {
            is_line_start = false;
        }

        // TODO: maybe check for any whitespace too?
        if remaining.is_empty() {
            break;
        }

        let (token, len_read) = tokenize_single_token(remaining)?;
        match token {
            TokenKind::Indentation(_) => {
                is_line_start = true;
                row += 1;
                col_start = 1;
                col_end = col_start + len_read;
            }
            _ => {
                col_end = col_start + len_read;
            }
        }

        // let start = input.len() - remaining.len();
        // let end = start + len_read;

        tokens.push(Token::new(
            //
            token, col_start, col_end, row,
        ));

        col_start = col_end;
        remaining = &remaining[len_read..];
    }

    Ok(tokens)
}
