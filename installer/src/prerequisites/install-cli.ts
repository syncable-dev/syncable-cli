import { execCommand, prependCargoToPath } from '../utils.js';

/**
 * Install or update sync-ctl via cargo.
 *
 * Always uses --force when `force` is true so that an outdated version
 * is replaced with the latest from crates.io.
 *
 * After installation, cargo/bin is added to the current process PATH
 * so subsequent checks within the installer can find the binary.
 */
export async function installSyncCtl(force: boolean = false): Promise<boolean> {
  try {
    const cmd = force
      ? 'cargo install syncable-cli --force'
      : 'cargo install syncable-cli';
    await execCommand(cmd);

    // Ensure the binary is on PATH for the rest of this installer session
    prependCargoToPath();
    return true;
  } catch {
    return false;
  }
}
