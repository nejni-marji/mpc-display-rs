#[allow(clippy::missing_panics_doc)]
pub mod music {
    use std::borrow::Cow::Borrowed;
    
    use std::fmt;
    use std::sync::{
        mpsc,
        Mutex,
    };
    use std::thread;
    use std::time::Duration;
    use mpd::{
        Client,
        Idle,
        Query,
        search::Window,
        Song,
        song::QueuePlace,
        State,
        Subsystem,
        Term,
    };
    use terminal_size::terminal_size;
    use textwrap;

    const HEADER_HEIGHT: u32 = 4;

    #[allow(unused_imports)]
    use debug_print::{
        debug_print as dprint,
        debug_println as dprintln,
        debug_eprint as deprint,
        debug_eprintln as deprintln,
    };

    #[derive(Debug,Default)]
    pub struct Player {
        // TODO: does this need to be a mutex?
        client: Mutex<Client>,
        pub data: DataCache,
        // quit: bool,
    }

    #[derive(Debug,Default,Clone)]
    // TODO: does not need to be public
    pub struct DataCache {
        // everything that can potentially be missing is an Option type.
        // the exception to this is queue_total, which theoretically would be 0
        // when there is no value, but i've chosen to force it into an Option
        // anyway, for consistency and because it makes things cooler later.
        song: Song,
        queue: Vec<Song>,
        artist: Option<String>,
        title: Option<String>,
        album: Option<String>,
        album_track: Option<u32>,
        album_total: Option<u32>,
        queue_track: Option<QueuePlace>,
        queue_total: Option<u32>,
        time_curr: Option<Duration>,
        time_total: Option<Duration>,
        state: State,
        volume: i8,
        ersc_opts: Vec<bool>,
        crossfade: Option<Duration>,
    }

    pub struct Playlist {
    }

    // TODO: once playlist display is implementented, you should cache the playlist string in the Player and only actually draw it when DataCache.update_playlist() is called!
    impl Player {
        #[must_use] pub fn new(client: Client) -> Self {
            Self {
                client: Mutex::new(client),
                data: DataCache::new(),
                // quit: false,
            }
        }

        pub fn init(&mut self) {
            let data = &mut self.data;
            data.update_status(&self.client);
            data.update_song(&self.client);
            data.update_playlist(&self.client);
        }

