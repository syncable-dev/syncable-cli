import { describe, it, expect } from 'vitest';
import { fileURLToPath } from 'url';
import { parseFrontmatter, loadSkills } from '../src/skills.js';

describe('parseFrontmatter', () => {
  it('extracts name and description from frontmatter', () => {
    const content = `---
name: syncable-analyze
description: Use when analyzing a project
---

## Purpose

Analyze stuff.`;

    const result = parseFrontmatter(content);
    expect(result.frontmatter.name).toBe('syncable-analyze');
    expect(result.frontmatter.description).toBe('Use when analyzing a project');
    expect(result.body).toContain('## Purpose');
    expect(result.body).toContain('Analyze stuff.');
  });

  it('handles multi-line description', () => {
    const content = `---
name: test-skill
description: First line and more text here.
---

Body here.`;

    const result = parseFrontmatter(content);
    expect(result.frontmatter.name).toBe('test-skill');
    expect(result.frontmatter.description).toContain('First line');
    expect(result.body).toBe('Body here.');
  });

  it('throws on missing frontmatter', () => {
    expect(() => parseFrontmatter('no frontmatter here')).toThrow();
  });
});

describe('loadSkills', () => {
  it('loads skills from a directory with commands/ and workflows/', () => {
    const skills = loadSkills(fileURLToPath(new URL('../../skills/', import.meta.url)));
    expect(skills.length).toBe(11);

    const names = skills.map((s) => s.frontmatter.name);
    expect(names).toContain('syncable-analyze');
    expect(names).toContain('syncable-deploy-pipeline');
  });

  it('categorizes skills as command or workflow', () => {
    const skills = loadSkills(fileURLToPath(new URL('../../skills/', import.meta.url)));
    const commands = skills.filter((s) => s.category === 'command');
    const workflows = skills.filter((s) => s.category === 'workflow');
    expect(commands.length).toBe(7);
    expect(workflows.length).toBe(4);
  });
});
