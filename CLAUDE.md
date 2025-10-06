# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build the project
cargo build

# Run the CLI
cargo run -- <command>

# Enable debug logging (useful for development)
RUST_LOG=debug cargo run -- <command>

# Check compilation without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

## CLI Commands

This is a CLI application with multiple subcommands for managing file watchers:

```bash
# Create a new watcher (prompts for directory path)
watchers create <name>

# Start a watcher (starts systemd service)
watchers start <name>

# Stop a watcher (stops systemd service)
watchers stop <name>

# Delete a watcher configuration
watchers delete <name>

# List all configured watchers
watchers list

# Run daemon directly (hidden command, used by systemd)
watchers __daemon <name>
```

## Configuration

Watcher configurations are stored in YAML files at:
- Linux: `~/.config/watchers/<name>.yml` (or `.yaml`)

Each config file contains:
- `watch_dir`: Directory to watch for file changes
- `commit_delay_secs`: Debounce delay in seconds before creating commits
- `auto_push`: Whether to auto-push commits after creation

## Architecture Overview

This is a CLI-based file system watcher that automatically creates git commits when files change. It uses systemd user services to run watchers as background daemons.

### Core Components

**CLI Layer (`src/cli.rs`, `src/main.rs`)**
- Clap-based CLI with subcommands for watcher lifecycle management
- Commands dispatch to corresponding functions in `src/watcher.rs`
- Async runtime (Tokio) for systemd interaction

**Watcher (`src/watcher.rs`)**
- Main orchestrator that sets up file system watching using `notify` crate
- Contains a `Debouncer` instance that delays commit creation until file activity stops
- Provides CRUD operations for watcher management (create, start, stop, delete, list)
- `run_daemon()` is the entry point for the background daemon process

**Debouncer (`src/debouncer.rs`)**
- Thread-safe timer mechanism using condition variables and mutexes
- Cancels previous timers when new events occur
- Executes callback after quiet period (no new events for configured delay)
- Uses `EventContext` to pass data to callbacks
- Generic over callback function type

**Git Operations (`src/git.rs`)**
- `EventContext`: Helper struct that carries `repo_path` and `config` to callbacks
- `handle_event()`: Called by debouncer after quiet period, creates commits and optionally pushes
- `get_commit_message()`: Generates structured commit messages showing deleted/modified/added files
- `push_commits()`: Handles push to remote using SSH keys (hardcoded to `~/.ssh/id_ed25519`)
- Repository is opened fresh in each callback to avoid thread safety issues with `git2::Repository`

**Systemd Integration (`src/systemd.rs`)**
- Uses `zbus` for D-Bus communication with systemd
- Manages systemd user services via `ManagerProxy`
- Template unit file (`watchers@.service`) allows multiple named instances
- Unit file is installed to `~/.config/systemd/user/` on first use

**File Utilities (`src/file_utils.rs`)**
- `was_modification()`: Filters file system events to only modification types (Create, Modify, Remove)

**Config (`src/config.rs`)**
- Loads configurations from YAML files in user config directory
- Default `commit_delay_secs` is 60 seconds
- Default `auto_push` is true

### Data Flow

1. User runs `watchers create <name>` which:
   - Prompts for directory path
   - Creates config file at `~/.config/watchers/<name>.yml`
   - Starts and enables systemd service

2. Systemd service runs `watchers __daemon <name>` which:
   - Loads config for the named watcher
   - Creates `Watcher` instance with `handle_event` callback
   - Starts file system watching with `notify` crate

3. On file changes:
   - `notify` detects events and filters via `was_modification()`
   - Each modification triggers `Debouncer::on_event()` with `EventContext`
   - Debouncer resets timer on each new event
   - After quiet period (no events for `commit_delay_secs`), calls `handle_event()`

4. In `handle_event()`:
   - Opens repository (or creates if doesn't exist)
   - Gets changed files via `git2` status
   - Generates commit message listing all changes
   - Creates commit with all changes staged
   - If `auto_push` is enabled, pushes to remote using SSH key

### Key Design Decisions

- **Multiple named watchers**: Each watcher is a separate systemd service instance using template units (`watchers@.service`)
- **Thread-safe debouncing**: Uses condition variables for efficient timer cancellation across threads
- **Repository handling**: Opens repository fresh in callbacks to avoid thread safety issues
- **Event filtering**: File modification check happens before debouncer, not in git handler
- **Systemd integration**: Uses D-Bus instead of shelling out to `systemctl`
- **SSH authentication**: Hardcoded to `~/.ssh/id_ed25519` for git push operations

### Known Limitations

- No support for submodules yet (TODO in code)
- No file ignore patterns implemented yet (TODO in code)
- SSH key path is hardcoded
- No tests written yet