        pub fn display(&mut self) {
            dprintln!("[startup]");
            println!("{}", self.data);

            #[cfg(debug_assertions)]
            let mut counter_idle = 0;
            loop {
                // prepare channel
                // TODO: this doesn't need to be a boolean. we can just check if try_recv() is Ok or not.
                let (tx, rx) = mpsc::channel::<bool>();

                // spawn thread if we need it
                if self.data.state == State::Play {
                    // clone data for thread
                    // TODO: replace clone with something that nulls the playlist
                    let data = self.data.clone();

                    // assign thread handle to external variable
                    _ = thread::spawn(move || {
                        Self::delay_thread(rx, data);
                    });
                }

                // wait for idle, then print
                self.idle();
                #[cfg(debug_assertions)] {
                    counter_idle += 1;
                }
                dprintln!("[idle: {counter_idle}]");
                println!("{}", self.data);

                // send signal to kill thread
                let _ = tx.send(true);
            }
        }

        fn delay_thread(rx: mpsc::Receiver<bool>, mut data: DataCache) {
            const DELAY: u64 = 1;
            #[cfg(debug_assertions)]
            let mut counter_delay = 0;
            loop {
                // sleep before doing anything
                thread::sleep(Duration::from_secs(DELAY));
                let signal = rx.try_recv().unwrap_or(false);

                // check quit signal, otherwise continue
                if signal {
                    dprintln!("[duration: break!]");
                    break
                }

                // increment time only when we print,
                // otherwise it can break things.
                data.increment_time(DELAY);
                #[cfg(debug_assertions)] {
                    counter_delay += 1;
                }
                dprintln!("[duration: {counter_delay}]");
                println!("{data}");

                // TODO: is this necessary?
                // thread::sleep(Duration::from_millis(100));
            }
        }

        fn idle(&mut self) {
            // use client to idle. no early drop
            let mut conn = self.client.lock()
                .expect("should have client");
            let subsystems = conn.wait(&[
                Subsystem::Player, Subsystem::Mixer,
                Subsystem::Options, Subsystem::Playlist,
            ]).unwrap_or_default();
            drop(conn);

            dprintln!("[subsystems: {subsystems:?}]");
            for i in subsystems {
                let data = &mut self.data;
                match i {
                    Subsystem::Player => {
                        data.update_status(&self.client);
                        data.update_song(&self.client);
                    }
                    Subsystem::Mixer | Subsystem::Options => {
                        data.update_status(&self.client);
                    }
                    Subsystem::Playlist => {
                        data.update_playlist(&self.client);
                    }
                    _ => {}
                }
            }
        }
    }

    impl DataCache {
        #[must_use] pub fn new() -> Self {
            Self::default()
        }

        fn update_status(&mut self, client: &Mutex<Client>) {
            // use client to get some data
            let mut conn = client.lock()
                .expect("should have client");
            let status = conn.status()
                .unwrap_or_default();
            drop(conn);

            // modify data
            self.queue_track = status.song;
            self.queue_total = match status.queue_len {
                0 => None,
                s => Some(s),
            };
            self.time_curr = status.elapsed;
            self.time_total = status.duration;
            self.state = status.state;
            self.volume = status.volume;
            self.ersc_opts = vec![
                status.repeat, status.random,
                status.single, status.consume];
            self.crossfade = status.crossfade;
        }

        fn update_song(&mut self, client: &Mutex<Client>) {
            // use client to get some data
            let mut conn = client.lock()
                .expect("should have client");
            let song = conn.currentsong()
                .unwrap_or_default().unwrap_or_default();
            drop(conn);

            // update song
            self.song = song.clone();

            // try to get album
            let album = Self::get_metadata(&song, "album");

            // try to get album progress
            let album_progress = Self::get_album_nums(client, album.clone(), song.clone());
            let (album_track, album_total) = match album_progress {
                Some(s) => (Some(s.0), Some(s.1)),
                None => (None, None),
            };

            // mutate data
            self.artist = song.artist;
            self.title = song.title;
            self.album = album;
            self.album_track = album_track;
            self.album_total = album_total;
        }

        fn update_playlist(&mut self, client: &Mutex<Client>) {
            // use client to get some data
            let mut conn = client.lock()
                .expect("should have client");
            let queue = conn.queue()
                .unwrap_or_default();
            drop(conn);

            // TODO: how to .enumerate() this into Vec<(u32, Song)>?
            self.queue = queue;
        }

        fn increment_time(&mut self, n: u64) {
            self.time_curr = self.time_curr.map(|t| t + Duration::from_secs(n));
        }

        // TODO: optimize this by caching the result on a per-album basis
        fn get_album_nums(client: &Mutex<Client>, album: Option<String>, song: Song) -> Option<(u32, u32)> {
            // build query
            let mut query = Query::new();
            query.and(Term::Tag(Borrowed("Album")), album?);
            let window = Window::from((0,u32::from(u16::MAX))); // TODO: make const?
            // lock client and search
            let mut conn = client.lock()
                .expect("should have client");
            let search = conn.search(&query, window);
            drop(conn);
            // parse search
            match search {
                Err(_) => { None },
                Ok(search) => {
                    // dprintln!("{search:?}");
                    let mut track = None;
                    for (k, v) in song.tags {
                        if k == "Track" {
                            track = Some(v);
                        }
                    }
                    // return numeric value
                    match track?.parse() {
                        Err(_) => { None },
                        Ok(track) =>
                        {
                            Some((track, u32::try_from(search.len()).expect("should be able to cast search")))
                        }
                    }
                }
            }
        }

        fn get_pretty_time(dur: Option<Duration>) -> Option<String> {
            let n = dur?.as_secs();
            let (min, sec) = (n / 60, n % 60);
            Some(format!("{min}:{sec:0>2}"))
        }

        fn get_ersc(&self) -> String {
            let mut ersc = String::new();
            let base = ['e', 'r', 's', 'c'];
            let ersc_opts = self.ersc_opts
                .clone();
            for (i, v) in base.iter().enumerate() {
                ersc.push(
                    // this unwrap_or is... middling at best, i think
                    // TODO: move this unwrap into display?
                    if *ersc_opts.get(i).unwrap_or(&false) {
                        v.to_ascii_uppercase()
                    } else {
                        *v
                    }
                );
            }
            ersc
        }

        // TODO: should not be public
        #[must_use] pub fn get_metadata(song: &Song, tag: &str) -> Option<String> {
            let mut value = None;
            for (k, v) in &song.tags {
                if k.to_ascii_lowercase() == tag.to_ascii_lowercase() {
                    value = Some(v);
                }
            }
            // TODO: why cloned?
            value.cloned()
        }

        fn print_header(&self) -> String {
            // artist, title, albtrack, albtot, alb, state, qtrack, qtot,
            // elapsed_pretty, duration_pretty, percent, ersc_str, volume

            // start defining some variables

            // TODO: find a way to make this better?
            const UNKNOWN: &str = "?";

            let artist = self.artist
                .clone().unwrap_or(UNKNOWN.to_string());
            let title = self.title
                .clone().unwrap_or(UNKNOWN.to_string());

            // dprintln!("self.album_track: {:?}", self.album_track);
            let album_track = match self.album_track {
                Some(s) => s.to_string(),
                None =>UNKNOWN.to_string(),
            };
            let album_total = match self.album_total {
                Some(s) => s.to_string(),
                None =>UNKNOWN.to_string(),
            };

            let album = self.album
                .clone().unwrap_or(UNKNOWN.to_string());

            let state = match self.state {
                State::Play => "|>",
                State::Pause => "[]",
                State::Stop => "><",
            };

            let queue_track = match self.queue_track {
                Some(s) => (s.pos+1).to_string(),
                None =>UNKNOWN.to_string(),
            };
            let queue_total = match self.queue_total {
                Some(s) => s.to_string(),
                None =>UNKNOWN.to_string(),
            };

            let elapsed_pretty = Self::get_pretty_time(self.time_curr)
                .unwrap_or(UNKNOWN.to_string());
            let duration_pretty = Self::get_pretty_time(self.time_total)
                .unwrap_or(UNKNOWN.to_string());

            let percent = match (self.time_curr, self.time_total) {
                (Some(curr), Some(total)) => {
                    (100*curr.as_secs()/total.as_secs()).to_string()
                },
                _ =>UNKNOWN.to_string()
            };
            let ersc_str = self.get_ersc();
            let volume = self.volume;
            let crossfade = match self.crossfade {
                Some(t) => format!(" (x: {})", t.as_secs()),
                None => String::new(),
            };

            // apply coloring!!!
            // TODO: can a macro be useful here?
            // TODO: should these be constants?
            let col_artist = "\x1b[1;36m";  // bold cyan
            let col_title  = "\x1b[1;34m";  // bold blue
            let col_track  = "\x1b[32m";    // green
            let col_album  = "\x1b[36m";    // cyan
            let col_play   = "\x1b[32m";    // green
            let col_pause  = "\x1b[31m";    // red
            let _col_plist = "\x1b[1m";     // bold
            let col_end    = "\x1b[0m";     // reset

            let col_state = match self.state {
                State::Play => col_play,
                State::Pause | State::Stop =>
                    col_pause,
            };

            // final format text
            format!(
                "{col_artist}{artist}{col_end} * {col_title}{title}{col_end}\n({col_track}#{album_track}/{album_total}{col_end}) {col_album}{album}{col_end}\n{col_state}{state} {queue_track}/{queue_total}: {elapsed_pretty}/{duration_pretty}, {percent}%{col_end}\n{col_state}{ersc_str}, {volume}%{crossfade}{col_end}"
                )
        }

        // TODO: clean this up after it's done
            #[allow(clippy::let_and_return)]
        fn print_queue(&self) -> String {
            // get terminal size
            let (w, h) = terminal_size()
                .expect("should have terminal");
            let height = u32::from(h.0)-1;
            let width  = u32::from(w.0);
            dprintln!("terminal is {h:?} height and {w:?} width");

            // get some other variables
            let queue_size: u32 = self.queue.len().try_into().unwrap_or(0);
            let song_pos = match self.song.place {
                Some(p) => p.pos,
                None => 0,
            };

            // determine padding for format_song()
            let padding = 1 + queue_size
                .checked_ilog10()
                .unwrap_or_default();

            // queue to vec of song-strings
            let mut counter = 0;
            let queue = self.queue
                .clone().iter().map( |i| {
                    counter += 1;
                    let is_curr = counter == song_pos+1;
                    Self::format_song(i.clone(), counter, padding, is_curr)
            })
            .collect::<Vec<_>>();

            // prepare to crop the queue
            let head = Self::get_centered_index(
                height-HEADER_HEIGHT,
                queue_size,
                song_pos,
                );
            // tail = min(plSize, head+height)
            let tail = std::cmp::min(
                queue_size,
                head + height-HEADER_HEIGHT,
                );

            dprintln!("head: {head}");
            dprintln!("tail: {tail}");

            // first cropped queue
            let queue = queue.get(head as usize..tail as usize)
				.unwrap_or_default();

            // textual queue
            let queue = queue.join("\n");

            // wrapped queue
            let _indent = "......";
            let opt = textwrap::Options::new(
                width.try_into().expect("nothing should be that big")
                );
            let queue = textwrap::wrap(&queue, opt);

            // re-crop the queue
            let queue_size: u32 = self.queue.len().try_into().expect("nothing should be that big");
            let mut song_pos: Option<u32> = None;
            for (i, v) in queue.iter().enumerate() {
                if v.starts_with('\x1b') {
                    song_pos = Some(i.try_into().expect("nothing should be that big"));
                }
            }
            let song_pos = song_pos.unwrap_or(0);

            // prepare to crop the queue
            let head = Self::get_centered_index(
                height-HEADER_HEIGHT,
                queue_size,
                song_pos,
                );
            // tail = min(plSize, head+height)
            let tail = std::cmp::min(
                queue_size,
                head + height-HEADER_HEIGHT,
                );

            dprintln!("head: {head}");
            dprintln!("tail: {tail}");

            // second cropped queue
            let queue = queue.get(head as usize..tail as usize)
				.unwrap_or_default();

            // join queue
            let queue = queue.join("\n");

            // finally return
            queue
        }

        fn format_song(song: Song, index: u32, padding: u32, is_curr: bool) -> String {
            let padding = padding.try_into().expect("nothing should be that big");

            let songtext = format!("{} * {} * {}",
                song.title.clone().unwrap_or_else(|| "???".to_string()),
                song.artist.clone().unwrap_or_else(|| "???".to_string()),
                Self::get_metadata(&song, "album").unwrap_or("?".to_string()),
                );

            let (ansi1, ansi2) = match is_curr {
                true => ("\x1b[1m", "\x1b[0m"),
                false => ("", ""),
            };

            format!("{ansi1}  {index:>padding$}  {songtext}{ansi2}")
        }

        // ported directly from python, i did my best...
        // display size, total queue size, current position in queue
        fn get_centered_index(display: u32, total: u32, curr: u32) -> u32 {
            dprintln!("[get_centered_index()]\n[display: {display}, total: {total}, curr: {curr}]");
            if total <= display {
                return 0
            }

            let half = (display-1)/2;
            let head = curr as i32 - half as i32;
            let tail = if display%2 == 0 {
                curr+half+1
            } else {
                curr+half
            };

            // values are invalid if the start of the list is before 0, or if the end of the list is after the end of the list
            let head_err = head < 0;
            let tail_err = tail >= total;
            match (head_err, tail_err) {
                (true, true) => { 0 } // this should never happen?
                (true, false) => { 0 }
                (false, true) => { total - display }
                (false, false) => { head.try_into().expect("this should be impossible. i think?") }
            }
        }
    }

    impl fmt::Display for DataCache {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}\n{}",
                 self.print_header(),
                 self.print_queue(),
                 )
        }
    }

    impl Default for Playlist {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Playlist {
        #[must_use] pub fn new() -> Self {
            Self {
            }
        }
    }
}

