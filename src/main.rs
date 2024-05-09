use std::sync::Mutex;
use mpd::Client;
use mpc_display_rs::music::DataCache;

fn main() {
    let conn = Client::connect("127.0.0.1:6600").expect("should get client");
    let mut data = DataCache::new(Mutex::new(conn));

    println!("[startup]");
    loop {
        println!("{data}");
        data.idle();
    }
}
