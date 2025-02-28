#[derive(Debug,Default,Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct MusicOpts {
    pub verbose: bool,
    pub ratings: bool,
    pub progress: bool,
    pub easter: bool,
}

#[derive(Debug,Default,Clone)]
pub enum ExitCode {
    #[default]
    Unknown,
    Quit,
    Error,
}
