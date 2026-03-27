import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { countInstalledSkills } from '../../src/commands/status.js';
import fs from 'fs';
import path from 'path';
import os from 'os';

const tmpDir = path.join(os.tmpdir(), 'syncable-status-test-' + Date.now());

beforeEach(() => {
  fs.mkdirSync(tmpDir, { recursive: true });
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

describe('countInstalledSkills', () => {
  it('counts .md files in commands/ and workflows/ (Claude format)', () => {
    fs.mkdirSync(path.join(tmpDir, 'commands'), { recursive: true });
    fs.mkdirSync(path.join(tmpDir, 'workflows'), { recursive: true });
    fs.writeFileSync(path.join(tmpDir, 'commands', 'a.md'), '');
    fs.writeFileSync(path.join(tmpDir, 'commands', 'b.md'), '');
    fs.writeFileSync(path.join(tmpDir, 'workflows', 'c.md'), '');
    expect(countInstalledSkills(tmpDir, 'claude')).toBe(3);
  });

  it('counts directories (Codex format)', () => {
    fs.mkdirSync(path.join(tmpDir, 'syncable-analyze'), { recursive: true });
    fs.writeFileSync(path.join(tmpDir, 'syncable-analyze', 'SKILL.md'), '');
    fs.mkdirSync(path.join(tmpDir, 'syncable-security'), { recursive: true });
    fs.writeFileSync(path.join(tmpDir, 'syncable-security', 'SKILL.md'), '');
    expect(countInstalledSkills(tmpDir, 'codex')).toBe(2);
  });

  it('counts .mdc files (Cursor format)', () => {
    fs.writeFileSync(path.join(tmpDir, 'syncable-analyze.mdc'), '');
    fs.writeFileSync(path.join(tmpDir, 'syncable-security.mdc'), '');
    fs.writeFileSync(path.join(tmpDir, 'other.mdc'), '');
    expect(countInstalledSkills(tmpDir, 'cursor')).toBe(2);
  });

  it('returns 0 when directory does not exist', () => {
    expect(countInstalledSkills('/nonexistent', 'claude')).toBe(0);
  });
});
