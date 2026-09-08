//! Build-time parser for the versioned `.sui` format (ADR-017).

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenSpec {
    pub id: String,
    pub content_id: String,
    pub tabs_id: String,
    pub tab_height: u32,
    pub tabs: Vec<TabSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabSpec {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileError {
    offset: usize,
    message: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for CompileError {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokenKind {
    Ident(String),
    String(String),
    Number(u32),
    LBrace,
    RBrace,
    Equals,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    offset: usize,
    kind: TokenKind,
}

pub fn compile(source: &str) -> Result<ScreenSpec, CompileError> {
    let tokens = tokenize(source)?;
    Parser { tokens, cursor: 0 }.screen()
}

fn tokenize(source: &str) -> Result<Vec<Token>, CompileError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b' ' | b'\t' | b'\r' | b'\n' => cursor += 1,
            b'#' => {
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
            }
            b'{' => {
                tokens.push(Token {
                    offset: cursor,
                    kind: TokenKind::LBrace,
                });
                cursor += 1;
            }
            b'}' => {
                tokens.push(Token {
                    offset: cursor,
                    kind: TokenKind::RBrace,
                });
                cursor += 1;
            }
            b'=' => {
                tokens.push(Token {
                    offset: cursor,
                    kind: TokenKind::Equals,
                });
                cursor += 1;
            }
            b'"' => {
                let start = cursor;
                cursor += 1;
                let mut value = String::new();
                while cursor < bytes.len() && bytes[cursor] != b'"' {
                    let rest = &source[cursor..];
                    let ch = rest.chars().next().ok_or_else(|| error(start, "invalid UTF-8"))?;
                    if ch == '\\' {
                        return Err(error(cursor, "escapes are not supported in SUI v1"));
                    }
                    value.push(ch);
                    cursor += ch.len_utf8();
                }
                if cursor == bytes.len() {
                    return Err(error(start, "unterminated string"));
                }
                cursor += 1;
                tokens.push(Token {
                    offset: start,
                    kind: TokenKind::String(value),
                });
            }
            byte if byte.is_ascii_digit() => {
                let start = cursor;
                while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                    cursor += 1;
                }
                let value = source[start..cursor]
                    .parse()
                    .map_err(|_| error(start, "number is out of range"))?;
                tokens.push(Token {
                    offset: start,
                    kind: TokenKind::Number(value),
                });
            }
            byte if is_ident_byte(byte) => {
                let start = cursor;
                while cursor < bytes.len() && is_ident_byte(bytes[cursor]) {
                    cursor += 1;
                }
                tokens.push(Token {
                    offset: start,
                    kind: TokenKind::Ident(source[start..cursor].to_string()),
                });
            }
            _ => return Err(error(cursor, "unexpected character")),
        }
    }
    Ok(tokens)
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.')
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn screen(mut self) -> Result<ScreenSpec, CompileError> {
        self.keyword("sui")?;
        let version = self.number()?;
        if version != 1 {
            return Err(self.fail(format!("unsupported SUI version {version}")));
        }
        self.keyword("screen")?;
        let id = self.ident()?;
        self.kind(TokenKind::LBrace)?;
        self.keyword("column")?;
        self.kind(TokenKind::LBrace)?;
        self.keyword("content")?;
        self.keyword("id")?;
        self.kind(TokenKind::Equals)?;
        let content_id = self.ident()?;
        self.keyword("fill")?;
        self.keyword("tabs")?;
        self.keyword("id")?;
        self.kind(TokenKind::Equals)?;
        let tabs_id = self.ident()?;
        self.keyword("height")?;
        self.kind(TokenKind::Equals)?;
        let tab_height = self.number()?;
        self.kind(TokenKind::LBrace)?;

        let mut tabs = Vec::new();
        while !self.next_is(&TokenKind::RBrace) {
            self.keyword("tab")?;
            let tab_id = self.ident()?;
            self.keyword("label")?;
            self.kind(TokenKind::Equals)?;
            let label = self.string()?;
            self.keyword("icon")?;
            self.kind(TokenKind::Equals)?;
            let icon = self.ident()?;
            self.keyword("action")?;
            self.kind(TokenKind::Equals)?;
            let action = self.ident()?;
            tabs.push(TabSpec {
                id: tab_id,
                label,
                icon,
                action,
            });
        }
        self.kind(TokenKind::RBrace)?;
        self.kind(TokenKind::RBrace)?;
        self.kind(TokenKind::RBrace)?;
        if self.cursor != self.tokens.len() {
            return Err(self.fail("trailing tokens"));
        }
        if tabs.is_empty() {
            return Err(self.fail("tabs block must not be empty"));
        }
        let mut ids = std::collections::HashSet::new();
        if tabs.iter().any(|tab| !ids.insert(tab.id.as_str())) {
            return Err(self.fail("duplicate tab id"));
        }

        Ok(ScreenSpec {
            id,
            content_id,
            tabs_id,
            tab_height,
            tabs,
        })
    }

    fn keyword(&mut self, expected: &str) -> Result<(), CompileError> {
        let offset = self.offset();
        match self.take() {
            Some(TokenKind::Ident(value)) if value == expected => Ok(()),
            _ => Err(error(offset, format!("expected `{expected}`"))),
        }
    }

    fn ident(&mut self) -> Result<String, CompileError> {
        let offset = self.offset();
        match self.take() {
            Some(TokenKind::Ident(value)) => Ok(value),
            _ => Err(error(offset, "expected identifier")),
        }
    }

    fn string(&mut self) -> Result<String, CompileError> {
        let offset = self.offset();
        match self.take() {
            Some(TokenKind::String(value)) => Ok(value),
            _ => Err(error(offset, "expected quoted UTF-8 string")),
        }
    }

    fn number(&mut self) -> Result<u32, CompileError> {
        let offset = self.offset();
        match self.take() {
            Some(TokenKind::Number(value)) => Ok(value),
            _ => Err(error(offset, "expected number")),
        }
    }

    fn kind(&mut self, expected: TokenKind) -> Result<(), CompileError> {
        let offset = self.offset();
        match self.take() {
            Some(value) if value == expected => Ok(()),
            _ => Err(error(offset, format!("expected {expected:?}"))),
        }
    }

    fn take(&mut self) -> Option<TokenKind> {
        let token = self.tokens.get(self.cursor)?.kind.clone();
        self.cursor += 1;
        Some(token)
    }

    fn next_is(&self, expected: &TokenKind) -> bool {
        self.tokens
            .get(self.cursor)
            .map(|token| &token.kind == expected)
            .unwrap_or(false)
    }

    fn offset(&self) -> usize {
        self.tokens
            .get(self.cursor)
            .map(|token| token.offset)
            .unwrap_or_else(|| self.tokens.last().map(|token| token.offset + 1).unwrap_or(0))
    }

    fn fail(&self, message: impl Into<String>) -> CompileError {
        error(self.offset(), message)
    }
}

fn error(offset: usize, message: impl Into<String>) -> CompileError {
    CompileError {
        offset,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::compile;

    const VALID: &str = r#"
        sui 1
        screen root {
          column {
            content id=content fill
            tabs id=root-tabs height=300 {
              tab now label="Сейчас" icon=now action=select_root:now
              tab me label="Я" icon=person action=select_root:me
            }
          }
        }
    "#;

    #[test]
    fn parses_utf8_screen() {
        let screen = compile(VALID).unwrap();
        assert_eq!(screen.id, "root");
        assert_eq!(screen.tabs[0].label, "Сейчас");
        assert_eq!(screen.tabs[1].action, "select_root:me");
    }

    #[test]
    fn rejects_unknown_version() {
        let error = compile(&VALID.replace("sui 1", "sui 2")).unwrap_err();
        assert!(error.to_string().contains("unsupported SUI version 2"));
    }

    #[test]
    fn rejects_duplicate_tab_ids() {
        let error = compile(&VALID.replace("tab me", "tab now")).unwrap_err();
        assert!(error.to_string().contains("duplicate tab id"));
    }
}
