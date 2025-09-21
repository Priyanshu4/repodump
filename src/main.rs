use anyhow::{Context, Result};
use clap::Parser;
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(name = "repodump")]
#[command(about = "Generate LLM-friendly text files from directories and git repositories")]
#[command(version = "0.1.0")]
struct Cli {
    /// Path to the directory or git repository
    path: Option<PathBuf>,

    /// Output file path
    #[arg(short = 'o', long = "output", default_value = "repodump.txt")]
    output: PathBuf,

    /// Only include the directory structure but not the file contents
    #[arg(short = 't', long = "tree")]
    tree_only: bool,

    /// Only include the file contents but not the directory structure
    #[arg(short = 'c', long = "contents")]
    contents_only: bool,

    /// Ignore .gitignore files
    #[arg(short = 'g', long = "ignore-gitignore")]
    ignore_gitignore: bool,

    /// Only include files any of matching these patterns
    #[arg(short = 'f', long = "filter")]
    filter: Vec<String>,

    /// Exclude files matching any of these patterns
    #[arg(short = 'e', long = "exclude")]
    exclude: Vec<String>,

    /// Include files matching any of these patterns, overriding exclusions
    #[arg(short = 'i', long = "include")]
    include: Vec<String>,

    /// Apply custom filter and exclusion patterns to the directory structure tree
    #[arg(short = 'p', long = "prune-tree")]
    prune_tree: bool,

    /// Add prompt text at the bottom of the repodump file
    #[arg(short = 'm', long = "prompt")]
    prompt: Option<String>,

    /// Do not output a summary to stdout
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
}

/// Represents file filtering configuration
struct FileFilter {
    filter_globs: globset::GlobSet,
    exclude_globs: globset::GlobSet,
    include_globs: globset::GlobSet,
}

impl FileFilter {
    /// Creates a new FileFilter from command line patterns
    ///
    /// # Arguments
    /// * `filter` - Patterns for files to include (if empty, all files pass filter)
    /// * `exclude` - Patterns for files to exclude
    /// * `include` - Patterns for files to force include
    ///
    /// # Examples
    /// ```
    /// let filter = FileFilter::new(
    ///     vec!["*.rs".to_string()],
    ///     vec!["target/*".to_string()],
    ///     vec!["Cargo.toml".to_string()]
    /// ).unwrap();
    /// ```
    fn new(filter: Vec<String>, exclude: Vec<String>, include: Vec<String>) -> Result<Self> {
        let filter_globs = build_globset(filter)?;
        let exclude_globs = build_globset(exclude)?;
        let include_globs = build_globset(include)?;

        Ok(FileFilter {
            filter_globs,
            exclude_globs,
            include_globs,
        })
    }

    /// Determines if a file should be included based on filtering rules
    ///
    /// # Arguments
    /// * `path` - The file path to check
    ///
    /// # Returns
    /// `true` if the file should be included, `false` otherwise
    ///
    /// # Examples
    /// ```
    /// let filter = FileFilter::new(vec![], vec!["*.tmp".to_string()], vec![]).unwrap();
    /// assert!(!filter.should_include("temp.tmp"));
    /// assert!(filter.should_include("main.rs"));
    /// ```
    fn should_include(&self, path: &Path) -> bool {
        // Step 1: Apply filter patterns (if any exist)
        if self.filter_globs.len() > 0 && !self.filter_globs.is_match(&path) {
            return false;
        }

        // Step 2: Apply exclude patterns
        if self.exclude_globs.is_match(&path) {
            // Step 3: Check if include patterns override exclusion
            return self.include_globs.is_match(&path);
        }

        true
    }
}

/// Builds a GlobSet from a vector of pattern strings
///
/// # Arguments
/// * `patterns` - Vector of glob pattern strings
///
/// # Returns
/// A compiled GlobSet or an error if patterns are invalid
fn build_globset(patterns: Vec<String>) -> Result<globset::GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob =
            Glob::new(&pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?;
        builder.add(glob);
    }
    builder.build().context("Failed to build glob set")
}

