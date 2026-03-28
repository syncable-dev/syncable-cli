import { describe, it, expect } from 'vitest';
import { transformForClaude } from '../../src/transformers/claude.js';

const sampleSkill = {
  frontmatter: { name: 'syncable-analyze', description: 'Use when analyzing a project' },
  body: '## Purpose\n\nAnalyze stuff.',
  category: 'command' as const,
  filename: 'syncable-analyze.md',
};

describe('transformForClaude', () => {
  it('creates skill directory with SKILL.md', () => {
    const result = transformForClaude(sampleSkill);
    expect(result.length).toBe(1);
    expect(result[0].relativePath).toBe('skills/syncable-analyze/SKILL.md');
  });

  it('uses description-only frontmatter (no name field)', () => {
    const result = transformForClaude(sampleSkill);
    expect(result[0].content).toContain('---');
    expect(result[0].content).toContain('description:');
    expect(result[0].content).not.toContain('name:');
    expect(result[0].content).toContain('## Purpose');
    expect(result[0].content).toContain('Analyze stuff.');
  });

  it('quotes the description for YAML safety', () => {
    const result = transformForClaude(sampleSkill);
    expect(result[0].content).toMatch(/description: ".*"/);
  });

  it('uses same path format for workflows', () => {
    const workflow = { ...sampleSkill, category: 'workflow' as const };
    const result = transformForClaude(workflow);
    // Plugin format doesn't distinguish commands vs workflows — all go under skills/
    expect(result[0].relativePath).toBe('skills/syncable-analyze/SKILL.md');
  });
});
