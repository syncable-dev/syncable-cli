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
