import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

export function transformForWindsurf(skill: Skill): TransformResult[] {
  const filename = skill.frontmatter.name + '.md';
  const content = `---\ntrigger: always\ndescription: "Syncable CLI: ${skill.frontmatter.description}"\n---\n\n${skill.body}`;
  return [{ relativePath: filename, content }];
}
