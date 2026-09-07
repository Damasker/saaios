fn main() {
    // Some C-library bindings in this dependency tree (e.g. the xkbcommon
    // crate) link via a bare `#[link(name = "...")]` with no build.rs of
    // their own, rather than going through pkg-config -- on a normal host
    // build the system linker's default search paths cover this, but our
    // cross sysroot (ADR-008, os/targets/panther/build-cross-sysroot.sh)
    // isn't one of those, so the static .a files there are otherwise
    // invisible to the linker. Add it explicitly.
    #[cfg(feature = "panther-hardware")]
    {
        if let Ok(sysroot) = std::env::var("PKG_CONFIG_SYSROOT_DIR") {
            println!("cargo:rustc-link-search=native={sysroot}/usr/local/lib");
        }
    }
}
