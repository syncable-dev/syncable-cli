import fs from 'fs';
import path from 'path';
import os from 'os';
import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

const PLUGIN_NAME = 'syncable-cli-skills';
const PLUGIN_VERSION = '0.1.0';
const MARKETPLACE_NAME = 'syncable';

/**
 * Transform a skill into Claude Code plugin format.
 * Each skill becomes a directory with SKILL.md inside skills/<skill-name>/
 */
export function transformForClaude(skill: Skill): TransformResult[] {
  // Skill name from filename (strip .md extension)
  const skillName = skill.filename.replace(/\.md$/, '');

  // Build YAML-safe description (double-quoted, no inner unescaped quotes)
  const safeDesc = skill.frontmatter.description
    .replace(/"/g, '\\"')
    .replace(/: /g, ' - ') // Remove colons that break YAML
    .replace(/Trigger on:.*$/, '') // Strip trigger phrases
    .trim();

  // Only description in frontmatter — directory name is the skill name
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

/**
 * Write the plugin.json manifest.
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
    license: 'MIT',
    keywords: ['syncable', 'devops', 'security', 'deployment', 'kubernetes', 'docker', 'iac'],
  };

  fs.writeFileSync(path.join(manifestDir, 'plugin.json'), JSON.stringify(manifest, null, 2));
}

/**
 * Register the plugin in installed_plugins.json.
 */
function registerPlugin(cacheDir: string): void {
  const pluginsFile = path.join(os.homedir(), '.claude', 'plugins', 'installed_plugins.json');

  let data: { version: number; plugins: Record<string, unknown[]> } = { version: 2, plugins: {} };

  if (fs.existsSync(pluginsFile)) {
    try {
      data = JSON.parse(fs.readFileSync(pluginsFile, 'utf-8'));
    } catch {
      // Corrupted file — start fresh
      data = { version: 2, plugins: {} };
    }
  }

  const key = `${PLUGIN_NAME}@${MARKETPLACE_NAME}`;
  const now = new Date().toISOString();

  data.plugins[key] = [
    {
      scope: 'user',
      installPath: cacheDir,
      version: PLUGIN_VERSION,
      installedAt: now,
      lastUpdated: now,
    },
  ];

  fs.mkdirSync(path.dirname(pluginsFile), { recursive: true });
  fs.writeFileSync(pluginsFile, JSON.stringify(data, null, 2));
}

/**
 * Register the marketplace in known_marketplaces.json so Claude Code knows about it.
 */
function registerMarketplace(): void {
  const marketFile = path.join(os.homedir(), '.claude', 'plugins', 'known_marketplaces.json');

  let data: Record<string, unknown> = {};

  if (fs.existsSync(marketFile)) {
    try {
      data = JSON.parse(fs.readFileSync(marketFile, 'utf-8'));
    } catch {
      data = {};
    }
  }

  // Only add if not already present
  if (!data[MARKETPLACE_NAME]) {
    data[MARKETPLACE_NAME] = {
      source: {
        source: 'github',
        repo: 'syncable-dev/syncable-cli',
      },
      installLocation: path.join(os.homedir(), '.claude', 'plugins', 'marketplaces', MARKETPLACE_NAME),
      lastUpdated: new Date().toISOString(),
    };

    fs.writeFileSync(marketFile, JSON.stringify(data, null, 2));
  }
}

/**
 * Full Claude Code plugin installation:
 * 1. Write SKILL.md files into plugin cache
 * 2. Write plugin.json manifest
 * 3. Register in installed_plugins.json
 * 4. Register marketplace in known_marketplaces.json
 */
export function installClaudePlugin(skills: Skill[]): { cacheDir: string; skillCount: number } {
  const cacheDir = getClaudePluginCacheDir();

  // Clear old skills
  const skillsDir = path.join(cacheDir, 'skills');
  if (fs.existsSync(skillsDir)) {
    fs.rmSync(skillsDir, { recursive: true });
  }

  // Write each skill as skills/<name>/SKILL.md
  for (const skill of skills) {
    const results = transformForClaude(skill);
    for (const { relativePath, content } of results) {
      const fullPath = path.join(cacheDir, relativePath);
      fs.mkdirSync(path.dirname(fullPath), { recursive: true });
      fs.writeFileSync(fullPath, content);
    }
  }

  // Write plugin manifest
  writePluginManifest(cacheDir);

  // Register plugin
  registerPlugin(cacheDir);

  // Register marketplace
  registerMarketplace();

  return { cacheDir, skillCount: skills.length };
}

/**
 * Remove the Claude Code plugin.
 */
export function uninstallClaudePlugin(): void {
  const cacheDir = getClaudePluginCacheDir();

  // Remove cache directory
  if (fs.existsSync(cacheDir)) {
    fs.rmSync(cacheDir, { recursive: true });
  }

  // Remove from installed_plugins.json
  const pluginsFile = path.join(os.homedir(), '.claude', 'plugins', 'installed_plugins.json');
  if (fs.existsSync(pluginsFile)) {
    try {
      const data = JSON.parse(fs.readFileSync(pluginsFile, 'utf-8'));
      const key = `${PLUGIN_NAME}@${MARKETPLACE_NAME}`;
      delete data.plugins[key];
      fs.writeFileSync(pluginsFile, JSON.stringify(data, null, 2));
    } catch {
      // Ignore errors
    }
  }

  // Also clean up old-style flat skills if they exist
  const oldSkillsDir = path.join(os.homedir(), '.claude', 'skills', 'syncable');
  if (fs.existsSync(oldSkillsDir)) {
    fs.rmSync(oldSkillsDir, { recursive: true });
  }

  // Clean up flat files from failed earlier installs
  const flatSkillsDir = path.join(os.homedir(), '.claude', 'skills');
  if (fs.existsSync(flatSkillsDir)) {
    for (const file of fs.readdirSync(flatSkillsDir)) {
      if (file.startsWith('syncable-') && file.endsWith('.md')) {
        fs.unlinkSync(path.join(flatSkillsDir, file));
      }
    }
  }
}
