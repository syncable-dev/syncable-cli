export type AgentName = 'claude' | 'cursor' | 'windsurf' | 'codex' | 'gemini';

export interface AgentConfig {
  name: AgentName;
  displayName: string;
  installType: 'global' | 'project';
  detect: () => Promise<boolean>;
  getSkillPath: () => string;
}
