#![allow(clippy::missing_panics_doc)]

use std::io;
use std::io::Write;
use std::process::exit;

#[derive(Debug, Default, Clone)]
pub struct MusicOpts {
    pub verbose: bool,
    pub ratings: bool,
    pub easter: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum ExitCode {
    #[default]
    Unknown,
    Quit,
    Error,
}

pub fn start_ansi() {
    // hide cursor, enable alternate buffer
    print!("\x1b[?25l\x1b[?1049h");
    io::stdout().flush().expect("can't flush buffer");
}

pub fn stop_ansi() {
    // unhide cursor, disable alternate buffer
    print!("\x1b[?25h\x1b[?1049l");
    io::stdout().flush().expect("can't flush buffer");
}

pub fn clean_exit(exitcode: ExitCode) -> ! {
    exit(match exitcode {
        ExitCode::Unknown | ExitCode::Quit => 0,
        ExitCode::Error => {
            println!("mpc-display-rs: disconnected from server.");
            1
        }
    });
}
