mod display;
mod input;

pub use display::options::MusicOpts;

use display::Display;
use input::KeyHandler;

use std::thread;

use mpd::Client;
use uuid::Uuid;

pub struct Player;

impl Player {
    pub fn init(address: &str, format: Vec<String>, options: MusicOpts)
    {
        // generate UUID for proper quit handling
        let uuid = Uuid::new_v4();

        // initialize display
        let Ok(display_client) = Client::connect(address) else {
            return Self::error(address)
        };
        let mut display = Display::new(display_client, format,
            uuid, options);
        let display = thread::spawn(move || { display.init() });

        // initialize input
        let Ok(input_client) = Client::connect(address) else {
            return Self::error(address)
        };
        let input = KeyHandler::new(input_client, uuid);
        let input = thread::spawn(move || { input.init() });

        // join threads and check for panics
        let _ = display.join();
        let _ = input.join();
    }

    fn error(address: &str) {
        println!("Cannot connect to server: {address}");
    }
}
