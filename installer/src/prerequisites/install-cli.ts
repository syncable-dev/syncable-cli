import { execCommand } from '../utils.js';

export async function installSyncCtl(force: boolean = false): Promise<boolean> {
  try {
    const cmd = force
      ? 'cargo install syncable-cli --force'
      : 'cargo install syncable-cli';
    await execCommand(cmd);
    return true;
  } catch {
    return false;
  }
}
