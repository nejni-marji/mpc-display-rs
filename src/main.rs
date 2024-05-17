/*
use textwrap;
fn main() {

    let s1 = "\x1b[41mあいうえおあいうえおあいうえお1234567890";
    let s1 = textwrap::fill(s1, 7);
    println!("{s1}");

    println!("---");

    let s2 = "あいうえおあいうえおあいうえお1234567890";
    let s2 = textwrap::fill(s2, 7);
    println!("{s2}");

    println!("---");

    let s3 = "1234567\x1b[41mああああああああああ";
    let s3 = textwrap::fill(s3, 7);
    println!("{s3}");

    println!("---");

    let s4 = "1234567ああああああああああ";
    let s4 = textwrap::fill(s4, 7);
    println!("{s4}");

}
*/


use mpd::Client;
use mpc_display_rs::music::Player;

fn main() {
    let conn = Client::connect("127.0.0.1:6600").expect("should get client");

    let mut player = Player::new(conn);
    player.init();
    player.display();

    // let mut counter_plist = 0;
    // let plist: Vec<(u32, String)> = conn.queue().unwrap().into_iter().map(
    //     |i| {
    //         counter_plist += 1;
    //         (counter_plist, i.title.unwrap())
    //     })
    // .collect();




    // println!("{plist:?}");

}

