import { describe, it, expect } from 'vitest';
import { transformForGemini } from '../../src/transformers/gemini.js';
import { Skill } from '../../src/skills.js';

const sampleSkill: Skill = {
  frontmatter: { name: 'syncable-analyze', description: 'Analyze stuff' },
  body: '## Purpose\n\nAnalyze.',
  category: 'command',
  filename: 'syncable-analyze.md',
};

describe('transformForGemini', () => {
  it('creates skill directory with SKILL.md', () => {
    const result = transformForGemini(sampleSkill);
    expect(result.length).toBe(1);
    expect(result[0].relativePath).toBe('syncable-analyze/SKILL.md');
  });

  it('includes frontmatter with name and description', () => {
    const result = transformForGemini(sampleSkill);
    expect(result[0].content).toContain('name: syncable-analyze');
    expect(result[0].content).toContain('description: Analyze stuff');
  });

  it('includes skill body content', () => {
    const result = transformForGemini(sampleSkill);
    expect(result[0].content).toContain('## Purpose');
    expect(result[0].content).toContain('Analyze.');
  });
});
