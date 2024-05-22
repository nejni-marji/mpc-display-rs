# mpc-display-rs
This is a client for Music Player Daemon.

`mpc-display-rs` was originally a companion program for `mpc`, but now supports keyboard input to control playback! How exciting... it's finally a real MPD client!

## Controls

Which keys do what are still up in the air, so... have fun!

## Usage

```
Usage: mpc-display-rs [OPTIONS]

Options:
  -H, --host <HOST>      Connect to server at address <HOST> [default: 127.0.0.1]
  -P, --port <PORT>      Connect to server on port <PORT> [default: 6600]
  -f, --format <FORMAT>  Comma-separated list of song metadata to display [default: title,artist,album]
  -t, --title            Equivalent to '--format title'
  -h, --help             Print help
  -V, --version          Print version
```

## Screenshots
![](images/demo1.png "demo 1")
