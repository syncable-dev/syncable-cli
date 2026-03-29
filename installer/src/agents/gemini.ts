import fs from 'fs';
import path from 'path';
import os from 'os';
import { AgentConfig } from './types.js';
import { commandExists } from '../utils.js';

/**
 * Find the Gemini CLI skills directory.
 * Gemini CLI stores skills under ~/.gemini/<profile>/skills/
 * The default profile is 'antigravity'.
 */
function findGeminiSkillsDir(): string {
  const geminiDir = path.join(os.homedir(), '.gemini');

  // Check for antigravity profile (default)
  const antigravitySkills = path.join(geminiDir, 'antigravity', 'skills');
  if (fs.existsSync(antigravitySkills)) {
    return antigravitySkills;
  }

  // Check for any profile with a skills directory
  if (fs.existsSync(geminiDir)) {
    try {
      const entries = fs.readdirSync(geminiDir);
      for (const entry of entries) {
        const skillsPath = path.join(geminiDir, entry, 'skills');
        if (fs.existsSync(skillsPath) && fs.statSync(skillsPath).isDirectory()) {
          return skillsPath;
        }
      }
    } catch {
      // Ignore errors
    }
  }

  // Default to antigravity profile
  return antigravitySkills;
}

export const geminiAgent: AgentConfig = {
  name: 'gemini',
  displayName: 'Gemini CLI',
  installType: 'global',
  detect: async () => {
    return fs.existsSync(path.join(os.homedir(), '.gemini')) || await commandExists('gemini');
  },
  getSkillPath: () => {
    return findGeminiSkillsDir();
  },
};
