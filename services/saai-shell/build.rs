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
    let screen = saai_ui_compiler::compile(&source).expect("compile root.sui");
    let tabs = screen
        .tabs
        .iter()
        .map(|tab| {
            format!(
                "TabDefinition {{ id: {:?}, label: {:?}, icon: {:?}, action: {:?} }}",
                tab.id, tab.label, tab.icon, tab.action
            )
        })
        .collect::<Vec<_>>()
        .join(",\n    ");
    let content_actions = screen
        .content_actions
        .iter()
        .map(|action| {
            format!(
                "ContentActionDefinition {{ id: {:?}, page: {:?}, top: {}, height: {}, label: {:?}, action: {:?} }}",
                action.id, action.page, action.top, action.height, action.label, action.action
            )
        })
        .collect::<Vec<_>>()
        .join(",\n    ");
    let generated = format!(
        "const ROOT_SCREEN_ID: &str = {:?};\n\
         const ROOT_CONTENT_ID: &str = {:?};\n\
         const ROOT_CONTENT_ACTIONS: &[ContentActionDefinition] = &[\n    {}\n];\n\
         const ROOT_TABS_ID: &str = {:?};\n\
         const ROOT_TAB_HEIGHT: u32 = {};\n\
         const ROOT_TABS: &[TabDefinition] = &[\n    {}\n];\n",
        screen.id, screen.content_id, content_actions, screen.tabs_id, screen.tab_height, tabs
    );
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(output.join("root_sui.rs"), generated).expect("write generated root SUI");
}
