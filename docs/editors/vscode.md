# VS Code Extension

The official VS Code extension for linthis provides seamless integration with the linthis CLI tool, offering real-time linting, formatting, and diagnostics directly in your editor.

## Installation

### From VS Code Marketplace

1. Open VS Code
2. Go to Extensions (`Cmd/Ctrl + Shift + X`)
3. Search for "Linthis"
4. Click "Install"

### From Source

```bash
cd vscode-linthis
npm install
npm run build
# Press F5 to launch extension development host
```

## Features

### 🔍 Real-time Linting via LSP

- Language Server Protocol (LSP) integration
- Real-time diagnostics as you type
- Automatic linting on file save (configurable)
- Support for multiple languages (Python, TypeScript, Rust, Go, Java, C/C++, Swift, Kotlin, and more)

### 🎨 Format on Save

- Automatically format files on save
- Configurable per-workspace or globally
- Silent operation with output logging
- Interactive prompt when enabling

### 🛠️ Commands

- **Linthis: Lint Document** - Manually trigger linting for current file
- **Linthis: Format Document** - Manually format current file
- **Linthis: Restart Language Server** - Restart LSP server if needed

## Configuration

### Settings

All settings are prefixed with `linthis.`:

#### Basic Settings

```json
{
  "linthis.enable": true,
  "linthis.executablePath": "linthis"
}
```

- `linthis.enable` (boolean, default: `true`) - Enable/disable the extension
- `linthis.executablePath` (string, default: `"linthis"`) - Path to linthis executable

#### Automatic Features

```json
{
  "linthis.lintOnSave": true,
  "linthis.formatOnSave": false
}
```

- `linthis.lintOnSave` (boolean, default: `true`) - Lint document on save
- `linthis.formatOnSave` (boolean, default: `false`) - Format document on save

#### Advanced Settings

```json
{
  "linthis.extraArgs": [],
  "linthis.trace.server": "off"
}
```

- `linthis.extraArgs` (array, default: `[]`) - Extra arguments for LSP server
- `linthis.trace.server` (string, default: `"off"`) - LSP server trace level

### Example Configuration

#### Minimal Setup

```json
{
  "linthis.enable": true
}
```

#### Full Auto Mode

```json
{
  "linthis.enable": true,
  "linthis.lintOnSave": true,
  "linthis.formatOnSave": true
}
```

#### Custom Executable Path

```json
{
  "linthis.executablePath": "/usr/local/bin/linthis"
}
```

## How It Works

### LSP Integration

The extension communicates with the linthis CLI via Language Server Protocol:

1. **Startup**: Extension launches `linthis lsp` as a child process
2. **Document Events**: VS Code sends document changes to LSP server
3. **Diagnostics**: LSP server runs linters and sends diagnostics back
4. **Display**: Diagnostics appear in Problems panel with `[linthis-ruff]` prefix

### Format on Save

When `formatOnSave` is enabled:

1. User saves file (`Cmd/Ctrl + S`)
2. VS Code saves the file to disk
3. Extension triggers `linthis -f -i <file>`
4. File is formatted in-place
5. VS Code detects change and reloads file

### Lint on Save vs Format on Save

| Feature | Lint on Save | Format on Save |
|---------|-------------|----------------|
| Trigger | Save event | After save |
| Method | LSP diagnostics | CLI format |
| Result | Problems panel | File modification |
| Default | Enabled | Disabled |

## Usage

### Daily Workflow

1. **Open a file** - LSP server starts automatically
2. **Edit code** - See real-time diagnostics in Problems panel
3. **Save file** (`Cmd/Ctrl + S`) - Auto-lint and format (if enabled)
4. **Check Problems** (`Cmd/Ctrl + Shift + M`) - Review all issues

### Manual Commands

Press `Cmd/Ctrl + Shift + P` to open command palette:

- Type "Linthis: Lint Document" - Manually lint current file
- Type "Linthis: Format Document" - Manually format current file
- Type "Linthis: Restart Language Server" - Restart if issues occur

### Format on Save Setup

When you enable `formatOnSave`, you'll see a prompt:

