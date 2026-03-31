import fs from 'fs';
import path from 'path';
import os from 'os';
import { Skill } from '../skills.js';
import { TransformResult } from './types.js';
import { execCommand, commandExists } from '../utils.js';

const PLUGIN_NAME = 'syncable-cli-skills';
const MARKETPLACE_NAME = 'syncable';
const MARKETPLACE_REPO = 'syncable-dev/syncable-cli';

/**
 * Transform a skill into Claude Code plugin format.
 * Each skill becomes a directory with SKILL.md inside skills/<skill-name>/
 */
export function transformForClaude(skill: Skill): TransformResult[] {
  const skillName = skill.filename.replace(/\.md$/, '');

  const safeDesc = skill.frontmatter.description
    .replace(/"/g, '\\"')
    .trim();

  const content = `---\ndescription: "${safeDesc}"\n---\n\n${skill.body}`;

  return [{ relativePath: `skills/${skillName}/SKILL.md`, content }];
}

/**
 * Root directory for all cached versions of this plugin.
 */
function getPluginCacheRoot(): string {
  return path.join(os.homedir(), '.claude', 'plugins', 'cache', MARKETPLACE_NAME, PLUGIN_NAME);
}

/**
 * Find the cache directory that Claude Code's CLI created (from `claude plugin install`).
 * This version is guaranteed to match the GitHub marketplace, so we piggyback on it
 * to avoid creating a version that Claude Code would later orphan.
 */
function findCliInstalledCacheDir(): string | null {
  const root = getPluginCacheRoot();
  if (!fs.existsSync(root)) return null;

  for (const entry of fs.readdirSync(root)) {
    const dir = path.join(root, entry);
    if (!fs.statSync(dir).isDirectory()) continue;
    // Return any version dir (there should be at most one after CLI install)
    return dir;
  }
  return null;
}

/**
 * Fallback cache directory when the CLI install didn't create one.
 * Uses "0.0.0" as a sentinel — the real version comes from the CLI.
 */
export function getClaudePluginCacheDir(): string {
  return path.join(getPluginCacheRoot(), '0.0.0');
}

// ────────────────────────────────────────────────────────────────────────────
// Installation strategy (in priority order):
//
//   1. `claude plugin marketplace add` + `claude plugin install`
//      The official documented flow. This registers the marketplace, clones the
//      plugin from the GitHub repo, caches it, AND auto-enables it in settings.
//      100 % guaranteed to work if the `claude` CLI is on PATH.
//
//   2. Manual write: cache files + enabledPlugins in settings.json
//      If the CLI is unavailable (user hasn't installed Claude Code yet, or
//      they're on a CI machine), we write the plugin files directly to the
//      cache directory AND register it in ~/.claude/settings.json so that
//      next time Claude Code starts, the plugin loads automatically.
// ────────────────────────────────────────────────────────────────────────────

/**
 * Try to install the plugin via the Claude Code CLI.
 * Returns true if it fully succeeded.
 */
async function tryClaudeCliInstall(): Promise<boolean> {
  const hasClaude = await commandExists('claude');
  if (!hasClaude) return false;

  try {
    // Step 1: Register the marketplace (idempotent — safe to re-add)
    await execCommand(`claude plugin marketplace add ${MARKETPLACE_REPO}`);
  } catch {
    // Marketplace may already exist — continue
  }

  try {
    // Step 2: Install the plugin (auto-enables in user scope)
    await execCommand(`claude plugin install ${PLUGIN_NAME}@${MARKETPLACE_NAME} --scope user`);
    return true;
  } catch {
    // install can fail if plugin already exists at same version — check settings
    try {
      const settingsPath = path.join(os.homedir(), '.claude', 'settings.json');
      if (fs.existsSync(settingsPath)) {
        const settings = JSON.parse(fs.readFileSync(settingsPath, 'utf-8'));
        const key = `${PLUGIN_NAME}@${MARKETPLACE_NAME}`;
        if (settings.enabledPlugins?.[key] === true) {
          // Already installed and enabled — that's fine
          return true;
        }
      }
    } catch {
      // Couldn't verify — fall through to manual path
    }
    return false;
  }
}

/**
 * Write the plugin.json manifest inside the cache directory.
 */
function writePluginManifest(cacheDir: string): void {
  const manifestDir = path.join(cacheDir, '.claude-plugin');
  fs.mkdirSync(manifestDir, { recursive: true });

  // Derive version from the cache directory name (e.g. ".../0.1.0" → "0.1.0")
  const version = path.basename(cacheDir);

  const manifest = {
    name: PLUGIN_NAME,
    description:
      'Syncable CLI skills for project analysis, security scanning, vulnerability detection, dependency auditing, IaC validation, Kubernetes optimization, and cloud deployment.',
    version,
    author: {
      name: 'Syncable',
      email: 'support@syncable.dev',
    },
    homepage: 'https://syncable.dev',
    repository: `https://github.com/${MARKETPLACE_REPO}`,
    license: 'MIT',
    keywords: ['syncable', 'devops', 'security', 'deployment', 'kubernetes', 'docker', 'iac'],
  };

  fs.writeFileSync(path.join(manifestDir, 'plugin.json'), JSON.stringify(manifest, null, 2));
}

/**
 * Enable the plugin in ~/.claude/settings.json.
 *
 * Per Claude Code docs, plugins are activated via the `enabledPlugins` key.
 * We also register the marketplace in `extraKnownMarketplaces` so that
 * Claude Code can discover future updates automatically.
 */
function enablePluginInSettings(): void {
  const settingsFile = path.join(os.homedir(), '.claude', 'settings.json');

  let settings: Record<string, unknown> = {};

  if (fs.existsSync(settingsFile)) {
    try {
      settings = JSON.parse(fs.readFileSync(settingsFile, 'utf-8'));
    } catch {
      try { fs.copyFileSync(settingsFile, settingsFile + '.bak'); } catch { /* */ }
      settings = {};
    }
  }

  // Enable the plugin
  if (!settings.enabledPlugins || typeof settings.enabledPlugins !== 'object') {
    settings.enabledPlugins = {};
  }
  const pluginKey = `${PLUGIN_NAME}@${MARKETPLACE_NAME}`;
  (settings.enabledPlugins as Record<string, boolean>)[pluginKey] = true;

  // Register the marketplace so Claude Code can auto-update
  if (!settings.extraKnownMarketplaces || typeof settings.extraKnownMarketplaces !== 'object') {
    settings.extraKnownMarketplaces = {};
  }
  const marketplaces = settings.extraKnownMarketplaces as Record<string, unknown>;
  // Always overwrite the marketplace entry to ensure it is canonical and free
  // of non-standard fields (e.g. a stale "path" override added by Claude Code
  // dev-mode that causes the plugin to be loaded from the local filesystem).
  marketplaces[MARKETPLACE_NAME] = {
    source: {
      source: 'github',
      repo: MARKETPLACE_REPO,
    },
  };

  fs.mkdirSync(path.dirname(settingsFile), { recursive: true });
  fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2));
}

