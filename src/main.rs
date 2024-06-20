mod player;

use player::Player;

use std::env;
use std::string::ToString;

use clap::Parser;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 6600;

fn main() {
    let args = Args::parse();
    #[cfg(debug_assertions)]
    println!("{args:?}");

    // get argument vars
    let host = args.host.map_or_else(
        || env::var("MPD_HOST").unwrap_or_else(
            |_| DEFAULT_HOST.to_string()
        ),
        |h| h,
    );
    let port = args.port.map_or_else(
        || env::var("MPD_PORT").map_or(
            DEFAULT_PORT,
            |p| p.parse()
            .expect("invalid value for port"),
        ),
        |p| p,
    );
    let address = format!("{host}:{port}");
    let format = if args.title {
        vec!["title".into()]
    } else if args.reverse {
        vec!["artist".into(), "album".into(), "title".into()]
    } else {
        args.format.unwrap_or_else(
            || vec!["title".into(), "artist".into(), "album".into()]
        )
    };

    Player::init(address, format, args.verbose, !args.no_ratings, args.easter);
}

/// Lightweight text-based MPD client
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
struct Args {

    /// Connect to server at address <HOST> (or $MPD_HOST)
    #[arg(short = 'H', long)]
    host: Option<String>,

    /// Connect to server on port <PORT> (or $MPD_PORT)
    #[arg(short = 'P', long)]
    port: Option<u16>,

    /// Comma-separated list of song metadata to display
    #[arg(short, long, value_delimiter = ',')]
    format: Option<Vec<String>>,

    /// Show redundant format fields
    #[arg(short, long)]
    verbose: bool,

    /// Hide ratings
    #[arg(short = 'R', long = "no-ratings")]
    no_ratings: bool,

    /// Equivalent to '--format title'
    #[arg(short, long)]
    title: bool,

    /// Equivalent to '--format artist,album,title'
    #[arg(short, long)]
    reverse: bool,

    /// Easter eggs?
    #[arg(long = "#$%!")]
    easter: bool,
}
