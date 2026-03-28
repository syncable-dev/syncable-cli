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
