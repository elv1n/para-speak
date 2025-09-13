# Changelog

## 2025-09-13

### Features
- Add transcription post-processing with configurable text replacements
  - Configure via `PARA_REPLACE` environment variable
  - Syntax: `"from:to"` for replacement, `"from:"` or `"from"` for removal
  - Multiple replacements separated by semicolons
  - Word-boundary aware matching (only whole words are replaced)
  - Uses Aho-Corasick algorithm for efficient pattern matching
  - Include default_replacements for common filler words (Uh, uh, ah, oh, um, Um, Oh, so. (with a dot)) 

### Bug Fixes
- Play error sound when recording is not started