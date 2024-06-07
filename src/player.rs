mod display;
mod input;

use display::Display;
use input::KeyHandler;

use std::io;
use std::io::Write;
use std::thread;

use mpd::Client;
use uuid::Uuid;

pub struct Player;

impl Player {
    pub fn init(address: String, format: Vec<String>) {
        // generate UUID for proper quit handling
        let uuid = Uuid::new_v4();

        // initialize display
        let display_client = Client::connect(&address)
            .expect("can't connect to client");
        let mut display = Display::new(display_client, format, uuid);
        let t = thread::spawn(move || { display.init() });

        // initialize input
        let parser_client = Client::connect(address)
            .expect("can't connect to client");
        let mut parser = KeyHandler::new(parser_client, uuid);
        parser.init();

        // join display thread before exit
        let _ = t.join();

        // reset terminal before exit
        print!("\x1b[?25h\x1b[2J");
        io::stdout().flush().expect("unable to flush buffer");
    }
}
