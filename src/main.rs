fn main() {
    let platform = snake::platform::Platform::detect();
    let wsl_note = if platform.is_wsl() {
        " (WSL detected)"
    } else {
        ""
    };

    println!("snake scaffold initialized{wsl_note}");
}
