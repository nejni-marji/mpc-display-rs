mod display;
mod input;

use display::Display;
use input::KeyHandler;

use std::thread;
use std::io;
use std::io::Write;

use mpd::Client;
use uuid::Uuid;

pub struct Player;

impl Player {
    pub fn init(address: String, format: Vec<String>) {
        // generate UUID for proper quit handling
        let uuid = Uuid::new_v4();

        // attempt to connect clients. all failure happens within this function.
        Self::connect(address, format, uuid);

        // reset terminal before exit
        print!("\x1b[?25h\x1b[2J");
        io::stdout().flush().expect("unable to flush buffer");
    }

    fn connect(address: String, format: Vec<String>, uuid: Uuid) {
        // run player
        let player_client = Client::connect(address.clone())
            .unwrap();
        let mut player = Display::new(player_client, format, uuid);
        thread::spawn(move || { player.init(); });

        // initialize input
        let parser_client = Client::connect(address)
            .unwrap();
        let mut parser = KeyHandler::new(parser_client, uuid);
        parser.init();
    }
}
