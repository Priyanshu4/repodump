#!/usr/bin/env python3
"""
Repository Dump Generator

A script that generates LLM-friendly text files containing directory structure
and file contents with comprehensive filtering options.
"""

import argparse
import fnmatch
import os
import sys
from pathlib import Path
from typing import List, Tuple, Set, Optional, Dict, Any, NamedTuple


class FileInfo(NamedTuple):
    """Information about a file or directory."""
    rel_path: str
    abs_path: str
    is_dir: bool
    include_in_structure: bool
    include_content: bool


def parse_gitignore_patterns(gitignore_path: str) -> List[str]:
    """
    Parse a .gitignore file and return a list of patterns.
    
    Args:
        gitignore_path: Path to the .gitignore file
        
    Returns:
        List of gitignore patterns
    """
    patterns = []
    try:
        with open(gitignore_path, 'r', encoding='utf-8', errors='ignore') as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#'):
                    patterns.append(line)
    except (IOError, OSError):
        pass
    return patterns


def matches_pattern(file_path: str, pattern: str, is_dir: bool = False) -> bool:
    """
    Check if a file path matches a gitignore-style pattern.
    
    Args:
        file_path: The file path to check (relative to root)
        pattern: The pattern to match against
        is_dir: Whether the file_path is a directory
        
    Returns:
        True if the pattern matches
    """
    original_pattern = pattern
    
    # Handle negation patterns
    if pattern.startswith('!'):
        return False  # Negation handled at higher level
    
    # Handle directory-only patterns
    directory_only = pattern.endswith('/')
    if directory_only:
        pattern = pattern[:-1]
        if not is_dir:
            return False
    
    # Handle absolute patterns (starting with /)
    if pattern.startswith('/'):
        pattern = pattern[1:]
        # Match from root only
        if '/' in pattern:
            return fnmatch.fnmatch(file_path, pattern)
        else:
            # Match top-level file/directory only
            return fnmatch.fnmatch(file_path.split('/')[0], pattern)
    
    # Convert paths to comparable format
    path_parts = file_path.split('/') if file_path else []
    
    # Handle patterns with directory separators
    if '/' in pattern:
        pattern_parts = pattern.split('/')
        
        # For directories, check if the directory path matches the pattern exactly
        if is_dir and file_path == pattern:
            return True
            
        # Check if file_path starts with pattern (for directory matching)
        if file_path.startswith(pattern + '/'):
            return True
            
        # Try matching the pattern at any position in the path
        for i in range(len(path_parts) - len(pattern_parts) + 1):
            test_parts = path_parts[i:i + len(pattern_parts)]
            test_path = '/'.join(test_parts)
            if fnmatch.fnmatch(test_path, pattern):
                return True
                
        # Check if any parent directory path matches the pattern
        for i in range(1, len(path_parts) + 1):
            parent_path = '/'.join(path_parts[:i])
            if fnmatch.fnmatch(parent_path, pattern):
                return True
                
        return False
    
    # Simple pattern matching - check each path component and full path
    # Check individual components
    for part in path_parts:
        if fnmatch.fnmatch(part, pattern):
            return True
    
    # Check full path for patterns like "*.txt"
    if fnmatch.fnmatch(file_path, pattern):
        return True
        
    # Check basename
    if file_path and fnmatch.fnmatch(os.path.basename(file_path), pattern):
        return True
    
    return False


def path_is_under_pattern(file_path: str, pattern: str) -> bool:
    """
    Check if a file path is under a directory pattern.
    
    Args:
        file_path: The file path to check
        pattern: The directory pattern
        
    Returns:
        True if the file is under the pattern directory
    """
    # Normalize pattern - remove trailing slash if present
    if pattern.endswith('/'):
        pattern = pattern[:-1]
    
    # Check if file is directly under or within the pattern directory
    return file_path.startswith(pattern + '/') or file_path == pattern


