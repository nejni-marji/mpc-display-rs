use mpd::Client;
use mpc_display_rs::music::Player;

fn main() {
    let conn = Client::connect("127.0.0.1:6600").expect("should get client");
    let mut player = Player::new(conn);

    println!("[startup]");
    loop {
        println!("{}", player.data);
        player.data.idle();
    }
}