/**
 * Full Claude Code plugin installation.
 *
 *  1. Try `claude plugin marketplace add` + `claude plugin install`
 *  2. Fall back to manual: write cache files + update settings.json
 */
export async function installClaudePlugin(skills: Skill[]): Promise<{ cacheDir: string; skillCount: number }> {
  // Step 1: Run the official CLI — this fetches marketplace.json from GitHub,
  // determines the correct version, caches the plugin, and registers it in
  // settings.json. The version it creates MATCHES what GitHub advertises,
  // so Claude Code will never orphan it.
  await tryClaudeCliInstall();

  // Step 2: Find the directory the CLI created. We write our skills INTO it
  // rather than creating a new version directory, because any version we
  // invent might not match the GitHub marketplace and get orphaned.
  let cacheDir = findCliInstalledCacheDir();

  if (cacheDir) {
    // Remove .orphaned_at if present (stale from a previous bad install)
    const orphanedFile = path.join(cacheDir, '.orphaned_at');
    if (fs.existsSync(orphanedFile)) fs.unlinkSync(orphanedFile);
  } else {
    // CLI install failed or claude isn't installed — use fallback path
    cacheDir = getClaudePluginCacheDir();
  }

  // Step 3: Clear and rewrite skills so the cache is always fresh.
  const skillsDir = path.join(cacheDir, 'skills');
  if (fs.existsSync(skillsDir)) {
    fs.rmSync(skillsDir, { recursive: true });
  }

  for (const skill of skills) {
    const results = transformForClaude(skill);
    for (const { relativePath, content } of results) {
      const fullPath = path.join(cacheDir, relativePath);
      fs.mkdirSync(path.dirname(fullPath), { recursive: true });
      fs.writeFileSync(fullPath, content);
    }
  }

  writePluginManifest(cacheDir);
  enablePluginInSettings();

  // Step 4: Also write skills to ~/.claude/skills/ for SDK-based integrations
  // (Zed ACP, etc.) that don't read the plugin cache.
  writeUserLevelSkills(skills);

  return { cacheDir, skillCount: skills.length };
}

