fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(message) = scratchcad_occt_rhai::cli::run(&args, &mut std::io::stdout()) {
        eprintln!("error: {message}");
        std::process::exit(1);
    }
}
