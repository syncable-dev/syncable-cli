import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

export function transformForCodex(skill: Skill): TransformResult[] {
  const content = `---\nname: ${skill.frontmatter.name}\ndescription: ${skill.frontmatter.description}\n---\n\n${skill.body}`;
  return [{ relativePath: `${skill.frontmatter.name}/SKILL.md`, content }];
}
