#[cfg(not(target_arch = "wasm32"))]
fn main() {
    bbb_client::run_desktop_main();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
