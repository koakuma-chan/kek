mod config;
mod file_processor;
mod output;

use std::env;
use std::process::exit;

use mimalloc::MiMalloc;

use atty::Stream;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    // Check if stdout is a TTY (i.e., not piped)
    // Business Logic Constraint: The program is designed to output structured data,
    // which is typically consumed by another process. Direct output to a terminal
    // is not its intended use case and might lead to an undesirable user experience
    // or misinterpretation of the output.
    if atty::is(Stream::Stdout) {
        eprintln!(
            "[ERROR] Program output must be piped to another command or redirected to a file."
        );
        eprintln!(
            "Example: {} | your_command",
            env::args().next().unwrap_or_else(|| "kek".to_string())
        );
        exit(1);
    }

    // Collect command line arguments, skipping the program name.
    // These will be printed at the end if any are provided.
    let cli_args: Vec<String> = env::args().skip(1).collect();
    let task_args_string: Option<String> = if cli_args.is_empty() {
        None
    } else {
        Some(cli_args.join(" "))
    };

    // Load application configuration
    let app_config = match config::load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("[ERROR] Configuration error: {}", e);
            exit(1);
        }
    };

    // Determine current working directory (base for relative paths and globbing)
    let working_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("[ERROR] Failed to get current working directory: {}", e);
            exit(1);
        }
    };

    let categories_data = match file_processor::process_all_categories(&app_config, &working_dir) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("[ERROR] Error processing files: {}", e);
            exit(1);
        }
    };

    // Business Logic Constraint: If no categories data is processed, and no task args,
    // there's nothing to output, so the program can exit gracefully.
    // If there are task_args, we still need to run write_output.
    if categories_data.is_empty() && task_args_string.is_none() {
        // Consider logging this to stderr if it's an unexpected empty result
        // eprintln!("[INFO] No data processed and no task arguments, exiting.");
        return;
    }

    if let Err(e) = output::write_output(&categories_data, task_args_string) {
        eprintln!("[ERROR] Error writing output to stdout: {}", e);
        exit(1);
    }
}
