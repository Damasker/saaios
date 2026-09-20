//! Build-time parser for the versioned `.sui` format (ADR-017).
//!
//! ADR-180 names the `.sui` v2 vocabulary. `compile()` still accepts
//! only `sui 1`. ADR-181/182 parse `sui 2` through `compile_v2()`.
//! ADR-183 keeps a frozen v1 rollback artifact. ADR-216 compiles
//! production `root.sui` through `compile_v2()` / `layout_v2()`.
//! ADR-184
//! emits layout/hit-test from that compiled v1 `ScreenSpec`. ADR-194
//! adds `layout_v2()` so public NOW tab hits match v1. ADR-195
//! converts logical `SafeInsets` through `EdgeInsets::from_safe`. ADR-196
//! names nested `tab` ids on `BottomNavigation` so `layout_v2` does
//! not borrow `compile_v1_rollback()`. ADR-185
//! gates third-party documents through `compile_v2_public()`. ADR-186
//! publishes the public example and stability labels.

mod layout;
mod rollback;
mod vocabulary;

pub use layout::{
    layout_v1_find, layout_v1_root, layout_v2, layout_v2_scrolled, v1_root_node,
    v1_tab_strip_height, v2_root_node,
};
pub use rollback::{compile_v1_rollback, V1_ROLLBACK_SOURCE};
pub use vocabulary::{
    sui_v2_a11y_roles, sui_v2_color_roles, sui_v2_composites, sui_v2_deferred, sui_v2_inset_values,
    sui_v2_is_component, sui_v2_is_deferred, sui_v2_is_privileged, sui_v2_is_public,
    sui_v2_is_surface, sui_v2_primitives, sui_v2_privileged, sui_v2_property_keys,
    sui_v2_scroll_values, sui_v2_spacing_tokens, sui_v2_stability, sui_v2_surfaces,
    sui_v2_text_roles, SuiV2Stability,
};

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenSpec {
    pub id: String,
    pub content_id: String,
    pub content_actions: Vec<ContentActionSpec>,
    pub tabs_id: String,
    pub tab_height: u32,
    pub tabs: Vec<TabSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentActionSpec {
    pub id: String,
    pub page: String,
    pub top: u32,
    pub height: u32,
    pub label: String,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabSpec {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub action: String,
}

/// ADR-181: a `sui 2` screen is an ordered list of vocabulary components.
/// ADR-199: optional nested `row` ids dock as the NOW footer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuiV2Screen {
    pub id: String,
    pub components: Vec<SuiV2Component>,
    pub rows: Vec<SuiV2Row>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SuiV2Props {
    pub text: Option<String>,
    pub color: Option<String>,
    pub spacing: Option<String>,
    pub inset: Option<String>,
    pub scroll: Option<String>,
    pub loc: Option<String>,
    pub focus: Option<u32>,
    pub a11y: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuiV2Component {
    pub type_name: String,
    pub props: SuiV2Props,
    pub tabs: Vec<SuiV2Tab>,
}

/// Nested `tab <id>` under `BottomNavigation`. Id is a public surface.
/// Action is `select_root:<id>` so hits match live v1 chrome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuiV2Tab {
    pub id: String,
    pub loc: Option<String>,
}

/// Nested `row <id>` on a `sui 2` screen. Footer ids (`apps`,
/// `intent`) match `now_footer_action_at`. Trailing list ids
/// (`refresh`, `scan`, `back`) match live list controls
/// (ADR-214).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuiV2Row {
    pub id: String,
    pub loc: Option<String>,
    pub action: String,
}

impl SuiV2Component {
    pub fn is_privileged(&self) -> bool {
        sui_v2_is_privileged(&self.type_name)
    }
}

impl SuiV2Screen {
    pub fn is_privileged(&self) -> bool {
        sui_v2_is_privileged(&self.id)
            || self
                .components
                .iter()
                .any(|component| component.is_privileged())
    }
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

pub fn compile_v2(source: &str) -> Result<SuiV2Screen, CompileError> {
    let tokens = tokenize(source)?;
    Parser { tokens, cursor: 0 }.v2_screen(false)
}

/// ADR-185: same grammar as `compile_v2()`, without privileged names.
pub fn compile_v2_public(source: &str) -> Result<SuiV2Screen, CompileError> {
    let tokens = tokenize(source)?;
    Parser { tokens, cursor: 0 }.v2_screen(true)
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
                    let ch = rest
                        .chars()
                        .next()
                        .ok_or_else(|| error(start, "invalid UTF-8"))?;
                    if ch == '\\' {
                        return Err(error(cursor, "escapes are not supported"));
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
        let mut content_actions = Vec::new();
        if self.next_is(&TokenKind::LBrace) {
            self.kind(TokenKind::LBrace)?;
            while !self.next_is(&TokenKind::RBrace) {
                self.keyword("action")?;
                let id = self.ident()?;
                self.keyword("page")?;
                self.kind(TokenKind::Equals)?;
                let page = self.ident()?;
                self.keyword("top")?;
                self.kind(TokenKind::Equals)?;
                let top = self.number()?;
                self.keyword("height")?;
                self.kind(TokenKind::Equals)?;
                let height = self.number()?;
                self.keyword("label")?;
                self.kind(TokenKind::Equals)?;
                let label = self.string()?;
                self.keyword("action")?;
                self.kind(TokenKind::Equals)?;
                let action = self.ident()?;
                content_actions.push(ContentActionSpec {
                    id,
                    page,
                    top,
                    height,
                    label,
                    action,
                });
            }
            self.kind(TokenKind::RBrace)?;
        }
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
        if content_actions
            .iter()
            .any(|action| action.height == 0 || !ids.insert(action.id.as_str()))
        {
            return Err(self.fail("content action id must be unique and height non-zero"));
        }
        ids.clear();
        if tabs.iter().any(|tab| !ids.insert(tab.id.as_str())) {
            return Err(self.fail("duplicate tab id"));
        }

        Ok(ScreenSpec {
            id,
            content_id,
            content_actions,
            tabs_id,
            tab_height,
            tabs,
        })
    }

    fn v2_screen(mut self, public_only: bool) -> Result<SuiV2Screen, CompileError> {
        self.keyword("sui")?;
        let version = self.number()?;
        if version != 2 {
            return Err(self.fail(format!("compile_v2 expected version 2, got {version}")));
        }
        self.keyword("screen")?;
        let id = self.ident()?;
        if !sui_v2_is_surface(&id) {
            return Err(self.fail(format!("unknown SUI v2 surface `{id}`")));
        }
        if public_only && sui_v2_is_privileged(&id) {
            return Err(self.fail(format!("privileged SUI v2 surface `{id}`")));
        }
        self.kind(TokenKind::LBrace)?;
        let mut components = Vec::new();
        let mut rows = Vec::new();
        let mut row_ids = std::collections::HashSet::new();
        while !self.next_is(&TokenKind::RBrace) {
            if self.next_is_ident("row") {
                rows.push(self.v2_row(&mut row_ids)?);
                continue;
            }
            self.keyword("component")?;
            let type_name = self.ident()?;
            if sui_v2_is_deferred(&type_name) {
                return Err(self.fail(format!("deferred SUI v2 name `{type_name}`")));
            }
            if !sui_v2_is_component(&type_name) {
                return Err(self.fail(format!("unknown SUI v2 component `{type_name}`")));
            }
            if public_only && sui_v2_is_privileged(&type_name) {
                return Err(self.fail(format!("privileged SUI v2 component `{type_name}`")));
            }
            self.kind(TokenKind::LBrace)?;
            let props = self.v2_props()?;
            let tabs = if type_name == "BottomNavigation" {
                self.v2_tabs(public_only)?
            } else if self.next_is_ident("tab") {
                return Err(self.fail("nested tabs only belong on BottomNavigation"));
            } else {
                Vec::new()
            };
            self.kind(TokenKind::RBrace)?;
            components.push(SuiV2Component {
                type_name,
                props,
                tabs,
            });
        }
        self.kind(TokenKind::RBrace)?;
        if self.cursor != self.tokens.len() {
            return Err(self.fail("trailing tokens"));
        }
        if components.is_empty() {
            return Err(self.fail("SUI v2 screen must name a component"));
        }
        Ok(SuiV2Screen {
            id,
            components,
            rows,
        })
    }

    fn v2_row(
        &mut self,
        ids: &mut std::collections::HashSet<String>,
    ) -> Result<SuiV2Row, CompileError> {
        self.keyword("row")?;
        let id = self.ident()?;
        let action = match id.as_str() {
            "apps" => "open_apps",
            "intent" => "open_intent_input",
            "refresh" => "list_refresh",
            "scan" => "list_scan",
            "back" => "list_back",
            _ => return Err(self.fail(format!("unknown SUI v2 row `{id}`"))),
        }
        .to_string();
        if !ids.insert(id.clone()) {
            return Err(self.fail("duplicate row id"));
        }
        self.kind(TokenKind::LBrace)?;
        let props = self.v2_props()?;
        self.kind(TokenKind::RBrace)?;
        Ok(SuiV2Row {
            id,
            loc: props.loc,
            action,
        })
    }

    fn v2_props(&mut self) -> Result<SuiV2Props, CompileError> {
        let mut props = SuiV2Props::default();
        let mut seen = std::collections::HashSet::new();
        while !self.next_is(&TokenKind::RBrace) && !self.next_is_ident("tab") {
            let key = self.ident()?;
            if !sui_v2_property_keys().contains(&key.as_str()) {
                return Err(self.fail(format!("unknown SUI v2 property `{key}`")));
            }
            if !seen.insert(key.clone()) {
                return Err(self.fail(format!("duplicate SUI v2 property `{key}`")));
            }
            self.kind(TokenKind::Equals)?;
            match key.as_str() {
                "text" => props.text = Some(self.v2_named_value("text", sui_v2_text_roles())?),
                "color" => props.color = Some(self.v2_named_value("color", sui_v2_color_roles())?),
                "spacing" => {
                    props.spacing = Some(self.v2_named_value("spacing", sui_v2_spacing_tokens())?)
                }
                "inset" => props.inset = Some(self.v2_named_value("inset", sui_v2_inset_values())?),
                "scroll" => {
                    props.scroll = Some(self.v2_named_value("scroll", sui_v2_scroll_values())?)
                }
                "a11y" => props.a11y = Some(self.v2_named_value("a11y", sui_v2_a11y_roles())?),
                "loc" => props.loc = Some(self.v2_loc()?),
                "focus" => props.focus = Some(self.number()?),
                _ => return Err(self.fail(format!("unknown SUI v2 property `{key}`"))),
            }
        }
        Ok(props)
    }

    fn v2_tabs(&mut self, public_only: bool) -> Result<Vec<SuiV2Tab>, CompileError> {
        let mut tabs = Vec::new();
        let mut ids = std::collections::HashSet::new();
        while self.next_is_ident("tab") {
            self.keyword("tab")?;
            let id = self.ident()?;
            if !sui_v2_is_surface(&id) {
                return Err(self.fail(format!("unknown SUI v2 tab `{id}`")));
            }
            if public_only && sui_v2_is_privileged(&id) {
                return Err(self.fail(format!("privileged SUI v2 tab `{id}`")));
            }
            if !ids.insert(id.clone()) {
                return Err(self.fail("duplicate tab id"));
            }
            self.kind(TokenKind::LBrace)?;
            let props = self.v2_props()?;
            self.kind(TokenKind::RBrace)?;
            tabs.push(SuiV2Tab { id, loc: props.loc });
        }
        Ok(tabs)
    }

    fn v2_named_value(&mut self, key: &str, allowed: &[&str]) -> Result<String, CompileError> {
        let value = self.ident()?;
        if !allowed.contains(&value.as_str()) {
            return Err(self.fail(format!("unknown SUI v2 {key} `{value}`")));
        }
        Ok(value)
    }

    fn v2_loc(&mut self) -> Result<String, CompileError> {
        match self.tokens.get(self.cursor).map(|token| &token.kind) {
            Some(TokenKind::Ident(_)) => self.ident(),
            Some(TokenKind::String(_)) => self.string(),
            _ => Err(self.fail("expected loc identifier or string")),
        }
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

    fn next_is_ident(&self, name: &str) -> bool {
        matches!(
            self.tokens.get(self.cursor).map(|token| &token.kind),
            Some(TokenKind::Ident(value)) if value == name
        )
    }

    fn offset(&self) -> usize {
        self.tokens
            .get(self.cursor)
            .map(|token| token.offset)
            .unwrap_or_else(|| {
                self.tokens
                    .last()
                    .map(|token| token.offset + 1)
                    .unwrap_or(0)
            })
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
    use super::{compile, compile_v2, compile_v2_public, sui_v2_stability, SuiV2Stability};

    const VALID: &str = r#"
        sui 1
        screen root {
          column {
            content id=content fill
            tabs id=root-tabs height=300 {
              tab now label="Сейчас" icon=now action=select_root:now
              tab me label="Система" icon=person action=select_root:me
            }
          }
        }
    "#;

    const VALID_V2: &str = r#"
        sui 2
        screen now {
          component ContextHeader {
            text = Title
            a11y = Heading
            loc = now.header
            focus = 0
            inset = safe
          }
          component ObjectSummary {
            text = Body
            color = TextPrimary
            a11y = Status
            loc = "now.object"
          }
          component BottomNavigation {
            a11y = Button
            focus = 1
            scroll = none
            inset = safe
          }
        }
    "#;

    #[test]
    fn parses_utf8_screen() {
        let screen = compile(VALID).unwrap();
        assert_eq!(screen.id, "root");
        assert!(screen.content_actions.is_empty());
        assert_eq!(screen.tabs[0].label, "Сейчас");
        assert_eq!(screen.tabs[1].action, "select_root:me");
    }

    #[test]
    fn parses_declarative_content_action() {
        let source = VALID.replace(
            "content id=content fill",
            "content id=content fill { action demo page=now top=430 height=220 label=\"Demo\" action=manage_app:org.saaios.demo }",
        );
        let screen = compile(&source).unwrap();
        let action = &screen.content_actions[0];
        assert_eq!(action.id, "demo");
        assert_eq!(action.page, "now");
        assert_eq!(action.top, 430);
        assert_eq!(action.height, 220);
        assert_eq!(action.action, "manage_app:org.saaios.demo");
    }

    #[test]
    fn rejects_unknown_version() {
        let error = compile(&VALID.replace("sui 1", "sui 2")).unwrap_err();
        assert!(error.to_string().contains("unsupported SUI version 2"));
    }

    #[test]
    fn production_root_sui_is_version_two() {
        let source = include_str!("../../../services/saai-shell/ui/root.sui");
        let screen = compile_v2(source).unwrap();
        assert_eq!(screen.id, "root");
        let tabs: Vec<&str> = screen
            .components
            .iter()
            .find(|component| component.type_name == "BottomNavigation")
            .map(|component| component.tabs.iter().map(|tab| tab.id.as_str()).collect())
            .unwrap_or_default();
        assert_eq!(tabs, ["now", "inbox", "spaces", "me"]);
        assert!(screen.rows.is_empty());
        assert!(compile_v2_public(source).is_ok());
    }

    #[test]
    fn rejects_duplicate_tab_ids() {
        let error = compile(&VALID.replace("tab me", "tab now")).unwrap_err();
        assert!(error.to_string().contains("duplicate tab id"));
    }

    #[test]
    fn compile_v2_names_proven_now_chrome() {
        let screen = compile_v2(VALID_V2).unwrap();
        assert_eq!(screen.id, "now");
        let names: Vec<&str> = screen
            .components
            .iter()
            .map(|component| component.type_name.as_str())
            .collect();
        assert_eq!(
            names,
            ["ContextHeader", "ObjectSummary", "BottomNavigation"]
        );
        assert!(screen
            .components
            .iter()
            .all(|component| !component.is_privileged()));
        let header = &screen.components[0];
        assert_eq!(header.props.text.as_deref(), Some("Title"));
        assert_eq!(header.props.a11y.as_deref(), Some("Heading"));
        assert_eq!(header.props.loc.as_deref(), Some("now.header"));
        assert_eq!(header.props.focus, Some(0));
        assert_eq!(header.props.inset.as_deref(), Some("safe"));
        assert_eq!(
            screen.components[1].props.loc.as_deref(),
            Some("now.object")
        );
        assert_eq!(
            screen.components[1].props.color.as_deref(),
            Some("TextPrimary")
        );
        assert_eq!(screen.components[2].props.focus, Some(1));
        assert_eq!(screen.components[2].props.scroll.as_deref(), Some("none"));
        assert!(screen.components[2].tabs.is_empty());
        assert!(screen.rows.is_empty());
    }

    #[test]
    fn compile_v2_nested_tabs_belong_only_on_bottom_navigation() {
        let screen = compile_v2_public(include_str!("../../../docs/os/ui/examples/now-public.sui"))
            .expect("public NOW");
        let ids: Vec<&str> = screen.components[3]
            .tabs
            .iter()
            .map(|tab| tab.id.as_str())
            .collect();
        assert_eq!(ids, ["now", "inbox", "spaces", "me"]);
        let header_tabs = compile_v2(
            r#"
            sui 2
            screen now {
              component ContextHeader {
                tab now {}
              }
            }
            "#,
        )
        .unwrap_err();
        assert!(header_tabs
            .to_string()
            .contains("nested tabs only belong on BottomNavigation"));
        let duplicate = compile_v2(
            r#"
            sui 2
            screen now {
              component BottomNavigation {
                tab now {}
                tab now {}
              }
            }
            "#,
        )
        .unwrap_err();
        assert!(duplicate.to_string().contains("duplicate tab id"));
        let lock_tab = compile_v2_public(
            r#"
            sui 2
            screen now {
              component BottomNavigation {
                tab lock {}
              }
            }
            "#,
        )
        .unwrap_err();
        assert!(lock_tab
            .to_string()
            .contains("privileged SUI v2 tab `lock`"));
        assert!(compile_v2(
            r#"
            sui 2
            screen now {
              component BottomNavigation {
                tab lock {}
              }
            }
            "#,
        )
        .is_ok());
    }

    #[test]
    fn compile_v2_nested_rows_name_the_now_footer() {
        let screen = compile_v2_public(include_str!("../../../docs/os/ui/examples/now-public.sui"))
            .expect("public NOW");
        let rows: Vec<(&str, &str)> = screen
            .rows
            .iter()
            .map(|row| (row.id.as_str(), row.action.as_str()))
            .collect();
        assert_eq!(
            rows,
            [("apps", "open_apps"), ("intent", "open_intent_input")]
        );
        let wifi_rows =
            compile_v2_public(include_str!("../../../docs/os/ui/examples/wifi-public.sui"))
                .expect("public Wi-Fi");
        let named: Vec<(&str, &str)> = wifi_rows
            .rows
            .iter()
            .map(|row| (row.id.as_str(), row.action.as_str()))
            .collect();
        assert_eq!(named, [("refresh", "list_refresh"), ("back", "list_back")]);
        let unknown = compile_v2(
            r#"
            sui 2
            screen now {
              component ContextHeader {}
              row search {}
            }
            "#,
        )
        .unwrap_err();
        assert!(unknown.to_string().contains("unknown SUI v2 row `search`"));
        let duplicate = compile_v2(
            r#"
            sui 2
            screen now {
              component ContextHeader {}
              row apps {}
              row apps {}
            }
            "#,
        )
        .unwrap_err();
        assert!(duplicate.to_string().contains("duplicate row id"));
    }

    #[test]
    fn compile_v2_marks_privileged_components() {
        let source = r#"
            sui 2
            screen now {
              component OrbHost {}
            }
        "#;
        let screen = compile_v2(source).unwrap();
        assert!(screen.components[0].is_privileged());
    }

    #[test]
    fn compile_v2_rejects_deferred_and_unknown_names() {
        let deferred = compile_v2(
            r#"
            sui 2
            screen now {
              component SpaceDetail {}
            }
        "#,
        )
        .unwrap_err();
        assert!(deferred
            .to_string()
            .contains("deferred SUI v2 name `SpaceDetail`"));
        let unknown = compile_v2(
            r#"
            sui 2
            screen now {
              component WidgetCard {}
            }
        "#,
        )
        .unwrap_err();
        assert!(unknown
            .to_string()
            .contains("unknown SUI v2 component `WidgetCard`"));
        let surface = compile_v2(
            r#"
            sui 2
            screen home {
              component ContextHeader {}
            }
        "#,
        )
        .unwrap_err();
        assert!(surface
            .to_string()
            .contains("unknown SUI v2 surface `home`"));
    }

    #[test]
    fn compile_v2_rejects_unknown_properties_and_empty_screens() {
        let props = compile_v2(
            r#"
            sui 2
            screen now {
              component ContextHeader { role = heading }
            }
        "#,
        )
        .unwrap_err();
        assert!(props.to_string().contains("unknown SUI v2 property `role`"));
        let text = compile_v2(
            r#"
            sui 2
            screen now {
              component ContextHeader { text = Headline }
            }
        "#,
        )
        .unwrap_err();
        assert!(text.to_string().contains("unknown SUI v2 text `Headline`"));
        let dup = compile_v2(
            r#"
            sui 2
            screen now {
              component ContextHeader { text = Title text = Body }
            }
        "#,
        )
        .unwrap_err();
        assert!(dup.to_string().contains("duplicate SUI v2 property `text`"));
        let empty = compile_v2(
            r#"
            sui 2
            screen now {
            }
        "#,
        )
        .unwrap_err();
        assert!(empty
            .to_string()
            .contains("SUI v2 screen must name a component"));
    }

    #[test]
    fn compile_stays_on_version_one() {
        let error = compile(VALID_V2).unwrap_err();
        assert!(error.to_string().contains("unsupported SUI version 2"));
        let error = compile_v2(VALID).unwrap_err();
        assert!(error.to_string().contains("compile_v2 expected version 2"));
    }

    #[test]
    fn compile_v2_public_accepts_the_now_sample_and_rejects_privileged() {
        let screen = compile_v2_public(VALID_V2).unwrap();
        assert!(!screen.is_privileged());
        let orb = compile_v2_public(
            r#"
            sui 2
            screen now {
              component OrbHost {}
            }
        "#,
        )
        .unwrap_err();
        assert!(orb
            .to_string()
            .contains("privileged SUI v2 component `OrbHost`"));
        assert!(compile_v2(
            r#"
            sui 2
            screen now {
              component OrbHost {}
            }
        "#,
        )
        .unwrap()
        .is_privileged());
        let lock = compile_v2_public(
            r#"
            sui 2
            screen lock {
              component ContextHeader {}
            }
        "#,
        )
        .unwrap_err();
        assert!(lock
            .to_string()
            .contains("privileged SUI v2 surface `lock`"));
        let build = include_str!("../../../services/saai-shell/build.rs");
        assert!(build.contains("saai_ui_compiler::compile_v2"));
        assert!(!build.contains("saai_ui_compiler::compile("));
        assert!(build.contains("ADR-216"));
    }

    #[test]
    fn public_now_example_compiles_and_stays_experimental() {
        const EXAMPLE: &str = include_str!("../../../docs/os/ui/examples/now-public.sui");
        let screen = compile_v2_public(EXAMPLE).unwrap();
        assert_eq!(screen.id, "now");
        assert!(!screen.is_privileged());
        let names: Vec<&str> = screen
            .components
            .iter()
            .map(|component| component.type_name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "ContextHeader",
                "ObjectSummary",
                "SurfacePattern",
                "BottomNavigation"
            ]
        );
        for name in names {
            assert_eq!(sui_v2_stability(name), Some(SuiV2Stability::Experimental));
        }
        assert!(compile(EXAMPLE)
            .unwrap_err()
            .to_string()
            .contains("unsupported SUI version 2"));
        let guide = include_str!("../../../docs/os/ui/sui-v2-public-api.md");
        assert!(guide.contains("now-public.sui"));
        assert!(guide.contains("Experimental"));
        assert!(guide.contains("compile_v2_public"));
        assert!(guide.contains("row apps"));
        assert!(guide.contains("open_object"));
        assert_eq!(screen.rows.len(), 2);
        const INBOX: &str = include_str!("../../../docs/os/ui/examples/inbox-public.sui");
        let inbox = compile_v2_public(INBOX).unwrap();
        assert_eq!(inbox.id, "inbox");
        assert!(!inbox.is_privileged());
        assert_eq!(inbox.components[1].type_name, "EventRow");
        assert_eq!(inbox.components[1].props.a11y.as_deref(), Some("Button"));
        assert!(guide.contains("inbox-public.sui"));
        const SPACES: &str = include_str!("../../../docs/os/ui/examples/spaces-public.sui");
        let spaces = compile_v2_public(SPACES).unwrap();
        assert_eq!(spaces.id, "spaces");
        assert!(!spaces.is_privileged());
        assert_eq!(spaces.components[1].type_name, "SpaceRow");
        assert_eq!(spaces.components[1].props.a11y.as_deref(), Some("Button"));
        assert!(guide.contains("spaces-public.sui"));
        const ME: &str = include_str!("../../../docs/os/ui/examples/me-public.sui");
        let me = compile_v2_public(ME).unwrap();
        assert_eq!(me.id, "me");
        assert!(!me.is_privileged());
        assert_eq!(me.components[1].type_name, "SystemSection");
        assert_eq!(me.components[1].props.a11y.as_deref(), Some("Heading"));
        assert_eq!(me.components[5].type_name, "SettingRow");
        assert_eq!(me.components[5].props.a11y.as_deref(), Some("Button"));
        assert_eq!(
            me.components[5].props.loc.as_deref(),
            Some("cycle_timezone")
        );
        assert!(guide.contains("me-public.sui"));
        const WIFI: &str = include_str!("../../../docs/os/ui/examples/wifi-public.sui");
        let wifi = compile_v2_public(WIFI).unwrap();
        assert_eq!(wifi.id, "wifi");
        assert!(!wifi.is_privileged());
        assert_eq!(wifi.components[1].type_name, "WifiRow");
        assert_eq!(wifi.components[1].props.a11y.as_deref(), Some("Button"));
        assert!(guide.contains("wifi-public.sui"));
        const BLUETOOTH: &str = include_str!("../../../docs/os/ui/examples/bluetooth-public.sui");
        let bluetooth = compile_v2_public(BLUETOOTH).unwrap();
        assert_eq!(bluetooth.id, "bluetooth");
        assert!(!bluetooth.is_privileged());
        assert_eq!(bluetooth.components[1].type_name, "BluetoothRow");
        assert_eq!(
            bluetooth.components[1].props.a11y.as_deref(),
            Some("Button")
        );
        assert!(guide.contains("bluetooth-public.sui"));
        const TRUSTED: &str = include_str!("../../../docs/os/ui/examples/trusted-privileged.sui");
        assert!(compile_v2_public(TRUSTED)
            .unwrap_err()
            .to_string()
            .contains("privileged SUI v2 component `TrustedClientRow`"));
        let trusted = compile_v2(TRUSTED).unwrap();
        assert_eq!(trusted.id, "trusted");
        assert!(trusted.is_privileged());
        assert_eq!(trusted.components[1].type_name, "TrustedClientRow");
        assert_eq!(trusted.components[1].props.a11y.as_deref(), Some("Button"));
        assert!(guide.contains("trusted-privileged.sui"));
        const CAPABILITY: &str =
            include_str!("../../../docs/os/ui/examples/capability-privileged.sui");
        assert!(compile_v2_public(CAPABILITY)
            .unwrap_err()
            .to_string()
            .contains("privileged SUI v2 component `CapabilityRow`"));
        let capability = compile_v2(CAPABILITY).unwrap();
        assert_eq!(capability.id, "me");
        assert!(capability.is_privileged());
        assert_eq!(capability.components[1].type_name, "CapabilityRow");
        assert_eq!(
            capability.components[1].props.a11y.as_deref(),
            Some("Status")
        );
        assert!(guide.contains("capability-privileged.sui"));
        for name in saai_ui_core::public_gallery_type_names() {
            assert_eq!(sui_v2_stability(name), Some(SuiV2Stability::Experimental));
        }
        for name in saai_ui_core::privileged_gallery_type_names() {
            assert_eq!(sui_v2_stability(name), Some(SuiV2Stability::Privileged));
        }
    }

    #[test]
    fn vui09_verification_ledger_is_not_sign_off() {
        let ledger = include_str!("../../../docs/os/ui/vui09-verification.md");
        assert!(ledger.contains("not Visual v1 sign-off"));
        assert!(ledger.contains("compile_v2()"));
        assert!(ledger.contains("layout_v2"));
        assert!(ledger.contains("cold boot"));
        assert!(ledger.contains("e8865301"));
        assert!(ledger.contains("5eb6a27f"));
        assert!(ledger.contains("3850427a"));
        assert!(ledger.contains("ADR-188"));
        assert!(ledger.contains("ADR-190"));
        assert!(ledger.contains("150%"));
        assert!(ledger.contains("ADR-191"));
        assert!(ledger.contains("Нет сети"));
        assert!(ledger.contains("ADR-192"));
        assert!(ledger.contains("unlocked"));
        assert!(ledger.contains("ADR-193"));
        assert!(ledger.contains("ADR-194"));
        assert!(ledger.contains("ADR-195"));
        assert!(ledger.contains("from_safe"));
        assert!(ledger.contains("ADR-196"));
        assert!(ledger.contains("nested tab"));
        assert!(ledger.contains("ADR-197"));
        assert!(ledger.contains("Caption"));
        assert!(ledger.contains("ADR-198"));
        assert!(ledger.contains("ADR-199"));
        assert!(ledger.contains("now_footer_action_rect"));
        assert!(ledger.contains("ADR-200"));
        assert!(ledger.contains("open_object"));
        assert!(ledger.contains("ADR-201"));
        assert!(ledger.contains("below Caption"));
        assert!(ledger.contains("ADR-202"));
        assert!(ledger.contains("stacked_row_rect"));
        assert!(ledger.contains("ADR-203"));
        assert!(ledger.contains("select_space"));
        assert!(ledger.contains("ADR-204"));
        assert!(ledger.contains("cycle_timezone"));
        assert!(ledger.contains("ADR-205"));
        assert!(ledger.contains("connect_wifi"));
        assert!(ledger.contains("ADR-206"));
        assert!(ledger.contains("pair_bluetooth"));
        assert!(ledger.contains("ADR-207"));
        assert!(ledger.contains("revoke_trusted_client"));
        assert!(ledger.contains("ADR-208"));
        assert!(ledger.contains("CapabilityRow"));
        assert!(ledger.contains("ADR-209"));
        assert!(ledger.contains("tap-unlock"));
        assert!(ledger.contains("ADR-210"));
        assert!(ledger.contains("saai-displayd"));
        assert!(ledger.contains("ADR-211"));
        assert!(ledger.contains("reboot -f"));
        assert!(ledger.contains("ADR-212"));
        assert!(ledger.contains("7-tap"));
        assert!(ledger.contains("Диагностика"));
        assert!(ledger.contains("ADR-213"));
        assert!(ledger.contains("thin tuning"));
        assert!(ledger.contains("ADR-214"));
        assert!(ledger.contains("list_back"));
        assert!(ledger.contains("stacked_trailing_rect"));
        assert!(ledger.contains("ADR-215"));
        assert!(ledger.contains("scrolled_row_rect"));
        assert!(ledger.contains("flatten_me_rows"));
        let limits = include_str!("../../../docs/os/ui/vui09-known-limitations.md");
        assert!(limits.contains("not Visual v1 sign-off"));
        assert!(limits.contains("ADR-196"));
        assert!(limits.contains("ADR-197"));
        assert!(limits.contains("ADR-198"));
        assert!(limits.contains("ADR-199"));
        assert!(limits.contains("nested `row`"));
        assert!(limits.contains("ADR-200"));
        assert!(limits.contains("open_object"));
        assert!(limits.contains("ADR-201"));
        assert!(limits.contains("below Caption"));
        assert!(limits.contains("ADR-202"));
        assert!(limits.contains("EventRow"));
        assert!(limits.contains("ADR-203"));
        assert!(limits.contains("SpaceRow"));
        assert!(limits.contains("ADR-204"));
        assert!(limits.contains("SettingRow"));
        assert!(limits.contains("ADR-205"));
        assert!(limits.contains("WifiRow"));
        assert!(limits.contains("ADR-206"));
        assert!(limits.contains("BluetoothRow"));
        assert!(limits.contains("ADR-207"));
        assert!(limits.contains("TrustedClientRow"));
        assert!(limits.contains("ADR-208"));
        assert!(limits.contains("CapabilityRow"));
        assert!(limits.contains("ADR-209"));
        assert!(limits.contains("tap-unlock"));
        assert!(limits.contains("ADR-210"));
        assert!(limits.contains("ADR-211"));
        assert!(limits.contains("ADR-212"));
        assert!(limits.contains("Диагностика"));
        assert!(limits.contains("ADR-213"));
        assert!(limits.contains("thin tuning"));
        assert!(limits.contains("ADR-214"));
        assert!(limits.contains("list_back"));
        assert!(limits.contains("stacked_trailing_rect"));
        assert!(limits.contains("ADR-215"));
        assert!(limits.contains("scrolled_row_rect"));
        assert!(limits.contains("flatten_me_rows"));
        assert!(limits.contains("ADR-216"));
        assert!(limits.contains("compile_v2()"));
        assert!(limits.contains("ADR-217"));
        assert!(limits.contains("now.sui"));
        assert!(limits.contains("ADR-218"));
        assert!(limits.contains("ADR-219"));
        assert!(limits.contains("ADR-220"));
        assert!(limits.contains("ADR-221"));
        assert!(limits.contains("ADR-222"));
        assert!(limits.contains("saai-displayd"));
        assert!(limits.contains("cold boot"));
        assert!(limits.contains("SpaceDetail"));
        assert!(limits.contains("Visual v2"));
        let build = include_str!("../../../services/saai-shell/build.rs");
        assert!(build.contains("saai_ui_compiler::compile_v2"));
        assert!(!build.contains("saai_ui_compiler::compile("));
        assert!(build.contains("ADR-216"));
        assert!(ledger.contains("ADR-216"));
        assert!(ledger.contains("ADR-217"));
        assert!(ledger.contains("now.sui"));
        assert!(ledger.contains("ADR-218"));
        assert!(ledger.contains("ADR-219"));
        assert!(ledger.contains("layout_v2_scrolled"));
        assert!(ledger.contains("ADR-220"));
        assert!(ledger.contains("now_grid_rect"));
        assert!(ledger.contains("manage_app"));
        assert!(ledger.contains("ADR-221"));
        assert!(ledger.contains("consent:accept"));
        assert!(ledger.contains("ADR-222"));
        assert!(ledger.contains("Keyboard"));
    }
}
