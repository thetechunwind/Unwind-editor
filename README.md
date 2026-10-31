# Tech Unwind (desktop)

The Tech Unwind video editor as a native app written in Rust. Same animations, same code format, same look.

## Run it

1. Install Rust (https://rustup.rs). Use the current stable version.
2. Linux only: `sudo apt install libasound2-dev pkg-config libxkbcommon-dev libwayland-dev`
3. `cargo run --release`

To get a Windows or Mac app without installing anything, put this folder on GitHub. The included workflow
(`.github/workflows/build.yml`) builds an app for Windows, Mac and Linux and attaches it to the run.

## Exporting MP4

Install ffmpeg (https://ffmpeg.org) and make sure the `ffmpeg` command works in a terminal.
Without ffmpeg you can still export PNG frames, and ffmpeg is also used as a backup reader for audio formats
the app cannot open itself.

## Adding files

Drag images and audio onto the window, or onto a lane of the timeline to place them where you drop.
Audio: WAV, MP3, AAC/M4A, OGG, FLAC, AIFF, CAF and more are read directly. Anything else (Opus, WMA, AMR...)
is converted through ffmpeg if it is installed.

## Code

Open **Code** in the top bar, paste code and it builds the timeline. Edit either side and the other follows.
Files can be referred to by name once added, or by a path: `"file": "C:/clips/voice.wav"`.

## From the command line

    tech_unwind examples/binance-hack.json            open the editor with a code file
    tech_unwind render examples/binance-hack.json out.mp4
    tech_unwind frame examples/binance-hack.json 12.5 frame.png
    tech_unwind --sheet stills                        a still of every animation

## Keys

Space play/pause, S split, Ctrl+D duplicate, Delete remove, Left/Right step a frame (Shift: one second), Ctrl+Z undo.

## Not in this version yet

Video clips, and these animations from the web editor: line chart, donut chart, newspaper, news headline, map,
scroll, spin, hinge, leaderboard, lock, price card, YouTube recap and explainer. They draw a simple headline
card instead so a project that uses them still loads.
