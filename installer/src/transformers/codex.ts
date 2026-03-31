import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

export function transformForCodex(skill: Skill): TransformResult[] {
  const safeName = skill.frontmatter.name.replace(/"/g, '\\"');
  const safeDesc = skill.frontmatter.description.replace(/"/g, '\\"');
  const content = `---\nname: "${safeName}"\ndescription: "${safeDesc}"\n---\n\n${skill.body}`;
  return [{ relativePath: `${skill.frontmatter.name}/SKILL.md`, content }];
}
