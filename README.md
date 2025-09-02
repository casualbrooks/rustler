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
