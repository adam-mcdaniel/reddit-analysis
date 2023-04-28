use reddit_analyzer::*;
use clap::Parser;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
   /// Name of subreddits to collect
   subreddits: Vec<String>,
}

fn main() -> Result<(), ScrapeError> {
    let args = Args::parse();
    Vec::scrape(&args.subreddits)?;
    Ok(())
}
