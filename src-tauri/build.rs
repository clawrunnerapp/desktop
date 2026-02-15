fn main() {
    // Tauri's resource glob overflows the default 8 MB stack when processing
    // ~136k files in resources/openclaw/node_modules. Run in a thread with
    // a larger stack to avoid SIGABRT during build.
    let handler = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024) // 64 MB
        .spawn(|| tauri_build::build())
        .expect("failed to spawn build thread");

    handler.join().expect("build thread panicked");
}