def should_include_file(file_path: str, is_dir: bool, include_patterns: List[str], 
                       exclude_patterns: List[str], filter_patterns: List[str]) -> bool:
    """
    Determine if a file should be included based on filtering rules.
    
    Args:
        file_path: Relative path to the file
        is_dir: Whether the path is a directory
        include_patterns: Patterns that force inclusion
        exclude_patterns: Patterns that force exclusion
        filter_patterns: Patterns that limit inclusion (whitelist)
        
    Returns:
        True if file should be included
    """
    # Step 1: Apply filter patterns (whitelist) - if any filters exist, file must match at least one
    if filter_patterns:
        matched_filter = False
        for pattern in filter_patterns:
            if matches_pattern(file_path, pattern, is_dir):
                matched_filter = True
                break
            # Also check if this file is under a directory that matches the pattern
            if not is_dir and path_is_under_pattern(file_path, pattern):
                matched_filter = True
                break
        
        if not matched_filter:
            return False
    
    # Step 2: Apply exclude patterns
    excluded = False
    included_by_negation = False
    
    for pattern in exclude_patterns:
        if pattern.startswith('!'):
            # Handle negation patterns
            neg_pattern = pattern[1:]
            if matches_pattern(file_path, neg_pattern, is_dir):
                included_by_negation = True
        elif matches_pattern(file_path, pattern, is_dir):
            excluded = True
        # Also check if this file is under an excluded directory
        elif not is_dir and path_is_under_pattern(file_path, pattern):
            excluded = True
    
    if excluded and not included_by_negation:
        # Step 3: Check include patterns (override exclusions)
        override_exclusion = False
        for pattern in include_patterns:
            if matches_pattern(file_path, pattern, is_dir):
                override_exclusion = True
                break
            # Also check if this file is under an included directory
            if not is_dir and path_is_under_pattern(file_path, pattern):
                override_exclusion = True
                break
        
        if not override_exclusion:
            return False
    
    return True


def collect_gitignore_patterns(root_path: str, ignore_gitignore: bool = False) -> List[str]:
    """
    Collect all gitignore patterns from .gitignore files in the directory tree.
    
    Args:
        root_path: Root directory to search
        ignore_gitignore: Whether to ignore .gitignore files
        
    Returns:
        List of all gitignore patterns found
    """
    if ignore_gitignore:
        return []
    
    patterns = []
    root_abs = os.path.abspath(root_path)
    
    for dirpath, dirnames, filenames in os.walk(root_path):
        if '.gitignore' in filenames:
            gitignore_path = os.path.join(dirpath, '.gitignore')
            rel_dir = os.path.relpath(dirpath, root_path)
            file_patterns = parse_gitignore_patterns(gitignore_path)
            
            for pattern in file_patterns:
                if rel_dir != '.':
                    # For patterns in subdirectories, they need to be prefixed
                    # unless they start with / (absolute) or ! (negation)
                    if not pattern.startswith(('/','!')):
                        pattern = f"{rel_dir}/{pattern}"
                    elif pattern.startswith('!') and not pattern[1:].startswith('/'):
                        pattern = f"!{rel_dir}/{pattern[1:]}"
                patterns.append(pattern)
    
    return patterns


