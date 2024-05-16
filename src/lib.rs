pub mod music {
    use std::borrow::Cow::Borrowed;
    use std::fmt;
    use std::sync::{
        // Arc,
        Mutex,
    };
    #[allow(unused_imports)]
    use std::thread::{
        sleep,
        spawn,
    };
    use std::time::Duration;
    #[allow(unused_imports)]
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

    #[derive(Debug,Default)]
    pub struct Player {
        //TODO: does this need to be a mutex?
        client: Mutex<Client>,
        pub data: Mutex<DataCache>,
        #[allow(dead_code)]
        quit: bool,
    }

    #[derive(Debug,Default,Clone)]
    pub struct DataCache {
        // everything that can potentially be missing is an Option type.
        // the exception to this is queue_total, which theoretically would be 0
        // when there is no value, but i've chosen to force it into an Option
        // anyway, for consistency and because it makes things cooler later.
        song: Song,
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
    }

    impl Player {
        //TODO: how to document things?
        /// Player() does not self-initialize, see self.init().
        pub fn new(client: Client) -> Self {
            Self {
                client: Mutex::new(client),
                data: Mutex::new(DataCache::new()),
                quit: false,
            }
        }

        pub fn init(&mut self) {
            // TODO: have to handle this
            let mut data = self.data.lock().expect("should be able to lock data");
            data.update_status(&self.client);
            data.update_song(&self.client);
        }

        pub fn display(&mut self) {
            // TODO: create some sort of messaging system between this and the
            // display thread
            loop {
                self.idle();
                // TODO: send kill signal to thread
                let data = self.data.lock().unwrap();
                println!("{data}");
                if data.state == State::Play {
                    // TODO: spawn thread
                }
            }
        }

        fn idle(&mut self) {
            // use client to idle. no early drop
            let mut conn = self.client.lock()
                .expect("should have client");
            // sleep(Duration::from_secs(1));
            let subsystems = conn.wait(&[
                Subsystem::Player, Subsystem::Mixer,
                Subsystem::Options, Subsystem::Playlist,
            ]).unwrap();
            drop(conn);
            println!("subsystems: {subsystems:?}");
            for i in subsystems {
                let mut data = self.data.lock().unwrap();
                match i {
                    Subsystem::Player => {
                        data.update_status(&self.client);
                        data.update_song(&self.client);
                    }
                    Subsystem::Mixer | Subsystem::Options => {
                        data.update_status(&self.client);
                    }
                    Subsystem::Playlist => {
                        todo!();
                    }
                    _ => {}
                }
            }
        }

        /*
        pub fn display_loop(data: Arc<Mutex<DataCache>>) {
            const DELAY: u64 = 2;
            let mut data = data.lock().unwrap();
            println!("display_loop: {data:?}");
            // while !self.quit {
            loop {
                if data.state == State::Play {
                    println!("display_loop: PLAYING");
                    //TODO: this can be an id value, no clone required
                    let song_a = data.song.clone();
                    sleep(Duration::from_secs(DELAY));
                    let song_b = data.song.clone();
                    if song_a == song_b && data.state == State::Play {
                        data.increment_time(DELAY);
                    }
                    // if !self.quit {
                    if true {
                        println!("{}", data);
                    }
                } else {
                    data.increment_time(DELAY);
                    //TODO: what is this branch for?
                    // i guess it's for quitting?
                }
            }
        }
        pub fn idle_loop(data: Arc<Mutex<DataCache>>) {
            let data = data.lock().unwrap();
            println!("idle_loop: {data:?}");
        }
        */
    }

    impl DataCache {
        pub fn new() -> Self {
            //TODO: can this be a trait or something?
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
            let mut album = None;
            for (k, v) in &song.tags {
                if k == "Album" {
                    album = Some(v);
                }
            }

            // try to get album progress
            let album_progress = Self::get_album_nums(client, album, song.clone());
            let (album_track, album_total) = match album_progress {
                Some(s) => (Some(s.0), Some(s.1)),
                None => (None, None),
            };

            // mutate data
            self.artist = song.artist;
            self.title = song.title;
            //TODO why cloned?
            self.album = album.cloned();
            self.album_track = album_track;
            self.album_total = album_total;
        }

        pub fn increment_time(&mut self, n: u64) {
            self.time_curr = match self.time_curr {
                Some(t) => Some(t + Duration::from_secs(n)),
                None => None
            }
        }

        //TODO: optimize this by caching the result on a per-album basis
        fn get_album_nums(client: &Mutex<Client>, album: Option<&String>, song: Song) -> Option<(u32, u32)> {
            // build query
            let mut query = Query::new();
            query.and(Term::Tag(Borrowed("Album")), album?);
            let window = Window::from((0,u32::from(u16::MAX))); //TODO: make const?
            // lock client and search
            let mut conn = client.lock()
                .expect("should have client");
            let search = conn.search(&query, window);
            drop(conn);
            // parse search
            match search {
                Err(_) => { None },
                Ok(search) => {
                    // eprintln!("{search:?}");
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

        fn pretty_time(dur: Option<Duration>) -> Option<String> {
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
    }

    impl fmt::Display for DataCache {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // artist, title, albtrack, albtot, alb, state, qtrack, qtot,
            // elapsed_pretty, duration_pretty, percent, ersc_str, volume

            // start defining some variables

            //TODO: find a way to make this better?
            const UNKNOWN: &str = "?";

            let artist = self.artist
                .clone().unwrap_or(UNKNOWN.to_string());
            let title = self.title
                .clone().unwrap_or(UNKNOWN.to_string());

            // eprintln!("self.album_track: {:?}", self.album_track);
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

            let elapsed_pretty = Self::pretty_time(self.time_curr)
                .unwrap_or(UNKNOWN.to_string());
            let duration_pretty = Self::pretty_time(self.time_total)
                .unwrap_or(UNKNOWN.to_string());

            let percent = match (self.time_curr, self.time_total) {
                (Some(curr), Some(total)) => {
                    (100*curr.as_secs()/total.as_secs()).to_string()
                },
                _ =>UNKNOWN.to_string()
            };
            let ersc_str = self.get_ersc();
            let volume = self.volume;

            // apply coloring!!!
            // TODO: can a macro be useful here?
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
            write!(f,
                "{col_artist}{artist}{col_end} * {col_title}{title}{col_end}\n({col_track}#{album_track}/{album_total}{col_end}) {col_album}{album}{col_end}\n{col_state}{state} {queue_track}/{queue_total}: {elapsed_pretty}/{duration_pretty}, {percent}%{col_end}\n{col_state}{ersc_str}, {volume}%{col_end}"
                )
        }
    }
}
