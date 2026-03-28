import { type SampleType } from './db/index.js';
export interface LearnResult {
    learned: boolean;
    patternsUpdated: number;
    extracted: {
        greetings: string[];
        signoffs: string[];
        phrases: string[];
        stats: {
            wordCount: number;
            sentenceCount: number;
            avgWordLength: number;
            avgSentenceLength: number;
            emojiCount: number;
            exclamationCount: number;
            questionCount: number;
        };
    };
}
/**
 * Learn from a text sample
 */
export declare function learnFromSample(content: string, type: SampleType): LearnResult;
export interface EnhanceResult {
    enhanced: string;
    changes: Array<{
        original: string;
        replacement: string;
        reason: string;
    }>;
}
/**
 * Enhance a reply based on learned style
 */
export declare function enhanceReply(reply: string): EnhanceResult;
