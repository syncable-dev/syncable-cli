# NPX Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an npx-installable TypeScript CLI (`syncable-cli-skills`) that installs the Syncable CLI, detects AI coding agents, and installs skills in each agent's native format.

**Architecture:** Commander-based CLI with four commands (install/uninstall/update/status). Modular design: agent detection, format transformers, prerequisite installers, and skill loading are isolated units. Pure-function transformers make testing straightforward. ESM throughout (chalk@5, ora@8, inquirer@9 are ESM-only).

**Tech Stack:** TypeScript (ESM), commander, inquirer, ora, chalk, fs-extra, vitest (testing)

**Spec:** `docs/superpowers/specs/2026-03-27-npx-installer-design.md`

---

## File Structure

```
installer/
├── package.json
├── tsconfig.json
├── vitest.config.ts
├── .gitignore
├── scripts/
│   └── copy-skills.js          # prebuild: copy ../skills/ into installer/skills/
├── src/
│   ├── index.ts                # CLI entrypoint (commander setup)
│   ├── constants.ts            # MIN_SYNC_CTL_VERSION, SKILL_MARKER_START/END
│   ├── utils.ts                # shell exec, version parsing, platform helpers
│   ├── skills.ts               # load bundled skill files, parse frontmatter
│   ├── agents/
│   │   ├── types.ts            # AgentConfig interface, AgentName enum
│   │   ├── detect.ts           # detect all installed agents
│   │   ├── claude.ts           # Claude Code agent config
│   │   ├── cursor.ts           # Cursor agent config
│   │   ├── windsurf.ts         # Windsurf agent config
│   │   ├── codex.ts            # Codex agent config
│   │   └── gemini.ts           # Gemini CLI agent config
│   ├── transformers/
│   │   ├── types.ts            # Skill type, TransformResult type
│   │   ├── claude.ts           # no-op (native format)
│   │   ├── codex.ts            # wrap in SKILL.md directory structure
│   │   ├── cursor.ts           # convert to .mdc format
│   │   ├── windsurf.ts         # convert to windsurf rule format
│   │   └── gemini.ts           # concatenate into GEMINI.md section
│   ├── prerequisites/
│   │   ├── check.ts            # check cargo, sync-ctl presence and versions
│   │   ├── install-rustup.ts   # install Rust toolchain via rustup
│   │   └── install-cli.ts      # cargo install syncable-cli
│   └── commands/
│       ├── install.ts          # install command (default)
│       ├── uninstall.ts        # remove skills from agents
│       ├── update.ts           # uninstall + install
│       └── status.ts           # show what's installed where
├── tests/
│   ├── utils.test.ts
│   ├── skills.test.ts
│   ├── agents/
│   │   └── detect.test.ts
│   ├── transformers/
│   │   ├── claude.test.ts
│   │   ├── codex.test.ts
│   │   ├── cursor.test.ts
│   │   ├── windsurf.test.ts
│   │   └── gemini.test.ts
│   ├── prerequisites/
│   │   └── check.test.ts
│   └── commands/
│       ├── install.test.ts
│       ├── uninstall.test.ts
│       └── status.test.ts
├── skills/                     # build artifact (gitignored), copied from ../skills/
└── dist/                       # compiled JS output (gitignored)
```

Each unit has a single responsibility. Transformers are pure functions. Agent configs are data objects with detection logic. Commands orchestrate the pieces.

---

### Task 1: Project scaffolding

**Files:**
- Create: `installer/package.json`
- Create: `installer/tsconfig.json`
- Create: `installer/vitest.config.ts`
- Create: `installer/.gitignore`
- Create: `installer/scripts/copy-skills.js`

- [ ] **Step 1: Create `installer/package.json`**

```json
{
  "name": "syncable-cli-skills",
  "version": "0.1.0",
  "type": "module",
  "description": "Install Syncable CLI skills for AI coding agents",
  "bin": {
    "syncable-cli-skills": "./dist/index.js"
  },
  "files": [
    "dist/",
    "skills/"
  ],
  "engines": {
    "node": ">=18.0.0"
  },
  "scripts": {
    "prebuild": "node scripts/copy-skills.js",
    "build": "tsc",
    "test": "vitest run",
    "test:watch": "vitest",
    "prepublishOnly": "npm run build"
  },
  "dependencies": {
    "commander": "^12.0.0",
    "inquirer": "^9.0.0",
    "ora": "^8.0.0",
    "chalk": "^5.0.0",
    "fs-extra": "^11.0.0"
  },
  "devDependencies": {
    "@types/fs-extra": "^11.0.0",
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0",
    "vitest": "^3.0.0"
  }
}
```

- [ ] **Step 2: Create `installer/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "declaration": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "tests", "skills"]
}
```

- [ ] **Step 3: Create `installer/vitest.config.ts`**

```typescript
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    root: '.',
    include: ['tests/**/*.test.ts'],
  },
});
```

- [ ] **Step 4: Create `installer/.gitignore`**

```
node_modules/
dist/
skills/
```

- [ ] **Step 5: Create `installer/scripts/copy-skills.js`**

This script copies `../skills/` into `installer/skills/` at build time using `fs-extra`. Cross-platform safe.

```javascript
import { copySync, removeSync, existsSync } from 'fs-extra';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const source = resolve(__dirname, '..', '..', 'skills');
const dest = resolve(__dirname, '..', 'skills');

if (!existsSync(source)) {
  console.error('Error: skills/ directory not found at', source);
  process.exit(1);
}

removeSync(dest);
copySync(source, dest);

console.log(`Copied skills from ${source} to ${dest}`);
```

- [ ] **Step 6: Install dependencies**

Run: `cd installer && npm install`

- [ ] **Step 7: Verify TypeScript compiles (empty project)**

Create a minimal `installer/src/index.ts`:

```typescript
#!/usr/bin/env node
console.log('syncable-cli-skills');
```

Run: `cd installer && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 8: Verify tests run (empty)**

Run: `cd installer && npm test`
Expected: vitest runs, 0 tests found.

- [ ] **Step 9: Verify copy-skills script works**

Run: `cd installer && node scripts/copy-skills.js`
Expected: `Copied skills from ...` message. `installer/skills/` directory now contains `commands/` and `workflows/` subdirs with 11 `.md` files.

- [ ] **Step 10: Commit**

```bash
git add installer/
git commit -m "chore(installer): scaffold npx installer project"
```

---

### Task 2: Constants and utilities module

**Files:**
- Create: `installer/src/constants.ts`
- Create: `installer/src/utils.ts`
- Test: `installer/tests/utils.test.ts`

- [ ] **Step 1: Write the failing tests for utils**

```typescript
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL — modules not found.

