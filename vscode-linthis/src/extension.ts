import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node';
import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

let client: LanguageClient | undefined;

/**
 * Find linthis executable in common locations
 */
function findLinthisExecutable(): string {
  const isWindows = process.platform === 'win32';
  const executableName = isWindows ? 'linthis.exe' : 'linthis';

  // Common installation paths
  const homeDir = os.homedir();
  const possiblePaths = [
    path.join(homeDir, '.cargo', 'bin', executableName),
    path.join(homeDir, '.local', 'bin', executableName),
    '/opt/homebrew/bin/linthis',
    '/usr/local/bin/linthis',
    '/usr/bin/linthis',
  ];

  for (const p of possiblePaths) {
    if (fs.existsSync(p)) {
      return p;
    }
  }

  // Check PATH
  const pathEnv = process.env.PATH || '';
  const pathDirs = pathEnv.split(path.delimiter);

  for (const dir of pathDirs) {
    const fullPath = path.join(dir, executableName);
    if (fs.existsSync(fullPath)) {
      return fullPath;
    }
  }

  // Default to just "linthis" and hope it's in PATH
  return 'linthis';
}

/**
 * Get the linthis executable path from config or auto-detect
 */
function getLinthisPath(config: vscode.WorkspaceConfiguration): string {
  const configuredPath = config.get<string>('executable.path', '');
  if (configuredPath && configuredPath.trim() !== '') {
    return configuredPath;
  }
  return findLinthisExecutable();
}

/**
 * Parse additional arguments string into array
 */
function parseAdditionalArguments(config: vscode.WorkspaceConfiguration): string[] {
  const argsString = config.get<string>('executable.additionalArguments', '');
  if (!argsString || argsString.trim() === '') {
    return [];
  }
  // Split by whitespace, filter empty strings
  return argsString.trim().split(/\s+/).filter(arg => arg.length > 0);
}

// Supported languages matching the LSP server
const SUPPORTED_LANGUAGES = [
  'rust',
  'python',
  'typescript',
  'javascript',
  'typescriptreact',
  'javascriptreact',
  'go',
  'java',
  'cpp',
  'c',
  'objective-c',
  'swift',
  'kotlin',
  'lua',
  'dart',
  'shellscript',
  'ruby',
  'php',
  'scala',
  'csharp',
];

