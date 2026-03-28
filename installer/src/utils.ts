import { exec as execCb } from 'child_process';
import { promisify } from 'util';
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
