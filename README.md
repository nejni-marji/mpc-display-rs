# mpc-display-rs
This is a program that displays the current state of an MPD server.

`mpc-display-rs` is a companion program for the standard client, `mpc`, and does not offer any control whatsoever. It handles text wrapping and is flicker-free, and tries to keep the current track in the center of the screen.

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
