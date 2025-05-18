# kek 

A repository serializer inspired by [yek](https://github.com/bodo-run/yek).

#### Features
- Outputs pseudo-XML.
- Uses `sendfile` for blazingly fast performance.
- Categorises files as `docs`, `src` or `other` (helps the model).
- Respects `.gitignore` and `.kekignore`

## Installation

```bash
cargo install --git https://github.com/koakuma-chan/kek
```

## Usage

Serialize the repository and write to the clipboard

```bash
kek | clip.exe
```

Add a task prompt

```bash
kek "Optimize code." | clip.exe # Adds <task>Optimize code.</task> at the end of the output.
```

Debug which files are included
```bash
kek | grep -A2 "<path>"
```

## Configuration

Configuration is managed via `kek.toml`. Alternatively, the path can be specified using the `KEK_CONFIG` environment variable.

```toml
# Example kek.toml (these aren't the defaults)

scan = [
    "../migrations", # include shared SQL migrations
    "." # include this project's files
]

[category]
# Globs for the 'docs' category.
# These are matched case-insensitively against relative file paths.
docs = [
    "*.md",
    "docs/**/*.txt",
    "LICENSE"
]

# Globs for the 'src' category.
# Matched case-insensitively.
src = [
    "*.rs",
    "src/**/*.js",
    "Makefile",
    "**/.*.conf" # Example to match hidden .conf files
]
```
