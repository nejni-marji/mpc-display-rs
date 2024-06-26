# mpc-display-rs
This is a client for Music Player Daemon.

`mpc-display-rs` was originally a companion program for `mpc`, but now supports keyboard input to control playback! How exciting... it's finally a real MPD client!

## Controls

* `h`, `?` - show help text

* `space` - pause/play

* `pk`, `nj` - prev/next track

* `H`, `L` - seek back/ahead

* `+0`, `-9` - volume up/down

* `ERSC` - repeat, random, single, consume

* `F` - shuffle (reorders queue in-place)

* `{`, `}` - adjust current track rating

* `M` - stops playback

* `x`, `X` - crossfade up/down

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

`mpc-display-rs` respects `MPD_HOST` and `MPD_PORT`.

## Screenshots
<!--![](images/demo1.png "demo 1")-->
![](images/demo2.png "demo")
![](images/demo3.png "demo")
