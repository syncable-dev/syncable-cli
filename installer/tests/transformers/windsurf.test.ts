import { describe, it, expect } from 'vitest';
import { transformForWindsurf } from '../../src/transformers/windsurf.js';

const sampleSkill = {
  frontmatter: { name: 'syncable-analyze', description: 'Use when analyzing a project' },
  body: '## Purpose\n\nAnalyze stuff.',
  category: 'command' as const,
  filename: 'syncable-analyze.md',
};

describe('transformForWindsurf', () => {
  it('creates a .md file with syncable- prefix', () => {
    const result = transformForWindsurf(sampleSkill);
    expect(result.length).toBe(1);
    expect(result[0].relativePath).toBe('syncable-analyze.md');
  });

  it('uses trigger: always', () => {
    const result = transformForWindsurf(sampleSkill);
    expect(result[0].content).toContain('trigger: always');
  });

  it('prefixes description with "Syncable CLI: "', () => {
    const result = transformForWindsurf(sampleSkill);
    expect(result[0].content).toContain('description: "Syncable CLI: Use when analyzing a project"');
  });

  it('preserves body', () => {
    const result = transformForWindsurf(sampleSkill);
    expect(result[0].content).toContain('Analyze stuff.');
  });
});
