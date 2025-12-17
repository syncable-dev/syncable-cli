/**
 * Syncable IDE Companion - Open Files Manager
 */

import * as vscode from 'vscode';
import type { File, IdeContext } from './types';

export const MAX_FILES = 10;
const MAX_SELECTED_TEXT_LENGTH = 16384;

/**
 * Keeps track of the workspace state, including open files, cursor position, and selected text.
 */
export class OpenFilesManager {
  private readonly onDidChangeEmitter = new vscode.EventEmitter<void>();
  readonly onDidChange = this.onDidChangeEmitter.event;
  private debounceTimer: NodeJS.Timeout | undefined;
  private openFiles: File[] = [];

  constructor(private readonly context: vscode.ExtensionContext) {
    const editorWatcher = vscode.window.onDidChangeActiveTextEditor((editor) => {
      if (editor && this.isFileUri(editor.document.uri)) {
        this.addOrMoveToFront(editor);
        this.fireWithDebounce();
      }
    });

    const selectionWatcher = vscode.window.onDidChangeTextEditorSelection((event) => {
      if (this.isFileUri(event.textEditor.document.uri)) {
        this.updateActiveContext(event.textEditor);
        this.fireWithDebounce();
      }
    });

    const closeWatcher = vscode.workspace.onDidCloseTextDocument((document) => {
      if (this.isFileUri(document.uri)) {
        this.remove(document.uri);
        this.fireWithDebounce();
      }
    });

    const deleteWatcher = vscode.workspace.onDidDeleteFiles((event) => {
      for (const uri of event.files) {
        if (this.isFileUri(uri)) {
          this.remove(uri);
        }
      }
      this.fireWithDebounce();
    });

    const renameWatcher = vscode.workspace.onDidRenameFiles((event) => {
      for (const { oldUri, newUri } of event.files) {
        if (this.isFileUri(oldUri)) {
          if (this.isFileUri(newUri)) {
            this.rename(oldUri, newUri);
          } else {
            this.remove(oldUri);
          }
        }
      }
      this.fireWithDebounce();
    });

    context.subscriptions.push(
      editorWatcher,
      selectionWatcher,
      closeWatcher,
      deleteWatcher,
      renameWatcher
    );

    if (vscode.window.activeTextEditor && this.isFileUri(vscode.window.activeTextEditor.document.uri)) {
      this.addOrMoveToFront(vscode.window.activeTextEditor);
    }
  }

  private isFileUri(uri: vscode.Uri): boolean {
    return uri.scheme === 'file';
  }

  private addOrMoveToFront(editor: vscode.TextEditor) {
    const currentActive = this.openFiles.find((f) => f.isActive);
    if (currentActive) {
      currentActive.isActive = false;
      currentActive.cursor = undefined;
      currentActive.selectedText = undefined;
    }

    const index = this.openFiles.findIndex((f) => f.path === editor.document.uri.fsPath);
    if (index !== -1) {
      this.openFiles.splice(index, 1);
    }

    this.openFiles.unshift({
      path: editor.document.uri.fsPath,
      timestamp: Date.now(),
      isActive: true,
    });

    if (this.openFiles.length > MAX_FILES) {
      this.openFiles.pop();
    }

    this.updateActiveContext(editor);
  }

  private remove(uri: vscode.Uri) {
    const index = this.openFiles.findIndex((f) => f.path === uri.fsPath);
    if (index !== -1) {
      this.openFiles.splice(index, 1);
    }
  }

  private rename(oldUri: vscode.Uri, newUri: vscode.Uri) {
    const index = this.openFiles.findIndex((f) => f.path === oldUri.fsPath);
    if (index !== -1) {
      this.openFiles[index].path = newUri.fsPath;
    }
  }

  private updateActiveContext(editor: vscode.TextEditor) {
    const file = this.openFiles.find((f) => f.path === editor.document.uri.fsPath);
    if (!file || !file.isActive) {
      return;
    }

    file.cursor = editor.selection.active
      ? {
          line: editor.selection.active.line + 1,
          character: editor.selection.active.character + 1,
        }
      : undefined;

    let selectedText: string | undefined = editor.document.getText(editor.selection) || undefined;
    if (selectedText && selectedText.length > MAX_SELECTED_TEXT_LENGTH) {
      selectedText = selectedText.substring(0, MAX_SELECTED_TEXT_LENGTH);
    }
    file.selectedText = selectedText;
  }

  private fireWithDebounce() {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }
    this.debounceTimer = setTimeout(() => {
      this.onDidChangeEmitter.fire();
    }, 50);
  }

  get state(): IdeContext {
    return {
      workspaceState: {
        openFiles: [...this.openFiles],
        isTrusted: vscode.workspace.isTrusted,
      },
    };
  }
}
