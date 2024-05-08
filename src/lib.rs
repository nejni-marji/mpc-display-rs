pub mod music {
    use std::time::Duration;
    use std::fmt;
    use std::borrow::Cow::Borrowed;
    use std::sync::Mutex;
    use mpd::{Client,Status,State,Song,Query,Term,search::Window};
    #[derive(Debug)]
    pub struct DataCache {
        artist: String,
        title: String,
        album: String,
        album_track: u32,
        album_total: u32,
        queue_track: u32,
        queue_total: u32,
        time_curr: Duration,
        time_total: Duration,
        state: State,
        volume: i8,
        ersc_opts: Vec<bool>,
    }

    impl DataCache {
        pub fn new(status: Status, song: Song, client: Mutex<Client>) -> DataCache {
            let mut album = None;
            for (k, v) in &song.tags {
                if k == "Album" {
                    album = Some(v)
                }
            }
            // this makes a server call, so we do it now rather during display
            // why cloned?
            let album_num = Self::get_album_nums(client, album.cloned(), song.clone());
            DataCache {
                artist: song.artist.unwrap(),
                title: song.title.unwrap(),
                album: album.expect("").to_string(),
                album_track: album_num.0,
                album_total: album_num.1,
                // queue_track: status.song.unwrap().pos + 1, // 0-indexed
                // TODO
                queue_track: match status.song {
                    Some(n) => n.pos+1,
                    None => 0,
                },
                queue_total: status.queue_len,
                time_curr: status.elapsed.unwrap(),
                time_total: status.duration.unwrap(),
                state: State::Play,
                volume: status.volume,
                ersc_opts: vec![
                    status.repeat, status.random,
                    status.single, status.consume],
            }
        }
        pub fn display_todo(&self) -> String {
            let state = match self.state {
                State::Play => "|>",
                State::Pause => "||",
                State::Stop => "??",
            };
            let time_curr_pretty = Self::pretty_time(self.time_curr);
            let time_total_pretty = Self::pretty_time(self.time_total);
            let percent = 100*self.time_curr.as_secs()/self.time_total.as_secs();
            let mode = self.get_mode();
            let volume = self.volume;

            // [artist] * [title]
            // (#2/8) [album]
            // |> 56/78: 2:27/2:59, 82%
            // Ersc, 70%

            format!("{0} * {1}\n(#{2}/{3}) {4}\n{state} {5}/{6}: {time_curr_pretty}/{time_total_pretty}, {percent}%\n{mode}, {volume}%",
            self.artist, self.title,
            self.album_track, self.album_total, self.album,
            self.queue_track, self.queue_total,
            )
        }
        fn get_mode(&self) -> String {
            let mut ersc = String::new();
            let base = ['e', 'r', 's', 'c'];
            for (i, v) in base.iter().enumerate() {
                ersc.push(
                    if self.ersc_opts[i] {
                        v.to_ascii_uppercase()
                    } else {
                        *v
                    }
                );
            }
            ersc
        }
        fn pretty_time(dur: Duration) -> String {
            let n = dur.as_secs();
            let (min, sec) = (n / 60, n % 60);
            format!("{min}:{sec:0>2}")
        }
        fn get_album_nums(client: Mutex<Client>, album: Option<String>, song: Song) -> (u32, u32) {
            let album = album.unwrap();
            let mut conn = client.lock().unwrap();
            let mut query = Query::new();
            query.and(Term::Tag(Borrowed("Album")), album);
            let window = Window::from((0,u16::MAX as u32)); //TODO: make const
            let search = conn.search(&query, window);
            let search = search.unwrap();
            // println!("-----\n{:?}\n-----", search);
            let mut track = None;
            for (k, v) in song.tags {
                if k == "Track" {
                    track = Some(v)
                }
            }
            let track = track.unwrap().parse().unwrap();
            (track, search.len() as u32)
            // (27, 99)
        }
    }
    impl fmt::Display for DataCache {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.display_todo())
        }
    }
}
