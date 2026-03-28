import { describe, it, expect } from 'vitest';
import { transformForCodex } from '../../src/transformers/codex.js';

const sampleSkill = {
  frontmatter: { name: 'syncable-analyze', description: 'Use when analyzing a project' },
  body: '## Purpose\n\nAnalyze stuff.',
  category: 'command' as const,
  filename: 'syncable-analyze.md',
};

describe('transformForCodex', () => {
  it('creates a directory with SKILL.md', () => {
    const result = transformForCodex(sampleSkill);
    expect(result.length).toBe(1);
    expect(result[0].relativePath).toBe('syncable-analyze/SKILL.md');
  });

  it('preserves frontmatter in SKILL.md', () => {
    const result = transformForCodex(sampleSkill);
    expect(result[0].content).toContain('name: syncable-analyze');
    expect(result[0].content).toContain('description: Use when analyzing a project');
  });

  it('preserves body content', () => {
    const result = transformForCodex(sampleSkill);
    expect(result[0].content).toContain('## Purpose');
    expect(result[0].content).toContain('Analyze stuff.');
  });
});
