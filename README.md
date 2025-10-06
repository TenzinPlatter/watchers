# Watchers

A CLI tool for managing file system watchers that automatically create git commits when files change. Each watcher runs as a systemd user service, allowing you to monitor multiple directories simultaneously.

## Features

- **Multiple named watchers**: Create and manage multiple watchers, each monitoring a different directory
- **Debounced commits**: Only creates commits after file activity stops
- **Automatic push**: Optionally push commits to remote repository
- **Systemd integration**: Watchers run as background services with automatic restart

## Installation

```bash
cargo install --path .
```
or
```bash
cargo install watchers
```

Build from source:

```bash
git clone https://github.com/TenzinPlatter/watchers.git
cd watchers
cargo build --release
```

## Quick Start

1. Create a new watcher:
```bash
watchers create my-project
# You'll be prompted for the directory path to watch
```

2. The watcher is now running! It will:
   - Monitor the specified directory for file changes
   - Wait for the configured delay after changes stop
   - Automatically create a git commit
   - Push to remote if `auto_push` is enabled

3. Manage your watchers:
```bash
# List all watchers
watchers list

# Stop a watcher
watchers stop my-project

# Start a watcher
watchers start my-project

# Delete a watcher
watchers delete my-project
```

## Configuration

Watcher configurations are stored as YAML files in `~/.config/watchers/<name>.yml`:

```yaml
name: my-project
watch_dir: /path/to/your/project
commit_delay_secs: 60  # Wait 60 seconds after last change
auto_push: true        # Automatically push commits
```

You can manually edit these files to adjust settings, then restart the watcher:

```bash
watchers stop my-project
watchers start my-project
```

## How It Works

1. Each watcher runs as a systemd user service (`watchers@<name>.service`)
2. The service monitors the configured directory for file changes
3. When changes occur a timer is started
4. If no changes occur for `commit_delay_secs` seconds then the changes are committed
6. If `auto_push` is enabled, the commit is pushed to the remote repository

## Development

```bash
# Build
cargo build

# Run a command
cargo run -- list
cargo run -- create test-watcher

# Run with debug logging
RUST_LOG=debug cargo run -- __daemon my-project

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Requirements

- Linux with systemd (uses systemd user services)
- Git repositories in watched directories
