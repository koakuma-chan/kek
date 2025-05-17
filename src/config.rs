use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

pub const DOCS_DESCRIPTION: &str = "Immutable documentation. Provided FOR REFERENCE ONLY.";
pub const SRC_DESCRIPTION: &str = "Source code files.";
pub const OTHER_DESCRIPTION: &str = "Other files.";

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct TomlCategoryGlobs {
    #[serde(default = "default_docs_globs_str_vec")]
    docs: Vec<String>,
    #[serde(default = "default_src_globs_str_vec")]
    src: Vec<String>,
}

impl Default for TomlCategoryGlobs {
    fn default() -> Self {
        Self {
            docs: default_docs_globs_str_vec(),
            src: default_src_globs_str_vec(),
        }
    }
}

/// Defines the root structure of the TOML configuration file.
#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
struct TomlConfig {
    /// Contains specifications for glob patterns belonging to different categories.
    #[serde(default)]
    category: TomlCategoryGlobs,
    /// List of directories to scan. Paths can be absolute or relative to the current working directory.
    /// If omitted, defaults to the current working directory ["."].
    #[serde(default = "default_scan_str_vec")]
    scan: Vec<String>,
}

fn default_scan_str_vec() -> Vec<String> {
    vec![".".to_string()]
}

// --- Default Glob Pattern Lists ---
// Business Logic Constraint: Default glob patterns are provided for 'docs' and 'src' categories.
// These are used if not overridden in the configuration file.
// Defaults include common extensions (as "*.ext" and ".*.ext") and common exact filenames.

fn default_docs_globs_str_vec() -> Vec<String> {
    let extensions = [
        "md", "txt", "mdx", "rst", "adoc", "asciidoc", "textile", "rtf", "tex", "log", "me", "pdf",
        "doc", "docx", "odt", "ppt", "pptx", "odp", "html", "htm", "man", "1", "2", "3", "4", "5",
        "6", "7", "8", "info",
    ];
    let exact_filenames = [
        "README",
        "README.md",
        "README.rst",
        "README.txt",
        "CONTRIBUTING",
        "CONTRIBUTING.md",
    ];

    extensions
        .into_iter()
        .flat_map(|ext| [format!("**/*.{}", ext), format!("**/.*.{}", ext)])
        .chain(exact_filenames.into_iter().map(String::from))
        .collect()
}

fn default_src_globs_str_vec() -> Vec<String> {
    let extensions = [
        "rs",
        "py",
        "js",
        "ts",
        "java",
        "c",
        "cpp",
        "h",
        "hpp",
        "go",
        "rb",
        "php",
        "swift",
        "kt",
        "kts",
        "cs",
        "pl",
        "lua",
        "r",
        "m",
        "scala",
        "groovy",
        "dart",
        "fs",
        "fsx",
        "fsi",
        "erl",
        "hrl",
        "ex",
        "exs",
        "elm",
        "clj",
        "cljs",
        "cljc",
        "edn",
        "hs",
        "lhs",
        "purs",
        "idr",
        "agda",
        "vb",
        "vbs",
        "pas",
        "d",
        "nim",
        "cr",
        "zig",
        "vala",
        "asm",
        "s",
        "sh",
        "bash",
        "zsh",
        "fish",
        "ps1",
        "bat",
        "cmd",
        "tcl",
        "awk",
        "applescript",
        "json",
        "yaml",
        "yml",
        "toml",
        "xml",
        "ini",
        "cfg",
        "conf",
        "env",
        "properties",
        "reg",
        "neon",
        "plist",
        "sql",
        "ddl",
        "dml",
        "psql",
        "plsql",
        "hql",
        "cql",
        "mk",
        "cmake",
        "gradle",
        "sbt",
        "csproj",
        "vbproj",
        "fsproj",
        "sln",
        "vcproj",
        "vcxproj",
        "xcconfig",
        "xcscheme",
        "podspec",
        "gemspec",
        "mod",
        "sum",
        "work",
        "tf",
        "tfvars",
        "hcl",
        "nomad",
        "sentinel",
        "rego",
        "css",
        "scss",
        "less",
        "sass",
        "styl",
        "postcss",
        "pcss",
        "svg",
        "xsl",
        "xslt",
        "jsp",
        "asp",
        "aspx",
        "cshtml",
        "vbhtml",
        "php3",
        "php4",
        "php5",
        "phtml",
        "ejs",
        "xhtml",
        "vue",
        "svelte",
        "jsx",
        "tsx",
        "htc",
        "tag",
        "tld",
        "dockerfile",
        "containerfile",
        "graphql",
        "gql",
        "proto",
        "thrift",
        "avsc",
        "xsd",
        "wsdl",
        "raml",
        "oas2",
        "oas3",
        "asyncapi",
        "patch",
        "diff",
        "erb",
        "haml",
        "slim",
        "pug",
        "jade",
        "mustache",
        "hbs",
        "handlebars",
        "liquid",
        "njk",
        "jinja",
        "j2",
        "twig",
        "ipynb",
        "gd",
        "sol",
        "qml",
        "vert",
        "frag",
        "geom",
        "comp",
        "tesc",
        "tese",
        "glsl",
        "hlsl",
        "cg",
        "metal",
        "rules",
        "yml_rules",
        "feature",
    ];
    let exact_filenames = vec![
        "Makefile",
        "makefile",
        "GNUmakefile",
        "CMakeLists.txt",
        "build.gradle",
        "pom.xml",
        "setup.py",
        "Rakefile",
        "rakefile",
        "Gemfile",
        "gemspec",
        "Podfile",
        "Fastfile",
        "Brewfile",
        "SConstruct",
        "wscript",
        "BUILD",
        "WORKSPACE",
        "package.json",
        "composer.json",
        "Pipfile",
        "requirements.txt",
        "Cargo.toml",
        "go.mod",
        "go.sum",
        "package-lock.json",
        "pnpm-lock.yaml",
        "pyproject.toml",
        "Dockerfile",
        "docker-compose.yml",
        "docker-compose.yaml",
        "Jenkinsfile",
        "Vagrantfile",
        "Procfile",
        ".gitlab-ci.yml",
        ".editorconfig",
        ".gitattributes",
        ".gitmodules",
        "babel.config.js",
        "webpack.config.js",
        "tsconfig.json",
        "vite.config.js",
        "tailwind.config.js",
        "justfile",
        "main.tf",
        "variables.tf",
        "outputs.tf",
        "terraform.tfvars",
    ];

    extensions
        .into_iter()
        .flat_map(|ext| [format!("**/*.{}", ext), format!("**/.*.{}", ext)])
        .chain(exact_filenames.into_iter().map(String::from))
        .collect()
}

