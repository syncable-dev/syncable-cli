import type { Tone } from '../db/index.js';
/**
 * Create OpenAI adapter using TanStack AI
 * Model is passed directly as string argument
 */
export declare function getOpenAIAdapter(): import("@tanstack/ai-openai").OpenAITextAdapter<"gpt-4o-mini">;
export declare const TONE_SYSTEM_PROMPTS: Record<Tone, string>;
/**
 * Build the complete system prompt for generating smart replies
 */
export declare function buildSmartReplyPrompt(tone: Tone): string;
