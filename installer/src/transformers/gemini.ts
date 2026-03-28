import { Skill } from '../skills.js';
import { SKILL_MARKER_START, SKILL_MARKER_END } from '../constants.js';

export function transformForGemini(skills: Skill[]): string {
  const sections = skills
    .map((s) => `### ${s.frontmatter.name}\n\n${s.body}`)
    .join('\n\n');

  return `${SKILL_MARKER_START}
## Syncable CLI Skills

The following skills describe how to use the Syncable CLI (sync-ctl) toolbox.

${sections}
${SKILL_MARKER_END}`;
}
