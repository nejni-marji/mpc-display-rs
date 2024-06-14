use std::borrow::Cow::Borrowed;
use std::fmt;
use std::io;
use std::io::Write;
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;

use mpd::{Client, Idle, message::Channel, Query, search::Window, Song, song::QueuePlace, State, Subsystem, Term};
use terminal_size::terminal_size;
use uuid::Uuid;

#[allow(unused_imports)]
use debug_print::{
    debug_print as dprint,
    debug_println as dprintln,
    debug_eprint as deprint,
    debug_eprintln as deprintln,
};

const UNKNOWN: &str = "?";

#[derive(Clone,Copy,Debug,Default,PartialEq)]
enum Signal {
    #[default]
    Normal,
    Help,
    Quit,
}

#[derive(Debug,Default)]
pub struct Display {
    client: Mutex<Client>,
    data: MusicData,
    signal: Signal,
    uuid: Uuid,
}

#[derive(Debug,Default,Clone)]
struct MusicData {
    // non-music data
    format: Vec<String>,
    verbose: bool,
    verbose_tags: Vec<bool>,
    show_ratings: bool,
    easter: bool,
    prev_album: Option<String>,
    prev_album_total: Option<u32>,
    // music data
    // `queue_total` does not need to be Option, but it is anyway for consistency.
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

impl Display {
    #[must_use]
    pub fn new(client: Client, format: Vec<String>,
        uuid: Uuid, verbose: bool, ratings: bool, easter: bool) -> Self
    {
        Self {
            client: Mutex::new(client),
            data: MusicData::new(format, verbose, ratings, easter),
            signal: Signal::default(),
            uuid,
        }
    }

    pub fn init(&mut self) {
        let data = &mut self.data;
        data.update_status(&self.client);
        data.update_song(&self.client);
        data.update_playlist(&self.client);
        data.update_sticker(&self.client);

        self.display();
    }

    pub fn display(&mut self) {
        dprintln!("[startup]");
        self.data.display();

        #[cfg(debug_assertions)]
        let mut counter_idle = 0;
        loop {
            // check signal status
            match self.signal {
                Signal:: Normal => {},
                Signal::Quit => {
                    break
                },
                Signal::Help => {
                    Self::helptext();
                },
            }

            // prepare channel
            let (tx, rx) = mpsc::channel();

            if self.signal == Signal::Normal {
                // spawn thread if we need it
                if self.data.state == State::Play {
                    // clone data for thread
                    let data = self.data.clone();

                    // assign thread handle to external variable
                    _ = thread::spawn(move || {
                        Self::delay_thread(&rx, data);
                    });
                }
            }

            // wait for idle, then print
            self.idle();
            #[cfg(debug_assertions)] {
                counter_idle += 1;
            }

            if self.signal == Signal::Normal {
                dprintln!("[idle: {counter_idle}]");
                self.data.display();
            }

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
            .expect("can't lock client");
        let subsystems = conn.wait(&[
            Subsystem::Player, Subsystem::Mixer,
            Subsystem::Options, Subsystem::Queue,
            Subsystem::Subscription, Subsystem::Sticker,
        ]).unwrap_or_default();
        drop(conn);

        dprintln!("[subsystems: {subsystems:?}]");
        for i in subsystems {
            let data = &mut self.data;
            // always update status, delay thread requires it
            data.update_status(&self.client);
            match i {
                Subsystem::Player => {
                    data.update_song(&self.client);
                    data.update_sticker(&self.client);
                }
                Subsystem::Queue => {
                    data.update_playlist(&self.client);
                    data.update_song(&self.client);
                    data.update_sticker(&self.client);
                }
                Subsystem::Sticker => {
                    data.update_sticker(&self.client);
                }
                Subsystem::Subscription => {
                    // get channel list
                    let mut conn = self.client.lock()
                        .expect("can't lock client");
                    let channels = conn.channels().unwrap_or_default();
                    dprintln!("{channels:?}");
                    drop(conn);

                    // change signal based on channel/signal state
                    self.signal = if channels.contains(&Channel::new(
                        format!("help_{}", self.uuid.simple()).as_str()
                    ).expect("can't make help channel")) {

                        Signal::Help

                    } else if self.signal == Signal::Help {

                        Signal::Normal

                    } else if channels.contains(&Channel::new(
                        format!("quit_{}", self.uuid.simple()).as_str()
                    ).expect("can't make quit channel")) {

                        Signal::Quit

                    } else {
                        self.signal
                    }
                }
                _ => {}
            }
        }
    }

