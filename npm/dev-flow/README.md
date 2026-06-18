# @xin_yue/dev-flow

Engineering discipline for coding agents. Lightweight workflow management CLI.

## Install

```bash
npm install -g @xin_yue/dev-flow
```

This downloads the prebuilt `dow` binary for your platform from [GitHub Releases](https://github.com/daphnee-ovo/dev-flow/releases).

## Usage

```bash
dow setup          # Register with your coding agent
dow --help         # Show available commands
```

Then in your project, ask your coding agent:

```
/init
/task
```

## Supported Platforms

- Linux x86_64
- Linux aarch64
- macOS arm64 (Apple Silicon)
- Windows x86_64

## Alternative Install Methods

```bash
# Cargo (Rust toolchain required)
cargo install dev-flow

# Homebrew
brew install daphnee-ovo/tap/dev-flow

# Shell script
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash
```

## Documentation

See [GitHub repository](https://github.com/daphnee-ovo/dev-flow) for full documentation.
