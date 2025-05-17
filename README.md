# kek 

A repository serializer inspired by yek.

## Usage

### Basic Command

The program's output **must** be piped to another command or redirected to a file. Running it directly with output to a TTY will result in an error.

```bash
./kek |
```
Or redirect to a file:
```bash
./kek > output.xml
```

The program processes files in the current working directory by default.

### Task Arguments

You can pass additional arguments to `kek`. These arguments will be joined by spaces and included in the output within `<task>...</task>` tags.

```bash
./kek some important context for this run |
```

This would append `<task>some important context for this run</task>` to the output.

## Configuration

Configuration is managed via a TOML file, by default named `kek.toml` in the current working directory. Alternatively, the path to the configuration file can be specified using the `KEK_CONFIG` environment variable.

```toml
# Example kek.toml

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