/// Determines the target directory to process
///
/// # Arguments
/// * `path_arg` - Optional path argument from command line
///
/// # Returns
/// The resolved directory path or an error
fn resolve_target_directory(path_arg: Option<PathBuf>) -> Result<PathBuf> {
    match path_arg {
        Some(path) => {
            if !path.exists() {
                anyhow::bail!("Directory does not exist: {}", path.display());
            }
            Ok(path)
        }
        None => {
            let current_dir = std::env::current_dir().context("Failed to get current directory")?;

            // Check if we're in a git repository
            match gix::discover(&current_dir) {
                Ok(repo) => {
                    let git_dir = repo.git_dir();
                    let repo_root = git_dir.parent().context("Failed to get repository root")?;
                    Ok(repo_root.to_path_buf())
                }
                Err(_) => {
                    anyhow::bail!("The current directory is not a git repository. For use outside of git repositories, please provide a directory path.");
                }
            }
        }
    }
}

/// Collects all files in the directory that pass the filter
///
/// # Arguments
/// * `root_path` - The root directory to scan
/// * `filter` - The file filter to apply
/// * `ignore_gitignore` - Whether to ignore .gitignore files
///
/// # Returns
/// A vector of file paths that should be included
fn collect_files(
    root_path: &Path,
    filter: &FileFilter,
    ignore_gitignore: bool,
) -> Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(root_path);
    builder.hidden(false); // Include hidden files by default

    if ignore_gitignore {
        builder.git_ignore(false);
        builder.git_exclude(false);
        builder.git_global(false);
    } else {
        // Respect .gitignore even if not a git repo
        builder.add_custom_ignore_filename(".gitignore");
    }

    let mut files = Vec::new();

    for result in builder.build() {
        let entry = result.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_file() {
            let relative_path = path
                .strip_prefix(root_path)
                .context("Failed to create relative path")?;

            if filter.should_include(relative_path) {
                files.push(relative_path.to_path_buf());
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Generates a directory tree structure as a string
///
/// # Arguments
/// * `root_path` - The root directory
/// * `files` - List of files to include in the tree
///
/// # Returns
/// A formatted directory tree string
fn generate_directory_tree(root_path: &Path, files: &[PathBuf]) -> Result<String> {
    let mut tree = String::new();
    let root_name = root_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("root"))
        .to_string_lossy();

    tree.push_str("Directory Structure:\n");
    tree.push_str(&format!("└── {}/\n", root_name));

    if files.is_empty() {
        return Ok(tree);
    }

    // Build a hierarchical structure
    let mut file_tree: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut all_dirs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    // Process each file to build the tree structure
    for file in files {
        let file_path_str = file.to_string_lossy().replace('\\', "/");

        // Add all parent directories
        if let Some(parent) = file.parent() {
            let parent_str = parent.to_string_lossy().replace('\\', "/");
            if !parent_str.is_empty() {
                all_dirs.insert(parent_str.clone());
                file_tree.entry(parent_str).or_insert_with(Vec::new);
            }
        }

        // Add the file to its parent directory
        let parent_key = file
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| String::new());

        file_tree
            .entry(parent_key)
            .or_insert_with(Vec::new)
            .push(file_path_str);
    }

    // Create a sorted list of all entries with their depths and types
    let mut entries: Vec<(usize, String, bool)> = Vec::new(); // (depth, path, is_dir)

    // Add directories first
    for dir in &all_dirs {
        let depth = if dir.is_empty() {
            0
        } else {
            dir.matches('/').count() + 1
        };
        entries.push((depth, dir.clone(), true));
    }

    // Add files
    for file in files {
        let file_str = file.to_string_lossy().replace('\\', "/");
        let depth = file_str.matches('/').count() + 1;
        entries.push((depth, file_str, false));
    }

    // Sort by depth first, then by path
    entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    // Generate the tree with proper indentation
    for (depth, path_str, is_dir) in entries {
        let indent = "    ".repeat(depth);

        let name = if path_str.is_empty() {
            continue; // Skip empty paths
        } else {
            Path::new(&path_str)
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(&path_str))
                .to_string_lossy()
        };

        if is_dir {
            tree.push_str(&format!("{}├── {}/\n", indent, name));
        } else {
            tree.push_str(&format!("{}├── {}\n", indent, name));
        }
    }

    Ok(tree)
}

