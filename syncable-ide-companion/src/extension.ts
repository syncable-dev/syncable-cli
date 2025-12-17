/**
 * Syncable IDE Companion Extension
 *
 * This extension enables seamless integration between the Syncable CLI/Agent
 * and VS Code. It provides:
 *
 * - Native diff views for file changes proposed by the agent
 * - IDE context sharing (open files, active file, cursor position)
 * - Accept/reject workflow for AI-proposed changes
 */

import * as vscode from 'vscode';
import { IDEServer } from './ide-server';
import { DiffContentProvider, DiffManager, DIFF_SCHEME } from './diff-manager';

const INFO_MESSAGE_SHOWN_KEY = 'syncableInfoMessageShown';

let ideServer: IDEServer;
let logger: vscode.OutputChannel;
let log: (message: string) => void = () => {};

function createLogger(context: vscode.ExtensionContext, outputChannel: vscode.OutputChannel) {
  return (message: string) => {
    const isLoggingEnabled = vscode.workspace
      .getConfiguration('syncable.debug.logging')
      .get('enabled', false);

    if (isLoggingEnabled) {
      const timestamp = new Date().toISOString();
      outputChannel.appendLine(`[${timestamp}] ${message}`);
    }
  };
}

export async function activate(context: vscode.ExtensionContext) {
  logger = vscode.window.createOutputChannel('Syncable IDE Companion');
  log = createLogger(context, logger);
  log('Extension activated');

  const diffContentProvider = new DiffContentProvider();
  const diffManager = new DiffManager(log, diffContentProvider);

  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument((doc) => {
      if (doc.uri.scheme === DIFF_SCHEME) {
        diffManager.cancelDiff(doc.uri);
      }
    }),
    vscode.workspace.registerTextDocumentContentProvider(DIFF_SCHEME, diffContentProvider),
    vscode.commands.registerCommand('syncable.diff.accept', (uri?: vscode.Uri) => {
      const docUri = uri ?? vscode.window.activeTextEditor?.document.uri;
      if (docUri && docUri.scheme === DIFF_SCHEME) {
        diffManager.acceptDiff(docUri);
      }
    }),
    vscode.commands.registerCommand('syncable.diff.cancel', (uri?: vscode.Uri) => {
      const docUri = uri ?? vscode.window.activeTextEditor?.document.uri;
      if (docUri && docUri.scheme === DIFF_SCHEME) {
        diffManager.cancelDiff(docUri);
      }
    })
  );

  ideServer = new IDEServer(log, diffManager);
  try {
    await ideServer.start(context);
    log('Syncable IDE Companion started successfully');
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    log(`Failed to start IDE server: ${message}`);
  }

  if (!context.globalState.get(INFO_MESSAGE_SHOWN_KEY)) {
    void vscode.window.showInformationMessage(
      'Syncable IDE Companion extension successfully installed.'
    );
    context.globalState.update(INFO_MESSAGE_SHOWN_KEY, true);
  }

  context.subscriptions.push(
    vscode.workspace.onDidChangeWorkspaceFolders(() => {
      ideServer.syncEnvVars();
    }),
    vscode.workspace.onDidGrantWorkspaceTrust(() => {
      ideServer.syncEnvVars();
    }),
    vscode.commands.registerCommand('syncable.showStatus', () => {
      logger.show();
      const serverStatus = ideServer ? 'Running' : 'Stopped';
      vscode.window.showInformationMessage(
        `Syncable IDE Companion Status: ${serverStatus}. Check Output panel for details.`
      );
    })
  );
}

export async function deactivate(): Promise<void> {
  log('Extension deactivated');
  try {
    if (ideServer) {
      await ideServer.stop();
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    log(`Failed to stop IDE server during deactivation: ${message}`);
  } finally {
    if (logger) {
      logger.dispose();
    }
  }
}
