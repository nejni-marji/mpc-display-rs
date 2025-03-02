#[derive(Debug,Default,Clone)]
pub struct MusicOpts {
    pub verbose: bool,
    pub ratings: bool,
    pub easter: bool,
}

#[derive(Debug,Default,Clone,Copy,PartialEq)]
pub enum ExitCode {
    #[default]
    Unknown,
    Quit,
    Error,
}
