use std::io;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};
use mpd::{
    Client,
    message::Channel,
    State,
};
use uuid::Uuid;

#[allow(unused_imports)]
use debug_print::{
    debug_print as dprint,
    debug_println as dprintln,
    debug_eprint as deprint,
    debug_eprintln as deprintln,
};

pub struct KeyHandler {
    client: Arc<Mutex<Client>>,
    uuid: Uuid,
}

impl KeyHandler {
    #[must_use] pub fn new(client: Client, uuid: Uuid) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            uuid,
        }
    }

    pub fn init(&self) {
        // create keepalive thread
        let client = Arc::clone(&self.client);
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(60));
                client
                    .lock()
                    .expect("can't get command connection")
                    .status()
                    .expect("failed keepalive!");
                }
        });

        loop {
            let ch = getch().unwrap_or_default();
            // returns "quit"
            let mut conn = self.client.lock().expect("can't get command connection");
            if self.handle_key(ch, &mut conn) {
                break
            }
        }
    }

    // huge match statement to handle keyboard input. returns "quit" param.
    fn handle_key(&self, ch: char, conn: &mut Client) -> bool {
        match ch {
            // helptext
            'h' | '?' | '/' => {
                return self.handle_help(conn);
            }
            // quit
            'q' | 'Q' => {

                let _ = conn.unsubscribe(
                    Channel::new(format!("help_{}",
                        self.uuid.simple()).as_str()
                    ).expect("can't make help channel"));


                dprintln!("input: quitting!");
                let _ = conn.subscribe(
                    Channel::new(format!("quit_{}",
                        self.uuid.simple()).as_str()
                    ).expect("can't make quit channel")
                );
                return true;
            }

            // space for pause/play
            ' ' => {
                let state = conn.status().unwrap_or_default().state;
                match state {
                    State::Play => {
                        let _ = conn.pause(true);
                    }
                    State::Pause | State::Stop => {
                        let _ = conn.play();
                    }
                }
            }

            // prev
            'p' | 'k' => { let _ = conn.prev(); }
            // next
            'n' | 'j' => { let _ = conn.next(); }
            // volume up
            '=' | '+' | '0' | ')' => {
                let vol = conn.status().unwrap_or_default().volume;
                let vol = std::cmp::min(100, vol+5);
                let _ = conn.volume(vol);
            }
            // volume down
            '-' | '_' | '9' | '(' => {
                let vol = conn.status().unwrap_or_default().volume;
                // volume is i8, so you can do this
                let vol = std::cmp::max(0, vol-5);
                let _ = conn.volume(vol);
            }

            // seek backwards
            'H' => {
                let time = conn.status().unwrap_or_default()
                    .elapsed.unwrap_or_default();
                let time = if time.as_secs() <= 10 {
                    Duration::from_secs(0)
                } else {
                    time - Duration::from_secs(10)
                };
                let _ = conn.rewind(time);
            }
            // seek forwards
            'L' => {
                let time = conn.status().unwrap_or_default()
                    .elapsed.unwrap_or_default();
                let time = time + Duration::from_secs(10);
                let _ = conn.rewind(time);
            }

            // ratings
            '{' => {
                Self::inc_rating(-1, conn);
                }
            '}' => {
                Self::inc_rating(1, conn);
            }

            // repeat
            'E' => {
                let state = conn.status().unwrap_or_default().repeat;
                let _ = conn.repeat(!state);
            }
            // random
            'R' => {
                let state = conn.status().unwrap_or_default().random;
                let _ = conn.random(!state);
            }
            // single
            'S' => {
                let state = conn.status().unwrap_or_default().single;
                let _ = conn.single(!state);
            }
            // consume
            'C' => {
                let state = conn.status().unwrap_or_default().consume;
                let _ = conn.consume(!state);
            }

            // shuffle
            'F' => { let _ = conn.shuffle(..); }

            // crossfade up
            'x' => {
                let crossfade = conn.status().unwrap_or_default()
                    .crossfade.unwrap_or_default();
                let crossfade = crossfade + Duration::from_secs(1);
                let _ = conn.crossfade(crossfade);
            }

            // crossfade down
            'X' => {
                let crossfade = conn.status().unwrap_or_default()
                    .crossfade.unwrap_or_default();
                if crossfade.as_secs() != 0 {
                    let crossfade = crossfade - Duration::from_secs(1);
                    let _ = conn.crossfade(crossfade);
                }
            }

            // stop
            'M' => { let _ = conn.stop(); }

            // default
            _ => {
                #[cfg(debug_assertions)]
                println!("getch(): {ch}");
            }
        }
        false
    }

    fn handle_help(&self, conn: &mut Client) -> bool {
        // make help channel
        let help_chan = Channel::new(format!("help_{}",
            self.uuid.simple()).as_str()
        ).expect("can't make help channel");

        // if help is in channels, unsubscribe
        if conn.channels().unwrap_or_default().contains(&help_chan) {
            dprintln!("input: -help_chan");
            let _ = conn.unsubscribe(help_chan);

            // toggle temp channel to force idle break
            let _ = conn.subscribe(
                Channel::new("tmp").expect("can't make temp channel")
            );
            let _ = conn.unsubscribe(
                Channel::new("tmp").expect("can't make temp channel")
            );

        // otherwise, subscribe to help channel and make a fake getch() loop
        } else {
            dprintln!("input: +help_chan");
            let _ = conn.subscribe(help_chan);

            // use our own getch() loop, to prevent sending commands to the server during helptext
            loop {
                let ch = getch().unwrap_or_default();
                // allow esc to close helptext by remapping it
                let ch = if ch == '\x1b' { '?' } else { ch };
                match ch {
                    // allowed inputs during helptext: esc, help, quit
                    'h' | '?' | '/' | 'q' | 'Q' => {
                        return self.handle_key(ch, conn);
                    },
                    _ => {}
                }
            }
        }

        false
    }

    fn inc_rating(inc: i8, conn: &mut Client) {
        let song = conn.currentsong()
            .unwrap_or_default().unwrap_or_default();
        let rating: i8 = conn.sticker("song", &song.file, "rating")
            .ok().map_or(
                -1,
                |r| r.parse().unwrap_or(-1)
            );

        let rating = (rating + inc).clamp(-1, 10);

        if rating == -1 {
            let _ = conn.delete_sticker(
                "song", &song.file, "rating");
        } else {
            let _ = conn.set_sticker(
                "song", &song.file, "rating", &rating.to_string());
        }
    }
}

fn getch() -> Result<char, io::Error> {
    let stdin = 0;
    let backup_termios = Termios::from_fd(stdin).expect("can't get file descriptor");

    // call this as a function so that we can always reset termios
    let ch = getch_raw();

    // reset the stdin to original termios data
    tcsetattr(stdin, TCSANOW, & backup_termios).expect("can't set terminal attributes");
    ch
}

fn getch_raw() -> Result<char, io::Error> {
    let stdin = 0;
    let mut termios = Termios::from_fd(stdin).expect("can't get file descriptor");
    // no echo and canonical mode
    termios.c_lflag &= !(ICANON | ECHO);
    tcsetattr(stdin, TCSANOW, &termios)?;

    let stdout = io::stdout();
    let mut reader = io::stdin();

    // read exactly one byte
    let mut buffer = [0;1];
    stdout.lock().flush()?;
    reader.read_exact(&mut buffer)?;

    Ok(buffer[0].into())
}
