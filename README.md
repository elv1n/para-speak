# Para-Speak

Local speech-to-text CLI tool powered by NVIDIA Parakeet model. Minimal idle footprint, powerful customizable shortcuts, audio feedback, and extensible controller API for custom integrations.

 Built in Rust for speed and minimal resource usage, it integrates Python ML models (Parakeet MLX) optimized for Apple Silicon.

**Note**: Para-speak is in its early stages and available on macOS only. Many decisions are still being made, and it will mature over time.

## Goals

- **Fast**: Rust-based with optimized audio pipeline, minimal latency
- **Lightweight**: Models load on-demand and release when idle to minimize resource usage  
- **Flexible**: Advanced keyboard shortcuts with sequences, combinations, and double-tap support
- **Practical**: Accurate enough for a daily use

## Quick Start

```bash
# 1. Set up environment and download model (first time only)
cargo run -p verify-cli

# 2. Run Para-speak:
./para-speak

# Note: Direct `cargo run -p para-speak-cli` requires PYO3_PYTHON env var
```

## Features

- Global keyboard shortcuts with advanced pattern support
- Automatic text insertion at cursor position
- Audio feedback for recording states
- Spotify volume control during recording
- Pause/resume recording

## Configuration

All settings are configured via environment variables. Create a `.env.local` file or export them in your shell.

**Important**: Para-speak only listens for keyboard shortcuts - it doesn't consume them! The keypress events still pass through to your system, so choose shortcuts that won't conflict with your other applications.

### Default Shortcuts

Para-speak comes with built-in default shortcuts:

- **Start recording**: `ControlLeft + ControlLeft` (double tap)
- **Stop recording**: `ControlLeft`
- **Cancel recording**: `Escape + Escape` (double tap)
- **Pause/resume**: No default shortcut

**Note**: Make sure double Control doesn't conflict with macOS dictation shortcut at `Keyboard > Dictation > Shortcut`

### Custom Configuration

You can override the defaults using environment variables. Create a `.env.local` file in the root of the project directory:

```bash
# Keyboard shortcuts
PARA_START_KEYS="double(ControlLeft, 300); CommandLeft+ShiftLeft+KeyY"
PARA_STOP_KEYS="ControlLeft; CommandLeft+ShiftLeft+KeyY"
PARA_CANCEL_KEYS="double(Escape, 300)"
PARA_PAUSE_KEYS="CommandLeft+Alt+Shift+KeyU"

# Core functionality
PARA_PASTE=true                          # Auto-paste transcribed text at cursor

# Spotify integration
PARA_SPOTIFY_RECORDING_VOLUME=30         # Set Spotify to specific volume (0-100)
PARA_SPOTIFY_REDUCE_BY=50                # OR reduce volume by amount (0-100)

# Transcription behavior
PARA_TRANSCRIBE_ON_PAUSE=true            # Experimental: transcribe when pausing (not just on stop)

# Advanced
PARA_SHORTCUT_RESOLUTION_DELAY_MS=50     # Delay for resolving shortcut conflicts
PARA_MEMORY_MONITOR=true                 # Enable memory usage reporting

# Debugging
PARA_DEBUG=true                          # Enable debug mode with verbose output
```

### All Configuration Options

| Option | Environment Variable | Description | Default |
|--------|---------------------|-------------|---------|
| `--debug` | `PARA_DEBUG` | Enable debug mode with verbose logging | `false` |
| `--paste` | `PARA_PASTE` | Automatically paste transcribed text at cursor | `false` |
| `--start-keys` | `PARA_START_KEYS` | Semicolon-separated list of key combinations to start recording | `double(ControlLeft, 300)` |
| `--stop-keys` | `PARA_STOP_KEYS` | Semicolon-separated list of key combinations to stop recording | `ControlLeft` |
| `--cancel-keys` | `PARA_CANCEL_KEYS` | Semicolon-separated list of key combinations to cancel recording | `double(Escape, 300)` |
| `--pause-keys` | `PARA_PAUSE_KEYS` | Semicolon-separated list of key combinations to pause recording | None |
| `--spotify-recording-volume` | `PARA_SPOTIFY_RECORDING_VOLUME` | Set Spotify to specific volume (0-100) during recording | None |
| `--spotify-reduce-by` | `PARA_SPOTIFY_REDUCE_BY` | Reduce Spotify volume by amount (0-100) during recording | None |
| `--transcribe-on-pause` | `PARA_TRANSCRIBE_ON_PAUSE` | Transcribe when pausing (not just on stop) | `false` |
| `--shortcut-resolution-delay-ms` | `PARA_SHORTCUT_RESOLUTION_DELAY_MS` | Delay for resolving shortcut conflicts (ms) | `50` |
| `--memory-monitor` | `PARA_MEMORY_MONITOR` | Enable memory usage reporting | `false` |

## Shortcut Syntax

The shortcut system supports complex patterns:

- **Single key**: `"F1"` or `"Escape"` or `"ControlLeft"`
- **Combination**: `"Cmd+Shift+A"` or `"CommandLeft+ShiftLeft+KeyY"` (all pressed together)
- **Double-tap**: `"double(ControlLeft, 300)"` (double-tap within 300ms)

Multiple shortcuts can be assigned to each action using semicolons:
```bash
export PARA_START_KEYS="F1; double(ControlLeft, 300)"  # F1 OR double-tap control
```

## Extensibility & Controllers

Para-speak uses a controller system that makes it easy to extend functionality. C

The [Spotify controller](https://github.com/elv1n/para-speak/tree/main/crates/integrations/components/src/spotify.rs) is one example - it adjusts music volume during recording. The same pattern can be used to build any type of asynchronous integration, or trigger any automation after recording is transcribed.

## Usage

1. Optionally set up custom environment variables (or use defaults)
2. Run the application:
   ```bash
   ./para-speak
   ```
3. Use your configured shortcut for recording start, stop, pause and resume.
4. Text appears at your cursor (if paste is enabled), copied to clipboard and printed to console (if debug is enabled)


## Architecture

```
┌─────────────────┐         ┌──────────────────┐
│   Rust Core     │  PyO3   │  Python ML       │
├─────────────────┤◄───────►├──────────────────┤
│ • Audio capture │         │ • Parakeet MLX   │
│ • Shortcuts     │         │ • Model loading  │
│ • System APIs   │         │ • Transcription  │
│ • Components    │         │                  │
└─────────────────┘         └──────────────────┘
```

The Rust core handles all system integration and performance-critical paths, while Python handles ML inference using MLX framework optimized for Apple Silicon.

## Platform Support

Para-speak is designed to be cross-platform with support for multiple models in future, though currently available for macOS only.

### Required Permissions

- **Microphone**: For audio capture
- **Accessibility**: For global keyboard shortcuts to work system-wide

## License

MIT