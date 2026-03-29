import { exec as execCb } from 'child_process';
import { promisify } from 'util';
import fs from 'fs';
import os from 'os';
import path from 'path';

const execAsync = promisify(execCb);

export interface Version {
  major: number;
  minor: number;
  patch: number;
}

export function parseVersion(input: string): Version | null {
  const match = input.match(/(\d+)\.(\d+)\.(\d+)/);
  if (!match) return null;
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
  };
}

export function compareVersions(a: Version, b: Version): number {
  if (a.major !== b.major) return a.major - b.major;
  if (a.minor !== b.minor) return a.minor - b.minor;
  return a.patch - b.patch;
}

export function isWindows(): boolean {
  return process.platform === 'win32';
}

export function homedir(): string {
  return os.homedir();
}

export function cargoBinDir(): string {
  return path.join(os.homedir(), '.cargo', 'bin');
}

export async function execCommand(command: string): Promise<{ stdout: string; stderr: string }> {
  return execAsync(command, { timeout: 300_000 });
}

export async function commandExists(command: string): Promise<boolean> {
  try {
    const cmd = isWindows() ? `where ${command}` : `which ${command}`;
    await execAsync(cmd);
    return true;
  } catch {
    return false;
  }
}

export function prependCargoToPath(): void {
  process.env.PATH = `${cargoBinDir()}${path.delimiter}${process.env.PATH}`;
}

/**
 * Check if sync-ctl is accessible from a fresh login shell.
 *
 * This is critical because the installer's Node.js process may have
 * ~/.cargo/bin in PATH (via prependCargoToPath), but the user's actual
 * shell — and more importantly, the AI agent's shell — may not.
 *
 * A false here means agents will fail with "sync-ctl: command not found"
 * even though the binary is installed.
 */
export async function isSyncCtlInLoginShell(): Promise<boolean> {
  try {
    if (isWindows()) {
      // On Windows, check if sync-ctl is in the system PATH
      await execAsync('where sync-ctl');
      return true;
    }

    // Spawn a fresh login shell to check — this mimics what agents do
    const shell = process.env.SHELL || '/bin/bash';
    await execAsync(`${shell} -l -c "which sync-ctl"`, { timeout: 10_000 });
    return true;
  } catch {
    return false;
  }
}

/**
 * Get the user's shell profile file path for PATH modifications.
 */
export function getShellProfile(): string {
  const shell = process.env.SHELL || '/bin/bash';

  if (shell.endsWith('/zsh')) {
    const zshrc = path.join(os.homedir(), '.zshrc');
    if (fs.existsSync(zshrc)) return zshrc;
    return path.join(os.homedir(), '.zprofile');
  }

  if (shell.endsWith('/fish')) {
    return path.join(os.homedir(), '.config', 'fish', 'config.fish');
  }

  // Default to bash
  const bashrc = path.join(os.homedir(), '.bashrc');
  if (fs.existsSync(bashrc)) return bashrc;
  return path.join(os.homedir(), '.bash_profile');
}
