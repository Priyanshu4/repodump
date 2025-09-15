use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

use crate::{collect_files, estimate_tokens, resolve_target_directory, FileFilter};

// Unit tests for individual functions
#[cfg(test)]
mod unit_tests {
    use super::*;

    // Test the FileFilter struct
    #[test]
    fn test_file_filter_new() {
        let filter = FileFilter::new(
            vec!["*.rs".to_string()],
            vec!["target/*".to_string()],
            vec!["src/main.rs".to_string()],
        )
        .unwrap();

        assert_eq!(filter.filter_globs.is_match("src/main.rs"), true);
        assert_eq!(filter.exclude_globs.is_match("target/debug/app"), true);
        assert_eq!(filter.include_globs.is_match("src/main.rs"), true);
    }

    #[test]
    fn test_file_filter_should_include_order() -> Result<()> {
        let filter = FileFilter::new(
            vec!["*.rs".to_string()],
            vec!["src/*".to_string()],
            vec!["src/main.rs".to_string()],
        )?;

        // Filter pattern is applied first
        assert_eq!(filter.should_include(&PathBuf::from("src/lib.js")), false);

        // Include pattern overrides exclude pattern
        assert_eq!(filter.should_include(&PathBuf::from("src/main.rs")), true);

        Ok(())
    }

    // Test resolve_target_directory function
    #[test]
    fn test_resolve_target_directory_explicit_path() -> Result<()> {
        let temp_dir = tempdir()?;
        let path = temp_dir.path().to_path_buf();
        let resolved_path = resolve_target_directory(Some(path.clone()))?;
        assert_eq!(resolved_path, path);
        Ok(())
    }

    #[test]
    #[should_panic(expected = "The current directory is not a git repository.")]
    fn test_resolve_target_directory_no_git_repo() {
        // This test requires running in a directory that is not a git repo
        // which might be difficult to guarantee in a testing environment.
        // A better approach would be to mock the gix::discover function.
        // For now, we rely on the `tempdir` which is not a git repository.
        let temp_dir = tempdir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        resolve_target_directory(None).unwrap();
    }

    // Test collect_files function
    #[test]
    fn test_collect_files_with_gitignore() -> Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path().join("repo");
        fs::create_dir(&root)?;
        fs::write(root.join(".gitignore"), "temp\n*.log")?;
        fs::write(root.join("src.rs"), "source code")?;
        fs::write(root.join("temp"), "temporary file")?;
        fs::write(root.join("output.log"), "log file")?;

        let filter = FileFilter::new(vec![], vec![], vec![])?;
        let mut files = collect_files(&root, &filter, false)?;
        files.sort();

        let mut expected_files = vec![PathBuf::from(".gitignore"), PathBuf::from("src.rs")];
        expected_files.sort();

        assert_eq!(files, expected_files);

        Ok(())
    }

    // Test estimate_tokens function
    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("This is a test!!"), 4); // 16 characters / 4 = 4
        assert_eq!(estimate_tokens("Hello, world!"), 3); // 13 characters / 4 = 3 (integer division)
        assert_eq!(estimate_tokens(""), 0);
    }
}
