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
    let format: Vec<_> = if args.title {
        vec!["title".to_string()]
    } else {
        args.format
        .as_str()
        .split(',')
        .map(ToString::to_string)
        .collect()
    };

    Player::init(address, format, args.verbose);
}

/// Displays the current state of an MPD server.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {

    /// Connect to server at address <HOST>
    #[arg(short = 'H', long)]
    host: Option<String>,

    /// Connect to server on port <PORT>
    #[arg(short = 'P', long)]
    port: Option<u16>,

    /// Comma-separated list of song metadata to display
    // TODO: is there a way to use comma-separated lists with derive?
    #[arg(short, long,
          default_value_t = String::from("title,artist,album"))]
    format: String,

    /// Equivalent to '--format title'
    #[arg(short, long)]
    title: bool,

    /// Show redundant format fields
    #[arg(short, long)]
    verbose: bool,
}
