# gui.nvim

A Neovim GUI focusing on simplicity and performance.

**Work in progress**. Compiles and works only on macOS for now.

## Features

- **GPU rendered with [wgpu](https://github.com/gfx-rs/wgpu)** - supports Metal, Vulkan, and DX12 backends automatically.
- **Low input latency** - my biggest reason to use a GUI instead of a terminal. Terminals are great, but they are 1970s tech. Parsing escape sequences really slows down the pipeline, especially on TUIs like Neovim. A GUI can render the cells directly, without any escaping.
- **Cross-platform (soon)** - the dependencies are all cross-platform, but I haven't written the code to make it work on Windows or Linux yet.
- **Uses your existing Neovim installation** - it just gets `nvim` from your PATH.
- **macOS environment variables handling** - for macOS, there is a `gui-nvim env` command that generates environment variables based on your shell, overcoming the limitation of macOS GUI apps not inheriting shell environment.

## Requirements

- macOS 11.0 (Big Sur) or later
- [Neovim](https://neovim.io/) installed and available in your PATH
- [Rust toolchain](https://rustup.rs/) (for building from source)

## Installation

### From Github releases

Download the latest nightly release from the [releases page](https://github.com/fdemb/gui.nvim/releases).
After downloading, unzip the file and move the app to your Applications folder.

Note: you may need to remove the quarantine attribute from the downloaded file:

```sh
xattr -d com.apple.quarantine /Applications/gui.nvim.app
```

This is required because macOS blocks unsigned applications from running.
I don't have an Apple Developer account, so I can't sign the app for now.

### From source

```sh
# Clone the repository
git clone https://github.com/fdemb/gui.nvim.git
cd gui.nvim

# Install cargo-bundle if you don't have it
cargo install cargo-bundle

# Build and create the app bundle
cargo bundle --release

# Install to /Applications
cp -r target/release/bundle/osx/gui.nvim.app /Applications/
```

## Usage

Just run it from the macOS Finder or Spotlight.

You can also use it via the command line, but it's not intalled automatically to PATH yet.

Examples:

```sh
# Launch gui.nvim
gui-nvim

# Open a file
gui-nvim file.txt

# Pass arguments to Neovim
gui-nvim --clean file.txt
```

### macOS environment setup

When launching from Finder or Spotlight, GUI apps don't inherit your shell's environment variables (PATH, etc.). To fix this:

```sh
# Run this once from your terminal
gui-nvim env
```

This captures your shell environment, including PATH modifications from version managers like nvm, rbenv, pyenv, mise, and asdf.

## Configuration

gui.nvim reads configuration from `~/.config/gui-nvim/config.toml` (or `$XDG_CONFIG_HOME/gui-nvim/config.toml`).

```toml
[font]
family = "JetBrains Mono"  # Font family (uses guifont from Neovim if not set)
size = 14.0                # Font size in points

[performance]
vsync = "enabled"          # "enabled", "disabled", or "mailbox_if_available"
```

You can also set the font in Neovim using `guifont`:

```vim
set guifont=Fira\ Code:h14
```

## Acknowledgments

This project was inspired by and learned from:

- [Alacritty](https://github.com/alacritty/alacritty) - GPU-accelerated terminal emulator using OpenGL, written in Rust
- [Ghostty](https://github.com/ghostty-org/ghostty) - Fast, native terminal emulator written in Zig
- [Neovide](https://github.com/neovide/neovide) - Neovim GUI written in Rust, uses Skia for rendering

## License

MIT
