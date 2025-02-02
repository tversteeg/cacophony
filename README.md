![Cacophony!](doc/images/banner.png)

**Cacophony is a minimalist and ergonomic MIDI sequencer.** It's minimalist in that it doesn't have a lot of functionality MIDI sequencers have. It's ergonomic in that there is no mouse input and a very clean interface, allowing you to juggle less inputs and avoid awkward mouse motions.

![Screenshot of Cacophony](doc/images/screenshot.jpg)

[Buy Cacophony](https://subalterngames.itch.io/cacophony) (or compile it yourself).

[User-end documentation.](https://subalterngames.com/cacophony)

[Discord Server](https://discord.gg/fUapDXgTYj)

## How to compile

I compile Cacophony with Rust 1.71.0 for Linux, MacOS, or Windows. Below is a list of operating systems I've tested:

Linux:

- Ubuntu 18.04 i386 with X11
- Ubuntu 18.04 x64 with X11
- Ubuntu 20.04 x64 with X11
- Ubuntu 22.04 x64 with X11

MacOS:

- Catalina 10.15.7 x64
- Ventura 13.2.1 Apple Silicon

Windows:

- Windows 10 x64

### All platforms

1. Install Rust (stable)
2. Clone this repo

The instructions below will *compile* the code. To *run* it, do one of the following:

1. Move `data/` to the output directory (`target/release/`)
2. Replace `cargo build` in the instructions below with `cargo run`

If you want debug messages, remove `--release` The output will be `target/debug/` instead of `release/`

### Linux

#### Debian 11

1. `apt install clang cmake speech-dispatcher libspeechd-dev pkg-config libssl-dev librust-alsa-sys-dev`
2. `cargo build --release --features speech_dispatcher_0_9`

#### Debian 12

1. `apt install clang cmake speech-dispatcher libspeechd-dev pkg-config libssl-dev librust-alsa-sys-dev`
2. `cargo build --release --features speech_dispatcher_0_11`

#### Ubuntu 18

1. `apt install clang cmake speech-dispatcher libspeechd-dev pkg-config libssl-dev alsa`
2. `cargo build --release --features speech_dispatcher_0_9`

#### Ubuntu 20

1. `apt install clang cmake speech-dispatcher libspeechd-dev pkg-config libssl-dev alsa librust-alsa-sys-dev`
2. `cargo build --release --features speech_dispatcher_0_9`

#### Ubuntu 22

1. `apt install clang cmake speech-dispatcher libspeechd-dev pkg-config libssl-dev alsa librust-alsa-sys-dev`
2. `cargo build --release --features speech_dispatcher_0_11`

### MacOS

1. `cargo install cargo-bundle`
2. `cargo bundle --release`

### Windows

1. `cargo build --release`

## Tests

To test, just `cargo test --all`.

Sometimes when debugging, it's useful to create the same initial setup every time. To do this, you can pass input events in like this: `cargo run -- --events events.txt`

...where the contents of `events.txt` is something like:

```
NextPanel
AddTrack
EnableSoundFontPanel
SelectFile
```

## Upload

Assuming that you are Esther Alter and you have the relevant credentials on your computer, you can upload the website and create itch.io builds by doing this:

1. `cd py`
2. `py -3 build.py`
