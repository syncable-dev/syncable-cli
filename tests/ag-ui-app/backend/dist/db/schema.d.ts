import Database from 'better-sqlite3';
export type Tone = 'professional' | 'friendly' | 'apologetic' | 'assertive' | 'neutral';
export interface Conversation {
    id: string;
    original_message: string;
    context: string | null;
    intent: string | null;
    tone: Tone;
    created_at: string;
}
export interface Reply {
    id: string;
    conversation_id: string;
    content: string;
    reply_index: number;
    created_at: string;
}
export interface ConversationWithReplies extends Conversation {
    replies: Reply[];
}
export declare function initDb(): Database.Database;
