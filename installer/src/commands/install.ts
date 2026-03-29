import fs from 'fs';
import path from 'path';
import { Skill, loadSkills, getBundledSkillsDir } from '../skills.js';
import { installClaudePlugin } from '../transformers/claude.js';
import { transformForCodex } from '../transformers/codex.js';
import { transformForCursor } from '../transformers/cursor.js';
import { transformForWindsurf } from '../transformers/windsurf.js';
import { transformForGemini } from '../transformers/gemini.js';
import { SKILL_MARKER_START, SKILL_MARKER_END } from '../constants.js';
import { TransformResult } from '../transformers/types.js';

export async function writeSkillsForClaude(skills: Skill[], _destDir: string): Promise<void> {
  // Claude Code uses the plugin system — destDir is ignored.
  // installClaudePlugin tries the CLI first, then falls back to
  // writing cache files + enabling in settings.json.
  await installClaudePlugin(skills);
}

export function writeSkillsForCodex(skills: Skill[], destDir: string): void {
  for (const skill of skills) {
    const results = transformForCodex(skill);
    for (const { relativePath, content } of results) {
      const fullPath = path.join(destDir, relativePath);
      fs.mkdirSync(path.dirname(fullPath), { recursive: true });
      fs.writeFileSync(fullPath, content);
    }
  }
}

export function writeSkillsForCursor(skills: Skill[], destDir: string): void {
  fs.mkdirSync(destDir, { recursive: true });
  for (const skill of skills) {
    const results = transformForCursor(skill);
    for (const { relativePath, content } of results) {
      fs.writeFileSync(path.join(destDir, relativePath), content);
    }
  }
}

export function writeSkillsForWindsurf(skills: Skill[], destDir: string): void {
  fs.mkdirSync(destDir, { recursive: true });
  for (const skill of skills) {
    const results = transformForWindsurf(skill);
    for (const { relativePath, content } of results) {
      fs.writeFileSync(path.join(destDir, relativePath), content);
    }
  }
}

export function writeSkillsForGemini(skills: Skill[], destDir: string): void {
  // Gemini CLI uses skills/<skill-name>/SKILL.md format
  // destDir is ~/.gemini/skills/ (the documented user-level discovery path)
  for (const skill of skills) {
    const results = transformForGemini(skill);
    for (const { relativePath, content } of results) {
      const fullPath = path.join(destDir, relativePath);
      fs.mkdirSync(path.dirname(fullPath), { recursive: true });
      fs.writeFileSync(fullPath, content);
    }
  }
}

export interface InstallOptions {
  skipCli: boolean;
  dryRun: boolean;
  agents?: string[];
  globalOnly: boolean;
  projectOnly: boolean;
  yes: boolean;
  verbose: boolean;
}

export type AgentWriter = (skills: Skill[], destOrPath: string) => void | Promise<void>;

export const agentWriters: Record<string, AgentWriter> = {
  claude: writeSkillsForClaude,
  codex: writeSkillsForCodex,
  cursor: writeSkillsForCursor,
  windsurf: writeSkillsForWindsurf,
  gemini: writeSkillsForGemini,
};
