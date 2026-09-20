//! ADR-183: `.sui` v1 rollback stays a frozen `compile()` artifact.
//! ADR-216: production `root.sui` is `compile_v2()`.

use crate::{compile, CompileError, ScreenSpec};

/// Frozen v1 chrome. Production `root.sui` is sui 2 (ADR-216).
pub const V1_ROLLBACK_SOURCE: &str = r#"sui 1

screen root {
  column {
    content id=content fill
    tabs id=root-tabs height=300 {
      tab now label="Сейчас" icon=now action=select_root:now
      tab inbox label="Входящие" icon=inbox action=select_root:inbox
      tab spaces label="Пространства" icon=spaces action=select_root:spaces
      tab me label="Система" icon=person action=select_root:me
    }
  }
}
"#;

pub fn compile_v1_rollback() -> Result<ScreenSpec, CompileError> {
    compile(V1_ROLLBACK_SOURCE)
}

#[cfg(test)]
mod tests {
    use super::{compile_v1_rollback, V1_ROLLBACK_SOURCE};
    use crate::compile_v2;

    #[test]
    fn v1_rollback_keeps_the_four_root_tabs() {
        let screen = compile_v1_rollback().expect("frozen v1");
        assert_eq!(screen.id, "root");
        let labels: Vec<&str> = screen.tabs.iter().map(|tab| tab.label.as_str()).collect();
        assert_eq!(labels, ["Сейчас", "Входящие", "Пространства", "Система"]);
        assert!(screen.content_actions.is_empty());
        assert!(!V1_ROLLBACK_SOURCE.contains("inspect_selected_entity"));
        assert!(!V1_ROLLBACK_SOURCE.contains("open_intent_input"));
    }

    #[test]
    fn v1_rollback_source_is_not_sui_2() {
        let error = compile_v2(V1_ROLLBACK_SOURCE).unwrap_err();
        assert!(error.to_string().contains("compile_v2 expected version 2"));
    }

    #[test]
    fn shell_build_script_compiles_v2() {
        let build = include_str!("../../../services/saai-shell/build.rs");
        assert!(build.contains("saai_ui_compiler::compile_v2"));
        assert!(!build.contains("saai_ui_compiler::compile("));
        assert!(build.contains("ADR-216"));
    }
}
