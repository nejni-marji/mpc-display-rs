use std::borrow::Cow::Borrowed;
use std::fmt;
use std::io;
use std::io::Write;
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;
use mpd::{Client, Idle, Query, search::Window, Song, song::QueuePlace, State, Subsystem, Term};
use terminal_size::terminal_size;
use uuid::Uuid;

const UNKNOWN: &str = "?";

#[allow(unused_imports)]
use debug_print::{
    debug_print as dprint,
    debug_println as dprintln,
    debug_eprint as deprint,
    debug_eprintln as deprintln,
};

#[derive(Debug,Default)]
pub struct Player {
    //address: String,
    client: Mutex<Client>,
    data: MusicData,
    format: Vec<String>,
    quit: bool,
    uuid: Uuid,
}

#[derive(Debug,Default,Clone)]
struct MusicData {
    // non-music data
    format: Vec<String>,
    ditto_tags: Vec<bool>,
    prev_album: Option<String>,
    prev_album_total: Option<u32>,
    //use_ditto: bool,
    // everything that can potentially be missing is an Option type.
    // the exception to this is queue_total, which theoretically would be 0
    // when there is no value, but i've chosen to force it into an Option
    // anyway, for consistency and because it makes things cooler later.
    song: Song,
    queue: Vec<Song>,
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
    date: Option<String>,
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
    rating: Option<String>,
}