- [ ] **Step 3: Write `installer/src/constants.ts`**

```typescript
export const MIN_SYNC_CTL_VERSION = '0.35.0';
export const SKILL_MARKER_START = '<!-- SYNCABLE-CLI-SKILLS-START -->';
export const SKILL_MARKER_END = '<!-- SYNCABLE-CLI-SKILLS-END -->';
```

- [ ] **Step 4: Write `installer/src/utils.ts`**

```typescript
import { exec as execCb } from 'child_process';
import { promisify } from 'util';
import os from 'os';
import path from 'path';

const execAsync = promisify(execCb);

export interface Version {
  major: number;
  minor: number;
  patch: number;
}

export function parseVersion(input: string): Version | null {
  const match = input.match(/(\d+)\.(\d+)\.(\d+)/);
  if (!match) return null;
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
  };
}

export function compareVersions(a: Version, b: Version): number {
  if (a.major !== b.major) return a.major - b.major;
  if (a.minor !== b.minor) return a.minor - b.minor;
  return a.patch - b.patch;
}

export function isWindows(): boolean {
  return process.platform === 'win32';
}

export function homedir(): string {
  return os.homedir();
}

export function cargoBinDir(): string {
  return path.join(os.homedir(), '.cargo', 'bin');
}

export async function execCommand(command: string): Promise<{ stdout: string; stderr: string }> {
  return execAsync(command, { timeout: 300_000 });
}

export async function commandExists(command: string): Promise<boolean> {
  try {
    const cmd = isWindows() ? `where ${command}` : `which ${command}`;
    await execAsync(cmd);
    return true;
  } catch {
    return false;
  }
}

export function prependCargoToPath(): void {
  process.env.PATH = `${cargoBinDir()}${path.delimiter}${process.env.PATH}`;
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All 8 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add installer/src/constants.ts installer/src/utils.ts installer/tests/utils.test.ts
git commit -m "feat(installer): add constants and utils module with version parsing"
```

---

### Task 3: Skill loader module

**Files:**
- Create: `installer/src/skills.ts`
- Test: `installer/tests/skills.test.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
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
    // This test requires the copy-skills script to have run
    // The skills/ dir should exist after build
    // We'll test with the actual skills dir from the repo root
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL — module not found.

- [ ] **Step 3: Write `installer/src/skills.ts`**

```typescript
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

export interface SkillFrontmatter {
  name: string;
  description: string;
}

export interface Skill {
  frontmatter: SkillFrontmatter;
  body: string;
  category: 'command' | 'workflow';
  filename: string;
}

export function parseFrontmatter(content: string): { frontmatter: SkillFrontmatter; body: string } {
  const match = content.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
  if (!match) throw new Error('No frontmatter found');

  const frontmatterRaw = match[1];
  const body = match[2].trim();

  const nameMatch = frontmatterRaw.match(/^name:\s*(.+)$/m);
  const descMatch = frontmatterRaw.match(/^description:\s*(.+)$/m);

  if (!nameMatch || !descMatch) {
    throw new Error('Frontmatter must contain name and description');
  }

  return {
    frontmatter: {
      name: nameMatch[1].trim(),
      description: descMatch[1].trim(),
    },
    body,
  };
}

export function loadSkills(skillsDir: string): Skill[] {
  const skills: Skill[] = [];

  for (const category of ['commands', 'workflows'] as const) {
    const dir = path.join(skillsDir, category);
    if (!fs.existsSync(dir)) continue;

    const files = fs.readdirSync(dir).filter((f) => f.endsWith('.md'));
    for (const file of files) {
      const content = fs.readFileSync(path.join(dir, file), 'utf-8');
      const { frontmatter, body } = parseFrontmatter(content);
      const cat = category === 'commands' ? 'command' : 'workflow';
      skills.push({ frontmatter, body, category: cat, filename: file });
    }
  }

  return skills;
}

