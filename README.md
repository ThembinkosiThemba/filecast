# files: A TUI File Manager

A TUI file manager built with Rust. The project provides file navigation with persistent history tracking and file preview capabilities.

## Features

- **Smart Search**: Live filename filtering and content search using ripgrep/grep
- **Three-Pane Layout**: Switchable panes for history, file list, and preview
- **Vim-like Navigation**: Intuitive keybindings for fast file browsing
- **File Operations**: Launch files with system default applications
- **Persistent History**: SQLite-based tracking of recently accessed files
- **Navigation Stack**: Browser-like back/forward history (Ctrl+H, Ctrl+L)
- **Command Mode**: Execute shell commands directly from the TUI
- **File Preview**: Text file preview (up to 100KB) with metadata display
- **Hidden Files**: Toggle visibility of hidden files
- **Smart Filtering**: Real-time filename filtering without grep syntax
- **Content Search**: Search inside files using @ prefix with ripgrep/grep

## Installation

### From Crates.io

Installing directly from crates.io:

```bash
cargo install files-tui
```

This installs the binary as `files`.

### Building from Source

```bash
git clone https://github.com/ThembinkosiThemba/files.git
cd files
cargo build --release
```

The binary will be available at `target/release/files`.

## Usage

Run the application:

```bash
cargo run --release
# or if installed:
./target/release/files
```

### Keybindings

#### Normal Mode

| Key         | Action                                                |
| :---------- | :---------------------------------------------------- |
| `Tab`       | Cycle through panes (History → Files → Preview)       |
| `1`         | Focus History pane                                    |
| `2`         | Focus File List pane                                  |
| `3`         | Focus Preview pane                                    |
| `j` / Down  | Move selection down in active pane                    |
| `k` / Up    | Move selection up in active pane                      |
| `l` / Right | Enter directory or open file (File List/History pane) |
| `h` / Left  | Go to parent directory (File List pane only)          |
| `Enter`     | Enter directory or open file (File List/History pane) |
| `Backspace` | Go to parent directory (File List pane only)          |
| `Ctrl+H`    | Navigate backward in history                          |
| `Ctrl+L`    | Navigate forward in history                           |
| `/`         | Enter search mode (filename filter)                   |
| `.`         | Toggle hidden files visibility                        |
| `r`         | Refresh current directory                             |
| `:`         | Enter command mode                                    |
| `q`         | Quit application                                      |

#### Search Mode

| Key         | Action                                 |
| :---------- | :------------------------------------- |
| `Esc`       | Exit search mode and clear filter      |
| `Enter`     | Apply filter or execute content search |
| `Char`      | Add character to query (live filter)   |
| `Backspace` | Remove last character                  |
| `@prefix`   | Switch to content search mode          |

#### Command Mode

| Key         | Action                               |
| :---------- | :----------------------------------- |
| `Esc`       | Exit command mode                    |
| `Enter`     | Execute command in current directory |
| `Char`      | Add character to command             |
| `Backspace` | Remove last character                |

## Usage Examples

### Basic File Navigation

1. Start the application in any directory
2. Use `j`/`k` or arrow keys to navigate files
3. Press `Enter` or `l` to open files/enter directories
4. Press `h` or `Backspace` to go to parent directory

### Smart Search Examples

#### Filename Filtering (Live)

Press `/` and start typing to filter files in real-time:

- `/doc` - Shows only files containing "doc" in their name
- `/\.rs$` - Shows only Rust files (case-insensitive)
- Press `Esc` to clear the filter or `Enter` to keep it active

#### Content Search (No Grep Syntax Needed!)

Press `/` then type `@` followed by your search term:

- `/@TODO` - Find all files containing "TODO"
- `/@function main` - Find files with "function main"
- `/@error.*panic` - Search with regex pattern
  Results appear in the preview pane with file:line:content format

### Command Mode Examples

Press `:` to enter command mode and execute shell commands:

- `:git status` - Run git status in current directory
- `:mkdir newfolder` - Create a new directory
- `:ls -la` - List all files including hidden
- `:find . -name "*.log"` - Find log files

Output appears in the preview pane.

### Pane Switching

- Press `Tab` to cycle through panes (History → Files → Preview)
- Press `1` to focus History pane
- Press `2` to focus File List pane
- Press `3` to focus Preview pane

When History pane is focused, you can:

- Use `j`/`k` to navigate recent files
- Press `Enter` to open/navigate to selected item

### Hidden Files

- Press `.` to toggle hidden files visibility
- Useful for viewing .git, .config, and other dotfiles

### Directory Refresh

- Press `r` to refresh the current directory
- Useful after external changes to files

## Project Structure

```
files/
├── src/
│   ├── main.rs              # Entry point and event loop
│   └── core/
│       ├── mod.rs           # Module declarations
│       ├── app.rs           # Application state and logic
│       ├── event.rs         # Event definitions
│       ├── mode.rs          # Application modes (Normal, Search, Command)
│       ├── ui.rs            # UI rendering
│       ├── fs.rs            # File system operations
│       └── history.rs       # SQLite history tracking
└── Cargo.toml
```

## Technical Details

### Search Implementation

- **Filename filtering**: Case-insensitive substring matching with live updates
- **Content search**: Uses ripgrep (rg) if available, falls back to grep
- **No syntax required**: Just type what you're looking for, the tool handles the rest
- **Smart detection**: Automatically detects @ prefix for content vs filename search

### Command Execution

- Commands run in the current directory context
- Output captured and displayed in preview pane
- Uses tokio async process execution
- Both stdout and stderr are captured

### Performance

- SQLite database for efficient history tracking
- Async I/O for non-blocking command execution
- Lazy loading of file previews
- Efficient directory caching

## License

MIT License
