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
