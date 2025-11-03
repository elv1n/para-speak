# Changelog

## 2025-11-03

### Features
- Add experimental real-time transcription mode with continuous speech-to-text processing during recording

## 2025-09-16

### Features
- Multiple model support - ability to switch between different ML models
- Experimental Canary model support (nvidia/canary-1b-v2) added
- Initial UI crate foundation (in design phase)

## 2025-09-13

### Features
- Multiple model support
  - Configure ML model via `PARA_MODEL` environment variable
  - Support for various Parakeet model variants (0.6b-v3, 1.1b, ctc-0.6b, ctc-1.1b)
  - `--force` flag to allow experimental/unsupported models

- Add transcription post-processing with configurable text replacements
  - Configure via `PARA_REPLACE` environment variable
  - Syntax: `"from:to"` for replacement, `"from:"` or `"from"` for removal
  - Multiple replacements separated by semicolons
  - Word-boundary aware matching (only whole words are replaced)
  - Uses Aho-Corasick algorithm for efficient pattern matching
  - Include default_replacements for common filler words (Uh, uh, ah, oh, um, Um, Oh, so. (with a dot))

- Added `verify-cli list` command to display downloaded ML models with sizes 

### Improvements
- Audio recording now captures last 500ms to prevent cutting off speech
- Enhanced text replacements including "if if" -> "if" correction

### Bug Fixes
- Play error sound when recording is not started