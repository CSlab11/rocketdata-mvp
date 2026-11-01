use clap::Parser;

/// Classifica i paragrafi dei file Markdown con regole o con un LLM locale.
#[derive(Parser)]
#[command(version)]
struct Args {}

fn main() {
    Args::parse();
}