def collect_all_files(root_path: str, include_patterns: List[str], exclude_patterns: List[str], 
                     filter_patterns: List[str], filter_files_patterns: List[str], 
                     exclude_files_patterns: List[str]) -> List[FileInfo]:
    """
    Collect all files and determine their inclusion status in a single traversal.
    
    Args:
        root_path: Root directory path
        include_patterns: Patterns that force inclusion
        exclude_patterns: Patterns that force exclusion
        filter_patterns: Patterns that limit inclusion
        filter_files_patterns: Patterns that limit file content inclusion
        exclude_files_patterns: Patterns that exclude file contents
        
    Returns:
        List of FileInfo objects with inclusion decisions
    """
    all_files = []
    
    def process_directory(dir_path: str, rel_path: str = '') -> bool:
        """Process a directory and return True if it should be traversed."""
        is_root = rel_path == ''
        
        # Check if directory itself should be included in structure
        if not is_root:
            include_dir_in_structure = should_include_file(rel_path, True, include_patterns, 
                                                         exclude_patterns, filter_patterns)
            # Even if directory is not included in structure, we might still need to traverse it
            # if it contains files that should be included
            should_traverse = include_dir_in_structure
            
            # Check if any filter patterns might include files under this directory
            if not should_traverse and filter_patterns:
                for pattern in filter_patterns:
                    if pattern.startswith(rel_path + '/') or (not '/' in pattern):
                        should_traverse = True
                        break
            
            # Check if any include patterns might include files under this directory
            if not should_traverse and include_patterns:
                for pattern in include_patterns:
                    if pattern.startswith(rel_path + '/') or (not '/' in pattern):
                        should_traverse = True
                        break
            
            if not should_traverse:
                return False  # Don't traverse this directory
        else:
            include_dir_in_structure = True
        
        # Add directory to results
        if not is_root:
            all_files.append(FileInfo(
                rel_path=rel_path,
                abs_path=dir_path,
                is_dir=True,
                include_in_structure=include_dir_in_structure,
                include_content=False  # Directories don't have content
            ))
        
        try:
            entries = sorted(os.listdir(dir_path))
        except PermissionError:
            return False
        
        for entry in entries:
            entry_path = os.path.join(dir_path, entry)
            entry_rel_path = os.path.join(rel_path, entry) if rel_path else entry
            entry_rel_path = entry_rel_path.replace('\\', '/')  # Normalize path separators
            
            if os.path.isdir(entry_path):
                process_directory(entry_path, entry_rel_path)
            else:
                # Determine file inclusion
                include_in_structure = should_include_file(entry_rel_path, False, include_patterns, 
                                                         exclude_patterns, filter_patterns)
                
                # Determine content inclusion
                include_content = include_in_structure  # Start with structure decision
                
                if include_content and filter_files_patterns:
                    # Apply file-specific content filters
                    include_content = any(matches_pattern(entry_rel_path, pattern, False) 
                                        for pattern in filter_files_patterns)
                
                if include_content and exclude_files_patterns:
                    # Apply file-specific content exclusions
                    if any(matches_pattern(entry_rel_path, pattern, False) 
                           for pattern in exclude_files_patterns):
                        # Check if include patterns override for content
                        include_content = any(matches_pattern(entry_rel_path, pattern, False) 
                                            for pattern in include_patterns)
                
                all_files.append(FileInfo(
                    rel_path=entry_rel_path,
                    abs_path=entry_path,
                    is_dir=False,
                    include_in_structure=include_in_structure,
                    include_content=include_content
                ))
        
        return True
    
    process_directory(root_path)
    return all_files


def build_directory_structure(root_path: str, files: List[FileInfo]) -> str:
    """
    Build a tree-like directory structure representation.
    
    Args:
        root_path: Root directory path
        files: List of FileInfo objects
        
    Returns:
        String representation of directory structure
    """
    # Create a structure of files to include in tree
    structure_files = [f for f in files if f.include_in_structure]
    
    # Build tree structure
    def build_tree_recursive(current_path: str = '', prefix: str = '', is_last: bool = True) -> List[str]:
        lines = []
        
        if current_path == '':
            # Root directory
            root_name = os.path.basename(os.path.abspath(root_path))
            lines.append(f"└── {root_name}/")
            prefix = "    "
        
        # Get children of current path
        children = []
        current_level = current_path.count('/') if current_path else 0
        target_level = current_level + 1 if current_path else 0
        
        for file_info in structure_files:
            if current_path == '':
                # Root level - no parent directory
                if '/' not in file_info.rel_path:
                    children.append(file_info)
            else:
                # Check if this file is a direct child of current_path
                if (file_info.rel_path.startswith(current_path + '/') and 
                    file_info.rel_path.count('/') == current_path.count('/') + 1):
                    children.append(file_info)
        
        # Sort children - directories first, then files, both alphabetically
        children.sort(key=lambda x: (not x.is_dir, x.rel_path.split('/')[-1].lower()))
        
        # Add children to tree
        for i, child in enumerate(children):
            is_last_child = i == len(children) - 1
            connector = "└── " if is_last_child else "├── "
            
            name = child.rel_path.split('/')[-1]
            suffix = "/" if child.is_dir else ""
            lines.append(f"{prefix}{connector}{name}{suffix}")
            
            # Recursively add subdirectories
            if child.is_dir:
                new_prefix = prefix + ("    " if is_last_child else "│   ")
                lines.extend(build_tree_recursive(child.rel_path, new_prefix, is_last_child))
        
        return lines
    
    tree_lines = build_tree_recursive()
    return "Directory Structure:\n" + "\n".join(tree_lines)