impl Player {
    #[must_use] pub fn new(address: String, format: Vec<String>, uuid: Uuid) -> Self {
        Self {
            //address: address.clone(),
            client: Mutex::new(
                Client::connect(address)
                .expect("unable to lock client")
                ),
            data: MusicData::new(),
            format,
            quit: false,
            uuid,
        }
    }

    pub fn init(&mut self) {
        let data = &mut self.data;
        data.format.clone_from(&self.format);
        data.update_status(&self.client);
        data.update_song(&self.client);
        data.update_playlist(&self.client);
        data.update_sticker(&self.client);
    }

    pub fn display(&mut self) {
        dprintln!("[startup]");
        self.data.display();

        #[cfg(debug_assertions)]
        let mut counter_idle = 0;
        loop {
            // check quit status
            if self.quit {
                break
            }

            // prepare channel
            let (tx, rx) = mpsc::channel();

            // spawn thread if we need it
            if self.data.state == State::Play {
                // clone data for thread
                let data = self.data.clone();

                // assign thread handle to external variable
                _ = thread::spawn(move || {
                    Self::delay_thread(&rx, data);
                });
            }

            // wait for idle, then print
            self.idle();
            #[cfg(debug_assertions)] {
                counter_idle += 1;
            }
            dprintln!("[idle: {counter_idle}]");
            self.data.display();

            // send signal to kill thread
            let _ = tx.send(true);
        }
    }

    fn delay_thread(rx: &mpsc::Receiver<bool>, mut data: MusicData) {
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
            data.display();
        }
    }

    fn idle(&mut self) {
        // use client to idle. no early drop
        let mut conn = self.client.lock()
            .expect("unable to lock client");
        let subsystems = conn.wait(&[
            Subsystem::Player, Subsystem::Mixer,
            Subsystem::Options, Subsystem::Queue,
            Subsystem::Subscription, Subsystem::Sticker,
        ]).unwrap_or_default();
        drop(conn);

        dprintln!("[subsystems: {subsystems:?}]");
        for i in subsystems {
            let data = &mut self.data;
            match i {
                Subsystem::Player => {
                    data.update_status(&self.client);
                    data.update_song(&self.client);
                    data.update_sticker(&self.client);
                }
                Subsystem::Mixer | Subsystem::Options => {
                    data.update_status(&self.client);
                }
                Subsystem::Queue => {
                    data.update_playlist(&self.client);
                    data.update_song(&self.client);
                    data.update_sticker(&self.client);
                }
                Subsystem::Sticker => {
                    data.update_sticker(&self.client);
                    data.update_status(&self.client);
                }
                Subsystem::Subscription => {
                    // get channel list
                    let mut conn = self.client.lock()
                        .expect("unable to lock client");
                    let channels = conn.channels().unwrap_or_default();
                    dprintln!("{channels:?}");
                    drop(conn);

                    // check for quit channel
                    for i in &channels {
                        if *i == mpd::message::Channel::new(
                            format!("quit_{}",
                                self.uuid.simple()).as_str()
                            )
                            .expect("unable to make quit channel") {
                            self.quit = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl MusicData {
    #[must_use] pub fn new() -> Self {
        Self::default()
    }

    pub fn display(&self) {
        print!("\x1b[?25l\x1b[2J{self}\x1b[H");
        io::stdout().flush().expect("unable to flush buffer");
    }

    fn update_status(&mut self, client: &Mutex<Client>) {
        // use client to get some data
        let mut conn = client.lock()
            .expect("unable to lock client");
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
            .expect("unable to lock client");
        let song = conn.currentsong()
            .unwrap_or_default().unwrap_or_default();
        drop(conn);


        // get easier stuff first
        let album = Self::get_metadata(&song, "album");
        let album_track = Self::get_metadata(&song, "track")
            .and_then(|t| t.parse().ok());

        // get album total, optionally from cache
        let album_total = if album == self.prev_album {
            self.prev_album_total
        } else {
            Self::get_album_size(client, album.clone())
        };

        // update cache
        self.prev_album.clone_from(&album);
        self.prev_album_total = album_total;

        let date = Self::get_metadata(&song, "date");

        // mutate data
        self.song = song;
        self.artist.clone_from(&self.song.artist);
        self.title.clone_from(&self.song.title);
        self.album = album;
        self.date = date;
        self.album_track = album_track;
        self.album_total = album_total;
    }

    fn update_playlist(&mut self, client: &Mutex<Client>) {
        // use client to get some data
        let mut conn = client.lock()
            .expect("unable to lock client");
        let queue = conn.queue()
            .unwrap_or_default();
        // dprintln!("[update_playlist()]\n[{queue:?}]");
        drop(conn);

        // check for repeated values per tag
        let mut ditto_tags = Vec::new();
        for tag in &self.format {
            // skip title, that's silly
            if tag == "title" {
                ditto_tags.push(false);
                continue;
            }
            // check for equality, if anything is ever different then
            // immediately stop searching
            let mut is_ditto = true;
            // temp value for first song
            let temp1 = queue.first().map_or_else(
                || Some(String::new()),
                |s| Self::get_metadata(s, tag),
            );
            for song in &queue {
                // temp value for other songs
                let temp2 = Self::get_metadata(song, tag);
                // if different, break from song loop
                if temp1 != temp2 {
                    is_ditto = false;
                    break;
                }
            }
            dprintln!("[update_playlist()]\n[is {tag} ditto? {is_ditto}]");
            ditto_tags.push(is_ditto);
        }

        self.ditto_tags = ditto_tags;
        self.queue = queue;

        // if these are not the same length, we fucked up
        #[cfg(debug_assertions)]
        assert_eq!(
            self.format.clone().len(),
            self.ditto_tags.clone().len(),
        );
    }

    fn update_sticker(&mut self, client: &Mutex<Client>) {
        // use client to get some data
        let mut conn = client.lock()
            .expect("unable to lock client");
        let rating = conn.sticker("song", &self.song.file, "rating")
            .ok();
        // dprintln!("[update_playlist()]\n[{queue:?}]");
        drop(conn);

        self.rating = rating;
    }

    fn print_header(&self) -> String {
        const COL_ARTIST : &str = "\x1b[1;36m";  // bold cyan
        const COL_TITLE  : &str = "\x1b[1;34m";  // bold blue
        const COL_TRACK  : &str = "\x1b[32m";    // green
        const COL_ALBUM  : &str = "\x1b[36m";    // cyan
        const COL_DATE   : &str = "\x1b[33m";    // bold yellow
        const COL_RATING : &str = "\x1b[35;1m";  // bold magenta
        const COL_PLAY   : &str = "\x1b[32m";    // green
        const COL_PAUSE  : &str = "\x1b[31m";    // red
        const COL_END    : &str = "\x1b[0m";     // reset

        #[inline]
        fn fmt_r(r: &str) -> String { format!("rating: {r}") }

        // start defining some variables
        let artist = self.artist
            .clone().unwrap_or_else(|| UNKNOWN.to_string());
        let title = self.title
            .clone().unwrap_or_else(|| UNKNOWN.to_string());

        // dprintln!("self.album_track: {:?}", self.album_track);
        let album_track = self.album_track.map_or_else(
            || UNKNOWN.to_string(),
            |s| s.to_string()
            );
        let album_total = self.album_total.map_or_else(
            || UNKNOWN.to_string(),
            |s| s.to_string()
            );

        let album = self.album
            .clone().unwrap_or_else(|| UNKNOWN.to_string());

        let date = self.date
            .clone().unwrap_or_else(|| UNKNOWN.to_string());

        let state = match self.state {
            State::Play => "|>",
            State::Pause => "[]",
            State::Stop => "><",
        };

        let queue_track = self.queue_track.map_or_else(
            || UNKNOWN.to_string(),
            |s| (s.pos+1).to_string(),
            );
        let queue_total = self.queue_total.map_or_else(
            || UNKNOWN.to_string(),
            |s| s.to_string(),
            );

        let elapsed_pretty = Self::get_pretty_time(self.time_curr)
            .unwrap_or_else(|| UNKNOWN.to_string());
        let duration_pretty = Self::get_pretty_time(self.time_total)
            .unwrap_or_else(|| UNKNOWN.to_string());

        let percent = match (self.time_curr, self.time_total) {
            (Some(curr), Some(total)) => {
                (100*curr.as_secs()/total.as_secs()).to_string()
            },
            _ =>UNKNOWN.to_string()
        };

        let rating = self.rating
            .clone().map_or_else(
                || "?????".to_string(),
                |r| {
                    const STARS: [&str; 3] = ["<3", "< ", " ."];

                    match r.parse::<usize>() {
                        Err(_) => fmt_r(&r),
                        Ok(n) => {
                            if n > 10 {
                                return fmt_r(&r);
                            }
                            let (a, b) = (n/2, n%2);
                            let c = std::cmp::max(0, 5-a-b);

                            format!("{}{}{}",
                                STARS[0].repeat(a),
                                STARS[1].repeat(b),
                                STARS[2].repeat(c),
                            )
                        }
                    }
                });

        let ersc_str = self.get_ersc();
        let volume = self.volume;
        let crossfade = self.crossfade.map_or_else(
            String::new,
            |t| format!(" (x: {})", t.as_secs()),
            );

        // apply coloring!!!
        let col_state = match self.state {
            State::Play => COL_PLAY,
            State::Pause | State::Stop =>
                COL_PAUSE,
        };

        // final format text
        format!(
            "{COL_ARTIST}{artist}{COL_END} * {COL_TITLE}{title}{COL_END}\n({COL_TRACK}#{album_track}/{album_total}{COL_END}) {COL_ALBUM}{album}{COL_END} {COL_DATE}({date}){COL_END}\n{col_state}{state} {queue_track}/{queue_total}: {elapsed_pretty}/{duration_pretty}, {percent}%{COL_END}  {COL_RATING}{rating}{COL_END}\n{col_state}{ersc_str}, {volume}%{crossfade}{COL_END}"
            )
    }

    // TODO: clean this up after it's done. this is probably full of other bugs, lol!
    #[allow(clippy::let_and_return)]
    fn print_queue(&self, height: u32, width: u32, header_height: u32) -> String {
        // get some other variables
        let queue_size: u32 = self.queue.len().try_into().unwrap_or(0);
        let song_pos = self.song.place.map_or_else(
            || 0,
            |p| p.pos,
            );

        // determine padding for format_song()
        let padding = 1 + queue_size
            .checked_ilog10()
            .unwrap_or_default();

        // queue to vec of song-strings
        let mut counter = 0;
        let queue = self.queue
            .clone().iter().map(|i| {
                counter += 1;
                let is_curr = counter == song_pos+1;
                self.format_song(i, counter, padding, is_curr)
        })
        .collect::<Vec<_>>();

        // prepare to crop the queue
        let head = Self::get_centered_index(
            height-header_height,
            queue_size,
            song_pos,
            );
        let tail = std::cmp::min(
            queue_size,
            head + height-header_height,
            );
        let tail = std::cmp::min(
            tail, queue.len().try_into().unwrap_or(0)
            );

        dprintln!("head: {head}");
        dprintln!("tail: {tail}");
        dprintln!("len: {}", queue.len());

        // first cropped queue
        let queue = queue.get(head as usize..tail as usize)
            .unwrap_or_default();

        // textual queue
        let queue = queue.join("\n");

        // wrapped queue
        let opt = textwrap::Options::new(
            width.try_into().expect("nothing should be that big")
            );
        let queue = textwrap::wrap(&queue, opt);

        // CODE REUSE
        // get some new variables
        let queue_size: u32 = queue.len().try_into().expect("nothing should be that big");
        let mut song_pos: Option<u32> = None;
        for (i, v) in queue.iter().enumerate() {
            if v.starts_with('>') || v.starts_with('\x1b') {
                song_pos = Some(i.try_into().expect("nothing should be that big"));
            }
        }
        let song_pos = song_pos.unwrap_or(0);

        // prepare to crop the queue
        let head = Self::get_centered_index(
            height-header_height,
            queue_size,
            song_pos,
            );
        let tail = std::cmp::min(
            queue_size,
            head + height-header_height,
            );
        let tail = std::cmp::min(
            tail, queue.len().try_into().unwrap_or(0)
            );

        dprintln!("head: {head}");
        dprintln!("tail: {tail}");
        dprintln!("len: {}", queue.len());

        // second cropped queue
        let queue = queue.get(head as usize..tail as usize)
            .unwrap_or_default();
        // END CODE REUSE

        // create padding to add later
        let len = queue.len().try_into().unwrap_or(0);
        let mut diff: i32 = ((height-header_height) - len).
            try_into().unwrap_or(0);
        if diff < 0 {
            diff = 0;
        }
        diff += 1;
        let diff = diff.try_into().unwrap_or(0);
        let queue_padding = vec![""; diff].join("\n");

        // join queue and add padding
        let queue = queue.join("\n")
            + &queue_padding;

        // finally return
        queue
    }

    fn format_song(&self, song: &Song, index: u32, padding: u32, is_curr: bool) -> String {
        // get colors
        const COL_CURR   : &str = "\x1b[7m";     // reverse
        const COL_END    : &str = "\x1b[0m";     // reset
        let (ansi1, ansi2, curr) = if is_curr {
            (COL_CURR, COL_END, '>')
        } else {
            ("", "", ' ')
        };

        // get padding
        let padding = padding.try_into().expect("nothing should be that big");

        // get song text
        let mut tags = Vec::new();
        for (i, v) in self.format.clone().into_iter().enumerate() {
            if *self.ditto_tags.get(i).expect("assert that these are the same length") {
                continue;
            }
            let v = v.as_str();
            tags.push(Self::get_metadata(song, v).unwrap_or_else(|| UNKNOWN.to_string()));
        }

        let songtext = tags.join(" * ");

        format!("{ansi1}{curr} {index:>padding$}  {songtext}{ansi2}")
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
            (true, _) => { 0 }
            (false, true) => { total - display }
            (false, false) => { head.try_into().expect("this should be impossible. i think?") }
        }
    }

    fn increment_time(&mut self, n: u64) {
        self.time_curr = self.time_curr.map(|t| t + Duration::from_secs(n));
    }

    fn get_album_size(client: &Mutex<Client>, album: Option<String>) -> Option<u32> {
        // build query
        let mut query = Query::new();
        query.and(Term::Tag(Borrowed("Album")), album?);
        let window = Window::from((0,u32::from(u16::MAX)));
        // lock client and search
        let mut conn = client.lock()
            .expect("unable to lock client");
        let search = conn.search(&query, window);
        drop(conn);
        // parse search
        search.ok().map(|s| u32::try_from(s.len())
            .expect("can't cast search length"))
    }

    fn get_pretty_time(dur: Option<Duration>) -> Option<String> {
        let n = dur?.as_secs();
        let (min, sec) = (n / 60, n % 60);
        Some(format!("{min}:{sec:0>2}"))
    }

    fn get_ersc(&self) -> String {
        let mut ersc = String::new();
        let base = ['e', 'r', 's', 'c'];
        let ersc_opts = &self.ersc_opts;
        for (i, v) in base.iter().enumerate() {
            ersc.push(
                // this unwrap_or is... middling at best, i think
                if *ersc_opts.get(i).unwrap_or(&false) {
                    v.to_ascii_uppercase()
                } else {
                    *v
                }
            );
        }
        ersc
    }

    fn get_metadata(song: &Song, tag: &str) -> Option<String> {
        match tag {
            "title" => song.title.clone(),
            "artist" => song.artist.clone(),
            _ => {
                let mut value = None;
                for (k, v) in &song.tags {
                    if k.to_ascii_lowercase() == tag.to_ascii_lowercase() {
                        value = Some(v);
                    }
                }
                value.cloned()
            }
        }
    }

}

impl fmt::Display for MusicData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // get terminal height
        let (height, width) = match terminal_size() {
            Some((w,h)) => (u32::from(h.0), u32::from(w.0)),
            None => (24, 80),
        };
        dprintln!("[terminal: height {height}, width {width}]");

        // get header size
        let header = self.print_header();
        let opt = textwrap::Options::new(
            width.try_into().expect("nothing should be that big")
            );
        let header = textwrap::fill(
            header.as_str(), opt
            );
        let header_height = (1 + header.matches('\n').count())
            .try_into().expect("can't cast header size");
        dprintln!("[header_height: {header_height}]");

        write!(f, "{}\n{}",
            header,
            self.print_queue(height, width, header_height),
            )
    }
}