/**
 * Write skills to ~/.claude/skills/ for SDK-based integrations.
 * transformForClaude returns paths like "skills/name/SKILL.md", but
 * user-level skills live at ~/.claude/skills/name/SKILL.md, so we
 * strip the leading "skills/" prefix.
 */
function writeUserLevelSkills(skills: Skill[]): void {
  const userSkillsDir = path.join(os.homedir(), '.claude', 'skills');

  for (const skill of skills) {
    const skillName = skill.filename.replace(/\.md$/, '');
    const safeDesc = skill.frontmatter.description.replace(/"/g, '\\"').trim();
    const content = `---\ndescription: "${safeDesc}"\n---\n\n${skill.body}`;

    const outDir = path.join(userSkillsDir, skillName);
    fs.mkdirSync(outDir, { recursive: true });
    fs.writeFileSync(path.join(outDir, 'SKILL.md'), content);
  }
}

/**
 * Remove the Claude Code plugin.
 */
export async function uninstallClaudePlugin(): Promise<void> {
  // Try CLI first
  const hasClaude = await commandExists('claude');
  if (hasClaude) {
    try {
      await execCommand(`claude plugin uninstall ${PLUGIN_NAME}@${MARKETPLACE_NAME} --scope user`);
      return;
    } catch { /* fall through */ }
  }

  // Manual cleanup
  const cacheDir = getClaudePluginCacheDir();
  if (fs.existsSync(cacheDir)) {
    fs.rmSync(cacheDir, { recursive: true });
  }

  // Remove from enabledPlugins in settings.json
  const settingsFile = path.join(os.homedir(), '.claude', 'settings.json');
  if (fs.existsSync(settingsFile)) {
    try {
      const settings = JSON.parse(fs.readFileSync(settingsFile, 'utf-8'));
      const pluginKey = `${PLUGIN_NAME}@${MARKETPLACE_NAME}`;
      if (settings.enabledPlugins && typeof settings.enabledPlugins === 'object') {
        delete settings.enabledPlugins[pluginKey];
        fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2));
      }
    } catch { /* */ }
  }

  // Clean up legacy files from previous installer versions
  const legacyFiles = [
    path.join(os.homedir(), '.claude', 'plugins', 'installed_plugins.json'),
    path.join(os.homedir(), '.claude', 'plugins', 'known_marketplaces.json'),
  ];
  for (const legacyFile of legacyFiles) {
    if (fs.existsSync(legacyFile)) {
      try {
        const data = JSON.parse(fs.readFileSync(legacyFile, 'utf-8'));
        const key = `${PLUGIN_NAME}@${MARKETPLACE_NAME}`;
        if (data.plugins) delete data.plugins[key];
        if (data[MARKETPLACE_NAME]) delete data[MARKETPLACE_NAME];
        fs.writeFileSync(legacyFile, JSON.stringify(data, null, 2));
      } catch { /* */ }
    }
  }

  // Clean up user-level skills (both old flat files and new directory format)
  const userSkillsDir = path.join(os.homedir(), '.claude', 'skills');
  if (fs.existsSync(userSkillsDir)) {
    for (const entry of fs.readdirSync(userSkillsDir)) {
      if (entry.startsWith('syncable-')) {
        const entryPath = path.join(userSkillsDir, entry);
        const stat = fs.statSync(entryPath);
        if (stat.isDirectory()) {
          fs.rmSync(entryPath, { recursive: true });
        } else if (entry.endsWith('.md')) {
          fs.unlinkSync(entryPath);
        }
      }
    }
  }
}
