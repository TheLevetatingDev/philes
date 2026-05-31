# Philes

A minimal, fast file manager for macOS built with Rust and [Iced](https://github.com/iced-rs/iced).

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Requirements

- macOS
- [Rust](https://rustup.rs) (1.70 or newer)

## Installation

Clone the repo and build with Cargo:

```bash
git clone https://github.com/TheLevetatingDev/philes
cd philes
cargo build
```

Run it:

```bash
cargo run
```

Or run the compiled binary directly:

```bash
./target/release/philes
```

## Usage

| Action | Result |
|---|---|
| Single click | Select a file or folder |
| Shift + click | Extend selection to include a range |
| Double click | Open folder / open file in default app |
| ↑ Up button | Navigate to parent directory |

## Dependencies

| Crate | Purpose |
|---|---|
| [iced](https://crates.io/crates/iced) | GUI framework |
| [file_icon_provider](https://crates.io/crates/file_icon_provider) | Native macOS file icons |
| [open](https://crates.io/crates/open) | Open files in their default app |
| [dirs](https://crates.io/crates/dirs) | Locate the user's home directory |
