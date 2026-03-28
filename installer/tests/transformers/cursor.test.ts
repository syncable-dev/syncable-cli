import { describe, it, expect } from 'vitest';
import { transformForCursor } from '../../src/transformers/cursor.js';

const sampleSkill = {
  frontmatter: { name: 'syncable-analyze', description: 'Use when analyzing a project' },
  body: '## Purpose\n\nAnalyze stuff.',
  category: 'command' as const,
  filename: 'syncable-analyze.md',
};

describe('transformForCursor', () => {
  it('creates a .mdc file', () => {
    const result = transformForCursor(sampleSkill);
    expect(result.length).toBe(1);
    expect(result[0].relativePath).toBe('syncable-analyze.mdc');
  });

  it('uses alwaysApply: true frontmatter', () => {
    const result = transformForCursor(sampleSkill);
    expect(result[0].content).toContain('alwaysApply: true');
  });

  it('prefixes description with "Syncable CLI: "', () => {
    const result = transformForCursor(sampleSkill);
    expect(result[0].content).toContain('description: "Syncable CLI: Use when analyzing a project"');
  });

  it('includes empty globs field', () => {
    const result = transformForCursor(sampleSkill);
    expect(result[0].content).toContain('globs:');
  });

  it('drops name from frontmatter', () => {
    const result = transformForCursor(sampleSkill);
    const frontmatterSection = result[0].content.split('---')[1];
    expect(frontmatterSection).not.toContain('name:');
  });

  it('preserves body content', () => {
    const result = transformForCursor(sampleSkill);
    expect(result[0].content).toContain('## Purpose');
    expect(result[0].content).toContain('Analyze stuff.');
  });
});
