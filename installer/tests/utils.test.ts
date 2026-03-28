import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { parseVersion, compareVersions, isWindows } from '../src/utils.js';

describe('parseVersion', () => {
  it('parses semver string', () => {
    expect(parseVersion('1.2.3')).toEqual({ major: 1, minor: 2, patch: 3 });
  });

  it('extracts version from prefixed string', () => {
    expect(parseVersion('sync-ctl 0.36.0')).toEqual({ major: 0, minor: 36, patch: 0 });
  });

  it('extracts version from cargo output', () => {
    expect(parseVersion('cargo 1.79.0 (ffa9cf99a 2024-06-03)')).toEqual({ major: 1, minor: 79, patch: 0 });
  });

  it('returns null for unparseable string', () => {
    expect(parseVersion('no version here')).toBeNull();
  });
});

describe('compareVersions', () => {
  it('returns 0 for equal versions', () => {
    expect(compareVersions({ major: 1, minor: 2, patch: 3 }, { major: 1, minor: 2, patch: 3 })).toBe(0);
  });

  it('returns positive when a > b', () => {
    expect(compareVersions({ major: 2, minor: 0, patch: 0 }, { major: 1, minor: 9, patch: 9 })).toBeGreaterThan(0);
  });

  it('returns negative when a < b', () => {
    expect(compareVersions({ major: 0, minor: 35, patch: 0 }, { major: 0, minor: 36, patch: 0 })).toBeLessThan(0);
  });

  it('compares patch level', () => {
    expect(compareVersions({ major: 1, minor: 0, patch: 1 }, { major: 1, minor: 0, patch: 0 })).toBeGreaterThan(0);
  });
});

describe('isWindows', () => {
  it('returns a boolean', () => {
    expect(typeof isWindows()).toBe('boolean');
  });
});
