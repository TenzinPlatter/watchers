# Watchers

A file system watcher that automatically creates git commits when files change.

## Features

- **Debounced commits**: Only creates commits after file activity stops
- **Automatic push**: Optionally push commits to remote repository

## Quick Start

1. Create a configuration file:
```yaml
# config/config.yml
watch_dir: "/path/to/your/project"
commit_delay_secs: 3
auto_push: true
```

2. Run the watcher:
```bash
cargo run
```

The watcher will monitor the specified directory and automatically create commits when files change, waiting for the configured delay period to ensure no rapid-fire commits.

## Configuration

- `watch_dir`: Directory to monitor for changes
- `commit_delay_secs`: Seconds to wait after last change before committing
- `auto_push`: Whether to automatically push commits to remote

## Development

```bash
# Build
cargo build

# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test
```
