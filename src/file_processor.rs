use crate::config::{AppConfig, DOCS_DESCRIPTION, OTHER_DESCRIPTION, SRC_DESCRIPTION};

use std::path::{Path, PathBuf, Component};
use std::fs;

use lockfree::stack::Stack;
use ignore::WalkBuilder;
use rustc_hash::{FxHashMap, FxHashSet};

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

/// Creates a relative path from `base` to `target_path`.
/// Handles cases where `target_path` is not a direct descendant of `base` by using `../`.
/// Both paths should ideally be canonicalized before calling this function for robustness.
fn create_relative_path(base: &Path, target_path: &Path) -> Result<PathBuf, String> {
    // Attempt simple stripping first, common case if target is under base.
    if let Ok(stripped) = target_path.strip_prefix(base) {
        if stripped.as_os_str().is_empty() { // Path is same as base
             return Ok(PathBuf::from("."));
        }
        return Ok(stripped.to_path_buf());
    }

    let base_comps: Vec<Component<'_>> = base.components().collect();
    let target_comps: Vec<Component<'_>> = target_path.components().collect();

    let mut common_prefix_len = 0;
    while common_prefix_len < base_comps.len()
        && common_prefix_len < target_comps.len()
        && base_comps[common_prefix_len] == target_comps[common_prefix_len]
    {
        common_prefix_len += 1;
    }

    let mut rel_path = PathBuf::new();

    for _ in common_prefix_len..base_comps.len() {
        if base_comps[common_prefix_len..].iter().all(|c| matches!(c, Component::CurDir)) {
            continue;
        }
        rel_path.push(Component::ParentDir);
    }
    
    for comp_idx in common_prefix_len..target_comps.len() {
        match target_comps[comp_idx] {
            Component::RootDir | Component::Prefix(_) => {
                if rel_path.as_os_str().is_empty() && comp_idx +1 == target_comps.len() {
                    return Ok(PathBuf::from("."));
                }
            }
            _ => rel_path.push(target_comps[comp_idx]),
        }
    }
    
    if rel_path.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else {
        Ok(rel_path)
    }
}


#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum FileCategoryType {
    Docs,
    Src,
    Other,
}

impl FileCategoryType {
    fn get_description(&self) -> &'static str {
        match self {
            FileCategoryType::Docs => DOCS_DESCRIPTION,
            FileCategoryType::Src => SRC_DESCRIPTION,
            FileCategoryType::Other => OTHER_DESCRIPTION,
        }
    }
}

fn categorize_file(relative_path: &Path, config: &AppConfig) -> FileCategoryType {
    if config.docs.is_match(relative_path) {
        return FileCategoryType::Docs;
    }
    if config.src.is_match(relative_path) {
        return FileCategoryType::Src;
    }
    FileCategoryType::Other
}

