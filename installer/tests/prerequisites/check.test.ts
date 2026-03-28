import { describe, it, expect, vi } from 'vitest';
import { checkNodeVersion, PrereqStatus } from '../../src/prerequisites/check.js';

describe('checkNodeVersion', () => {
  it('returns ok for current Node version (>=18)', () => {
    const result = checkNodeVersion();
    expect(result.status).toBe('ok');
    expect(result.version).toBeDefined();
  });
});

describe('PrereqStatus', () => {
  it('has expected shape', () => {
    const status: PrereqStatus = {
      status: 'ok',
      version: '1.0.0',
    };
    expect(status.status).toBe('ok');
  });

  it('can represent missing', () => {
    const status: PrereqStatus = {
      status: 'missing',
    };
    expect(status.status).toBe('missing');
  });

  it('can represent outdated', () => {
    const status: PrereqStatus = {
      status: 'outdated',
      version: '0.30.0',
    };
    expect(status.status).toBe('outdated');
  });
});
