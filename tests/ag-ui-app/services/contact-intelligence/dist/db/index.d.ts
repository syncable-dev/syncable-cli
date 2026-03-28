export type Relationship = 'colleague' | 'manager' | 'client' | 'vendor' | 'friend' | 'family' | 'other';
export type Formality = 'formal' | 'casual' | 'adaptive';
export interface Contact {
    id: string;
    name: string;
    email: string | null;
    relationship: Relationship;
    company: string | null;
    notes: string | null;
    formality: Formality;
    use_emojis: boolean;
    preferred_tone: string | null;
    created_at: string;
    updated_at: string;
}
export interface ContactInput {
    name: string;
    email?: string;
    relationship: Relationship;
    company?: string;
    notes?: string;
    preferences?: {
        formality?: Formality;
        useEmojis?: boolean;
        preferredTone?: string;
    };
}
export interface ContactInteraction {
    id: string;
    contact_id: string;
    conversation_id: string;
    direction: 'inbound' | 'outbound';
    summary: string;
    created_at: string;
}
export declare const dbOps: {
    createContact(input: ContactInput): Contact;
    getAllContacts(): Contact[];
    getContact(id: string): Contact | null;
    updateContact(id: string, input: Partial<ContactInput>): Contact | null;
    deleteContact(id: string): boolean;
    searchContacts(query: string): Contact[];
    recordInteraction(contactId: string, conversationId: string, direction: "inbound" | "outbound", summary: string): void;
    getInteractions(contactId: string, limit?: number): ContactInteraction[];
};
