# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build the project
cargo build

# Run the watcher
cargo run

# Enable debug logging (useful for development)
RUST_LOG=debug cargo run

# Check compilation without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Configuration

The application reads configuration from `./config/config.yml`. Required fields:
- `watch_dir`: Directory to watch for file changes
- `commit_delay_secs`: Debounce delay in seconds before creating commits
- `auto_push`: Whether to auto-push commits (currently not implemented)

Example config:
```yaml
watch_dir: "/path/to/watch"
commit_delay_secs: 3
auto_push: true
```

## Architecture Overview

This is a file system watcher that automatically creates git commits when files change. The architecture consists of:

### Core Components

**Watcher (`src/lib.rs`)**
- Main orchestrator that sets up file system watching
- Contains a `Debouncer` instance that delays commit creation until file activity stops
- Takes a callback function that processes file change events

**Debouncer (`src/debouncer.rs`)**
- Thread-safe timer mechanism using condition variables
- Cancels previous timers when new events occur
- Executes callback after quiet period (no new events for configured delay)
- Uses `EventContext` to pass data to callbacks

**EventContext (`src/git.rs`)**
- Helper struct that carries `repo_path` and `config` to timer callbacks
- Solves thread safety issues by avoiding shared repository references
- Repository is opened fresh in each callback execution

### Flow

1. File system events are detected by `notify` crate
2. Events are filtered (`was_modification` check) before triggering debouncer
3. Each event triggers debouncer with `EventContext`
4. After quiet period, debouncer calls `handle_event` with git repository access
5. Git operations: detect changes, generate commit message, create commit

### Key Design Decisions

- **Thread-safe debouncing**: Uses condition variables for efficient timer cancellation
- **Repository handling**: Opens repository fresh in callbacks to avoid thread safety issues with git2::Repository
- **Event filtering**: File modification check happens before debouncer, not in git handler
- **Logging**: Uses `log` crate with `env_logger` - control via `RUST_LOG` environment variable

### Future TODOs
- Implement auto-push functionality
- Add submodule support
- Add file ignore patterns
- Implement fetch/autoupdate features