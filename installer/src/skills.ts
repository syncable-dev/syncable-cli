import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

export interface SkillFrontmatter {
  name: string;
  description: string;
}

export interface Skill {
  frontmatter: SkillFrontmatter;
  body: string;
  category: 'command' | 'workflow';
  filename: string;
}

export function parseFrontmatter(content: string): { frontmatter: SkillFrontmatter; body: string } {
  const match = content.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
  if (!match) throw new Error('No frontmatter found');

  const frontmatterRaw = match[1];
  const body = match[2].trim();

  const nameMatch = frontmatterRaw.match(/^name:\s*(.+)$/m);
  const descMatch = frontmatterRaw.match(/^description:\s*(.+)$/m);

  if (!nameMatch || !descMatch) {
    throw new Error('Frontmatter must contain name and description');
  }

  return {
    frontmatter: {
      name: nameMatch[1].trim(),
      description: descMatch[1].trim(),
    },
    body,
  };
}

export function loadSkills(skillsDir: string): Skill[] {
  const skills: Skill[] = [];

  for (const category of ['commands', 'workflows'] as const) {
    const dir = path.join(skillsDir, category);
    if (!fs.existsSync(dir)) continue;

    const files = fs.readdirSync(dir).filter((f) => f.endsWith('.md'));
    for (const file of files) {
      const content = fs.readFileSync(path.join(dir, file), 'utf-8');
      const { frontmatter, body } = parseFrontmatter(content);
      const cat = category === 'commands' ? 'command' : 'workflow';
      skills.push({ frontmatter, body, category: cat, filename: file });
    }
  }

  return skills;
}

export function getBundledSkillsDir(): string {
  return fileURLToPath(new URL('../skills/', import.meta.url));
}
