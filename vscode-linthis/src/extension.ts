import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

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

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const config = vscode.workspace.getConfiguration('linthis');

  if (!config.get<boolean>('enable', true)) {
    return;
  }

  // Start the language client
  client = createLanguageClient(config);

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('linthis.lint', async () => {
      const editor = vscode.window.activeTextEditor;
      if (editor && client) {
        // Trigger a save to run diagnostics
        await editor.document.save();
        vscode.window.showInformationMessage('Linthis: Lint completed');
      }
    }),

    vscode.commands.registerCommand('linthis.format', async () => {
      const editor = vscode.window.activeTextEditor;
      if (editor) {
        await vscode.commands.executeCommand('editor.action.formatDocument');
      }
    }),

    vscode.commands.registerCommand('linthis.restart', async () => {
      if (client) {
        await client.stop();
        client = createLanguageClient(config);
        await client.start();
        vscode.window.showInformationMessage('Linthis: Language server restarted');
      }
    })
  );

  // Start the client
  await client.start();
  context.subscriptions.push(client);
}

function createLanguageClient(
  config: vscode.WorkspaceConfiguration
): LanguageClient {
  const executablePath = config.get<string>('executablePath', 'linthis');
  const extraArgs = config.get<string[]>('extraArgs', []);

  const serverOptions: ServerOptions = {
    command: executablePath,
    args: ['lsp', '--mode', 'stdio', ...extraArgs],
    transport: TransportKind.stdio,
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
    outputChannelName: 'Linthis',
    traceOutputChannel: vscode.window.createOutputChannel('Linthis Trace'),
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
