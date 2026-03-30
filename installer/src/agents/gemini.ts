import fs from 'fs';
import path from 'path';
import os from 'os';
import { AgentConfig } from './types.js';
import { commandExists } from '../utils.js';

/**
 * Gemini CLI agent configuration.
 *
 * Per the official Gemini CLI documentation, skills are discovered from:
 *   1. Workspace skills: .gemini/skills/ or .agents/skills/ (project-level)
 *   2. User skills:      ~/.gemini/skills/ or ~/.agents/skills/ (global)
 *   3. Extension skills:  bundled within installed extensions
 *
 * For global installation, we use ~/.gemini/skills/ which is the documented
 * user-level skills directory. There is NO profile subdirectory in the
 * skill discovery path — ~/.gemini/antigravity/skills/ is NOT a valid
 * skill location.
 *
 * Reference: https://geminicli.com/docs/cli/skills/
 */
export const geminiAgent: AgentConfig = {
  name: 'gemini',
  displayName: 'Gemini CLI',
  installType: 'global',
  detect: async () => {
    return fs.existsSync(path.join(os.homedir(), '.gemini')) || await commandExists('gemini');
  },
  getSkillPath: () => {
    // User-level skills directory — Gemini CLI auto-discovers skills here
    return path.join(os.homedir(), '.gemini', 'skills');
  },
};
