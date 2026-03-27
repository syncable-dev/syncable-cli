import { execCommand, commandExists, parseVersion, compareVersions, cargoBinDir } from '../utils.js';
import { MIN_SYNC_CTL_VERSION } from '../constants.js';
import fs from 'fs';
import path from 'path';

export interface PrereqStatus {
  status: 'ok' | 'missing' | 'outdated';
  version?: string;
}

export function checkNodeVersion(): PrereqStatus {
  const version = process.version;
  const parsed = parseVersion(version);
  if (!parsed || parsed.major < 18) {
    return { status: 'outdated', version };
  }
  return { status: 'ok', version };
}

export async function checkCargo(): Promise<PrereqStatus> {
  try {
    const { stdout } = await execCommand('cargo --version');
    const version = parseVersion(stdout);
    return { status: 'ok', version: version ? `${version.major}.${version.minor}.${version.patch}` : stdout.trim() };
  } catch {
    const cargoPath = path.join(cargoBinDir(), 'cargo');
    if (fs.existsSync(cargoPath)) {
      return { status: 'ok', version: 'unknown' };
    }
    return { status: 'missing' };
  }
}

export async function checkSyncCtl(): Promise<PrereqStatus> {
  try {
    const { stdout } = await execCommand('sync-ctl --version');
    const version = parseVersion(stdout);
    if (!version) {
      return { status: 'ok', version: stdout.trim() };
    }

    const minVersion = parseVersion(MIN_SYNC_CTL_VERSION);
    if (minVersion && compareVersions(version, minVersion) < 0) {
      return { status: 'outdated', version: `${version.major}.${version.minor}.${version.patch}` };
    }

    return { status: 'ok', version: `${version.major}.${version.minor}.${version.patch}` };
  } catch {
    return { status: 'missing' };
  }
}
