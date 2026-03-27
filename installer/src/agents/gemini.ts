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
