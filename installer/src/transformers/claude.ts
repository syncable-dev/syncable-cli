import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

export function transformForClaude(skill: Skill): TransformResult[] {
  const dir = skill.category === 'command' ? 'commands' : 'workflows';
  const content = `---\nname: ${skill.frontmatter.name}\ndescription: ${skill.frontmatter.description}\n---\n\n${skill.body}`;
  return [{ relativePath: `${dir}/${skill.filename}`, content }];
}
