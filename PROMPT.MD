Generate a Rust crate and application that takes a directory (typically a git repository) as input and generates an LLM friendly text file that contains both the directory structure and the output of the files. By default, the script should respect .gitignore files. Furthermore, the script should have options enabling the user to specify additional glob-style patterns to exclude or include. 

## Arguments

### Positional Arguments
The script should have no required arguments. 

There should be one optional positional argument for the path of the input repository or directory. If this argument is not provided, check if the current directory is within a git repository. If it is, use the root of this repository. If it is not, please print an error message: "The current directory is not a git repository. For use outside of git repositories, please provide a directory path."

### Named Arguments
The script should have several named arguments:
- `--output <file>`: Output file path. By default, the generated file is called `repodump.txt`
- `--tree`: Only include the directory structure but not the file contents.
- `--contents`: Only include the file contents but not the directory structure.
- `--ignore-gitignore`: Ignore .gitignore files.
- `--filter <patterns>`: Only include files matching these patterns.
- `--exclude <patterns>`: Exclude files matching these patterns.
- `--include <patterns>`: Include files matching these patterns, forcefully overriding exclusions by by other patterns.
- `--prompt <prompt_text>`: Add prompt text at the bottom of the repodump file. 
- `--quiet`: Do not output a summary of the generated file to stdout. 

For each of these arguments, you should also generate an aquedate short form. The first letter of each argument name should typically suffice.

## Inclusions and Exclusions
Inclusion and exclusion patterns should be applied in this order:
1. Filter patterns: Files that do match these patterns should be excluded
2. Exclude patterns: Files matching these patterns are excluded (including `.gitignore` and always-excluded patterns like `.git/`)
3. Include patterns: Files matching these patterns are re-included, overriding exclusions

There are certain files that should always be excluded unless specifically overrided with an include:
- .git/

## Output Format 

### Output File

#### Directory Structure
The directory structure should be at the top of the text file. The format should look something like this:
```
Directory Structure:
└── repository/
    ├── README.md
    ├── .gitignore
    ├── src/
    │   ├── folder1/
    │   │   ├── __init__.py
    │   │   ├── __main__.py
    │   │   ├── subfolder/
    │   │   │   ├── __init__.py
    │   │   │   └── file1.py
    │   │   └── utils/
    │   │       ├── __init__.py
    │   │       ├── file1.py
    │   │       ├── file2.py
    │   │       └── file3.py
    │   ├── folder2/
    ...
```

#### File Contents
After the directory structure, the contents of each file should follow. Before each file, write a header that looks like this:

```
================================================
FILE: README.md
================================================
```

If a file is not in the root of the directory, its path should be included in the header:

```
================================================
FILE: src/folder1/__main__.py
================================================
```

After the contents of a file finishes, include a line of whitespace before the header of the next file.

#### Prompt Text
If the user provided the prompt argument, additional prompt text should be added at the bottom of the file. It should look like this:

```
Prompt: <USER PROVIDED TEXT>
```

### Stdout Summary
Your code should output a summary of the generated file. This summary should include the directory name, the number of files included in the directory structure, the number of files included in the file contents, the size of the generated file and the estimated number of LLM tokens in the generated file. 

```
Repository: example-repository
Files in structure: 3   
Files in contents: 3    
Output size: 14781 bytes
Estimated tokens: 3560  
```

#### Token Estimation
To estimate the number of LLM tokens in the generated file, divide the number of characters by 4.

## Dependencies
Your code should not require any non-Rust dependencies. All dependencies should be cross-platform out of the box. You should use the following dependencies:
- `gix`: To check if the cwd is a Git repo and find the root
- `ignore`: For .gitignore parsing
- `globset`: Parse command line glob patterns

## Code Style
- Your code should be clean, readable and modular. 
- Your code should be as short and simple as possible, but not at the cost of readability, modularity or functionality. 
    - In general, avoid having large functions and opt for small, simple and easily understandable functions.
- Each function in your code should have detailed docstrings.
    - Doctests are a huge bonus where they make sense.
