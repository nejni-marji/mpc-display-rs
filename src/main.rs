use std::sync::Mutex;
use mpd::Client;
use mpc_display_rs::music::DataCache;
// use std::net::TcpStream;

fn main() {
    let mut conn = Client::connect("127.0.0.1:6600").unwrap();

    let data = DataCache::new(
        conn.status().unwrap(),
        conn.currentsong().unwrap().unwrap(),
        Mutex::new(conn),
        );

    println!("---\n{}\n---", data);
    println!("---\n{:?}\n---", data);
}
