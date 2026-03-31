import fs from 'fs';
import path from 'path';
import os from 'os';
import { Skill } from '../skills.js';
import { TransformResult } from './types.js';
import { execCommand, commandExists } from '../utils.js';

const PLUGIN_NAME = 'syncable-cli-skills';
const PLUGIN_VERSION = '0.1.8';
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
 * Get the plugin cache directory for Claude Code.
 */
export function getClaudePluginCacheDir(): string {
  return path.join(
    os.homedir(),
    '.claude',
    'plugins',
    'cache',
    MARKETPLACE_NAME,
    PLUGIN_NAME,
    PLUGIN_VERSION
  );
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

  const manifest = {
    name: PLUGIN_NAME,
    description:
      'Syncable CLI skills for project analysis, security scanning, vulnerability detection, dependency auditing, IaC validation, Kubernetes optimization, and cloud deployment.',
    version: PLUGIN_VERSION,
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
  // Try the official CLI first — this handles enabledPlugins registration.
  // We don't return early on success because the CLI may have cached an old
  // version of the plugin that is missing the skills directory (e.g. from a
  // previous install before skills were added, or from a stale npx cache).
  // We always write skills directly to the cache so they're guaranteed to exist.
  await tryClaudeCliInstall();

  const cacheDir = getClaudePluginCacheDir();

  // Remove stale older-version cache entries so Claude Code doesn't load an
  // empty/outdated version instead of the current one.
  const pluginRootDir = path.dirname(cacheDir);
  if (fs.existsSync(pluginRootDir)) {
    for (const entry of fs.readdirSync(pluginRootDir)) {
      if (entry !== PLUGIN_VERSION) {
        fs.rmSync(path.join(pluginRootDir, entry), { recursive: true, force: true });
      }
    }
  }

  // Clear old skills and rewrite them so the cache is always up to date.
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

  return { cacheDir, skillCount: skills.length };
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

  // Clean up old flat-file skills
  const oldDirs = [
    path.join(os.homedir(), '.claude', 'skills', 'syncable'),
  ];
  for (const dir of oldDirs) {
    if (fs.existsSync(dir)) fs.rmSync(dir, { recursive: true });
  }

  const flatSkillsDir = path.join(os.homedir(), '.claude', 'skills');
  if (fs.existsSync(flatSkillsDir)) {
    for (const file of fs.readdirSync(flatSkillsDir)) {
      if (file.startsWith('syncable-') && file.endsWith('.md')) {
        fs.unlinkSync(path.join(flatSkillsDir, file));
      }
    }
  }
}