pub fn process_all_categories(
    config: &AppConfig,
    working_dir: &Path, 
) -> Result<Vec<CategoryData>, String> {
    
    let categorized_results_stack = Stack::<Result<(FileCategoryType, FileData), String>>::new();
    
    let canonical_working_dir = working_dir.canonicalize().map_err(|e| {
        format!(
            "Failed to canonicalize working directory {:?}: {}",
            working_dir, e
        )
    })?;

    let mut walk_builder_opt: Option<WalkBuilder> = None;
    let mut has_valid_scan_paths = false;

    for scan_dir_config_path in &config.scan {
        let current_scan_target_abs = if scan_dir_config_path.is_absolute() {
            scan_dir_config_path.clone()
        } else {
            working_dir.join(scan_dir_config_path)
        };

        let canonical_scan_root = match fs::canonicalize(&current_scan_target_abs) {
            Ok(p) => p,
            Err(e) => {
                eprintln!(
                    "[WARNING] Failed to canonicalize scan directory {:?} (configured as {:?}): {}. Skipping.",
                    current_scan_target_abs, scan_dir_config_path, e
                );
                continue;
            }
        };

        if !canonical_scan_root.is_dir() {
            eprintln!(
                "[WARNING] Scan path {:?} (configured as {:?}, resolved to {:?}) is not a directory. Skipping.",
                scan_dir_config_path, current_scan_target_abs, canonical_scan_root
            );
            continue;
        }
        
        has_valid_scan_paths = true;

        match walk_builder_opt.as_mut() {
            Some(builder) => {
                builder.add(canonical_scan_root);
            }
            None => {
                let mut new_builder = WalkBuilder::new(canonical_scan_root);
                new_builder
                    .standard_filters(true) 
                    .add_custom_ignore_filename(".kekignore");
                walk_builder_opt = Some(new_builder);
            }
        }
    }

    if !has_valid_scan_paths || walk_builder_opt.is_none() {
        eprintln!("[INFO] No valid scan directories to process.");
        return Ok(Vec::new()); // No valid paths to walk, return empty
    }

    // We can unwrap here because has_valid_scan_paths ensures walk_builder_opt is Some
    let walk_builder = walk_builder_opt.unwrap();
    
    // References for the parallel closure
    let config_ref = config; 
    let canonical_working_dir_ref = &canonical_working_dir;
    let results_stack_ref = &categorized_results_stack;

    walk_builder.build_parallel().run(|| {
        let thread_local_config = config_ref;
        let thread_local_canonical_cwd = canonical_working_dir_ref;
        let thread_local_results_stack = results_stack_ref;

        Box::new(move |entry_result| {
            match entry_result {
                Ok(entry) => {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        let path_from_walker = entry.path();
                        
                        let file_absolute_path_canonical = match fs::canonicalize(path_from_walker) {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to canonicalize path for file {:?}: {}. Skipping file.",
                                    path_from_walker, e
                                );
                                return ignore::WalkState::Continue;
                            }
                        };

                        let metadata = match entry.metadata() {
                            Ok(md) => md,
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to get metadata for file {:?}: {}. Skipping file.",
                                    file_absolute_path_canonical, e
                                );
                                return ignore::WalkState::Continue;
                            }
                        };
                        let file_size = metadata.len();

                        let relative_path_to_cwd = match create_relative_path(thread_local_canonical_cwd, &file_absolute_path_canonical) {
                            Ok(path) => path,
                            Err(e_str) => {
                                eprintln!(
                                    "Warning: Failed to create relative path for {:?} (base {:?}): {}. Skipping file.",
                                    file_absolute_path_canonical, thread_local_canonical_cwd, e_str
                                );
                                return ignore::WalkState::Continue;
                            }
                        };
                        
                        let category_type = categorize_file(&relative_path_to_cwd, thread_local_config);
                        
                        let file_data = FileData {
                            relative_path: relative_path_to_cwd,
                            absolute_path: file_absolute_path_canonical,
                            size: file_size,
                        };
                        thread_local_results_stack.push(Ok((category_type, file_data)));
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Error walking directory entry: {}", e);
                }
            }
            ignore::WalkState::Continue
        })
    });

    let mut grouped_files: FxHashMap<FileCategoryType, Vec<FileData>> = FxHashMap::default();
    let mut processed_abs_paths: FxHashSet<PathBuf> = FxHashSet::default();

    for result in categorized_results_stack {
        match result {
            Ok((category_type, file_data)) => {
                if processed_abs_paths.insert(file_data.absolute_path.clone()) {
                    grouped_files
                        .entry(category_type)
                        .or_default()
                        .push(file_data);
                }
            }
            Err(e) => {
                eprintln!("[ERROR] An error occurred during file data collection: {}", e);
            }
        }
    }

    let mut all_category_data = Vec::new();
    let category_types_to_consider = [
        FileCategoryType::Docs,
        FileCategoryType::Src,
        FileCategoryType::Other,
    ];

    for cat_type in category_types_to_consider.iter() {
        if let Some(files) = grouped_files.remove(cat_type) {
            if files.is_empty() { continue; }
            let total_category_size: u64 = files.iter().map(|f| f.size).sum();
            all_category_data.push(CategoryData {
                description_text: cat_type.get_description().to_string(),
                files,
                total_size: total_category_size,
            });
        }
    }

    all_category_data.sort_by(|a, b| b.total_size.cmp(&a.total_size));

    Ok(all_category_data)
}
