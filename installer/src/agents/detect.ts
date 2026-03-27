import { AgentConfig, AgentName } from './types.js';
import { claudeAgent } from './claude.js';
import { codexAgent } from './codex.js';
import { cursorAgent } from './cursor.js';
import { windsurfAgent } from './windsurf.js';
import { geminiAgent } from './gemini.js';

export function allAgents(): AgentConfig[] {
  return [claudeAgent, cursorAgent, windsurfAgent, codexAgent, geminiAgent];
}

export function getAgent(name: AgentName): AgentConfig | undefined {
  return allAgents().find((a) => a.name === name);
}

export interface DetectionResult {
  agent: AgentConfig;
  detected: boolean;
}

export async function detectAgents(): Promise<DetectionResult[]> {
  const agents = allAgents();
  const results: DetectionResult[] = [];

  for (const agent of agents) {
    const detected = await agent.detect();
    results.push({ agent, detected });
  }

  return results;
}
