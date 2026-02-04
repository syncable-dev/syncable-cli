import { dbOps, type SampleType, type PatternType, type VocabularyLevel } from './db/index.js'

// Common greetings to detect
const COMMON_GREETINGS = [
  'hi', 'hey', 'hello', 'dear', 'good morning', 'good afternoon', 'good evening',
  'greetings', 'hiya', 'howdy', 'sup', 'yo'
]

// Common sign-offs to detect
const COMMON_SIGNOFFS = [
  'best', 'thanks', 'thank you', 'regards', 'cheers', 'sincerely', 'warmly',
  'take care', 'best regards', 'kind regards', 'warm regards', 'many thanks',
  'talk soon', 'later', 'bye', 'ciao', 'xo', 'love'
]

// Professional words (indicate formal style)
const PROFESSIONAL_WORDS = [
  'regarding', 'furthermore', 'therefore', 'accordingly', 'pursuant',
  'subsequently', 'henceforth', 'aforementioned', 'herewith', 'notwithstanding',
  'acknowledge', 'appreciate', 'consideration', 'implementation'
]

// Casual indicators
const CASUAL_INDICATORS = [
  'gonna', 'wanna', 'kinda', 'gotta', 'yeah', 'nope', 'yup', 'cool',
  'awesome', 'stuff', 'things', 'btw', 'fyi', 'asap', 'lol', 'haha'
]

export interface LearnResult {
  learned: boolean
  patternsUpdated: number
  extracted: {
    greetings: string[]
    signoffs: string[]
    phrases: string[]
    stats: {
      wordCount: number
      sentenceCount: number
      avgWordLength: number
      avgSentenceLength: number
      emojiCount: number
      exclamationCount: number
      questionCount: number
    }
  }
}

/**
 * Learn from a text sample
 */
export function learnFromSample(content: string, type: SampleType): LearnResult {
  const trimmed = content.trim()
  if (!trimmed) {
    return {
      learned: false,
      patternsUpdated: 0,
      extracted: { greetings: [], signoffs: [], phrases: [], stats: getEmptyStats() }
    }
  }

  // Save the sample
  dbOps.saveSample(trimmed, type)

  // Extract patterns
  const greetings = extractGreetings(trimmed)
  const signoffs = extractSignoffs(trimmed)
  const phrases = extractPhrases(trimmed)
  const stats = analyzeStats(trimmed)

  let patternsUpdated = 0

  // Save extracted patterns
  for (const greeting of greetings) {
    dbOps.updatePattern('greeting', greeting)
    patternsUpdated++
  }

  for (const signoff of signoffs) {
    dbOps.updatePattern('signoff', signoff)
    patternsUpdated++
  }

  for (const phrase of phrases) {
    dbOps.updatePattern('phrase', phrase)
    patternsUpdated++
  }

  // Update the aggregated profile
  updateAggregatedProfile()

  return {
    learned: true,
    patternsUpdated,
    extracted: { greetings, signoffs, phrases, stats }
  }
}

/**
 * Extract greetings from the start of text
 */
function extractGreetings(text: string): string[] {
  const found: string[] = []
  const lowerText = text.toLowerCase()
  const firstLine = text.split('\n')[0].trim()
  const lowerFirst = firstLine.toLowerCase()

  for (const greeting of COMMON_GREETINGS) {
    if (lowerFirst.startsWith(greeting)) {
      // Get the actual casing from the original
      const match = firstLine.substring(0, greeting.length + 20).split(/[,!\n]/)[0].trim()
      if (match && match.length < 30) {
        found.push(match)
        break
      }
    }
  }

  return found
}

/**
 * Extract sign-offs from the end of text
 */
function extractSignoffs(text: string): string[] {
  const found: string[] = []
  const lines = text.split('\n').filter(l => l.trim())
  const lastLines = lines.slice(-3)

  for (const line of lastLines) {
    const lowerLine = line.toLowerCase().trim()
    for (const signoff of COMMON_SIGNOFFS) {
      if (lowerLine.startsWith(signoff) || lowerLine === signoff) {
        const cleanSignoff = line.trim().replace(/[,!.]+$/, '').trim()
        if (cleanSignoff && cleanSignoff.length < 30) {
          found.push(cleanSignoff)
        }
        break
      }
    }
  }

  return [...new Set(found)]
}

/**
 * Extract common phrases (3-5 word combinations)
 */
function extractPhrases(text: string): string[] {
  const phrases: string[] = []
  const words = text.split(/\s+/)

  // Look for common phrase patterns
  const commonPhrases = [
    "I'd be happy to",
    "Let me know if",
    "Thanks for reaching out",
    "I hope this helps",
    "Please let me know",
    "Looking forward to",
    "Feel free to",
    "Don't hesitate to",
    "I wanted to",
    "Just wanted to",
    "Hope you're doing well",
    "Hope this finds you well"
  ]

  const lowerText = text.toLowerCase()
  for (const phrase of commonPhrases) {
    if (lowerText.includes(phrase.toLowerCase())) {
      phrases.push(phrase)
    }
  }

  return phrases.slice(0, 3) // Max 3 phrases per sample
}

/**
 * Analyze text statistics
 */
