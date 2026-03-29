import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AgentName } from '../../src/agents/types.js';
import { allAgents } from '../../src/agents/detect.js';

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

  it('claude, codex, and gemini are global, others are project', async () => {
    const agents = allAgents();
    const globalAgents = agents.filter((a) => a.installType === 'global');
    const projectAgents = agents.filter((a) => a.installType === 'project');

    expect(globalAgents.map((a) => a.name)).toContain('claude');
    expect(globalAgents.map((a) => a.name)).toContain('codex');
    expect(globalAgents.map((a) => a.name)).toContain('gemini');
    expect(projectAgents.map((a) => a.name)).toContain('cursor');
    expect(projectAgents.map((a) => a.name)).toContain('windsurf');
  });
});
