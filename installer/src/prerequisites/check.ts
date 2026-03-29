import { execCommand, commandExists, parseVersion, compareVersions, cargoBinDir } from '../utils.js';
import { MIN_SYNC_CTL_VERSION } from '../constants.js';
import fs from 'fs';
import path from 'path';
import https from 'https';

export interface PrereqStatus {
  status: 'ok' | 'missing' | 'outdated';
  version?: string;
  latestVersion?: string;
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

/**
 * Fetch the latest syncable-cli version from crates.io.
 * Returns null if the lookup fails (network error, timeout, etc.)
 */
export async function getLatestCratesVersion(): Promise<string | null> {
  return new Promise((resolve) => {
    const req = https.get(
      'https://crates.io/api/v1/crates/syncable-cli',
      { headers: { 'User-Agent': 'syncable-cli-skills-installer' }, timeout: 5_000 },
      (res) => {
        let data = '';
        res.on('data', (chunk: Buffer) => { data += chunk; });
        res.on('end', () => {
          try {
            const json = JSON.parse(data);
            const version = json?.crate?.max_version || json?.versions?.[0]?.num;
            resolve(version || null);
          } catch {
            resolve(null);
          }
        });
      },
    );
    req.on('error', () => resolve(null));
    req.on('timeout', () => { req.destroy(); resolve(null); });
  });
}

export async function checkSyncCtl(): Promise<PrereqStatus> {
  try {
    const { stdout } = await execCommand('sync-ctl --version');
    const version = parseVersion(stdout);
    if (!version) {
      return { status: 'ok', version: stdout.trim() };
    }

    const currentStr = `${version.major}.${version.minor}.${version.patch}`;

    // First check: is it below the hard minimum?
    const minVersion = parseVersion(MIN_SYNC_CTL_VERSION);
    if (minVersion && compareVersions(version, minVersion) < 0) {
      return { status: 'outdated', version: currentStr };
    }

    // Second check: is there a newer version on crates.io?
    const latestStr = await getLatestCratesVersion();
    if (latestStr) {
      const latest = parseVersion(latestStr);
      if (latest && compareVersions(version, latest) < 0) {
        return { status: 'outdated', version: currentStr, latestVersion: latestStr };
      }
    }

    return { status: 'ok', version: currentStr };
  } catch {
    return { status: 'missing' };
  }
}
