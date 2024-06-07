mod display;
mod input;

use display::Display;
use input::KeyHandler;

use std::thread;
use uuid::Uuid;

pub struct Player;

impl Player {
    pub fn new(address: String, format: Vec<String>) {
        // generate UUID for proper quit handling
        let uuid = Uuid::new_v4();

        // run player
        let mut player = Display::new(address.clone(), format, uuid);
        thread::spawn(move || { player.init(); });

        // initialize input
        let mut parser = KeyHandler::new(address, uuid);
        parser.init();
        print!("\x1b[?25h");
    }
}
