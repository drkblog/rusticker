use clap::Parser;

/// Rusticker CLI application
#[derive(Parser, Debug)]
#[command(
    name = "rusticker",
    version,
    disable_version_flag = true,
    about = "A Rust command-line application that demonstrates argument parsing with clap",
    long_about = None
)]
struct Args {
    /// Print version information
    #[arg(short = 'V', long = "version", action = clap::ArgAction::Version)]
    version: Option<bool>,

    /// Name of the person to greet
    #[arg(short, long)]
    name: Option<String>,
}

fn main() {
    let args = Args::parse();

    if let Some(ref name) = args.name {
        println!("Hello, {}!", name);
    } else {
        println!("Hello, World!");
    }
}