export function getBundledSkillsDir(): string {
  return fileURLToPath(new URL('../skills/', import.meta.url));
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All tests PASS (including the 5 new skill loader tests + 8 previous utils tests).

- [ ] **Step 5: Commit**

```bash
git add installer/src/skills.ts installer/tests/skills.test.ts
git commit -m "feat(installer): add skill loader with frontmatter parsing"
```

---

### Task 4: Agent types and detection

**Files:**
- Create: `installer/src/agents/types.ts`
- Create: `installer/src/agents/claude.ts`
- Create: `installer/src/agents/cursor.ts`
- Create: `installer/src/agents/windsurf.ts`
- Create: `installer/src/agents/codex.ts`
- Create: `installer/src/agents/gemini.ts`
- Create: `installer/src/agents/detect.ts`
- Test: `installer/tests/agents/detect.test.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AgentName } from '../src/agents/types.js';
import { allAgents } from '../src/agents/detect.js';

// We test the agent configs are well-formed and the detection logic
describe('agent configs', () => {
  it('each agent has required fields', async () => {
    const agents = allAgents();

    for (const agent of agents) {
      expect(agent.name).toBeDefined();
      expect(agent.displayName).toBeDefined();
      expect(agent.installType).toMatch(/^(global|project)$/);
      expect(typeof agent.detect).toBe('function');
      expect(typeof agent.getSkillPath).toBe('function');
    }
  });

  it('has 5 agents total', async () => {
    expect(allAgents().length).toBe(5);
  });

  it('claude and codex are global, others are project', async () => {
    const agents = allAgents();
    const globalAgents = agents.filter((a) => a.installType === 'global');
    const projectAgents = agents.filter((a) => a.installType === 'project');

    expect(globalAgents.map((a) => a.name)).toContain('claude');
    expect(globalAgents.map((a) => a.name)).toContain('codex');
    expect(projectAgents.map((a) => a.name)).toContain('cursor');
    expect(projectAgents.map((a) => a.name)).toContain('windsurf');
    expect(projectAgents.map((a) => a.name)).toContain('gemini');
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL — modules not found.

- [ ] **Step 3: Write `installer/src/agents/types.ts`**

```typescript
export type AgentName = 'claude' | 'cursor' | 'windsurf' | 'codex' | 'gemini';

export interface AgentConfig {
  name: AgentName;
  displayName: string;
  installType: 'global' | 'project';
  detect: () => Promise<boolean>;
  getSkillPath: () => string;
}
```

- [ ] **Step 4: Write the 5 agent config files**

`installer/src/agents/claude.ts`:

```typescript
import fs from 'fs';
import path from 'path';
import os from 'os';
import { AgentConfig } from './types.js';

export const claudeAgent: AgentConfig = {
  name: 'claude',
  displayName: 'Claude Code',
  installType: 'global',
  detect: async () => {
    return fs.existsSync(path.join(os.homedir(), '.claude'));
  },
  getSkillPath: () => {
    return path.join(os.homedir(), '.claude', 'skills', 'syncable');
  },
};
```

`installer/src/agents/codex.ts`:

```typescript
import fs from 'fs';
import path from 'path';
import os from 'os';
import { AgentConfig } from './types.js';
import { commandExists } from '../utils.js';

export const codexAgent: AgentConfig = {
  name: 'codex',
  displayName: 'Codex',
  installType: 'global',
  detect: async () => {
    return fs.existsSync(path.join(os.homedir(), '.codex')) || await commandExists('codex');
  },
  getSkillPath: () => {
    return path.join(os.homedir(), '.codex', 'skills');
  },
};
```

`installer/src/agents/cursor.ts`:

```typescript
import fs from 'fs';
import path from 'path';
import os from 'os';
import { AgentConfig } from './types.js';

export const cursorAgent: AgentConfig = {
  name: 'cursor',
  displayName: 'Cursor',
  installType: 'project',
  detect: async () => {
    return fs.existsSync(path.join(os.homedir(), '.cursor'));
  },
  getSkillPath: () => {
    return path.join(process.cwd(), '.cursor', 'rules');
  },
};
```

`installer/src/agents/windsurf.ts`:

```typescript
import fs from 'fs';
import path from 'path';
import os from 'os';
import { AgentConfig } from './types.js';

export const windsurfAgent: AgentConfig = {
  name: 'windsurf',
  displayName: 'Windsurf',
  installType: 'project',
  detect: async () => {
    return fs.existsSync(path.join(os.homedir(), '.codeium', 'windsurf'));
  },
  getSkillPath: () => {
    return path.join(process.cwd(), '.windsurf', 'rules');
  },
};
```

`installer/src/agents/gemini.ts`:

```typescript
import fs from 'fs';
import path from 'path';
import os from 'os';
import { AgentConfig } from './types.js';
import { commandExists } from '../utils.js';

export const geminiAgent: AgentConfig = {
  name: 'gemini',
  displayName: 'Gemini CLI',
  installType: 'project',
  detect: async () => {
    return fs.existsSync(path.join(os.homedir(), '.gemini')) || await commandExists('gemini');
  },
  getSkillPath: () => {
    return path.join(process.cwd(), 'GEMINI.md');
  },
};
```

- [ ] **Step 5: Write `installer/src/agents/detect.ts`**

```typescript
import { AgentConfig, AgentName } from './types.js';
import { claudeAgent } from './claude.js';
import { codexAgent } from './codex.js';
import { cursorAgent } from './cursor.js';
import { windsurfAgent } from './windsurf.js';
import { geminiAgent } from './gemini.js';

export function allAgents(): AgentConfig[] {
  return [claudeAgent, cursorAgent, windsurfAgent, codexAgent, geminiAgent];
}

export function getAgent(name: AgentName): AgentConfig | undefined {
  return allAgents().find((a) => a.name === name);
}

export interface DetectionResult {
  agent: AgentConfig;
  detected: boolean;
}

export async function detectAgents(): Promise<DetectionResult[]> {
  const agents = allAgents();
  const results: DetectionResult[] = [];

  for (const agent of agents) {
    const detected = await agent.detect();
    results.push({ agent, detected });
  }

  return results;
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All tests PASS.

- [ ] **Step 7: Commit**

```bash
git add installer/src/agents/ installer/tests/agents/
git commit -m "feat(installer): add agent detection for 5 AI coding agents"
```

---

### Task 5: Format transformers — types and Claude (no-op)

**Files:**
- Create: `installer/src/transformers/types.ts`
- Create: `installer/src/transformers/claude.ts`
- Test: `installer/tests/transformers/claude.test.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
import { describe, it, expect } from 'vitest';
import { transformForClaude } from '../src/transformers/claude.js';

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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL.

- [ ] **Step 3: Write `installer/src/transformers/types.ts`**

```typescript
import { Skill } from '../skills.js';

export interface TransformResult {
  relativePath: string;
  content: string;
}

export type TransformFn = (skill: Skill) => TransformResult[];
```

- [ ] **Step 4: Write `installer/src/transformers/claude.ts`**

```typescript
import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

export function transformForClaude(skill: Skill): TransformResult[] {
  const dir = skill.category === 'command' ? 'commands' : 'workflows';
  const content = `---\nname: ${skill.frontmatter.name}\ndescription: ${skill.frontmatter.description}\n---\n\n${skill.body}`;
  return [{ relativePath: `${dir}/${skill.filename}`, content }];
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add installer/src/transformers/types.ts installer/src/transformers/claude.ts installer/tests/transformers/claude.test.ts
git commit -m "feat(installer): add Claude Code transformer (no-op, native format)"
```

---

### Task 6: Format transformers — Codex

**Files:**
- Create: `installer/src/transformers/codex.ts`
- Test: `installer/tests/transformers/codex.test.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
import { describe, it, expect } from 'vitest';
import { transformForCodex } from '../src/transformers/codex.js';

const sampleSkill = {
  frontmatter: { name: 'syncable-analyze', description: 'Use when analyzing a project' },
  body: '## Purpose\n\nAnalyze stuff.',
  category: 'command' as const,
  filename: 'syncable-analyze.md',
};

describe('transformForCodex', () => {
  it('creates a directory with SKILL.md', () => {
    const result = transformForCodex(sampleSkill);
    expect(result.length).toBe(1);
    expect(result[0].relativePath).toBe('syncable-analyze/SKILL.md');
  });

  it('preserves frontmatter in SKILL.md', () => {
    const result = transformForCodex(sampleSkill);
    expect(result[0].content).toContain('name: syncable-analyze');
    expect(result[0].content).toContain('description: Use when analyzing a project');
  });

  it('preserves body content', () => {
    const result = transformForCodex(sampleSkill);
    expect(result[0].content).toContain('## Purpose');
    expect(result[0].content).toContain('Analyze stuff.');
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL.

- [ ] **Step 3: Write `installer/src/transformers/codex.ts`**

```typescript
import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

export function transformForCodex(skill: Skill): TransformResult[] {
  const content = `---\nname: ${skill.frontmatter.name}\ndescription: ${skill.frontmatter.description}\n---\n\n${skill.body}`;
  return [{ relativePath: `${skill.frontmatter.name}/SKILL.md`, content }];
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add installer/src/transformers/codex.ts installer/tests/transformers/codex.test.ts
git commit -m "feat(installer): add Codex transformer (SKILL.md directory format)"
```

---

### Task 7: Format transformers — Cursor

**Files:**
- Create: `installer/src/transformers/cursor.ts`
- Test: `installer/tests/transformers/cursor.test.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
import { describe, it, expect } from 'vitest';
import { transformForCursor } from '../src/transformers/cursor.js';

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
    // The .mdc frontmatter should NOT contain "name:"
    const frontmatterSection = result[0].content.split('---')[1];
    expect(frontmatterSection).not.toContain('name:');
  });

  it('preserves body content', () => {
    const result = transformForCursor(sampleSkill);
    expect(result[0].content).toContain('## Purpose');
    expect(result[0].content).toContain('Analyze stuff.');
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL.

- [ ] **Step 3: Write `installer/src/transformers/cursor.ts`**

```typescript
import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

export function transformForCursor(skill: Skill): TransformResult[] {
  const filename = skill.frontmatter.name + '.mdc';
  const content = `---\ndescription: "Syncable CLI: ${skill.frontmatter.description}"\nglobs:\nalwaysApply: true\n---\n\n${skill.body}`;
  return [{ relativePath: filename, content }];
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add installer/src/transformers/cursor.ts installer/tests/transformers/cursor.test.ts
git commit -m "feat(installer): add Cursor transformer (.mdc format with alwaysApply)"
```

---

### Task 8: Format transformers — Windsurf and Gemini

**Files:**
- Create: `installer/src/transformers/windsurf.ts`
- Create: `installer/src/transformers/gemini.ts`
- Test: `installer/tests/transformers/windsurf.test.ts`
- Test: `installer/tests/transformers/gemini.test.ts`

- [ ] **Step 1: Write the failing tests for Windsurf**

```typescript
import { describe, it, expect } from 'vitest';
import { transformForWindsurf } from '../src/transformers/windsurf.js';

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
```

- [ ] **Step 2: Write the failing tests for Gemini**

```typescript
import { describe, it, expect } from 'vitest';
import { transformForGemini } from '../src/transformers/gemini.js';
import { Skill } from '../src/skills.js';

const skills: Skill[] = [
  {
    frontmatter: { name: 'syncable-analyze', description: 'Analyze stuff' },
    body: '## Purpose\n\nAnalyze.',
    category: 'command',
    filename: 'syncable-analyze.md',
  },
  {
    frontmatter: { name: 'syncable-security', description: 'Security scan' },
    body: '## Purpose\n\nScan.',
    category: 'command',
    filename: 'syncable-security.md',
  },
];

describe('transformForGemini', () => {
  it('produces a single content block with markers', () => {
    const result = transformForGemini(skills);
    expect(result).toContain('<!-- SYNCABLE-CLI-SKILLS-START -->');
    expect(result).toContain('<!-- SYNCABLE-CLI-SKILLS-END -->');
  });

  it('includes all skills as sections', () => {
    const result = transformForGemini(skills);
    expect(result).toContain('### syncable-analyze');
    expect(result).toContain('### syncable-security');
  });

  it('includes skill body content', () => {
    const result = transformForGemini(skills);
    expect(result).toContain('Analyze.');
    expect(result).toContain('Scan.');
  });

  it('has header text', () => {
    const result = transformForGemini(skills);
    expect(result).toContain('## Syncable CLI Skills');
    expect(result).toContain('The following skills describe how to use the Syncable CLI');
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL.

- [ ] **Step 4: Write `installer/src/transformers/windsurf.ts`**

```typescript
import { Skill } from '../skills.js';
import { TransformResult } from './types.js';

export function transformForWindsurf(skill: Skill): TransformResult[] {
  const filename = skill.frontmatter.name + '.md';
  const content = `---\ntrigger: always\ndescription: "Syncable CLI: ${skill.frontmatter.description}"\n---\n\n${skill.body}`;
  return [{ relativePath: filename, content }];
}
```

- [ ] **Step 5: Write `installer/src/transformers/gemini.ts`**

```typescript
import { Skill } from '../skills.js';
import { SKILL_MARKER_START, SKILL_MARKER_END } from '../constants.js';

export function transformForGemini(skills: Skill[]): string {
  const sections = skills
    .map((s) => `### ${s.frontmatter.name}\n\n${s.body}`)
    .join('\n\n');

  return `${SKILL_MARKER_START}
## Syncable CLI Skills

The following skills describe how to use the Syncable CLI (sync-ctl) toolbox.

${sections}
${SKILL_MARKER_END}`;
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All tests PASS.

- [ ] **Step 7: Commit**

```bash
git add installer/src/transformers/windsurf.ts installer/src/transformers/gemini.ts installer/tests/transformers/
git commit -m "feat(installer): add Windsurf and Gemini format transformers"
```

---

### Task 9: Prerequisites checker

**Files:**
- Create: `installer/src/prerequisites/check.ts`
- Create: `installer/src/prerequisites/install-rustup.ts`
- Create: `installer/src/prerequisites/install-cli.ts`
- Test: `installer/tests/prerequisites/check.test.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
import { describe, it, expect, vi } from 'vitest';
import { checkNodeVersion, PrereqStatus } from '../src/prerequisites/check.js';

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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL.

- [ ] **Step 3: Write `installer/src/prerequisites/check.ts`**

```typescript
import { execCommand, commandExists, parseVersion, compareVersions, cargoBinDir } from '../utils.js';
import { MIN_SYNC_CTL_VERSION } from '../constants.js';
import fs from 'fs';
import path from 'path';

export interface PrereqStatus {
  status: 'ok' | 'missing' | 'outdated';
  version?: string;
}

export function checkNodeVersion(): PrereqStatus {
  const version = process.version;
  const parsed = parseVersion(version);
  if (!parsed || parsed.major < 18) {
    return { status: 'outdated', version };
  }
  return { status: 'ok', version };
}

export async function checkCargo(): Promise<PrereqStatus> {
  try {
    const { stdout } = await execCommand('cargo --version');
    const version = parseVersion(stdout);
    return { status: 'ok', version: version ? `${version.major}.${version.minor}.${version.patch}` : stdout.trim() };
  } catch {
    // Check if cargo exists at the known cargo bin path
    const cargoPath = path.join(cargoBinDir(), 'cargo');
    if (fs.existsSync(cargoPath)) {
      return { status: 'ok', version: 'unknown' };
    }
    return { status: 'missing' };
  }
}

export async function checkSyncCtl(): Promise<PrereqStatus> {
  try {
    const { stdout } = await execCommand('sync-ctl --version');
    const version = parseVersion(stdout);
    if (!version) {
      return { status: 'ok', version: stdout.trim() };
    }

    const minVersion = parseVersion(MIN_SYNC_CTL_VERSION);
    if (minVersion && compareVersions(version, minVersion) < 0) {
      return { status: 'outdated', version: `${version.major}.${version.minor}.${version.patch}` };
    }

    return { status: 'ok', version: `${version.major}.${version.minor}.${version.patch}` };
  } catch {
    return { status: 'missing' };
  }
}
```

- [ ] **Step 4: Write `installer/src/prerequisites/install-rustup.ts`**

```typescript
import { execCommand, isWindows, prependCargoToPath } from '../utils.js';

export async function installRustup(): Promise<boolean> {
  try {
    if (isWindows()) {
      // Step 1: Try winget
      try {
        await execCommand('winget install Rustlang.Rustup --accept-source-agreements --accept-package-agreements');
        prependCargoToPath();
        return true;
      } catch {
        // winget unavailable
      }

      // Step 2: Try downloading rustup-init.exe
      try {
        await execCommand('curl -sSf https://win.rustup.rs/x86_64 -o rustup-init.exe && .\\rustup-init.exe -y && del rustup-init.exe');
        prependCargoToPath();
        return true;
      } catch {
        // Download failed
      }

      // Step 3: Manual instructions
      console.error('Could not install Rust automatically. Install manually: https://rustup.rs');
      return false;
    } else {
      await execCommand('curl --proto \'=https\' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y');
      prependCargoToPath();
      return true;
    }
  } catch {
    return false;
  }
}
```

- [ ] **Step 5: Write `installer/src/prerequisites/install-cli.ts`**

```typescript
import { execCommand } from '../utils.js';

export async function installSyncCtl(force: boolean = false): Promise<boolean> {
  try {
    const cmd = force
      ? 'cargo install syncable-cli --force'
      : 'cargo install syncable-cli';
    await execCommand(cmd);
    return true;
  } catch {
    return false;
  }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All tests PASS.

- [ ] **Step 7: Commit**

```bash
git add installer/src/prerequisites/ installer/tests/prerequisites/
git commit -m "feat(installer): add prerequisite check and installation modules"
```

---

### Task 10: Install command

**Files:**
- Create: `installer/src/commands/install.ts`
- Test: `installer/tests/commands/install.test.ts`

This is the largest command. It orchestrates prerequisites, agent detection, skill installation.

- [ ] **Step 1: Write the failing tests**

Focus on the skill-writing logic (the testable core), not the interactive prompts.

```typescript
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeSkillsForClaude, writeSkillsForCodex, writeSkillsForCursor, writeSkillsForWindsurf, writeSkillsForGemini } from '../src/commands/install.js';
import { Skill } from '../src/skills.js';
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
  it('writes skills preserving commands/ and workflows/ structure', () => {
    writeSkillsForClaude(sampleSkills, tmpDir);
    expect(fs.existsSync(path.join(tmpDir, 'commands', 'syncable-analyze.md'))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, 'workflows', 'syncable-project-assessment.md'))).toBe(true);
  });

  it('preserves skill content', () => {
    writeSkillsForClaude(sampleSkills, tmpDir);
    const content = fs.readFileSync(path.join(tmpDir, 'commands', 'syncable-analyze.md'), 'utf-8');
    expect(content).toContain('name: syncable-analyze');
    expect(content).toContain('Analyze.');
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL.

- [ ] **Step 3: Write `installer/src/commands/install.ts`**

```typescript
import fs from 'fs';
import path from 'path';
import { Skill, loadSkills, getBundledSkillsDir } from '../skills.js';
import { transformForClaude } from '../transformers/claude.js';
import { transformForCodex } from '../transformers/codex.js';
import { transformForCursor } from '../transformers/cursor.js';
import { transformForWindsurf } from '../transformers/windsurf.js';
import { transformForGemini } from '../transformers/gemini.js';
import { SKILL_MARKER_START, SKILL_MARKER_END } from '../constants.js';

export function writeSkillsForClaude(skills: Skill[], destDir: string): void {
  for (const skill of skills) {
    const results = transformForClaude(skill);
    for (const { relativePath, content } of results) {
      const fullPath = path.join(destDir, relativePath);
      fs.mkdirSync(path.dirname(fullPath), { recursive: true });
      fs.writeFileSync(fullPath, content);
    }
  }
}

export function writeSkillsForCodex(skills: Skill[], destDir: string): void {
  for (const skill of skills) {
    const results = transformForCodex(skill);
    for (const { relativePath, content } of results) {
      const fullPath = path.join(destDir, relativePath);
      fs.mkdirSync(path.dirname(fullPath), { recursive: true });
      fs.writeFileSync(fullPath, content);
    }
  }
}

export function writeSkillsForCursor(skills: Skill[], destDir: string): void {
  fs.mkdirSync(destDir, { recursive: true });
  for (const skill of skills) {
    const results = transformForCursor(skill);
    for (const { relativePath, content } of results) {
      fs.writeFileSync(path.join(destDir, relativePath), content);
    }
  }
}

export function writeSkillsForWindsurf(skills: Skill[], destDir: string): void {
  fs.mkdirSync(destDir, { recursive: true });
  for (const skill of skills) {
    const results = transformForWindsurf(skill);
    for (const { relativePath, content } of results) {
      fs.writeFileSync(path.join(destDir, relativePath), content);
    }
  }
}

export function writeSkillsForGemini(skills: Skill[], filePath: string): void {
  const geminiContent = transformForGemini(skills);
  let existing = '';

  if (fs.existsSync(filePath)) {
    existing = fs.readFileSync(filePath, 'utf-8');

    // Replace existing section if present
    const startIdx = existing.indexOf(SKILL_MARKER_START);
    const endIdx = existing.indexOf(SKILL_MARKER_END);
    if (startIdx !== -1 && endIdx !== -1) {
      const before = existing.slice(0, startIdx);
      const after = existing.slice(endIdx + SKILL_MARKER_END.length);
      fs.writeFileSync(filePath, before + geminiContent + after);
      return;
    }
  }

  // Append to existing or create new
  const separator = existing && !existing.endsWith('\n') ? '\n\n' : existing ? '\n' : '';
  fs.writeFileSync(filePath, existing + separator + geminiContent + '\n');
}

export interface InstallOptions {
  skipCli: boolean;
  dryRun: boolean;
  agents?: string[];
  globalOnly: boolean;
  projectOnly: boolean;
  yes: boolean;
  verbose: boolean;
}

export type AgentWriter = (skills: Skill[], destOrPath: string) => void;

export const agentWriters: Record<string, AgentWriter> = {
  claude: writeSkillsForClaude,
  codex: writeSkillsForCodex,
  cursor: writeSkillsForCursor,
  windsurf: writeSkillsForWindsurf,
  gemini: writeSkillsForGemini,
};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add installer/src/commands/install.ts installer/tests/commands/install.test.ts
git commit -m "feat(installer): add install command with skill writers for all 5 agents"
```

---

### Task 11: Uninstall command

**Files:**
- Create: `installer/src/commands/uninstall.ts`
- Test: `installer/tests/commands/uninstall.test.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { removeSyncableSkills, removeGeminiSection } from '../src/commands/uninstall.js';
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL.

- [ ] **Step 3: Write `installer/src/commands/uninstall.ts`**

```typescript
import fs from 'fs';
import path from 'path';
import { SKILL_MARKER_START, SKILL_MARKER_END } from '../constants.js';

export function removeSyncableSkills(dirPath: string, globPattern?: string): void {
  if (!fs.existsSync(dirPath)) return;

  if (globPattern) {
    // Match entries by prefix pattern (e.g., "syncable-*.mdc" or "syncable-*")
    const prefix = globPattern.split('*')[0]; // "syncable-"
    const suffix = globPattern.split('*')[1]; // ".mdc" or ""
    const entries = fs.readdirSync(dirPath);
    for (const entry of entries) {
      if (entry.startsWith(prefix) && entry.endsWith(suffix)) {
        const fullPath = path.join(dirPath, entry);
        const stat = fs.statSync(fullPath);
        if (stat.isDirectory()) {
          fs.rmSync(fullPath, { recursive: true, force: true });
        } else {
          fs.unlinkSync(fullPath);
        }
      }
    }
  } else {
    // Remove the entire directory
    fs.rmSync(dirPath, { recursive: true, force: true });
  }
}

export function removeGeminiSection(filePath: string): void {
  if (!fs.existsSync(filePath)) return;

  const content = fs.readFileSync(filePath, 'utf-8');
  const startIdx = content.indexOf(SKILL_MARKER_START);
  const endIdx = content.indexOf(SKILL_MARKER_END);

  if (startIdx === -1 || endIdx === -1) return;

  const before = content.slice(0, startIdx);
  const after = content.slice(endIdx + SKILL_MARKER_END.length);

  // Clean up extra blank lines left behind
  const cleaned = (before + after).replace(/\n{3,}/g, '\n\n').trim() + '\n';
  fs.writeFileSync(filePath, cleaned);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add installer/src/commands/uninstall.ts installer/tests/commands/uninstall.test.ts
git commit -m "feat(installer): add uninstall command with glob removal and Gemini marker cleanup"
```

---

### Task 12: Status command

**Files:**
- Create: `installer/src/commands/status.ts`
- Test: `installer/tests/commands/status.test.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { countInstalledSkills } from '../src/commands/status.js';
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd installer && npm test`
Expected: FAIL.

- [ ] **Step 3: Write `installer/src/commands/status.ts`**

```typescript
import fs from 'fs';
import path from 'path';
import { AgentName } from '../agents/types.js';
import { SKILL_MARKER_START } from '../constants.js';

export function countInstalledSkills(dirOrPath: string, agent: AgentName | string): number {
  if (!fs.existsSync(dirOrPath)) return 0;

  switch (agent) {
    case 'claude': {
      let count = 0;
      for (const sub of ['commands', 'workflows']) {
        const dir = path.join(dirOrPath, sub);
        if (fs.existsSync(dir)) {
          count += fs.readdirSync(dir).filter((f) => f.endsWith('.md')).length;
        }
      }
      return count;
    }

    case 'codex': {
      return fs.readdirSync(dirOrPath)
        .filter((f) => f.startsWith('syncable-') && fs.statSync(path.join(dirOrPath, f)).isDirectory())
        .length;
    }

    case 'cursor': {
      return fs.readdirSync(dirOrPath)
        .filter((f) => f.startsWith('syncable-') && f.endsWith('.mdc'))
        .length;
    }

    case 'windsurf': {
      return fs.readdirSync(dirOrPath)
        .filter((f) => f.startsWith('syncable-') && f.endsWith('.md'))
        .length;
    }

    case 'gemini': {
      // For Gemini, the path is the GEMINI.md file itself
      if (!fs.existsSync(dirOrPath)) return 0;
      const content = fs.readFileSync(dirOrPath, 'utf-8');
      if (content.includes(SKILL_MARKER_START)) {
        // Count ### headings between markers
        const start = content.indexOf(SKILL_MARKER_START);
        const end = content.indexOf('<!-- SYNCABLE-CLI-SKILLS-END -->');
        if (start !== -1 && end !== -1) {
          const section = content.slice(start, end);
          return (section.match(/^### /gm) || []).length;
        }
      }
      return 0;
    }

    default:
      return 0;
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd installer && npm test`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add installer/src/commands/status.ts installer/tests/commands/status.test.ts
git commit -m "feat(installer): add status command with per-agent skill counting"
```

---

### Task 13: Update command

**Files:**
- Create: `installer/src/commands/update.ts`

- [ ] **Step 1: Write `installer/src/commands/update.ts`**

The update command is a thin orchestrator: uninstall then install. No separate tests needed — it composes already-tested functions.

```typescript
// Update is: uninstall + install for the same agents
// This is a thin wrapper used by the CLI entrypoint.
// The actual logic is in install.ts and uninstall.ts.

export { removeSyncableSkills, removeGeminiSection } from './uninstall.js';
export { writeSkillsForClaude, writeSkillsForCodex, writeSkillsForCursor, writeSkillsForWindsurf, writeSkillsForGemini } from './install.js';
```

- [ ] **Step 2: Commit**

```bash
git add installer/src/commands/update.ts
git commit -m "feat(installer): add update command (re-exports uninstall + install)"
```

---

### Task 14: CLI entrypoint

**Files:**
- Modify: `installer/src/index.ts`

This is the interactive orchestration layer. It uses commander for CLI parsing, inquirer for prompts, ora for spinners, and chalk for colors. This wires together all the modules.

- [ ] **Step 1: Write the full CLI entrypoint**

```typescript
#!/usr/bin/env node

import { Command } from 'commander';
import inquirer from 'inquirer';
import ora from 'ora';
import chalk from 'chalk';
import { createRequire } from 'module';
import { checkNodeVersion, checkCargo, checkSyncCtl } from './prerequisites/check.js';
import { installRustup } from './prerequisites/install-rustup.js';
import { installSyncCtl } from './prerequisites/install-cli.js';
import { detectAgents, allAgents } from './agents/detect.js';
import { AgentConfig, AgentName } from './agents/types.js';
import { loadSkills, getBundledSkillsDir } from './skills.js';
import {
  writeSkillsForClaude,
  writeSkillsForCodex,
  writeSkillsForCursor,
  writeSkillsForWindsurf,
  writeSkillsForGemini,
  InstallOptions,
} from './commands/install.js';
import { removeSyncableSkills, removeGeminiSection } from './commands/uninstall.js';
import { countInstalledSkills } from './commands/status.js';

const require = createRequire(import.meta.url);
const pkg = require('../package.json');

const program = new Command();

program
  .name('syncable-cli-skills')
  .description('Install Syncable CLI skills for AI coding agents')
  .version(pkg.version);

program
  .command('install', { isDefault: true })
  .description('Install sync-ctl and skills')
  .option('--skip-cli', 'Skip sync-ctl installation check')
  .option('--dry-run', 'Show what would be done without doing it')
  .option('--agents <list>', 'Comma-separated agent list')
  .option('--global-only', 'Only install global skills')
  .option('--project-only', 'Only install project-level rules')
  .option('-y, --yes', 'Skip confirmations')
  .option('--verbose', 'Show detailed output')
  .action(async (opts) => {
    console.log(chalk.bold('\n  Syncable CLI Skills Installer'));
    console.log('  ' + '─'.repeat(29) + '\n');

    // Check Node.js version
    const nodeCheck = checkNodeVersion();
    if (nodeCheck.status === 'outdated') {
      console.error(chalk.red(`  Node.js >= 18.0.0 required. Found: ${nodeCheck.version}`));
      process.exit(1);
    }
    console.log(chalk.green(`  ✓ Node.js ${nodeCheck.version}`));

    // Check prerequisites
    if (!opts.skipCli) {
      const cargoStatus = await checkCargo();
      const syncCtlStatus = await checkSyncCtl();

      if (cargoStatus.status === 'ok') {
        console.log(chalk.green(`  ✓ cargo ${cargoStatus.version}`));
      } else {
        console.log(chalk.red('  ✗ cargo not found'));
      }

      if (syncCtlStatus.status === 'ok') {
        console.log(chalk.green(`  ✓ sync-ctl v${syncCtlStatus.version}`));
      } else if (syncCtlStatus.status === 'outdated') {
        console.log(chalk.yellow(`  ⚠ sync-ctl v${syncCtlStatus.version} (outdated)`));
      } else {
        console.log(chalk.red('  ✗ sync-ctl not found'));
      }

      // Install missing prerequisites
      if (cargoStatus.status === 'missing') {
        console.log(chalk.yellow('\n  sync-ctl requires Rust\'s cargo package manager.\n'));
        const { installRust } = opts.yes
          ? { installRust: true }
          : await inquirer.prompt([{ type: 'confirm', name: 'installRust', message: 'Install Rust toolchain via rustup?', default: true }]);

        if (installRust) {
          const spinner = ora('  Installing rustup...').start();
          const success = await installRustup();
          if (success) {
            spinner.succeed('  Rust toolchain installed');
          } else {
            spinner.fail('  Failed to install Rust. Install manually: https://rustup.rs');
          }
        }
      }

      if (syncCtlStatus.status === 'missing' || syncCtlStatus.status === 'outdated') {
        const cargoNow = await checkCargo();
        if (cargoNow.status === 'ok') {
          const message = syncCtlStatus.status === 'outdated'
            ? 'Update syncable-cli via cargo?'
            : 'Install syncable-cli via cargo?';
          const { installCli } = opts.yes
            ? { installCli: true }
            : await inquirer.prompt([{ type: 'confirm', name: 'installCli', message, default: true }]);

          if (installCli) {
            const spinner = ora('  Running: cargo install syncable-cli').start();
            const force = syncCtlStatus.status === 'outdated';
            const success = await installSyncCtl(force);
            if (success) {
              spinner.succeed('  sync-ctl installed');
            } else {
              spinner.fail('  Failed to install sync-ctl. Try: cargo install syncable-cli');
            }
          }
        }
      }
    }

    // Detect agents
    console.log(chalk.bold('\n  Detecting AI coding agents...\n'));
    const detectionResults = await detectAgents();

    for (const { agent, detected } of detectionResults) {
      if (detected) {
        console.log(chalk.green(`  ✓ ${agent.displayName} detected`));
      } else {
        console.log(chalk.dim(`  ✗ ${agent.displayName} not detected`));
      }
    }

    // Determine which agents to install for
    let selectedAgents: AgentConfig[];

    if (opts.agents) {
      const names = opts.agents.split(',').map((n: string) => n.trim()) as AgentName[];
      selectedAgents = allAgents().filter((a) => names.includes(a.name));
    } else if (opts.globalOnly) {
      selectedAgents = detectionResults.filter((r) => r.detected && r.agent.installType === 'global').map((r) => r.agent);
    } else if (opts.projectOnly) {
      selectedAgents = detectionResults.filter((r) => r.detected && r.agent.installType === 'project').map((r) => r.agent);
    } else if (opts.yes) {
      selectedAgents = detectionResults.filter((r) => r.detected).map((r) => r.agent);
    } else {
      const choices = detectionResults.map((r) => ({
        name: `${r.agent.displayName} — ${r.agent.installType} install`,
        value: r.agent.name,
        checked: r.detected,
      }));

      const { agents } = await inquirer.prompt([{
        type: 'checkbox',
        name: 'agents',
        message: 'Which agents should receive Syncable skills?',
        choices,
      }]);

      selectedAgents = allAgents().filter((a) => agents.includes(a.name));
    }

    if (selectedAgents.length === 0) {
      console.log(chalk.yellow('\n  No agents selected. Nothing to install.'));
      return;
    }

    // Load and install skills
    const skills = loadSkills(getBundledSkillsDir());
    const commandCount = skills.filter((s) => s.category === 'command').length;
    const workflowCount = skills.filter((s) => s.category === 'workflow').length;

    for (const agent of selectedAgents) {
      const spinner = ora(`  Installing skills for ${agent.displayName}...`).start();

      if (opts.dryRun) {
        spinner.info(`  Would install ${skills.length} skills for ${agent.displayName}`);
        continue;
      }

      try {
        const dest = agent.getSkillPath();
        switch (agent.name) {
          case 'claude':
            writeSkillsForClaude(skills, dest);
            break;
          case 'codex':
            writeSkillsForCodex(skills, dest);
            break;
          case 'cursor':
            writeSkillsForCursor(skills, dest);
            break;
          case 'windsurf':
            writeSkillsForWindsurf(skills, dest);
            break;
          case 'gemini':
            writeSkillsForGemini(skills, dest);
            break;
        }
        spinner.succeed(`  ${skills.length} skills installed for ${agent.displayName}`);
      } catch (err) {
        spinner.fail(`  Failed to install skills for ${agent.displayName}: ${err}`);
      }
    }

    // Summary
    console.log('\n  ' + '─'.repeat(29));
    console.log(chalk.green.bold('  ✓ Setup complete!\n'));
    console.log(`  Installed:`);
    console.log(`    • ${commandCount} command skills + ${workflowCount} workflow skills`);
    console.log(`    • Agents: ${selectedAgents.map((a) => a.displayName).join(', ')}`);
    console.log(`\n  Try it: Open Claude Code and say "assess this project"\n`);
  });

program
  .command('uninstall')
  .description('Remove skills from agents')
  .option('--agents <list>', 'Comma-separated agent list')
  .option('-y, --yes', 'Skip confirmations')
  .action(async (opts) => {
    const agents = opts.agents
      ? allAgents().filter((a) => opts.agents.split(',').includes(a.name))
      : allAgents();

    if (!opts.yes) {
      const { confirm } = await inquirer.prompt([{
        type: 'confirm',
        name: 'confirm',
        message: `Remove Syncable skills from ${agents.map((a) => a.displayName).join(', ')}?`,
        default: false,
      }]);
      if (!confirm) return;
    }

    for (const agent of agents) {
      const spinner = ora(`  Removing skills from ${agent.displayName}...`).start();
      try {
        const dest = agent.getSkillPath();
        switch (agent.name) {
          case 'claude':
            removeSyncableSkills(dest);
            break;
          case 'codex':
            removeSyncableSkills(dest, 'syncable-*');
            break;
          case 'cursor':
            removeSyncableSkills(dest, 'syncable-*.mdc');
            break;
          case 'windsurf':
            removeSyncableSkills(dest, 'syncable-*.md');
            break;
          case 'gemini':
            removeGeminiSection(dest);
            break;
        }
        spinner.succeed(`  Skills removed from ${agent.displayName}`);
      } catch (err) {
        spinner.fail(`  Failed to remove skills from ${agent.displayName}: ${err}`);
      }
    }
  });

program
  .command('update')
  .description('Update skills to latest version')
  .option('--agents <list>', 'Comma-separated agent list')
  .option('-y, --yes', 'Skip confirmations')
  .action(async (opts) => {
    // Uninstall then install
    const yesFlag = opts.yes ? ['--yes'] : [];
    const agentsFlag = opts.agents ? ['--agents', opts.agents] : [];
    await program.commands.find((c) => c.name() === 'uninstall')!.parseAsync(['node', 'x', ...agentsFlag, ...yesFlag]);
    await program.commands.find((c) => c.name() === 'install')!.parseAsync(['node', 'x', '--skip-cli', ...agentsFlag, ...yesFlag]);
  });

program
  .command('status')
  .description('Show what is installed and where')
  .action(async () => {
    console.log(chalk.bold('\n  Syncable CLI Skills Status\n'));

    const detectionResults = await detectAgents();
    const syncCtlStatus = await checkSyncCtl();
    const cargoStatus = await checkCargo();

    console.log('  Agent         Status       Location');
    console.log('  ' + '─'.repeat(60));

    for (const { agent } of detectionResults) {
      const dest = agent.getSkillPath();
      const count = countInstalledSkills(dest, agent.name);
      if (count > 0) {
        console.log(`  ${agent.displayName.padEnd(14)} ${chalk.green('✓ installed')}  ${dest} (${count} skills)`);
      } else {
        console.log(`  ${agent.displayName.padEnd(14)} ${chalk.dim('✗ not installed')}`);
      }
    }

    console.log();
    if (syncCtlStatus.status === 'ok') {
      console.log(`  sync-ctl      ${chalk.green('✓')} v${syncCtlStatus.version}`);
    } else {
      console.log(`  sync-ctl      ${chalk.red('✗ not found')}`);
    }
    if (cargoStatus.status === 'ok') {
      console.log(`  cargo         ${chalk.green('✓')} v${cargoStatus.version}`);
    } else {
      console.log(`  cargo         ${chalk.red('✗ not found')}`);
    }
    console.log();
  });

program.parse();
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd installer && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 3: Test the CLI locally**

Run: `cd installer && node scripts/copy-skills.js && npx tsc && node dist/index.js --help`
Expected: Shows help text with install/uninstall/update/status commands.

Run: `cd installer && node dist/index.js status`
Expected: Shows agent status table.

- [ ] **Step 4: Commit**

```bash
git add installer/src/index.ts
git commit -m "feat(installer): add CLI entrypoint with commander, inquirer, ora, chalk"
```

---

### Task 15: Integration test and final verification

**Files:**
- None new — verification of everything working together.

- [ ] **Step 1: Run the full test suite**

Run: `cd installer && npm test`
Expected: All tests PASS. Verify count matches expectations (~35-40 tests across all files).

- [ ] **Step 2: Run a dry-run install**

Run: `cd installer && node dist/index.js install --dry-run --yes`
Expected: Shows what would be installed for detected agents without writing files.

- [ ] **Step 3: Run the copy-skills script and verify skill count**

Run: `cd installer && node scripts/copy-skills.js && ls skills/commands/ skills/workflows/`
Expected: 7 command files + 4 workflow files.

- [ ] **Step 4: Verify .gitignore excludes build artifacts**

Run: `cd installer && cat .gitignore`
Expected: Contains `node_modules/`, `dist/`, `skills/`.

- [ ] **Step 5: Final commit if any fixes were needed**

```bash
git add -A installer/
git commit -m "chore(installer): final integration fixes"
```

---

## Summary

| Task | Component | Files | Tests |
|------|-----------|-------|-------|
| 1 | Project scaffolding | 5 | 0 (infra) |
| 2 | Constants + utils | 2 | ~8 |
| 3 | Skill loader | 1 | ~5 |
| 4 | Agent types + detection | 7 | ~3 |
| 5 | Claude transformer | 2 | ~3 |
| 6 | Codex transformer | 1 | ~3 |
| 7 | Cursor transformer | 1 | ~6 |
| 8 | Windsurf + Gemini transformers | 2 | ~8 |
| 9 | Prerequisites | 3 | ~4 |
| 10 | Install command | 1 | ~8 |
| 11 | Uninstall command | 1 | ~5 |
| 12 | Status command | 1 | ~4 |
| 13 | Update command | 1 | 0 (thin wrapper) |
| 14 | CLI entrypoint | 1 | 0 (integration) |
| 15 | Integration verification | 0 | manual |

**Total: ~30 source files, ~57 tests, 15 tasks, ~15 commits**