// Helper function to format a document
async function formatDocument(
  filePath: string,
  config: vscode.WorkspaceConfiguration,
  outputChannel: vscode.OutputChannel,
  showMessages: boolean = true
): Promise<boolean> {
  const executablePath = getLinthisPath(config);
  const additionalArgs = parseAdditionalArguments(config);

  try {
    if (showMessages) {
      outputChannel.appendLine(`[info] Formatting file: ${filePath}`);
    }

    // Run linthis format-only
    const args = ['-f', '-i', filePath, ...additionalArgs];
    const command = `"${executablePath}" ${args.map(a => `"${a}"`).join(' ')}`;
    outputChannel.appendLine(`[info] Running: ${command}`);

    const result = execSync(command, {
      encoding: 'utf-8',
      cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath,
      timeout: 10000,
    });

    if (showMessages) {
      outputChannel.appendLine(`[info] Format result: ${result}`);
      vscode.window.showInformationMessage('Linthis: Document formatted successfully');
    }
    return true;
  } catch (error: any) {
    // execSync throws on non-zero exit code
    const stdout = error.stdout || '';
    const stderr = error.stderr || '';
    const exitCode = error.status || 0;

    outputChannel.appendLine(`[error] Format exit code: ${exitCode}`);
    if (stdout) outputChannel.appendLine(`[error] stdout: ${stdout}`);
    if (stderr) outputChannel.appendLine(`[error] stderr: ${stderr}`);

    // Check if the error is due to syntax errors
    const output = stdout + stderr;
    if (showMessages) {
      if (output.includes('formatting errors') || output.includes('syntax error')) {
        vscode.window.showErrorMessage(
          'Linthis: Cannot format file with syntax errors. Fix syntax errors first.'
        );
      } else if (exitCode !== 0) {
        vscode.window.showErrorMessage(
          `Linthis: Format failed (exit code ${exitCode}). Check Output panel for details.`
        );
      } else {
        const errorMsg = error.message || String(error);
        outputChannel.appendLine(`[error] Format failed: ${errorMsg}`);
        vscode.window.showErrorMessage(`Linthis: Format failed. Check Output panel for details.`);
      }
    }
    return false;
  }
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const config = vscode.workspace.getConfiguration('linthis');
  const outputChannel = vscode.window.createOutputChannel('Linthis');

  if (!config.get<boolean>('enable', true)) {
    outputChannel.appendLine('[info] Extension is disabled via linthis.enable setting');
    return;
  }

  // Register commands first (so they're available even if LSP fails)
  context.subscriptions.push(
    vscode.commands.registerCommand('linthis.lint', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showWarningMessage('Linthis: No active editor');
        return;
      }

      if (!client) {
        vscode.window.showWarningMessage('Linthis: Language server not running');
        return;
      }

      // Save to trigger LSP diagnostics
      if (editor.document.isDirty) {
        await editor.document.save();
      }

      // Wait a moment for diagnostics to update
      await new Promise(resolve => setTimeout(resolve, 500));

      // Get diagnostics for this file
      const diagnostics = vscode.languages.getDiagnostics(editor.document.uri);

      // Linthis LSP server prefixes diagnostics with "linthis-" (e.g., "linthis-ruff")
      // This makes it clear these diagnostics come from linthis
      const linthisDiagnostics = diagnostics.filter(d =>
        d.source && (d.source.startsWith('linthis-') || d.source === 'linthis')
      );

      if (linthisDiagnostics.length > 0) {
        vscode.window.showInformationMessage(
          `Linthis: Found ${linthisDiagnostics.length} issue(s). Check Problems panel.`
        );
        // Focus on Problems panel
        vscode.commands.executeCommand('workbench.actions.view.problems');
      } else if (diagnostics.length > 0) {
        vscode.window.showInformationMessage(
          `Found ${diagnostics.length} issue(s) from other tools. Linthis found no issues.`
        );
      } else {
        vscode.window.showInformationMessage('Linthis: No issues found! ✓');
      }
    }),

    vscode.commands.registerCommand('linthis.format', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showWarningMessage('Linthis: No active editor');
        return;
      }

      // Save the file first
      if (editor.document.isDirty) {
        await editor.document.save();
      }

      const filePath = editor.document.uri.fsPath;
      await formatDocument(filePath, config, outputChannel, true);
    }),

    vscode.commands.registerCommand('linthis.restart', async () => {
      if (client) {
        outputChannel.appendLine('[info] Restarting language server...');
        await client.stop();
        client = createLanguageClient(config, outputChannel);
        await startClientSafely(client, outputChannel);
        vscode.window.showInformationMessage('Linthis: Language server restarted');
      } else {
        vscode.window.showWarningMessage('Linthis: No language server to restart');
      }
    })
  );

  // Register format on save (always register, but check config each time)
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(async (document) => {
      // Check if format on save is enabled
      const currentConfig = vscode.workspace.getConfiguration('linthis');
      if (!currentConfig.get<boolean>('formatOnSave', false)) {
        return;
      }

      // Only format supported file types
      const supportedLanguages = [
        'rust', 'python', 'typescript', 'javascript',
        'typescriptreact', 'javascriptreact', 'go', 'java',
        'cpp', 'c', 'objective-c', 'swift', 'kotlin',
      ];

      if (!supportedLanguages.includes(document.languageId)) {
        return;
      }

      const filePath = document.uri.fsPath;
      outputChannel.appendLine(`[info] Format on save: ${filePath}`);

      // Format the file using linthis -f -i (in-place)
      const executablePath = getLinthisPath(currentConfig);
      const additionalArgs = parseAdditionalArguments(currentConfig);
      const args = ['-f', '-i', filePath, ...additionalArgs];
      const command = `"${executablePath}" ${args.map(a => `"${a}"`).join(' ')}`;

      try {
        const result = execSync(command, {
          encoding: 'utf-8',
          cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath,
          timeout: 10000,
        });

        outputChannel.appendLine(`[info] Format completed`);
      } catch (error: any) {
        // Log error details
        const exitCode = error.status || 0;
        outputChannel.appendLine(`[error] Format on save failed with exit code: ${exitCode}`);
        if (error.stdout) {
          outputChannel.appendLine(`[error] stdout: ${error.stdout}`);
        }
        if (error.stderr) {
          outputChannel.appendLine(`[error] stderr: ${error.stderr}`);
        }
      }
    })
  );

  // Register lint on open
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument(async (document) => {
      // Check if lint on open is enabled
      const currentConfig = vscode.workspace.getConfiguration('linthis');
      if (!currentConfig.get<boolean>('lintOnOpen', true)) {
        return;
      }

      // Only lint supported file types
      if (!SUPPORTED_LANGUAGES.includes(document.languageId)) {
        return;
      }

      // Only lint real files
      if (document.uri.scheme !== 'file') {
        return;
      }

      outputChannel.appendLine(`[info] Lint on open: ${document.uri.fsPath}`);

      // Delay to let LSP attach and then trigger diagnostics refresh
      setTimeout(async () => {
        if (client && client.isRunning()) {
          // Send a didSave notification to trigger LSP diagnostics
          // This is a workaround since LSP should already send diagnostics on open
          try {
            await client.sendNotification('textDocument/didSave', {
              textDocument: { uri: document.uri.toString() },
            });
          } catch (error) {
            outputChannel.appendLine(`[error] Lint on open failed: ${error}`);
          }
        }
      }, 500);
    })
  );

  // Log initial state
  if (config.get<boolean>('formatOnSave', false)) {
    outputChannel.appendLine('[info] Format on save is enabled');
  }
  if (config.get<boolean>('lintOnSave', true)) {
    outputChannel.appendLine('[info] Lint on save is enabled');
  } else {
    outputChannel.appendLine('[info] Lint on save is disabled');
  }
  if (config.get<boolean>('lintOnOpen', true)) {
    outputChannel.appendLine('[info] Lint on open is enabled');
  } else {
    outputChannel.appendLine('[info] Lint on open is disabled');
  }

  // Watch for configuration changes
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (event.affectsConfiguration('linthis.formatOnSave')) {
        const newConfig = vscode.workspace.getConfiguration('linthis');
        const formatOnSave = newConfig.get<boolean>('formatOnSave', false);

        if (formatOnSave) {
          outputChannel.appendLine('[info] Format on save enabled');

          // Show a helpful message to the user
          const action = await vscode.window.showInformationMessage(
            'Linthis: Format on save enabled. Format all open files now?',
            'Format All',
            'Skip'
          );

          if (action === 'Format All') {
            // Format all open text documents
            const supportedLanguages = [
              'rust', 'python', 'typescript', 'javascript',
              'typescriptreact', 'javascriptreact', 'go', 'java',
              'cpp', 'c', 'objective-c', 'swift', 'kotlin',
            ];

            const documentsToFormat = vscode.workspace.textDocuments.filter(doc =>
              supportedLanguages.includes(doc.languageId) &&
              doc.uri.scheme === 'file' // Only format real files, not settings
            );

            if (documentsToFormat.length > 0) {
              outputChannel.appendLine(`[info] Formatting ${documentsToFormat.length} open file(s)`);

              for (const doc of documentsToFormat) {
                await formatDocument(doc.uri.fsPath, newConfig, outputChannel, false);
              }

              vscode.window.showInformationMessage(
                `Linthis: Formatted ${documentsToFormat.length} file(s)`
              );
            } else {
              vscode.window.showInformationMessage('Linthis: No files to format');
            }
          }
        } else {
          outputChannel.appendLine('[info] Format on save disabled');
        }
      }
    })
  );

  // Start the language client
  try {
    client = createLanguageClient(config, outputChannel);
    await startClientSafely(client, outputChannel);
    context.subscriptions.push(client);
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    outputChannel.appendLine(`[error] Failed to start language server: ${errorMsg}`);
    vscode.window.showErrorMessage(
      `Linthis: Failed to start language server. Check Output panel for details.`
    );
    // Don't throw - allow extension to continue with commands available
  }
}

