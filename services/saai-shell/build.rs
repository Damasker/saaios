use std::{env, fs, path::PathBuf};

fn main() {
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
    let generated = format!(
        "const ROOT_SCREEN_ID: &str = {:?};\n\
         const ROOT_CONTENT_ID: &str = {:?};\n\
         const ROOT_TABS_ID: &str = {:?};\n\
         const ROOT_TAB_HEIGHT: u32 = {};\n\
         const ROOT_TABS: &[TabDefinition] = &[\n    {}\n];\n",
        screen.id, screen.content_id, screen.tabs_id, screen.tab_height, tabs
    );
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(output.join("root_sui.rs"), generated).expect("write generated root SUI");
}
