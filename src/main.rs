use std::string::ToString;
use std::thread;
use clap::Parser;
use mpc_display_rs::music::Player;
use mpc_display_rs::input::KeyHandler;
use uuid::Uuid;

fn main() {
    let args = Args::parse();
    #[cfg(debug_assertions)]
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

    // generate UUID for proper quit handling
    let uuid = Uuid::new_v4();

    // run player
    let mut player = Player::new(address.clone(), format, uuid);
    player.init();
    thread::spawn(move || { player.display(); });

    // initialize input
    let mut parser = KeyHandler::new(address, uuid);
    parser.init();
    println!("\x1b[?25h");
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
