import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeSkillsForClaude, writeSkillsForCodex, writeSkillsForCursor, writeSkillsForWindsurf, writeSkillsForGemini } from '../../src/commands/install.js';
import { transformForClaude } from '../../src/transformers/claude.js';
import { Skill } from '../../src/skills.js';
import fs from 'fs';
import path from 'path';
import os from 'os';

const tmpDir = path.join(os.tmpdir(), 'syncable-installer-test-' + Date.now());

const sampleSkills: Skill[] = [
  {
    frontmatter: { name: 'syncable-analyze', description: 'Analyze' },
    body: '## Purpose\n\nAnalyze.',
    category: 'command',
    filename: 'syncable-analyze.md',
  },
  {
    frontmatter: { name: 'syncable-project-assessment', description: 'Assess' },
    body: '## Purpose\n\nAssess.',
    category: 'workflow',
    filename: 'syncable-project-assessment.md',
  },
];

beforeEach(() => {
  fs.mkdirSync(tmpDir, { recursive: true });
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

describe('writeSkillsForClaude', () => {
  // Claude Code uses the plugin marketplace system.
  // writeSkillsForClaude installs to ~/.claude/plugins/cache/ (not to a custom dest dir).
  // We test the transform function directly instead to avoid writing to the real home dir.
  it('transform produces skills/<name>/SKILL.md structure', () => {
    const result = transformForClaude(sampleSkills[0]);
    expect(result[0].relativePath).toBe('skills/syncable-analyze/SKILL.md');
    expect(result[0].content).toContain('description:');
    expect(result[0].content).toContain('Analyze.');
  });

  it('transform uses YAML-safe description without name field', () => {
    const result = transformForClaude(sampleSkills[0]);
    expect(result[0].content).not.toContain('name:');
    expect(result[0].content).toMatch(/description: ".*"/);
  });
});

describe('writeSkillsForCodex', () => {
  it('writes each skill as a directory with SKILL.md', () => {
    writeSkillsForCodex(sampleSkills, tmpDir);
    expect(fs.existsSync(path.join(tmpDir, 'syncable-analyze', 'SKILL.md'))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, 'syncable-project-assessment', 'SKILL.md'))).toBe(true);
  });
});

describe('writeSkillsForCursor', () => {
  it('writes .mdc files', () => {
    writeSkillsForCursor(sampleSkills, tmpDir);
    expect(fs.existsSync(path.join(tmpDir, 'syncable-analyze.mdc'))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, 'syncable-project-assessment.mdc'))).toBe(true);
  });

  it('uses alwaysApply frontmatter', () => {
    writeSkillsForCursor(sampleSkills, tmpDir);
    const content = fs.readFileSync(path.join(tmpDir, 'syncable-analyze.mdc'), 'utf-8');
    expect(content).toContain('alwaysApply: true');
  });
});

describe('writeSkillsForWindsurf', () => {
  it('writes .md files with trigger: always', () => {
    writeSkillsForWindsurf(sampleSkills, tmpDir);
    const content = fs.readFileSync(path.join(tmpDir, 'syncable-analyze.md'), 'utf-8');
    expect(content).toContain('trigger: always');
  });
});

describe('writeSkillsForGemini', () => {
  it('writes content with markers to a file', () => {
    const filePath = path.join(tmpDir, 'GEMINI.md');
    writeSkillsForGemini(sampleSkills, filePath);
    const content = fs.readFileSync(filePath, 'utf-8');
    expect(content).toContain('<!-- SYNCABLE-CLI-SKILLS-START -->');
    expect(content).toContain('<!-- SYNCABLE-CLI-SKILLS-END -->');
    expect(content).toContain('### syncable-analyze');
  });

  it('appends to existing file without destroying content', () => {
    const filePath = path.join(tmpDir, 'GEMINI.md');
    fs.writeFileSync(filePath, '# My Project\n\nExisting content.\n');
    writeSkillsForGemini(sampleSkills, filePath);
    const content = fs.readFileSync(filePath, 'utf-8');
    expect(content).toContain('# My Project');
    expect(content).toContain('Existing content.');
    expect(content).toContain('<!-- SYNCABLE-CLI-SKILLS-START -->');
  });

  it('replaces existing Syncable section on re-install', () => {
    const filePath = path.join(tmpDir, 'GEMINI.md');
    fs.writeFileSync(filePath, '# Header\n<!-- SYNCABLE-CLI-SKILLS-START -->\nold content\n<!-- SYNCABLE-CLI-SKILLS-END -->\n# Footer\n');
    writeSkillsForGemini(sampleSkills, filePath);
    const content = fs.readFileSync(filePath, 'utf-8');
    expect(content).toContain('# Header');
    expect(content).toContain('# Footer');
    expect(content).not.toContain('old content');
    expect(content).toContain('### syncable-analyze');
  });
});
