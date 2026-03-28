export type SampleType = 'selected_reply' | 'custom_edit' | 'sent_message';
export type PatternType = 'greeting' | 'signoff' | 'phrase' | 'word';
export type VocabularyLevel = 'professional' | 'casual' | 'mixed';
export interface StyleSample {
    id: string;
    content: string;
    type: SampleType;
    word_count: number;
    sentence_count: number;
    created_at: string;
}
export interface StylePattern {
    id: string;
    pattern_type: PatternType;
    value: string;
    frequency: number;
    last_used: string;
}
export interface StyleProfile {
    id: string;
    avg_sentence_length: number;
    avg_word_length: number;
    vocabulary_level: VocabularyLevel;
    emoji_usage: number;
    exclamation_frequency: number;
    question_frequency: number;
    total_samples: number;
    updated_at: string;
}
export declare const dbOps: {
    saveSample(content: string, type: SampleType): StyleSample;
    updatePattern(type: PatternType, value: string): void;
    getPatterns(type: PatternType, limit?: number): StylePattern[];
    getAllPatterns(): StylePattern[];
    getProfile(): StyleProfile;
    updateProfile(updates: Partial<Omit<StyleProfile, "id" | "updated_at">>): void;
    getSampleCount(): number;
    getRecentSamples(limit?: number): StyleSample[];
    clearAll(): void;
};
