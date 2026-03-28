import { describe, it, expect } from 'vitest';
import { transformForGemini } from '../../src/transformers/gemini.js';
import { Skill } from '../../src/skills.js';

const skills: Skill[] = [
  {
    frontmatter: { name: 'syncable-analyze', description: 'Analyze stuff' },
    body: '## Purpose\n\nAnalyze.',
    category: 'command',
    filename: 'syncable-analyze.md',
  },
  {
    frontmatter: { name: 'syncable-security', description: 'Security scan' },
    body: '## Purpose\n\nScan.',
    category: 'command',
    filename: 'syncable-security.md',
  },
];

describe('transformForGemini', () => {
  it('produces a single content block with markers', () => {
    const result = transformForGemini(skills);
    expect(result).toContain('<!-- SYNCABLE-CLI-SKILLS-START -->');
    expect(result).toContain('<!-- SYNCABLE-CLI-SKILLS-END -->');
  });

  it('includes all skills as sections', () => {
    const result = transformForGemini(skills);
    expect(result).toContain('### syncable-analyze');
    expect(result).toContain('### syncable-security');
  });

  it('includes skill body content', () => {
    const result = transformForGemini(skills);
    expect(result).toContain('Analyze.');
    expect(result).toContain('Scan.');
  });

  it('has header text', () => {
    const result = transformForGemini(skills);
    expect(result).toContain('## Syncable CLI Skills');
    expect(result).toContain('The following skills describe how to use the Syncable CLI');
  });
});
