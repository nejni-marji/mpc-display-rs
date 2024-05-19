use std::string::ToString;
use clap::Parser;
use mpd::Client;
use mpc_display_rs::music::Player;

fn main() {
    let args = Args::parse();
    println!("{args:?}");

    // get argument vars
    let address = format!("{}:{}", args.host, args.port);
    let format: Vec<_> = if args.title {
        vec!["title".to_string()]
    } else {
        args.format
        .as_str()
        .split(',')
        .map(ToString::to_string)
        .collect()
    };

    // make player object
    let conn = Client::connect(address).expect("should have client");
    let mut player = Player::new(conn, format);
    player.init();
    player.display();
}

/// Displays the current state of an MPD server.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {

    /// Connect to server at address <HOST>
    #[arg(short = 'H', long,
          default_value_t = String::from("127.0.0.1"))]
    host: String,

    /// Connect to server on port <PORT>
    #[arg(short = 'P', long,
          default_value_t = 6600)]
    port: u16,

    /// Comma-separated list of song metadata to display
    // TODO: is there a way to use comma-separated lists with derive?
    #[arg(short, long,
          default_value_t = String::from("title,artist,album"))]
    format: String,

    /// Equivalent to '--format title'
    #[arg(short, long)]
    title: bool,
}