/// Generates file contents section as a string
///
/// # Arguments
/// * `root_path` - The root directory
/// * `files` - List of files to include
///
/// # Returns
/// A formatted string containing all file contents
fn generate_file_contents(root_path: &Path, files: &[PathBuf]) -> Result<String> {
    let mut contents = String::new();

    for (i, file_path) in files.iter().enumerate() {
        let full_path = root_path.join(file_path);

        if i > 0 {
            contents.push('\n');
        }

        contents.push_str("================================================\n");
        contents.push_str(&format!("FILE: {}\n", file_path.to_string_lossy()));
        contents.push_str("================================================\n");

        match fs::read_to_string(&full_path) {
            Ok(file_content) => {
                contents.push_str(&file_content);
                if !file_content.ends_with('\n') {
                    contents.push('\n');
                }
            }
            Err(_) => {
                contents.push_str("[Binary file or read error]\n");
            }
        }
    }

    Ok(contents)
}

/// Estimates the number of LLM tokens in the text
///
/// # Arguments
/// * `text` - The text to analyze
///
/// # Returns
/// Estimated number of tokens (characters / 4)
///
/// # Examples
/// ```
/// assert_eq!(estimate_tokens("Hello world"), 2);
/// assert_eq!(estimate_tokens("This is a test"), 3);
/// ```
fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 4
}

/// Prints a summary of the generated file to stdout
///
/// # Arguments
/// * `root_path` - The processed directory
/// * `structure_file_count` - Number of files in structure
/// * `content_file_count` - Number of files with contents
/// * `output_size` - Size of output file in bytes
/// * `token_count` - Estimated token count
fn print_summary(
    root_path: &Path,
    structure_file_count: usize,
    content_file_count: usize,
    output_size: usize,
    token_count: usize,
) {
    let repo_name = root_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("unknown"))
        .to_string_lossy();

    println!("Repository: {}", repo_name);
    println!("Files in structure: {}", structure_file_count);
    println!("Files in contents: {}", content_file_count);
    println!("Output size: {} bytes", output_size);
    println!("Estimated tokens: {}", token_count);
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Resolve target directory
    let target_dir = resolve_target_directory(cli.path)?;

    // Create an exclude filter that always excludes .git
    let exclude_git = vec![".git".to_string(), ".git/**".to_string()];
    let mut all_excludes = cli.exclude.clone();
    all_excludes.extend(exclude_git.clone());

    // Gather files for content section
    let content_filter = FileFilter::new(cli.filter, all_excludes, cli.include.clone())?;
    let content_files = collect_files(&target_dir, &content_filter, cli.ignore_gitignore)?;

    // Gather files for tree structure section
    let tree_files = if cli.prune_tree {
        // If pruning tree, use the same files as content section
        content_files.clone()
    } else {
        let tree_filter = FileFilter::new(vec![], exclude_git, cli.include.clone())?;
        collect_files(&target_dir, &tree_filter, cli.ignore_gitignore)?
    };

    // Generate output content
    let mut output_content = String::new();

    let structure_file_count;
    if !cli.contents_only {
        // Generate tree
        let tree = generate_directory_tree(&target_dir, &tree_files)?;
        output_content.push_str(&tree);
        output_content.push('\n');
        structure_file_count = tree_files.len()
    } else {
        structure_file_count = 0;
    }

    let content_file_count;
    if !cli.tree_only {
        // Generate file contents
        let contents = generate_file_contents(&target_dir, &content_files)?;
        output_content.push_str(&contents);
        content_file_count = content_files.len()
    } else {
        content_file_count = 0
    };

    // Add prompt if provided
    if let Some(prompt) = cli.prompt {
        output_content.push('\n');
        output_content.push_str(&format!("Prompt: {}\n", prompt));
    }

    // Write output file
    fs::write(&cli.output, &output_content)
        .with_context(|| format!("Failed to write output file: {}", cli.output.display()))?;

    // Print summary unless quiet mode
    if !cli.quiet {
        let output_size = output_content.len();
        let token_count = estimate_tokens(&output_content);

        print_summary(
            &target_dir,
            structure_file_count,
            content_file_count,
            output_size,
            token_count,
        );
    }

    Ok(())
}
