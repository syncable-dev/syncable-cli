import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

export function transformForCursor(skill: Skill): TransformResult[] {
  const filename = skill.frontmatter.name + '.mdc';
  const content = `---\ndescription: "Syncable CLI: ${skill.frontmatter.description}"\nglobs:\nalwaysApply: true\n---\n\n${skill.body}`;
  return [{ relativePath: filename, content }];
}
