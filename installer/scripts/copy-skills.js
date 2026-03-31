import pkg from 'fs-extra';
const { copySync, removeSync, existsSync, mkdirpSync, writeFileSync, readdirSync, readFileSync } = pkg;
import { resolve, dirname, basename } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const source = resolve(__dirname, '..', '..', 'skills');
const dest = resolve(__dirname, '..', 'skills');

// Read the canonical version from package.json
const packageJson = JSON.parse(readFileSync(resolve(__dirname, '..', 'package.json'), 'utf-8'));
const version = packageJson.version;

if (!existsSync(source)) {
  console.error('Error: skills/ directory not found at', source);
  process.exit(1);
}

// Copy raw skills to installer/skills/ (used by the npm package at runtime)
removeSync(dest);
copySync(source, dest);
console.log(`Copied skills from ${source} to ${dest}`);

// ── Sync version across all files that reference it ─────────────────

// 1. installer/plugins/syncable-cli-skills/.claude-plugin/plugin.json
const pluginJsonPath = resolve(__dirname, '..', 'plugins', 'syncable-cli-skills', '.claude-plugin', 'plugin.json');
if (existsSync(pluginJsonPath)) {
  const pluginJson = JSON.parse(readFileSync(pluginJsonPath, 'utf-8'));
  pluginJson.version = version;
  writeFileSync(pluginJsonPath, JSON.stringify(pluginJson, null, 2) + '\n');
  console.log(`Synced plugin.json version to ${version}`);
}

// 2. .claude-plugin/marketplace.json (repo root)
const marketplacePath = resolve(__dirname, '..', '..', '.claude-plugin', 'marketplace.json');
if (existsSync(marketplacePath)) {
  const marketplace = JSON.parse(readFileSync(marketplacePath, 'utf-8'));
  if (marketplace.metadata) marketplace.metadata.version = version;
  if (marketplace.plugins) {
    for (const plugin of marketplace.plugins) {
      if (plugin.name === 'syncable-cli-skills') {
        plugin.version = version;
      }
    }
  }
  writeFileSync(marketplacePath, JSON.stringify(marketplace, null, 2) + '\n');
  console.log(`Synced marketplace.json version to ${version}`);
}

// 3. PLUGIN_VERSION constant in src/transformers/claude.ts
const claudeTsPath = resolve(__dirname, '..', 'src', 'transformers', 'claude.ts');
if (existsSync(claudeTsPath)) {
  let claudeTs = readFileSync(claudeTsPath, 'utf-8');
  claudeTs = claudeTs.replace(
    /const PLUGIN_VERSION = '[^']+';/,
    `const PLUGIN_VERSION = '${version}';`
  );
  writeFileSync(claudeTsPath, claudeTs);
  console.log(`Synced PLUGIN_VERSION to ${version}`);
}

// ── Regenerate plugin skills ────────────────────────────────────────

const pluginSkillsDir = resolve(__dirname, '..', 'plugins', 'syncable-cli-skills', 'skills');
removeSync(pluginSkillsDir);

function transformSkillFile(filePath) {
  const raw = readFileSync(filePath, 'utf-8');
  const match = raw.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
  if (!match) return null;

  const frontmatterRaw = match[1];
  const body = match[2];

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
