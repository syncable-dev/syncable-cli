import Database from 'better-sqlite3';
import { type Tone, type Conversation, type Reply, type ConversationWithReplies } from './schema.js';
export declare function getDb(): Database.Database;
export declare const dbOps: {
    createConversation(message: string, tone: Tone, context?: string, intent?: string): string;
    saveReplies(conversationId: string, replies: string[]): void;
    getHistory(limit?: number): ConversationWithReplies[];
    getConversation(id: string): ConversationWithReplies | null;
    deleteConversation(id: string): boolean;
    logEvent(eventType: "generate" | "copy" | "select" | "delete", conversationId?: string, metadata?: object): void;
};
export type { Tone, Conversation, Reply, ConversationWithReplies };
