use crate::config::{AppConfig, DOCS_DESCRIPTION, OTHER_DESCRIPTION, SRC_DESCRIPTION};

use std::path::{Path, PathBuf};

use lockfree::stack::Stack;

use ignore::WalkBuilder;

use rustc_hash::FxHashMap;

#[derive(Debug)]
pub struct FileData {
    pub relative_path: PathBuf,
    pub absolute_path: PathBuf,
    pub size: u64,
}

#[derive(Debug)]
pub struct CategoryData {
    pub description_text: String,
    pub files: Vec<FileData>,
    pub total_size: u64,
}

/// Strips the base path from a full path to get a relative path.
fn get_relative_path(base: &Path, full_path: &Path) -> Result<PathBuf, String> {
    full_path
        .strip_prefix(base)
        .map(|p| p.to_path_buf())
        .map_err(|e| {
            format!(
                "Failed to create relative path for {:?} (base {:?}): {}",
                full_path, base, e
            )
        })
}

/// Represents the three fixed categories for files.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum FileCategoryType {
    Docs,
    Src,
    Other,
}

impl FileCategoryType {
    /// Gets the fixed description for this category type.
    fn get_description(&self) -> &'static str {
        match self {
            FileCategoryType::Docs => DOCS_DESCRIPTION,
            FileCategoryType::Src => SRC_DESCRIPTION,
            FileCategoryType::Other => OTHER_DESCRIPTION,
        }
    }
}

/// Determines the category of a file based on its relative path and the application configuration globs.
///
/// Business Logic Constraint: Glob patterns are matched case-insensitively against the relative path.
/// Business Logic Constraint: 'docs' globs are checked first. If a match, categorized as 'Docs'.
/// Else, 'src' globs are checked. If a match, categorized as 'Src'.
/// Otherwise, the file is categorized as 'Other'.
fn categorize_file(relative_path: &Path, config: &AppConfig) -> FileCategoryType {
    if config.docs.is_match(relative_path) {
        return FileCategoryType::Docs;
    }
    if config.src.is_match(relative_path) {
        return FileCategoryType::Src;
    }
    FileCategoryType::Other
}

/// Processes all files in the working directory, categorizes them, and prepares data for output.
///
/// 1. Walks the directory (respecting ignore files like .gitignore, .kekignore) to find all files.
/// 2. For each file, determines its category ('docs', 'src', or 'other') based on matching its
///    relative path against configured glob patterns. Also retrieves file size.
/// 3. Groups files by these categories. Files within categories are not explicitly sorted further;
///    their order depends on the parallel directory traversal.
/// 4. Creates `CategoryData` for each of the three fixed categories if they contain any files.
///    Calculates total file size for each category.
/// 5. Returns a list of `CategoryData` (for non-empty categories) sorted by their `total_size`
///    in descending order (largest categories first). If categories have the same total size,
///    their relative order is not strictly defined beyond the initial processing order.
///
/// Business Logic Constraint: Categories with no matching files are omitted from the output.
/// Business Logic Constraint: File metadata (like size) read errors for individual files will result in overall processing failure.
pub fn process_all_categories(
    config: &AppConfig,
    working_dir: &Path,
) -> Result<Vec<CategoryData>, String> {
    let mut walk_builder = WalkBuilder::new(working_dir);
    walk_builder
        .standard_filters(true)
        .add_custom_ignore_filename(".kekignore");

    // Use a thread-safe collection for results
    let categorized_results = Stack::<Result<(FileCategoryType, FileData), String>>::new();
    let config_ref = config;
    let working_dir_ref = working_dir;

    // Process entries directly in the walker
    walk_builder.build_parallel().run(|| {
        // Create references for this thread
        let results = &categorized_results;
        let config = config_ref;
        let working_dir = working_dir_ref;

        Box::new(move |entry_result| {
            match entry_result {
                Ok(entry) => {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        // Process the entry directly here
                        let absolute_path = entry.path().to_path_buf();

                        // Get metadata and file size
                        let metadata = match entry.metadata() {
                            Ok(md) => md,
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to get metadata for file {:?}: {}",
                                    absolute_path, e
                                );
                                return ignore::WalkState::Continue;
                            }
                        };
                        let file_size = metadata.len();

                        // Get relative path and categorize
                        let relative_path = match get_relative_path(working_dir, &absolute_path) {
                            Ok(path) => path,
                            Err(e) => {
                                eprintln!("Warning: {}", e);
                                return ignore::WalkState::Continue;
                            }
                        };

                        let category_type = categorize_file(&relative_path, config);
                        let result = Ok((
                            category_type,
                            FileData {
                                relative_path,
                                absolute_path,
                                size: file_size,
                            },
                        ));

                        // Push the processed result
                        results.push(result);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Error walking directory: {}", e);
                }
            }
            ignore::WalkState::Continue
        })
    });

    let mut grouped_files: FxHashMap<FileCategoryType, Vec<FileData>> = FxHashMap::default();
    for result in categorized_results {
        let (category_type, file_data) = result?;
        grouped_files
            .entry(category_type)
            .or_default()
            .push(file_data);
    }

    let mut all_category_data = Vec::new();
    // Define the order for processing categories to ensure consistent behavior if sizes are equal,
    // though final sort is by size.
    let category_types_to_consider = [
        FileCategoryType::Docs,
        FileCategoryType::Src,
        FileCategoryType::Other,
    ];

    for cat_type in category_types_to_consider.iter() {
        // Use .remove() if the order from category_types_to_consider is the desired iteration order
        // and we want to consume the entries from the map.
        // Or use .get() if we want to iterate in a specific order but keep the map intact for some reason
        // (not needed here).
        if let Some(files) = grouped_files.remove(cat_type) {
            // Business Logic Constraint: Files within a category are not explicitly sorted.
            // Their order is determined by the parallel directory traversal and subsequent collection.
            let total_category_size: u64 = files.iter().map(|f| f.size).sum();
            all_category_data.push(CategoryData {
                description_text: cat_type.get_description().to_string(),
                files,
                total_size: total_category_size,
            });
        }
        // If a category type from category_types_to_consider had no files, it simply won't be found in
        // grouped_files, and nothing will be added to all_category_data for it, which is desired.
    }

    // Business Logic Constraint: Categories are sorted by total_size descending.
    // Larger categories appear first.
    // Tie-breaking for categories with identical total_size is based on the order
    // they were added to `all_category_data`, which in turn depends on `category_types_to_consider`
    // and then the order from `grouped_files.remove()`.
    all_category_data.sort_by(|a, b| b.total_size.cmp(&a.total_size));

    Ok(all_category_data)
}
