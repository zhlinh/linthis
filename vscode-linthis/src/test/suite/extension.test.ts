import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Extension Test Suite', () => {
  vscode.window.showInformationMessage('Start all tests.');

  test('Extension should be present', () => {
    assert.ok(vscode.extensions.getExtension('linthis.linthis'));
  });

  test('Extension should activate', async () => {
    const ext = vscode.extensions.getExtension('linthis.linthis');
    assert.ok(ext);
    await ext!.activate();
    assert.strictEqual(ext!.isActive, true);
  });

  test('Commands should be registered', async () => {
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes('linthis.lint'));
    assert.ok(commands.includes('linthis.format'));
    assert.ok(commands.includes('linthis.restart'));
  });

  test('Configuration should have expected settings', () => {
    const config = vscode.workspace.getConfiguration('linthis');
    assert.ok(config.has('enable'));
    assert.ok(config.has('lintOnSave'));
    assert.ok(config.has('formatOnSave'));
    assert.ok(config.has('executablePath'));
  });
});
