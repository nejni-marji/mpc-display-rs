mod display;
mod input;

use display::Display;
use input::KeyHandler;

use std::io;
use std::io::Write;
use std::thread;
use std::time::Duration;

use mpd::Client;
use uuid::Uuid;

pub struct Player;

const TIMEOUT_COUNT: i32 = 10;
const TIMEOUT_DELAY: u64 = 1;

impl Player {
    pub fn init(address: String, format: Vec<String>) {
        // generate UUID for proper quit handling
        let uuid = Uuid::new_v4();

        // loop around connect()
        for i in 0..TIMEOUT_COUNT {
            let result = Self::connect(
                address.clone(), format.clone(), uuid.clone());
            if let Ok(_) = result {
                // reset terminal before exit
                print!("\x1b[?25h\x1b[2J");
                io::stdout().flush().expect("unable to flush buffer");

                // exit loop
                break
            }
            println!("connection failed {i}/{TIMEOUT_COUNT}");
            thread::sleep(Duration::from_secs(TIMEOUT_DELAY));
        }
    }

    // all failure gets passed up to this function
    fn connect(address: String, format: Vec<String>, uuid: Uuid) -> Result<(), mpd::error::Error> {
        // run player
        let player_client = Client::connect(&address)?;
        let mut player = Display::new(player_client, format, uuid);
        let thr = thread::spawn(move || { player.init(); });

        // initialize input
        let parser_client = Client::connect(address)?;
        let mut parser = KeyHandler::new(parser_client, uuid);
        let par = parser.init();

        // join display thread before exit
        let dsp = thr.join().unwrap();

        Ok(())
    }
}
