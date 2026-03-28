import pkg from 'fs-extra';
const { copySync, removeSync, existsSync } = pkg;
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const source = resolve(__dirname, '..', '..', 'skills');
const dest = resolve(__dirname, '..', 'skills');

if (!existsSync(source)) {
  console.error('Error: skills/ directory not found at', source);
  process.exit(1);
}

removeSync(dest);
copySync(source, dest);

console.log(`Copied skills from ${source} to ${dest}`);