    fn helptext() {
        // TODO: make this... not a const? and dynamically format the text?
        const HELPTEXT: &str = "\x1b[2J
  \x1b[1mh, ?\x1b[0m ......show help text
  \x1b[1mspace\x1b[0m .....pause/play
  \x1b[1mpk, nj\x1b[0m ....prev/next track
  \x1b[1mH, L\x1b[0m ......seek back/ahead
  \x1b[1m+0, -9\x1b[0m ....volume up/down
  \x1b[1mERSC\x1b[0m ......repeat, random, single, consume
  \x1b[1mF\x1b[0m .........shuffle (reorders queue in-place)
  \x1b[1m{, }\x1b[0m ......adjust current track rating
  \x1b[1mM\x1b[0m .........stops playback
  \x1b[1mx, X\x1b[0m ......crossfade up/down

  \x1b[1;35m~made by aurora~\x1b[0m\
\x1b[H";

        println!("{HELPTEXT}");
    }
}

impl MusicData {
    #[must_use]
    pub fn new(format: Vec<String>,
        verbose: bool, show_ratings: bool, easter: bool) -> Self
    {
        Self {
            format,
            verbose,
            show_ratings,
            easter,
            ..Self::default()
        }
    }

    pub fn display(&self) {
        print!("\x1b[2J{self}\x1b[H");
        io::stdout().flush().expect("can't flush buffer");
    }

    fn update_status(&mut self, client: &Mutex<Client>) {
        // use client to get some data
        let mut conn = client.lock()
            .expect("can't lock client");
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
            .expect("can't lock client");
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
            .expect("can't lock client");
        let queue = conn.queue()
            .unwrap_or_default();
        drop(conn);

        // default case
        if self.verbose {
            self.verbose_tags = Vec::new();
        } else {
            // check for repeated values per tag
            let mut verbose_tags = Vec::new();
            for tag in &self.format {
                // skip title, that's silly
                if tag == "title" {
                    verbose_tags.push(false);
                    continue;
                }
                // check for equality, if anything is ever different then
                // immediately stop searching
                let mut is_verbose = true;
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
                        is_verbose = false;
                        break;
                    }
                }
                dprintln!("[update_playlist()]\n[is {tag} verbose? {is_verbose}]");
                verbose_tags.push(is_verbose);
            }
            self.verbose_tags = verbose_tags;
        }
        // always assign queue
        self.queue = queue;

    }

    fn update_sticker(&mut self, client: &Mutex<Client>) {
        // use client to get some data
        let mut conn = client.lock()
            .expect("can't lock client");
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

        // start defining some variables
        let artist = self.artist
            .clone().unwrap_or_else(|| UNKNOWN.into());
        let title = self.title
            .clone().unwrap_or_else(|| {
                let file = self.song.file.clone();
                file.split('/').last()
                    .unwrap_or(UNKNOWN).into()
            });

        // dprintln!("self.album_track: {:?}", self.album_track);
        let album_track = self.album_track.map_or_else(
            || UNKNOWN.into(),
            |s| s.to_string()
        );
        let album_total = self.album_total.map_or_else(
            || UNKNOWN.into(),
            |s| s.to_string()
        );

        let album = self.album
            .clone().unwrap_or_else(|| UNKNOWN.into());

        let date = self.date
            .clone().unwrap_or_else(|| UNKNOWN.into());

        let state = match self.state {
            State::Play => "|>",
            State::Pause => "[]",
            State::Stop => "><",
        };

        let queue_track = self.queue_track.map_or_else(
            || UNKNOWN.into(),
            |s| (s.pos+1).to_string(),
        );
        let queue_total = self.queue_total.map_or_else(
            || UNKNOWN.into(),
            |s| s.to_string(),
        );

        let elapsed_pretty = Self::get_pretty_time(self.time_curr)
            .unwrap_or_else(|| UNKNOWN.into());
        let duration_pretty = Self::get_pretty_time(self.time_total)
            .unwrap_or_else(|| UNKNOWN.into());

        let percent = match (self.time_curr, self.time_total) {
            (Some(curr), Some(total)) => {
                (100*curr.as_secs()/total.as_secs()).to_string()
            },
            _ =>UNKNOWN.into()
        };

        let rating = self.get_rating();

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

    fn get_rating(&self) -> String {
        fn fmt_r(r: &str) -> String { format!("rating: {r}") }

        if self.show_ratings && !self.easter {
            self.rating
                .clone().map_or_else(
                    || " ? ? ? ? ?".into(),
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
                    })
        } else if self.easter {
            const CHRISTGAU: [&str; 11] = ["🦃", "💣" , " ✂️", "😐",
            "⭐", "⭐ ⭐", "⭐ ⭐ ⭐", "B+", "A-", "A", "A+"];
            format!("\x1b[40m {} \x1b[0m", CHRISTGAU[
                self.rating
                .clone()
                .unwrap_or_default()
                .parse::<usize>()
                .unwrap_or_default()
                .clamp(0, 11)
            ])
        } else {
            String::new()
        }
    }

    #[allow(clippy::let_and_return)]
    fn print_queue(&self, height: u32, width: u32, header_height: u32) -> String {
        // get height of queue
        let queue_height = height-header_height;

        // get size of queue and current song index
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

        // filter and string-ify the queue
        let queue = Self::filter_queue(&queue,
            queue_height, queue_size, song_pos)
            .join("\n");

        // wrap the queue
        let opt = textwrap::Options::new(
            width.try_into().expect("nothing should be that big")
        );
        let queue = textwrap::wrap(&queue, opt);

        // (again) get size of queue and current song index
        let queue_size: u32 = queue.len().try_into().expect("nothing should be that big");
        let mut song_pos: Option<u32> = None;
        for (i, v) in queue.iter().enumerate() {
            if v.starts_with('>') || v.starts_with('\x1b') {
                song_pos = Some(i.try_into().expect("nothing should be that big"));
            }
        }
        let song_pos = song_pos.unwrap_or(0);

        // filter the queue
        let queue = Self::filter_queue(&queue,
            queue_height, queue_size, song_pos);

        // create padding to add later
        let len = queue.len().try_into().unwrap_or(0);
        let mut diff: i32 = ((queue_height) - len).
            try_into().unwrap_or(0);
        if diff < 0 {
            diff = 0;
        }
        diff += 1;
        let diff = diff.try_into().unwrap_or(0);
        let queue_padding = vec![""; diff].join("\n");

        // string-ify and add padding to queue
        let queue = queue.join("\n") + &queue_padding;

        // finally return
        queue
    }

    #[allow(clippy::let_and_return)]
    fn filter_queue<T>(queue: &[T],
        queue_height: u32, queue_size: u32, song_pos: u32) -> &[T]
    {
        // get variables to filter the queue
        let head = Self::get_centered_index(
            queue_height,
            queue_size,
            song_pos,
        );
        let tail = std::cmp::min(
            queue_size,
            head + queue_height,
        );
        let tail = std::cmp::min(
            tail, queue.len().try_into().unwrap_or(0)
        );

        dprintln!("head: {head}");
        dprintln!("tail: {tail}");
        dprintln!("len: {}", queue.len());

        // actually filter the queue
        let queue = queue.get(head as usize..tail as usize)
            .unwrap_or_default();

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
            if !self.verbose && *self.verbose_tags.get(i).unwrap_or(&false) {
                continue;
            }
            tags.push(Self::get_metadata(song, &v).unwrap_or_else(|| UNKNOWN.into()));
        }

        let songtext = tags.join("  *  ");

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
        #[allow(clippy::cast_possible_wrap)]
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
            .expect("can't lock client");
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
