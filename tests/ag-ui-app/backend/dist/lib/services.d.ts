/**
 * Service Client Helpers
 * Query the microservices for enhanced reply generation
 */
export interface SentimentResult {
    sentiment: 'positive' | 'negative' | 'neutral' | 'mixed';
    confidence: number;
    emotions: Array<{
        emotion: string;
        score: number;
    }>;
    urgency: 'low' | 'medium' | 'high' | 'critical';
    keyPoints: string[];
    suggestedApproach: string;
}
export interface ContactMatchResult {
    matchedContact: {
        id: string;
        name: string;
        relationship: string;
        company: string | null;
        formality: string;
        use_emojis: boolean;
        preferred_tone: string | null;
    } | null;
    confidence: number;
    relationshipContext: string;
}
export interface StyleProfile {
    totalSamples: number;
    averageSentenceLength: number;
    commonGreetings: string[];
    commonSignoffs: string[];
    vocabularyLevel: 'professional' | 'casual' | 'mixed';
    usesEmojis: boolean;
    commonPhrases: string[];
    punctuationStyle: {
        exclamationFrequency: number;
        questionFrequency: number;
    };
}
/**
 * Analyze sentiment of a message
 */
export declare function analyzeSentiment(message: string): Promise<SentimentResult | null>;
/**
 * Match message to a known contact
 */
export declare function matchContact(message: string): Promise<ContactMatchResult | null>;
/**
 * Get the user's writing style profile
 */
export declare function getStyleProfile(): Promise<StyleProfile | null>;
/**
 * Learn from a selected/edited reply
 */
export declare function learnFromReply(content: string, type: 'selected_reply' | 'custom_edit' | 'sent_message'): Promise<boolean>;
/**
 * Check health of all services
 */
export declare function checkServicesHealth(): Promise<{
    sentiment: boolean;
    contacts: boolean;
    style: boolean;
}>;
/**
 * Build enhanced context from all services
 */
export declare function buildEnhancedContext(message: string): Promise<string>;