async function startClientSafely(
  client: LanguageClient,
  outputChannel: vscode.OutputChannel
): Promise<void> {
  outputChannel.appendLine('[info] Starting language server...');

  // Use a timeout to prevent indefinite hanging
  const startPromise = client.start();
  const timeoutPromise = new Promise<never>((_, reject) => {
    setTimeout(() => reject(new Error('Language server start timeout (10s)')), 10000);
  });

  try {
    await Promise.race([startPromise, timeoutPromise]);
    outputChannel.appendLine('[info] Language server started successfully');
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    outputChannel.appendLine(`[error] Failed to start: ${errorMsg}`);
    throw error;
  }
}

function createLanguageClient(
  config: vscode.WorkspaceConfiguration,
  outputChannel: vscode.OutputChannel
): LanguageClient {
  const executablePath = getLinthisPath(config);
  const additionalArgs = parseAdditionalArguments(config);

  outputChannel.appendLine(`[info] Using linthis executable: ${executablePath}`);
  outputChannel.appendLine(`[info] LSP arguments: lsp ${additionalArgs.join(' ')}`);

  const serverOptions: ServerOptions = {
    command: executablePath,
    args: ['lsp', ...additionalArgs],
  };

  const documentSelector = SUPPORTED_LANGUAGES.map((language) => ({
    scheme: 'file',
    language,
  }));

  const clientOptions: LanguageClientOptions = {
    documentSelector,
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/.linthis.toml'),
    },
    outputChannel,
    traceOutputChannel: vscode.window.createOutputChannel('Linthis Trace'),
    middleware: {
      didSave: async (document, next) => {
        // Check lintOnSave configuration before sending didSave notification
        const currentConfig = vscode.workspace.getConfiguration('linthis');
        if (currentConfig.get<boolean>('lintOnSave', true)) {
          return next(document);
        }
        // If lintOnSave is disabled, don't send didSave notification to LSP
        return Promise.resolve();
      },
    },
  };

  return new LanguageClient(
    'linthis',
    'Linthis Language Server',
    serverOptions,
    clientOptions
  );
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
  }
}
