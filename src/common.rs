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

#[allow(clippy::missing_panics_doc)]
pub fn clean_exit(exitcode: ExitCode) {
    // unhide cursor, disable alternate buffer
    print!("\x1b[?25h\x1b[?1049l");
    io::stdout().flush().expect("can't flush buffer");

    exit(match exitcode {
        ExitCode::Unknown | ExitCode::Quit => 0,
        ExitCode::Error => {
            println!("mpc-display-rs: disconnected from server.");
            1
        }
    });
}