/// Application configuration, derived from `TomlConfig`.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub docs: GlobSet,
    pub src: GlobSet,
    pub scan: Vec<PathBuf>,
}

/// Helper function to build a GlobSet from a list of pattern strings.
/// Configures globs to be case-insensitive and allows '*' to match path separators.
fn build_glob_set(glob_strings: &[String], category_name: &str) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for glob_str in glob_strings {
        // Business Logic Constraint: Globs are case-insensitive.
        // Business Logic Constraint: '*' in globs can match path separators like '/'.
        let glob = GlobBuilder::new(glob_str)
            .case_insensitive(true)
            .literal_separator(false)
            .build()
            .map_err(|e| {
                format!(
                    "Invalid glob pattern in '{}' -> \"{}\": {}",
                    category_name, glob_str, e
                )
            })?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|e| format!("Failed to build glob set for '{}': {}", category_name, e))
}

/// Loads the application configuration from the TOML file specified by the KEK_CONFIG environment variable,
/// or `kek.toml` by default.
///
/// The configuration file can specify:
/// - `scan`: A list of paths to scan. Defaults to `["."]` (current working directory).
///   Paths are relative to the current working directory unless absolute.
/// - `category.docs`: Glob patterns for 'docs' category.
/// - `category.src`: Glob patterns for 'src' category.
///
/// If the configuration file doesn't exist or specific settings are omitted, defaults are used.
///
/// Business Logic Constraint: Glob patterns are matched case-insensitively against relative file paths.
/// Business Logic Constraint: Glob compilation errors will cause configuration loading to fail.
pub fn load_config() -> Result<AppConfig, String> {
    let config_path_str = env::var("KEK_CONFIG").unwrap_or_else(|_| "kek.toml".to_string());
    let config_path = PathBuf::from(config_path_str);

    let toml_config: TomlConfig = if !config_path.exists() {
        TomlConfig::default()
    } else {
        let config_content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file {:?}: {}", config_path, e))?;

        toml::from_str(&config_content).map_err(|e| {
            format!(
                "Failed to parse configuration file (TOML) {:?}: {}",
                config_path, e
            )
        })?
    };

    let docs_globset = build_glob_set(&toml_config.category.docs, "docs")?;
    let src_globset = build_glob_set(&toml_config.category.src, "src")?;

    let scan: Vec<PathBuf> = toml_config
        .scan
        .into_iter()
        .map(PathBuf::from)
        .collect();
    
    // Business Logic Constraint: If scan is empty after parsing (e.g. user provided empty list),
    // it should behave as if default ["."] was used.
    let scan = if scan.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        scan
    };


    // Business Logic Constraint: A file path will be categorized by the first matching glob list,
    // in the order of 'docs', then 'src'. If a file matches globs from both lists,
    // it will be categorized as 'docs' due to this precedence.

    Ok(AppConfig {
        docs: docs_globset,
        src: src_globset,
        scan
    })
}
