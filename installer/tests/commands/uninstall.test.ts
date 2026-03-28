import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { removeSyncableSkills, removeGeminiSection } from '../../src/commands/uninstall.js';
import fs from 'fs';
import path from 'path';
import os from 'os';

const tmpDir = path.join(os.tmpdir(), 'syncable-uninstall-test-' + Date.now());

beforeEach(() => {
  fs.mkdirSync(tmpDir, { recursive: true });
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

describe('removeSyncableSkills', () => {
  it('removes a directory and its contents', () => {
    const skillDir = path.join(tmpDir, 'syncable');
    fs.mkdirSync(path.join(skillDir, 'commands'), { recursive: true });
    fs.writeFileSync(path.join(skillDir, 'commands', 'test.md'), 'test');
    removeSyncableSkills(skillDir);
    expect(fs.existsSync(skillDir)).toBe(false);
  });

  it('removes glob-matched files', () => {
    fs.writeFileSync(path.join(tmpDir, 'syncable-analyze.mdc'), 'test');
    fs.writeFileSync(path.join(tmpDir, 'syncable-security.mdc'), 'test');
    fs.writeFileSync(path.join(tmpDir, 'other-rule.mdc'), 'keep');
    removeSyncableSkills(tmpDir, 'syncable-*.mdc');
    expect(fs.existsSync(path.join(tmpDir, 'syncable-analyze.mdc'))).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, 'syncable-security.mdc'))).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, 'other-rule.mdc'))).toBe(true);
  });

  it('no-ops when directory does not exist', () => {
    expect(() => removeSyncableSkills(path.join(tmpDir, 'nonexistent'))).not.toThrow();
  });
});

describe('removeGeminiSection', () => {
  it('removes content between markers', () => {
    const filePath = path.join(tmpDir, 'GEMINI.md');
    fs.writeFileSync(filePath, '# Header\n<!-- SYNCABLE-CLI-SKILLS-START -->\nstuff\n<!-- SYNCABLE-CLI-SKILLS-END -->\n# Footer\n');
    removeGeminiSection(filePath);
    const content = fs.readFileSync(filePath, 'utf-8');
    expect(content).toContain('# Header');
    expect(content).toContain('# Footer');
    expect(content).not.toContain('stuff');
    expect(content).not.toContain('SYNCABLE-CLI-SKILLS');
  });

  it('no-ops when file does not exist', () => {
    expect(() => removeGeminiSection(path.join(tmpDir, 'nope.md'))).not.toThrow();
  });

  it('no-ops when no markers found', () => {
    const filePath = path.join(tmpDir, 'GEMINI.md');
    fs.writeFileSync(filePath, '# Just a normal file\n');
    removeGeminiSection(filePath);
    const content = fs.readFileSync(filePath, 'utf-8');
    expect(content).toBe('# Just a normal file\n');
  });
});
