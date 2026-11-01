//! Lexer: turns UTF-8 source text into a flat token stream (§2).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    /// `fn`, `add`, `calc` … (`[a-z_][a-z0-9_]*`)
    Ident(String),
    /// `Int32`, `Stdout` … (`[A-Z][A-Za-z0-9]*`)
    TypeIdent(String),
    /// `%1`, `%q`
    Temp(String),
    /// `123`, `-5`, `0xFF`, `1_000` (kept as text; typed later)
    Int(String),
    /// `"..."` with escapes resolved
    Str(String),
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Arrow,
    Eq,
    EqEq,
    Colon,
    Comma,
    Newline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub tok: Tok,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub line: u32,
    pub col: u32,
    pub message: String,
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut out = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut line: u32 = 1;
    let mut col: u32 = 1;
    let mut line_has_token = false;

    while i < chars.len() {
        let c = chars[i];
        let start_col = col;
        let is_word = |ch: char| ch.is_ascii_alphanumeric() || ch == '_';
        let fail = |message: String| LexError {
            line,
            col: start_col,
            message,
        };

        // Whitespace, newlines and comments.
        match c {
            '\n' => {
                // One Newline per non-empty line; blank lines collapse.
                if line_has_token {
                    out.push(Token {
                        tok: Tok::Newline,
                        line,
                        col: start_col,
                    });
                }
                line_has_token = false;
                line += 1;
                col = 1;
                i += 1;
                continue;
            }
            ' ' | '\t' | '\r' => {
                i += 1;
                col += 1;
                continue;
            }
            '#' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                    col += 1;
                }
                continue;
            }
            _ => {}
        }

        // Punctuation (one or two characters).
        let punct = match c {
            '{' => Some((Tok::LBrace, 1)),
            '}' => Some((Tok::RBrace, 1)),
            '(' => Some((Tok::LParen, 1)),
            ')' => Some((Tok::RParen, 1)),
            '[' => Some((Tok::LBracket, 1)),
            ']' => Some((Tok::RBracket, 1)),
            ':' => Some((Tok::Colon, 1)),
            ',' => Some((Tok::Comma, 1)),
            '=' if chars.get(i + 1) == Some(&'=') => Some((Tok::EqEq, 2)),
            '=' => Some((Tok::Eq, 1)),
            '-' if chars.get(i + 1) == Some(&'>') => Some((Tok::Arrow, 2)),
            _ => None,
        };
        if let Some((tok, width)) = punct {
            out.push(Token {
                tok,
                line,
                col: start_col,
            });
            line_has_token = true;
            i += width;
            col += width as u32;
            continue;
        }

        // Words: integers, temporaries, identifiers.
        let tok = match c {
            '-' | '0'..='9' => {
                let s = i;
                i += 1;
                while i < chars.len() && is_word(chars[i]) {
                    i += 1;
                }
                let text: String = chars[s..i].iter().collect();
                if text == "-" {
                    return Err(fail("expected a digit after '-'".into()));
                }
                col += (i - s) as u32;
                Tok::Int(text)
            }
            '%' => {
                let s = i + 1;
                i += 1;
                while i < chars.len() && is_word(chars[i]) {
                    i += 1;
                }
                if i == s {
                    return Err(fail("expected a name after '%'".into()));
                }
                col += (i - s + 1) as u32;
                Tok::Temp(chars[s..i].iter().collect())
            }
            'a'..='z' | '_' | 'A'..='Z' => {
                let s = i;
                i += 1;
                while i < chars.len() && is_word(chars[i]) {
                    i += 1;
                }
                let text: String = chars[s..i].iter().collect();
                col += (i - s) as u32;
                if c.is_ascii_uppercase() {
                    Tok::TypeIdent(text)
                } else {
                    Tok::Ident(text)
                }
            }
            '"' => {
                i += 1;
                col += 1;
                let mut text = String::new();
                loop {
                    let Some(&ch) = chars.get(i) else {
                        return Err(fail("unterminated string".into()));
                    };
                    i += 1;
                    col += 1;
                    match ch {
                        '"' => break,
                        '\n' => return Err(fail("newline inside string".into())),
                        '\\' => {
                            let Some(&e) = chars.get(i) else {
                                return Err(fail("unterminated escape".into()));
                            };
                            i += 1;
                            col += 1;
                            match e {
                                'n' => text.push('\n'),
                                't' => text.push('\t'),
                                '\\' => text.push('\\'),
                                '"' => text.push('"'),
                                'u' => {
                                    if chars.get(i) != Some(&'{') {
                                        return Err(fail("expected '{' after \\u".into()));
                                    }
                                    let s = i + 1;
                                    let mut j = s;
                                    while j < chars.len() && chars[j] != '}' {
                                        j += 1;
                                    }
                                    let hex: String = chars[s..j].iter().collect();
                                    let cp =
                                        u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32);
                                    let Some(cp) = cp else {
                                        return Err(fail(format!(
                                            "invalid unicode escape \\u{{{hex}}}"
                                        )));
                                    };
                                    text.push(cp);
                                    col += (j + 1 - i) as u32;
                                    i = j + 1;
                                }
                                other => return Err(fail(format!("unknown escape '\\{other}'"))),
                            }
                        }
                        other => text.push(other),
                    }
                }
                Tok::Str(text)
            }
            other => return Err(fail(format!("unexpected character '{other}'"))),
        };
        out.push(Token {
            tok,
            line,
            col: start_col,
        });
        line_has_token = true;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        lex(src).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn lexes_an_anf_binding_line() {
        assert_eq!(
            toks("%1 = add 10 20\n"),
            vec![
                Tok::Temp("1".into()),
                Tok::Eq,
                Tok::Ident("add".into()),
                Tok::Int("10".into()),
                Tok::Int("20".into()),
                Tok::Newline,
            ]
        );
    }

    #[test]
    fn lexes_a_function_header_with_arrow_and_type() {
        assert_eq!(
            toks("fn main() -> Int32"),
            vec![
                Tok::Ident("fn".into()),
                Tok::Ident("main".into()),
                Tok::LParen,
                Tok::RParen,
                Tok::Arrow,
                Tok::TypeIdent("Int32".into()),
            ]
        );
    }

    #[test]
    fn lexes_negative_hex_and_underscored_integers() {
        assert_eq!(
            toks("-5 0xFF 1_000"),
            vec![
                Tok::Int("-5".into()),
                Tok::Int("0xFF".into()),
                Tok::Int("1_000".into())
            ]
        );
    }

    #[test]
    fn lexes_strings_with_escapes() {
        assert_eq!(toks(r#""a\"b\n""#), vec![Tok::Str("a\"b\n".into())]);
    }

    #[test]
    fn drops_comments_and_blank_lines_but_keeps_one_newline() {
        assert_eq!(
            toks("ret %3 # done\n\n  \n}"),
            vec![
                Tok::Ident("ret".into()),
                Tok::Temp("3".into()),
                Tok::Newline,
                Tok::RBrace
            ]
        );
    }

    #[test]
    fn records_line_and_column_of_each_token() {
        let t = lex("fn\n  %q = div a b").unwrap();
        let temp = t.iter().find(|t| t.tok == Tok::Temp("q".into())).unwrap();
        assert_eq!((temp.line, temp.col), (2, 3));
    }

    #[test]
    fn rejects_unknown_characters_with_position() {
        let e = lex("add 1 @").unwrap_err();
        assert_eq!((e.line, e.col), (1, 7));
    }

    #[test]
    fn lexes_example_clause_with_double_equals() {
        assert_eq!(
            toks("example main == 85"),
            vec![
                Tok::Ident("example".into()),
                Tok::Ident("main".into()),
                Tok::EqEq,
                Tok::Int("85".into()),
            ]
        );
    }
}
