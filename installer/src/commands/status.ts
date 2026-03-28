import fs from 'fs';
import path from 'path';
import { AgentName } from '../agents/types.js';
import { SKILL_MARKER_START } from '../constants.js';
import { getClaudePluginCacheDir } from '../transformers/claude.js';

export function countInstalledSkills(dirOrPath: string, agent: AgentName | string): number {
  if (!fs.existsSync(dirOrPath)) return 0;

  switch (agent) {
    case 'claude': {
      // Check plugin cache location
      const cacheDir = getClaudePluginCacheDir();
      const skillsDir = path.join(cacheDir, 'skills');
      if (fs.existsSync(skillsDir)) {
        return fs.readdirSync(skillsDir)
          .filter((f) => fs.statSync(path.join(skillsDir, f)).isDirectory())
          .length;
      }
      // Fallback: check old location
      let count = 0;
      for (const sub of ['commands', 'workflows']) {
        const dir = path.join(dirOrPath, sub);
        if (fs.existsSync(dir)) {
          count += fs.readdirSync(dir).filter((f) => f.endsWith('.md')).length;
        }
      }
      return count;
    }

    case 'codex': {
      return fs.readdirSync(dirOrPath)
        .filter((f) => f.startsWith('syncable-') && fs.statSync(path.join(dirOrPath, f)).isDirectory())
        .length;
    }

    case 'cursor': {
      return fs.readdirSync(dirOrPath)
        .filter((f) => f.startsWith('syncable-') && f.endsWith('.mdc'))
        .length;
    }

    case 'windsurf': {
      return fs.readdirSync(dirOrPath)
        .filter((f) => f.startsWith('syncable-') && f.endsWith('.md'))
        .length;
    }

    case 'gemini': {
      if (!fs.existsSync(dirOrPath)) return 0;
      const content = fs.readFileSync(dirOrPath, 'utf-8');
      if (content.includes(SKILL_MARKER_START)) {
        const start = content.indexOf(SKILL_MARKER_START);
        const end = content.indexOf('<!-- SYNCABLE-CLI-SKILLS-END -->');
        if (start !== -1 && end !== -1) {
          const section = content.slice(start, end);
          return (section.match(/^### /gm) || []).length;
        }
      }
      return 0;
    }

    default:
      return 0;
  }
}
