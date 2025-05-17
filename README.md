# Kek Repository Serializer

## Overview

The Kek Repository Serializer is a command-line tool designed to scan a directory (typically a software repository), categorize files based on configurable glob patterns, and serialize this information, including file content, into a structured, pseudo-XML format streamed to standard output. This output is intended to be piped to other processes for further analysis or ingestion.

Key features include:
*   **File Categorization**: Files are categorized into `docs`, `src`, or `other` based on user-defined or default glob patterns.
*   **Configurable Globs**: Glob patterns for `docs` and `src` categories can be customized via a `kek.toml` file.
*   **Ignore File Support**: Respects `.gitignore` and a custom `.kekignore` file for excluding files and directories from processing.
*   **Parallel Processing**: Utilizes parallel directory traversal for improved performance on multi-core systems.
*   **Efficient Output**: Uses `sendfile` (on supported platforms) for zero-copy streaming of file content, minimizing overhead.
*   **Task Arguments**: Allows passing arbitrary command-line arguments that are included in the output, useful for associating context with the serialized data.

## Prerequisites

*   Rust toolchain (stable version recommended).
*   Build tools (typically `cargo`).

## Building

Clone the repository and build the project using Cargo:

```bash
# git clone <repository_url>
# cd <repository_directory>
cargo build --release
```
The executable will be located at `target/release/kek` (or the package name if different). For simplicity, this document will refer to the executable as `kek`.

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

```bash
export KEK_CONFIG=/path/to/my/custom_kek.toml
./kek |
```

If `kek.toml` is not found and `KEK_CONFIG` is not set, default glob patterns will be used.

### `kek.toml` Structure

The configuration file can define glob patterns for `docs` and `src` categories.

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

*   **`[category.docs]`**: An array of glob patterns for files considered documentation.
*   **`[category.src]`**: An array of glob patterns for files considered source code.

### Glob Pattern Behavior

*   **Case-Insensitive**: Glob patterns are matched case-insensitively.
*   **Path Separators**: `*` can match path separators (e.g., `a*b` can match `a/x/b`).
*   **Hidden Files**: Standard glob `*` (e.g., `*.log`) might not match files starting with a dot (e.g., `.hidden.log`). To match hidden files, patterns must explicitly account for the leading dot (e.g., `.*.log` or specific names like `.env`). The default internal glob patterns for extensions automatically include variants for hidden files (e.g., generating both `**/*.ext` and `**/.*.ext`).
*   **Error Handling**: If any glob pattern in the configuration is invalid, the program will fail to start, reporting a configuration error.

### Default Glob Patterns

If `kek.toml` is not found or specific categories are not defined, extensive default lists of glob patterns are used for `docs` and `src` categories. These defaults cover a wide range of common file extensions and names associated with documentation and source code.

*   **Default Docs Globs**: Include common documentation extensions (e.g., `.md`, `.txt`, `.pdf`, `.html`), Readme files, Contribution guidelines, etc.
*   **Default Src Globs**: Include a very broad range of programming language extensions, build system files (e.g., `Makefile`, `pom.xml`, `package.json`), configuration files (e.g., `.json`, `.yaml`, `.toml`), and other development-related files.

Files not matching any `docs` or `src` globs are categorized as `other`.

## Output Format

The program outputs a pseudo-XML structure. **Note**: This is not strictly valid XML; for example, special characters in file paths or content are not escaped.

The general structure is as follows:

```xml
<category>
  <description>Category description text</description>
  <files>
    <file>
      <path>relative/path/to/file.ext</path>
      <content>
Raw content of the file...
      </content>
    </file>
    <!-- More <file> entries -->
  </files>
</category>
<!-- More <category> entries -->
<task>Optional task arguments joined by spaces</task>
```
