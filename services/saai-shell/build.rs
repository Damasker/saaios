use std::{env, fs, path::PathBuf, process::Command};

/// S14: no build-number/version concept existed anywhere in the project
/// before this -- `system_identity()` (crates/system-tools) reports
/// device/kernel facts but nothing about which commit is actually
/// running. Short git hash, captured at compile time; `"unknown"` if
/// this isn't a git checkout (e.g. a source tarball) rather than
/// failing the build over a cosmetic string.
fn build_id() -> String {
    Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|hash| hash.trim().to_string())
        .filter(|hash| !hash.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn main() {
    println!("cargo:rustc-env=SAAIOS_BUILD_ID={}", build_id());
    // `.git/HEAD` itself only changes on checkout (branch switch) --
    // every ordinary commit instead appends to `.git/logs/HEAD` (the
    // reflog), regardless of which branch it's on, so that's the file
    // that actually needs watching for this to stay correct across a
    // plain `git commit`.
    println!("cargo:rerun-if-changed=../../.git/logs/HEAD");
    let source_path = "ui/root.sui";
    println!("cargo:rerun-if-changed={source_path}");
    let source = fs::read_to_string(source_path).expect("read root.sui");
    // ADR-216: production chrome is compile_v2(). v1 stays frozen in
    // compile_v1_rollback(). Labels/icons are the proven v1 chrome.
    let screen = saai_ui_compiler::compile_v2(&source).expect("compile root.sui v2");
    let nav = screen
        .components
        .iter()
        .find(|component| component.type_name == "BottomNavigation")
        .expect("root.sui v2 BottomNavigation");
    let tabs = nav
        .tabs
        .iter()
        .map(|tab| {
            let (label, icon, action) = match tab.id.as_str() {
                "now" => ("Сейчас", "now", "select_root:now"),
                "spaces" => ("Пространства", "spaces", "select_root:spaces"),
                "search" => ("Поиск", "search", "select_root:search"),
                "me" => ("Система", "person", "select_root:me"),
                other => panic!("unknown root tab `{other}`"),
            };
            format!(
                "TabDefinition {{ id: {:?}, label: {:?}, icon: {:?}, action: {:?} }}",
                tab.id, label, icon, action
            )
        })
        .collect::<Vec<_>>()
        .join(",\n    ");
    let generated = format!(
        "const ROOT_SCREEN_ID: &str = {:?};\n\
         const ROOT_CONTENT_ID: &str = {:?};\n\
         const ROOT_CONTENT_ACTIONS: &[ContentActionDefinition] = &[];\n\
         const ROOT_TABS_ID: &str = {:?};\n\
         const ROOT_TAB_HEIGHT: u32 = {};\n\
         const ROOT_TABS: &[TabDefinition] = &[\n    {}\n];\n",
        screen.id,
        format!("{}-content", screen.id),
        "BottomNavigation",
        300u32,
        tabs
    );
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(output.join("root_sui.rs"), generated).expect("write generated root SUI");
}
