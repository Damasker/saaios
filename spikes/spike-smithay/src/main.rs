use smithay::reexports::wayland_server::Display;

fn main() {
    let display: Display<()> = Display::new().expect("failed to create wayland display");
    let _handle = display.handle();
    println!("spike-smithay: wayland_server::Display created (via smithay re-export)");
}
