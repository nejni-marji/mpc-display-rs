use mpd::Client;
use mpc_display_rs::music::Player;

fn main() {
    let conn = Client::connect("127.0.0.1:6600").expect("should have client");

    let mut player = Player::new(conn);
    player.init();
    player.display();
}
