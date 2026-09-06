use wayland_server::Display;

fn main() {
    let display: Display<()> = Display::new().expect("failed to create wayland display");
    let _handle = display.handle();
    println!("spike-minimal: wayland_server::Display created (raw wayland-server crate)");
}
