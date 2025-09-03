# Rustler Project

## Prerequisites
### Install Rust and Cargo

**Linux or macOS**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows**

Download and run [rustup-init.exe](https://win.rustup.rs/) or install via
the Windows package manager:

```powershell
winget install -e --id Rustlang.Rustup
```

After installation, restart your terminal and verify the tools are available:

```bash
rustc --version
cargo --version
```

## Running the Project
To launch the Five-Card Draw Poker CLI:

```bash
cargo run --manifest-path poker_draw_cli/Cargo.toml
```

On Windows PowerShell, use the same command:

```powershell
cargo run --manifest-path poker_draw_cli/Cargo.toml
```

For a release build that produces an executable:

```bash
cargo build --manifest-path poker_draw_cli/Cargo.toml --release
# Run ./poker_draw_cli/target/release/poker_draw_cli
# On Windows: .\poker_draw_cli\target\release\poker_draw_cli.exe
```

## Hand History Logging
Every game session generates a unique table name and records each hand as it
plays out. Public actions such as bets, folds, and pots are timestamped and
emitted to stdout at the end of a match. A separate private log captures each
player's hidden cards and final hands so the complete game can be replayed
later. Each entry includes a millisecond timestamp so action order and replay
timing can be reconstructed precisely.
