import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

export function transformForCursor(skill: Skill): TransformResult[] {
  const filename = skill.frontmatter.name + '.mdc';
  const safeDesc = skill.frontmatter.description.replace(/"/g, '\\"');
  const content = `---\ndescription: "Syncable CLI: ${safeDesc}"\nglobs:\nalwaysApply: true\n---\n\n${skill.body}`;
  return [{ relativePath: filename, content }];
}
