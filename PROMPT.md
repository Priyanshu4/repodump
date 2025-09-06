Generate a Python script that takes a directory (typically a git repository) as input and generates an LLM friendly text file that contains both the directory structure and the output of the files. By default, the script should respect .gitignore files. Furthermore, the script should have options enabling the user to specify additional patterns to exclude or include. These patterns should match the same format as gitignore patterns. There should be an option allowing users to include only a given pattern and exclude everything that does not match this pattern.

## Arguments
For argument parsing, the code should use the argparse libary.

### Required Arguments
The script should only have one required argument. This argument is for the path of the input repository or directory.

### Optional Arguments
The script should have several optional arguments:
- `--output <file>`: Output file path. By default, the generated file is called `repodump.txt`
- `--tree`: Only include the directory structure but not the file contents.
- `--contents`: Only include the file contents but not the directory structure.
- `--ignore-gitignore`: Ignore .gitignore files.
- `--filter <patterns>`: Only include files matching these patterns (applies to both structure and contents).
- `--filter-files <patterns>`: Only include **file contents** for files whose names match these patterns. The directory structure is still shown for all files.
- `--exclude <patterns>`: Exclude files matching these patterns (from both directory structure and file contents).
- `--exclude-files <patterns>`: Exclude **file contents** for files whose names match these patterns. The directory structure is still shown.
- `--exclude-from <exclusion_file>`: A file structured like a gitignore which contains patterns to ignore.
- `--include <patterns>`: A list of patterns to include in the directory structure and file contents, overriding exclusions by by other patterns.
- `--prompt <prompt_text>`: Add prompt text at the bottom of the repodump file. 
- `--quiet`: Do not output a summary of the generated file to stdout. 

For each of these arguments, you should also generated an aquedate short form.

## Inclusions and Exclusions
Inclusion and exclusion patterns should be applied in this order:
1. All files not matching the filters are excluded
2. All files matching the exclude patterns (including the `.gitignore` and always excluded patterns) are excluded
3. All files matching the include patterns are re-included.

Exclusions that only apply to content should be applied after exclusions that apply to both content and directory structure. For instance, `--filter-files` should be applied after `--filter`.

There are certain files that should always be excluded unless specifically overrided with an include:
- *.git/

Always excluded patterns and patterns in the .gitignore are to be excluded from the directory structure and the file contents. Please make sure that you exclude .git folders and all files within them from both the directory structure and file contents.

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
Your code should not require any dependencies that are not within the Python standard library. Your code should work on Python 3.6 and up.

## Code Style
- Your code should be contained within in a single script, but it must be clean and modular. 
    - For instance, do not rewrite the pattern parsing logic multiple times. One function should handle all the pattern parsing.
- Your code should be as short and simple as possible, but not at the cost of readability, modularity or functionality. 
    - In general, avoid having large functions and opt for small, simple and easily understandable functions.
- Your code should be using type annotations. 
- Each function in your code should have detailed docstrings.
- Your code should expose a function that matches the exclusion and inclusion arguments of the script itself. The function should return a tuple that contains some representation of the directory structure and a list of filepaths to include in the file contents.
