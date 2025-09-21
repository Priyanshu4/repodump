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
    fn test_resolve_target_directory_explicit_repo() -> Result<()> {
        let temp_dir = tempdir()?;
        let path = temp_dir.path().to_path_buf();
        let resolved_path = resolve_target_directory(Some(path.clone()))?;
        assert_eq!(resolved_path, path);
        Ok(())
    }

    #[test]
    fn test_resolve_target_directory_nonexistent_repo() {
        let nonexistent_path = PathBuf::from("/this/path/does/not/exist");
        let result = resolve_target_directory(Some(nonexistent_path));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Directory does not exist"));
    }

    #[test]
    fn test_resolve_target_directory_no_repo_arg() {
        // This test should fail when run outside a git repository
        let result = resolve_target_directory(None);

        // The result depends on whether the test is run in a git repository
        match result {
            Ok(_) => {
                // We're in a git repository, so it should succeed
            }
            Err(e) => {
                // We're not in a git repository, so it should fail with the expected message
                assert!(e
                    .to_string()
                    .contains("The current directory is not a git repository"));
            }
        }
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

    #[test]
    fn test_collect_files_ignore_gitignore() -> Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path().join("repo");
        fs::create_dir(&root)?;
        fs::write(root.join(".gitignore"), "temp\n*.log")?;
        fs::write(root.join("src.rs"), "source code")?;
        fs::write(root.join("temp"), "temporary file")?;
        fs::write(root.join("output.log"), "log file")?;

        let filter = FileFilter::new(vec![], vec![], vec![])?;
        let mut files = collect_files(&root, &filter, true)?;
        files.sort();

        let mut expected_files = vec![
            PathBuf::from(".gitignore"),
            PathBuf::from("output.log"),
            PathBuf::from("src.rs"),
            PathBuf::from("temp"),
        ];
        expected_files.sort();

        assert_eq!(files, expected_files);

        Ok(())
    }

    // Test collect_files function
    #[test]
    fn test_exclude_dot_git_folder() -> Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path().join("repo");
        fs::create_dir(&root)?;
        fs::create_dir(root.join(".git"))?;
        fs::write(root.join(".git/config"), "config")?;
        fs::write(root.join(".git/HEAD"), "head")?;
        fs::write(root.join(".gitignore"), "temp\n*.log")?;
        fs::write(root.join("src.rs"), "source code")?;
        fs::write(root.join("temp"), "temporary file")?;
        fs::write(root.join("output.log"), "log file")?;

        let exclude_git: Vec<String> = vec![".git".to_string(), ".git/**".to_string()];
        let filter = FileFilter::new(vec![], exclude_git, vec![])?;
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

    #[test]
    fn test_estimate_tokens_unicode() {
        assert_eq!(estimate_tokens("🦀🦀🦀🦀"), 1); // 4 unicode characters / 4 = 1
        assert_eq!(estimate_tokens("café"), 1); // 4 characters (including é) / 4 = 1
    }
}

// Integration tests
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_file_filter_integration() -> Result<()> {
        // Test the complete filtering workflow
        let filter = FileFilter::new(
            vec!["*.rs".to_string(), "*.toml".to_string()], // filter: only .rs and .toml files
            vec!["target/*".to_string()],                   // exclude: target directory
            vec!["target/important.rs".to_string()], // include: override exclusion for this file
        )?;

        // Should pass filter
        assert!(filter.should_include(&PathBuf::from("src/main.rs")));
        assert!(filter.should_include(&PathBuf::from("Cargo.toml")));

        // Should fail filter step
        assert!(!filter.should_include(&PathBuf::from("README.md")));

        // Should be excluded
        assert!(!filter.should_include(&PathBuf::from("target/debug/app")));

        // Should be included due to override
        assert!(filter.should_include(&PathBuf::from("target/important.rs")));

        Ok(())
    }

    #[test]
    fn test_directory_tree_structure() -> Result<()> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path().join("test_repo");

        // Create directory structure
        fs::create_dir_all(&root.join("src/utils"))?;
        fs::create_dir_all(&root.join("docs"))?;

        // Create files
        fs::write(root.join("README.md"), "readme content")?;
        fs::write(root.join("src/main.rs"), "fn main() {}")?;
        fs::write(root.join("src/utils/helpers.rs"), "// helpers")?;
        fs::write(root.join("docs/guide.md"), "# Guide")?;

        let files = vec![
            PathBuf::from("README.md"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/utils/helpers.rs"),
            PathBuf::from("docs/guide.md"),
        ];

        let tree = crate::generate_directory_tree(&root, &files)?;

        // Verify tree contains expected elements
        assert!(tree.contains("Directory Structure:"));
        assert!(tree.contains("test_repo/"));
        assert!(tree.contains("README.md"));
        assert!(tree.contains("src/"));
        assert!(tree.contains("main.rs"));
        assert!(tree.contains("utils/"));
        assert!(tree.contains("helpers.rs"));
        assert!(tree.contains("docs/"));
        assert!(tree.contains("guide.md"));

        Ok(())
    }
}
