import pkg from 'fs-extra';
const { copySync, removeSync, existsSync, mkdirpSync, writeFileSync, readdirSync, readFileSync } = pkg;
import { resolve, dirname, basename } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const source = resolve(__dirname, '..', '..', 'skills');
const dest = resolve(__dirname, '..', 'skills');

if (!existsSync(source)) {
  console.error('Error: skills/ directory not found at', source);
  process.exit(1);
}

// Copy raw skills to installer/skills/ (used by the npm package at runtime)
removeSync(dest);
copySync(source, dest);
console.log(`Copied skills from ${source} to ${dest}`);

// Also regenerate installer/plugins/syncable-cli-skills/skills/
// so the Claude Code marketplace plugin stays in sync with the source skills.
const pluginSkillsDir = resolve(__dirname, '..', 'plugins', 'syncable-cli-skills', 'skills');
removeSync(pluginSkillsDir);

function transformSkillFile(filePath) {
  const raw = readFileSync(filePath, 'utf-8');
  // Parse YAML frontmatter (---\n...\n---\n)
  const match = raw.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
  if (!match) return null;

  const frontmatterRaw = match[1];
  const body = match[2];

  // Extract description value (handles multi-line descriptions with quotes)
  const descMatch = frontmatterRaw.match(/^description:\s*(.+)$/m);
  if (!descMatch) return null;

  const desc = descMatch[1].trim().replace(/^["']|["']$/g, '');
  const safeDesc = desc.replace(/"/g, '\\"');

  return `---\ndescription: "${safeDesc}"\n---\n${body}`;
}

let skillCount = 0;
for (const category of ['commands', 'workflows']) {
  const categoryDir = resolve(source, category);
  if (!existsSync(categoryDir)) continue;

  for (const file of readdirSync(categoryDir)) {
    if (!file.endsWith('.md')) continue;
    const skillName = basename(file, '.md');
    const content = transformSkillFile(resolve(categoryDir, file));
    if (!content) {
      console.warn(`Warning: could not parse frontmatter for ${file}, skipping`);
      continue;
    }
    const outDir = resolve(pluginSkillsDir, skillName);
    mkdirpSync(outDir);
    writeFileSync(resolve(outDir, 'SKILL.md'), content);
    skillCount++;
  }
}

console.log(`Generated ${skillCount} plugin skills at ${pluginSkillsDir}`);
