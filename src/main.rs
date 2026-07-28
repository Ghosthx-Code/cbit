pub mod args;
pub mod lua;

fn main() {
    let config = lua::load_cbit_config().unwrap_or_default();
    if let Err(e) = args::parse_with_config(config) {
        eprintln!("\x1b[1;31m{:>12}\x1b[0m {}", "Error", e);
        std::process::exit(1);
    }
}
