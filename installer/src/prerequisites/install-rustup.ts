import { execCommand, isWindows, prependCargoToPath } from '../utils.js';

export async function installRustup(): Promise<boolean> {
  try {
    if (isWindows()) {
      // Step 1: Try winget
      try {
        await execCommand('winget install Rustlang.Rustup --accept-source-agreements --accept-package-agreements');
        prependCargoToPath();
        return true;
      } catch {
        // winget unavailable
      }

      // Step 2: Try downloading rustup-init.exe
      try {
        await execCommand('curl -sSf https://win.rustup.rs/x86_64 -o rustup-init.exe && .\\rustup-init.exe -y && del rustup-init.exe');
        prependCargoToPath();
        return true;
      } catch {
        // Download failed
      }

      // Step 3: Manual instructions
      console.error('Could not install Rust automatically. Install manually: https://rustup.rs');
      return false;
    } else {
      await execCommand('curl --proto \'=https\' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y');
      prependCargoToPath();
      return true;
    }
  } catch {
    return false;
  }
}