```
Linthis: Format on save enabled. Format all open files now?
[Format All]  [Skip]
```

- **Format All** - Immediately format all open code files
- **Skip** - Wait until next save to format

## Diagnostic Sources

Diagnostics in the Problems panel are prefixed with their source:

- `[linthis-ruff]` - Python linting (ruff)
- `[linthis-eslint]` - JavaScript/TypeScript linting (eslint)
- `[linthis-clippy]` - Rust linting (clippy)
- `[linthis]` - Generic linthis diagnostics

This makes it clear which tool found each issue.

## Supported Languages

The extension supports the same languages as the linthis CLI:

- Python
- TypeScript / JavaScript (including React)
- Rust
- Go
- Java
- C / C++ / Objective-C
- Swift
- Kotlin
- Lua
- Dart
- Shell Script
- Ruby
- PHP
- Scala
- C#

## Troubleshooting

### Extension Not Working

**Check Output Panel**:
1. View → Output
2. Select "Linthis" from dropdown
3. Look for error messages

**Common Issues**:

- `Extension is disabled` - Check `linthis.enable` setting
- `Language server not running` - Check if `linthis` is in PATH
- `Command not found` - Install linthis CLI: `cargo install --path .`

### No Diagnostics on Save

**Check**:
1. `linthis.lintOnSave` is `true`
2. File type is supported
3. LSP server is running (check Output panel)
4. Look in **Problems panel**, not Output panel

### Format on Save Not Working

**Check**:
1. `linthis.formatOnSave` is `true`
2. File type is supported
3. File has no syntax errors
4. Check Output panel for error logs

### LSP Server Timeout

If you see "Language server start timeout":

1. Check if `linthis lsp` runs successfully:
   ```bash
   linthis lsp
   ```
2. Restart extension: "Linthis: Restart Language Server"
3. Check `linthis.trace.server` for detailed logs

## Output Logging

The extension logs to the Output panel ("Linthis" channel):

### Startup Logs
```
[info] Using linthis executable: linthis
[info] LSP arguments: lsp
[info] Starting language server...
[info] Language server started successfully
[info] Lint on save is enabled
[info] Format on save is enabled
```

### Format Logs
```
[info] Format on save: /path/to/file.py
[info] Format completed
```

### Error Logs
```
[error] Format on save failed with exit code: 1
[error] stdout: formatting errors found
```

## Performance

### Small Files
- Lint: < 100ms
- Format: < 200ms
- Nearly instant feedback

### Large Files
- Lint: < 1s
- Format: 1-3s
- Consider disabling auto-format for very large files

### Optimization Tips

1. Use `.linthisignore` to exclude large files
2. Disable `formatOnSave` for large projects
3. Use manual commands when needed
4. Configure LSP timeout if needed

## Best Practices

### Recommended Settings

For most projects:
```json
{
  "linthis.enable": true,
  "linthis.lintOnSave": true,
  "linthis.formatOnSave": true
}
```

### Per-Language Settings

Disable other formatters to avoid conflicts:

```json
{
  "[python]": {
    "editor.formatOnSave": false,
    "editor.defaultFormatter": "linthis.linthis"
  },
  "[typescript]": {
    "editor.formatOnSave": false,
    "editor.defaultFormatter": "linthis.linthis"
  }
}
```

### Workspace Settings

Create `.vscode/settings.json` in your project:

```json
{
  "linthis.enable": true,
  "linthis.lintOnSave": true,
  "linthis.formatOnSave": true,
  "files.associations": {
    ".linthis.toml": "toml"
  }
}
```

## Development

### Building from Source

```bash
git clone https://github.com/lint-group/linthis
cd linthis/vscode-linthis
npm install
npm run build
```

### Testing

Press `F5` in VS Code to launch Extension Development Host.

### Debugging

1. Set breakpoints in TypeScript files
2. Press `F5` to start debugging
3. Use Debug Console to inspect variables

## Related Documentation

- [Linthis CLI Documentation](../reference/cli.md)
- [Configuration Reference](../reference/configuration.md)
- [Language Support](../languages/index.md)

## Feedback

Report issues at: https://github.com/lint-group/linthis/issues
