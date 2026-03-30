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
