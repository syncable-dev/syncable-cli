import fs from 'fs';
import path from 'path';
import os from 'os';
import { AgentConfig } from './types.js';
import { commandExists } from '../utils.js';

/**
 * Codex agent configuration.
 *
 * Per OpenAI Codex documentation, skills are discovered from:
 *   - Project-level: .codex/skills/ (checked into repo)
 *   - User-level:    ~/.codex/skills/ (personal, cross-project)
 *   - System:        ~/.codex/skills/.system/ (built-in, read-only)
 *
 * The installer writes to ~/.codex/skills/ for global installation.
 *
 * IMPORTANT: Users must run `codex --enable skills` for skills to be active.
 * The $skill-installer and $skill-creator system skills can also manage skills.
 *
 * Reference: https://developers.openai.com/codex/skills
 */
export const codexAgent: AgentConfig = {
  name: 'codex',
  displayName: 'Codex',
  installType: 'global',
  detect: async () => {
    return fs.existsSync(path.join(os.homedir(), '.codex')) || await commandExists('codex');
  },
  getSkillPath: () => {
    // Codex discovers user-level skills from ~/.codex/skills/
    return path.join(os.homedir(), '.codex', 'skills');
  },
};
