import fs from 'fs';
import path from 'path';
import { Skill, loadSkills, getBundledSkillsDir } from '../skills.js';
import { installClaudePlugin } from '../transformers/claude.js';
import { transformForCodex } from '../transformers/codex.js';
import { transformForCursor } from '../transformers/cursor.js';
import { transformForWindsurf } from '../transformers/windsurf.js';
import { transformForGemini } from '../transformers/gemini.js';
import { SKILL_MARKER_START, SKILL_MARKER_END } from '../constants.js';

export function writeSkillsForClaude(skills: Skill[], _destDir: string): void {
  // Claude Code uses the plugin marketplace system — destDir is ignored.
  // Skills are installed as a plugin at ~/.claude/plugins/cache/syncable/...
  installClaudePlugin(skills);
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

export function writeSkillsForGemini(skills: Skill[], filePath: string): void {
  const geminiContent = transformForGemini(skills);
  let existing = '';

  if (fs.existsSync(filePath)) {
    existing = fs.readFileSync(filePath, 'utf-8');

    // Replace existing section if present
    const startIdx = existing.indexOf(SKILL_MARKER_START);
    const endIdx = existing.indexOf(SKILL_MARKER_END);
    if (startIdx !== -1 && endIdx !== -1) {
      const before = existing.slice(0, startIdx);
      const after = existing.slice(endIdx + SKILL_MARKER_END.length);
      fs.writeFileSync(filePath, before + geminiContent + after);
      return;
    }
  }

  // Append to existing or create new
  const separator = existing && !existing.endsWith('\n') ? '\n\n' : existing ? '\n' : '';
  fs.writeFileSync(filePath, existing + separator + geminiContent + '\n');
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

export type AgentWriter = (skills: Skill[], destOrPath: string) => void;

export const agentWriters: Record<string, AgentWriter> = {
  claude: writeSkillsForClaude,
  codex: writeSkillsForCodex,
  cursor: writeSkillsForCursor,
  windsurf: writeSkillsForWindsurf,
  gemini: writeSkillsForGemini,
};