def read_file_content(file_path: str) -> str:
    """
    Read file content with fallback encoding handling.
    
    Args:
        file_path: Path to the file
        
    Returns:
        File content as string
    """
    encodings = ['utf-8', 'latin-1', 'cp1252']
    
    for encoding in encodings:
        try:
            with open(file_path, 'r', encoding=encoding) as f:
                return f.read()
        except (UnicodeDecodeError, UnicodeError):
            continue
        except (IOError, OSError) as e:
            return f"[Error reading file: {e}]"
    
    return "[Error: Could not decode file with any supported encoding]"


def generate_repodump(root_path: str, output_path: str, tree_only: bool = False, 
                     contents_only: bool = False, ignore_gitignore: bool = False,
                     filter_patterns: Optional[List[str]] = None,
                     filter_files_patterns: Optional[List[str]] = None,
                     exclude_patterns: Optional[List[str]] = None,
                     exclude_files_patterns: Optional[List[str]] = None,
                     exclude_from_file: Optional[str] = None,
                     include_patterns: Optional[List[str]] = None,
                     prompt_text: Optional[str] = None) -> Tuple[str, int, int]:
    """
    Generate a repository dump file with directory structure and file contents.
    
    Args:
        root_path: Path to the root directory
        output_path: Path for the output file
        tree_only: Only include directory structure
        contents_only: Only include file contents
        ignore_gitignore: Ignore .gitignore files
        filter_patterns: Only include files matching these patterns
        filter_files_patterns: Only include file contents for files matching these patterns
        exclude_patterns: Exclude files matching these patterns
        exclude_files_patterns: Exclude file contents for files matching these patterns
        exclude_from_file: File containing exclusion patterns
        include_patterns: Include files matching these patterns (overrides exclusions)
        prompt_text: Additional prompt text to add at the end
        
    Returns:
        Tuple of (directory_structure_str, files_in_structure_count, files_in_contents_count)
    """
    # Initialize pattern lists
    filter_patterns = filter_patterns or []
    filter_files_patterns = filter_files_patterns or []
    exclude_patterns = exclude_patterns or []
    exclude_files_patterns = exclude_files_patterns or []
    include_patterns = include_patterns or []
    
    # Add always excluded patterns
    always_excluded = ['.git/', '.git']
    exclude_patterns.extend(always_excluded)
    
    # Collect gitignore patterns
    gitignore_patterns = collect_gitignore_patterns(root_path, ignore_gitignore)
    exclude_patterns.extend(gitignore_patterns)
    
    # Add patterns from exclude file
    if exclude_from_file:
        if not os.path.isfile(exclude_from_file):
            print(f"Warning: Exclude file '{exclude_from_file}' not found", file=sys.stderr)
        else:
            file_patterns = parse_gitignore_patterns(exclude_from_file)
            exclude_patterns.extend(file_patterns)
    
    # Collect all files with inclusion decisions
    all_files = collect_all_files(root_path, include_patterns, exclude_patterns, 
                                filter_patterns, filter_files_patterns, exclude_files_patterns)
    
    # Count files
    files_in_structure = sum(1 for f in all_files if not f.is_dir and f.include_in_structure)
    files_in_contents = sum(1 for f in all_files if not f.is_dir and f.include_content)
    
    output_parts = []
    
    # Generate directory structure
    structure = ""
    if not contents_only:
        structure = build_directory_structure(root_path, all_files)
        output_parts.append(structure)
    
    # Generate file contents
    if not tree_only:
        content_files = [f for f in all_files if not f.is_dir and f.include_content]
        content_files.sort(key=lambda x: x.rel_path)
        
        if content_files and not contents_only:
            output_parts.append("")  # Add separator
        
        for file_info in content_files:
            # File header
            header = f"{'=' * 48}\nFILE: {file_info.rel_path}\n{'=' * 48}"
            output_parts.append(header)
            
            # File content
            content = read_file_content(file_info.abs_path)
            output_parts.append(content)
            output_parts.append("")  # Add whitespace after file
    
    # Add prompt text
    if prompt_text:
        if output_parts and output_parts[-1] != "":
            output_parts.append("")
        output_parts.append(f"Prompt: {prompt_text}")
    
    # Write output file
    output_content = "\n".join(output_parts)
    try:
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(output_content)
    except (IOError, OSError) as e:
        print(f"Error writing output file: {e}", file=sys.stderr)
        sys.exit(1)
    
    return structure, files_in_structure, files_in_contents


