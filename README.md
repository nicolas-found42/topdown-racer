# Topdown Racer

## Prerequisites

- Rust (stable toolchain)
- Bevy prerequisites for your platform (if you run into runtime issues)

## Quick setup

From repository root:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
```

Because this repository includes `rust-toolchain.toml`, `cargo` will prefer the pinned
`stable` toolchain automatically once installed.

## Sanity check commands

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo run --package topdown-racer
```

Start a three-lap Race against three AI Opponents with Enter or the Start Race
button. Choose the opponent pace in the menu with 1–3.

| Control | Action |
| --- | --- |
| W / Up | Throttle |
| S / Down | Brake; keep holding at a stop to reverse |
| A / Left, D / Right | Steer |
| Space | Handbrake |
| T | Switch Manual / Autopilot; release driving keys after switching |
| Esc | Pause during a Race; return to menu from results |
| Enter / R / M / C while paused | Resume / Restart / Menu / Recover Car |
| F / V in menu | Raw or Smooth steering / Centered or Look Ahead camera |
| P in menu | Corner Practice: repeatable approach, section time and exit speed |
| B | Sound volume: 100%, 50%, muted |
| R / Enter on results | Restart with a fresh countdown |
| Q in the menu / Quit button | Quit the game |

The menu shows the saved best eligible Manual flying lap for the current Track
and steering policy. Autopilot laps, recovered laps, and the opening lap from
the grid cannot set that record. Eligible improvements save at lap completion
to `topdown-racer/records-v2.json` under the platform config directory (on macOS,
`~/Library/Application Support`). The legacy `best_lap.txt` is retained separately
and is not compared against current records.

Pause freezes Race time. Resume has a one-second preparation interval. Recovery
requires speed below 1 world unit/s and an unoccupied destination; it invalidates
the current lap and has a three-second simulation-time cooldown.


Corner Practice starts one Car at the same rolling speed before the braking and
right-left sequence. Cross the cyan approach gate, each orange route gate, and
the white exit gate in order. Results compare elapsed section time and exit
speed; R or Enter retries. Its records are separate from Race laps. Raw steering
remains the default; Smooth uses roughly 100 ms rise with faster release/reversal.
Human preference and audio audition are not inferred from automated tests.
