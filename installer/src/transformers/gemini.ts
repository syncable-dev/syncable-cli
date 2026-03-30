import { Skill } from '../skills.js';
import { TransformResult } from './types.js';
import { SKILL_MARKER_START, SKILL_MARKER_END } from '../constants.js';

/**
 * Transform a skill into Gemini CLI skill format.
 * Each skill becomes a directory with SKILL.md inside skills/<skill-name>/
 * Format: frontmatter with name + description, then markdown body.
 */
export function transformForGemini(skill: Skill): TransformResult[] {
  const skillName = skill.filename.replace(/\.md$/, '');

  // Gemini CLI loads name + description at startup, then activates the full
  // SKILL.md on demand when a task matches. Description should be concise
  // and clearly describe when to activate. Max ~125 chars recommended.
  const safeDesc = skill.frontmatter.description
    .replace(/"/g, '\\"');

  const content = `---\nname: "${skillName}"\ndescription: "${safeDesc}"\n---\n\n${skill.body}`;
  return [{ relativePath: `${skillName}/SKILL.md`, content }];
}

/**
 * Legacy: generate a flat GEMINI.md section for older Gemini CLI versions.
 * Used as a fallback when the skills directory approach isn't available.
 */
export function transformForGeminiLegacy(skills: Skill[]): string {
  const sections = skills
    .map((s) => `### ${s.frontmatter.name}\n\n${s.body}`)
    .join('\n\n');

  return `${SKILL_MARKER_START}
## Syncable CLI Skills

The following skills describe how to use the Syncable CLI (sync-ctl) toolbox.

${sections}
${SKILL_MARKER_END}`;
}
