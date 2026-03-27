import { describe, it, expect } from 'vitest';
import { transformForClaude } from '../../src/transformers/claude.js';

const sampleSkill = {
  frontmatter: { name: 'syncable-analyze', description: 'Use when analyzing a project' },
  body: '## Purpose\n\nAnalyze stuff.',
  category: 'command' as const,
  filename: 'syncable-analyze.md',
};

describe('transformForClaude', () => {
  it('returns files preserving directory structure', () => {
    const result = transformForClaude(sampleSkill);
    expect(result.length).toBe(1);
    expect(result[0].relativePath).toBe('commands/syncable-analyze.md');
  });

  it('preserves content exactly (no-op transform)', () => {
    const result = transformForClaude(sampleSkill);
    expect(result[0].content).toContain('---');
    expect(result[0].content).toContain('name: syncable-analyze');
    expect(result[0].content).toContain('## Purpose');
    expect(result[0].content).toContain('Analyze stuff.');
  });

  it('uses workflows/ for workflow skills', () => {
    const workflow = { ...sampleSkill, category: 'workflow' as const };
    const result = transformForClaude(workflow);
    expect(result[0].relativePath).toBe('workflows/syncable-analyze.md');
  });
});
