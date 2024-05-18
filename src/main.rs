use mpd::Client;
#[allow(unused_imports)]
use textwrap;
#[allow(unused_imports)]
use mpc_display_rs::music::{DataCache, Player};

fn main() {
    let conn = Client::connect("127.0.0.1:6600").expect("should get client");

    let mut player = Player::new(conn);
    player.init();
    player.display();


    /*
    #[allow(unused_variables)]
    let mut conn = conn;
    let mut counter_plist = 0;
    #[allow(unused_variables)]
    let plist: Vec<(u32, String)> = conn.queue().unwrap_or_default().iter().map(
        |i| {
            counter_plist += 1;
            (counter_plist, i.title.clone().unwrap())
        })
    .collect();
    println!("{plist:?}");
    */

    // let song = conn.currentsong().unwrap().unwrap();
    // println!("{song:?}");
    // let album = DataCache::get_song_tag(
    //     song, "album")
    //     .unwrap_or("???".to_string());
    // println!("album: {album}");
}