function analyzeStats(text: string) {
  const words = text.split(/\s+/).filter(w => w.length > 0)
  const sentences = text.split(/[.!?]+/).filter(s => s.trim().length > 0)

  // Count emojis (rough pattern)
  const emojiRegex = /[\u{1F300}-\u{1F9FF}]|[\u{2600}-\u{26FF}]|[\u{2700}-\u{27BF}]/gu
  const emojiCount = (text.match(emojiRegex) || []).length

  // Count punctuation
  const exclamationCount = (text.match(/!/g) || []).length
  const questionCount = (text.match(/\?/g) || []).length

  // Calculate averages
  const totalWordLength = words.reduce((sum, w) => sum + w.replace(/[^a-zA-Z]/g, '').length, 0)
  const avgWordLength = words.length > 0 ? totalWordLength / words.length : 0

  const totalSentenceLength = sentences.reduce((sum, s) => sum + s.split(/\s+/).length, 0)
  const avgSentenceLength = sentences.length > 0 ? totalSentenceLength / sentences.length : 0

  return {
    wordCount: words.length,
    sentenceCount: sentences.length,
    avgWordLength: Math.round(avgWordLength * 100) / 100,
    avgSentenceLength: Math.round(avgSentenceLength * 100) / 100,
    emojiCount,
    exclamationCount,
    questionCount
  }
}

function getEmptyStats() {
  return {
    wordCount: 0,
    sentenceCount: 0,
    avgWordLength: 0,
    avgSentenceLength: 0,
    emojiCount: 0,
    exclamationCount: 0,
    questionCount: 0
  }
}

/**
 * Update the aggregated profile based on all samples
 */
function updateAggregatedProfile(): void {
  const samples = dbOps.getRecentSamples(50)
  if (samples.length === 0) return

  // Calculate aggregate stats
  let totalWords = 0
  let totalSentences = 0
  let totalEmojis = 0
  let totalExclamations = 0
  let totalQuestions = 0
  let professionalScore = 0
  let casualScore = 0

  for (const sample of samples) {
    const stats = analyzeStats(sample.content)
    totalWords += stats.wordCount
    totalSentences += stats.sentenceCount
    totalEmojis += stats.emojiCount
    totalExclamations += stats.exclamationCount
    totalQuestions += stats.questionCount

    // Check vocabulary
    const lower = sample.content.toLowerCase()
    for (const word of PROFESSIONAL_WORDS) {
      if (lower.includes(word)) professionalScore++
    }
    for (const word of CASUAL_INDICATORS) {
      if (lower.includes(word)) casualScore++
    }
  }

  // Determine vocabulary level
  let vocabularyLevel: VocabularyLevel = 'mixed'
  if (professionalScore > casualScore * 2) {
    vocabularyLevel = 'professional'
  } else if (casualScore > professionalScore * 2) {
    vocabularyLevel = 'casual'
  }

  const avgSentenceLength = totalSentences > 0 ? totalWords / totalSentences : 0
  const emojiUsage = totalWords > 0 ? totalEmojis / totalWords : 0
  const exclamationFreq = totalSentences > 0 ? totalExclamations / totalSentences : 0
  const questionFreq = totalSentences > 0 ? totalQuestions / totalSentences : 0

  dbOps.updateProfile({
    avg_sentence_length: Math.round(avgSentenceLength * 100) / 100,
    vocabulary_level: vocabularyLevel,
    emoji_usage: Math.round(emojiUsage * 1000) / 1000,
    exclamation_frequency: Math.round(exclamationFreq * 100) / 100,
    question_frequency: Math.round(questionFreq * 100) / 100,
    total_samples: samples.length
  })
}

export interface EnhanceResult {
  enhanced: string
  changes: Array<{
    original: string
    replacement: string
    reason: string
  }>
}

/**
 * Enhance a reply based on learned style
 */
export function enhanceReply(reply: string): EnhanceResult {
  const changes: EnhanceResult['changes'] = []
  let enhanced = reply

  // Get top patterns
  const greetings = dbOps.getPatterns('greeting', 5)
  const signoffs = dbOps.getPatterns('signoff', 5)
  const profile = dbOps.getProfile()

  if (profile.total_samples < 3) {
    // Not enough data to make suggestions
    return { enhanced, changes }
  }

  // Suggest greeting replacement
  const topGreeting = greetings[0]?.value
  if (topGreeting) {
    const greetingPatterns = ['Hello', 'Hi', 'Hey', 'Dear']
    for (const pattern of greetingPatterns) {
      if (enhanced.startsWith(pattern) && !enhanced.startsWith(topGreeting)) {
        const regex = new RegExp(`^${pattern}(\\s|,|!)`)
        if (regex.test(enhanced)) {
          const newReply = enhanced.replace(regex, `${topGreeting}$1`)
          if (newReply !== enhanced) {
            changes.push({
              original: pattern,
              replacement: topGreeting,
              reason: `You typically use "${topGreeting}" as your greeting`
            })
            enhanced = newReply
            break
          }
        }
      }
    }
  }

  // Suggest signoff replacement
  const topSignoff = signoffs[0]?.value
  if (topSignoff) {
    const signoffPatterns = ['Best', 'Thanks', 'Regards', 'Cheers', 'Sincerely']
    const lines = enhanced.split('\n')
    const lastLineIdx = lines.length - 1

    for (const pattern of signoffPatterns) {
      if (lines[lastLineIdx].trim().startsWith(pattern) &&
          !lines[lastLineIdx].trim().startsWith(topSignoff)) {
        const regex = new RegExp(`^${pattern}`)
        if (regex.test(lines[lastLineIdx].trim())) {
          lines[lastLineIdx] = lines[lastLineIdx].replace(regex, topSignoff)
          changes.push({
            original: pattern,
            replacement: topSignoff,
            reason: `Your preferred sign-off is "${topSignoff}"`
          })
          enhanced = lines.join('\n')
          break
        }
      }
    }
  }

  return { enhanced, changes }
}
