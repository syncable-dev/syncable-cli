import { type Contact } from './db/index.js';
export interface MatchResult {
    matchedContact: Contact | null;
    confidence: number;
    relationshipContext: string;
}
/**
 * Try to match a message to a known contact
 */
export declare function matchMessageToContact(message: string): MatchResult;
/**
 * Get communication suggestions based on a contact
 */
export declare function getContactSuggestions(contact: Contact): string[];