def main():
    """Main function to handle command line arguments and execute the script."""
    parser = argparse.ArgumentParser(
        description="Generate LLM-friendly repository dumps with directory structure and file contents."
    )
    
    # Required argument
    parser.add_argument(
        "directory", 
        help="Path to the input repository or directory"
    )
    
    # Optional arguments with both long and short forms
    parser.add_argument(
        "-o", "--output", 
        default="repodump.txt",
        help="Output file path (default: repodump.txt)"
    )
    parser.add_argument(
        "-t", "--tree", 
        action="store_true",
        help="Only include the directory structure but not the file contents"
    )
    parser.add_argument(
        "-c", "--contents", 
        action="store_true",
        help="Only include the file contents but not the directory structure"
    )
    parser.add_argument(
        "-i", "--ignore-gitignore", 
        action="store_true",
        help="Ignore .gitignore files"
    )
    parser.add_argument(
        "-f", "--filter", 
        action="append", 
        dest="filter_patterns",
        help="Only include files matching these patterns (can be used multiple times)"
    )
    parser.add_argument(
        "-F", "--filter-files", 
        action="append", 
        dest="filter_files_patterns",
        help="Only include file contents for files matching these patterns"
    )
    parser.add_argument(
        "-e", "--exclude", 
        action="append", 
        dest="exclude_patterns",
        help="Exclude files matching these patterns (can be used multiple times)"
    )
    parser.add_argument(
        "-E", "--exclude-files", 
        action="append", 
        dest="exclude_files_patterns",
        help="Exclude file contents for files matching these patterns"
    )
    parser.add_argument(
        "-x", "--exclude-from", 
        dest="exclude_from_file",
        help="File containing patterns to exclude (gitignore format)"
    )
    parser.add_argument(
        "-I", "--include", 
        action="append", 
        dest="include_patterns",
        help="Include files matching these patterns, overriding exclusions"
    )
    parser.add_argument(
        "-p", "--prompt", 
        dest="prompt_text",
        help="Add prompt text at the bottom of the repodump file"
    )
    parser.add_argument(
        "-q", "--quiet", 
        action="store_true",
        help="Do not output a summary to stdout"
    )
    
    args = parser.parse_args()
    
    # Validate directory
    if not os.path.isdir(args.directory):
        print(f"Error: Directory '{args.directory}' does not exist or is not a directory", file=sys.stderr)
        sys.exit(1)
    
    # Validate mutually exclusive options
    if args.tree and args.contents:
        print("Error: --tree and --contents options are mutually exclusive", file=sys.stderr)
        sys.exit(1)
    
    # Generate the dump
    try:
        structure, files_in_structure, files_in_contents = generate_repodump(
            root_path=args.directory,
            output_path=args.output,
            tree_only=args.tree,
            contents_only=args.contents,
            ignore_gitignore=args.ignore_gitignore,
            filter_patterns=args.filter_patterns,
            filter_files_patterns=args.filter_files_patterns,
            exclude_patterns=args.exclude_patterns,
            exclude_files_patterns=args.exclude_files_patterns,
            exclude_from_file=args.exclude_from_file,
            include_patterns=args.include_patterns,
            prompt_text=args.prompt_text
        )
        
        # Output summary unless quiet
        if not args.quiet:
            # Get output file size
            output_size = os.path.getsize(args.output)
            
            # Calculate estimated tokens
            with open(args.output, 'r', encoding='utf-8') as f:
                content = f.read()
                estimated_tokens = len(content) // 4
            
            # Get directory name
            dir_name = os.path.basename(os.path.abspath(args.directory))
            
            print(f"Repository: {dir_name}")
            print(f"Files in structure: {files_in_structure}")
            print(f"Files in contents: {files_in_contents}")
            print(f"Output size: {output_size} bytes")
            print(f"Estimated tokens: {estimated_tokens}")
            
    except KeyboardInterrupt:
        print("\nOperation cancelled by user", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()