import OpenAI from 'openai'
import { dbOps, type SentimentResult } from './db/index.js'

// Lazy-initialize OpenAI client (only when API key is available)
let openai: OpenAI | null = null

function getOpenAI(): OpenAI | null {
  if (!openai && process.env.OPENAI_API_KEY) {
    openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY })
  }
  return openai
}

const ANALYSIS_PROMPT = `You are a sentiment and emotion analyzer. Analyze the following message and return a JSON object with:

1. "sentiment": Overall sentiment - one of: "positive", "negative", "neutral", "mixed"
2. "confidence": Your confidence in the sentiment (0.0 to 1.0)
3. "emotions": Array of detected emotions with scores, e.g. [{"emotion": "frustrated", "score": 0.8}]
   Possible emotions: happy, frustrated, confused, angry, anxious, grateful, excited, disappointed, hopeful, urgent
4. "urgency": Urgency level - one of: "low", "medium", "high", "critical"
5. "keyPoints": Array of key concerns or topics mentioned (max 3)
6. "suggestedApproach": A brief suggestion on how to respond (1 sentence)

Return ONLY valid JSON, no markdown or explanation.

Message to analyze:`

export async function analyzeSentiment(message: string): Promise<SentimentResult> {
  // Check cache first
  const cached = dbOps.getCached(message)
  if (cached) {
    console.log('[Analyzer] Cache hit')
    return cached
  }

  const client = getOpenAI()
  if (!client) {
    console.log('[Analyzer] No OpenAI API key, using fallback analysis')
    return fallbackAnalysis(message)
  }

  console.log('[Analyzer] Analyzing with OpenAI...')

  try {
    const response = await client.chat.completions.create({
      model: 'gpt-4o-mini',
      messages: [
        {
          role: 'system',
          content: ANALYSIS_PROMPT
        },
        {
          role: 'user',
          content: message
        }
      ],
      temperature: 0.3,
      max_tokens: 500
    })

    const content = response.choices[0]?.message?.content || ''

    // Parse the JSON response
    const result = parseAnalysisResponse(content)

    // Cache the result
    dbOps.saveCache(message, result)

    return result
  } catch (error) {
    console.error('[Analyzer] OpenAI error:', error)
    // Fall back to pattern-based analysis
    return fallbackAnalysis(message)
  }
}

function parseAnalysisResponse(content: string): SentimentResult {
  try {
    // Try to extract JSON from the response
    const jsonMatch = content.match(/\{[\s\S]*\}/)
    if (jsonMatch) {
      const parsed = JSON.parse(jsonMatch[0])
      return validateResult(parsed)
    }
  } catch (e) {
    console.error('[Analyzer] Parse error:', e)
  }

  // Return neutral fallback
  return getDefaultResult()
}

function validateResult(parsed: Record<string, unknown>): SentimentResult {
  const validSentiments = ['positive', 'negative', 'neutral', 'mixed']
  const validUrgencies = ['low', 'medium', 'high', 'critical']

  return {
    sentiment: validSentiments.includes(parsed.sentiment as string)
      ? (parsed.sentiment as SentimentResult['sentiment'])
      : 'neutral',
    confidence: typeof parsed.confidence === 'number'
      ? Math.min(1, Math.max(0, parsed.confidence))
      : 0.5,
    emotions: Array.isArray(parsed.emotions)
      ? parsed.emotions.slice(0, 5).map((e: Record<string, unknown>) => ({
          emotion: String(e.emotion || 'unknown'),
          score: typeof e.score === 'number' ? Math.min(1, Math.max(0, e.score)) : 0.5
        }))
      : [],
    urgency: validUrgencies.includes(parsed.urgency as string)
      ? (parsed.urgency as SentimentResult['urgency'])
      : 'medium',
    keyPoints: Array.isArray(parsed.keyPoints)
      ? parsed.keyPoints.slice(0, 3).map(String)
      : [],
    suggestedApproach: typeof parsed.suggestedApproach === 'string'
      ? parsed.suggestedApproach
      : 'Respond professionally and address their concerns.'
  }
}

function fallbackAnalysis(message: string): SentimentResult {
  const lowerMessage = message.toLowerCase()
  const patterns = dbOps.getEmotionPatterns()

  // Detect emotions based on patterns
  const detectedEmotions: Map<string, number> = new Map()

  for (const { pattern, emotion, weight } of patterns) {
    if (lowerMessage.includes(pattern.toLowerCase())) {
      const current = detectedEmotions.get(emotion) || 0
      detectedEmotions.set(emotion, Math.max(current, weight))
    }
  }

  const emotions = Array.from(detectedEmotions.entries())
    .map(([emotion, score]) => ({ emotion, score }))
    .sort((a, b) => b.score - a.score)
    .slice(0, 3)

  // Determine overall sentiment
  let sentiment: SentimentResult['sentiment'] = 'neutral'
  const negativeEmotions = ['frustrated', 'angry', 'anxious', 'disappointed']
  const positiveEmotions = ['happy', 'grateful', 'excited', 'hopeful']

  const hasNegative = emotions.some(e => negativeEmotions.includes(e.emotion))
  const hasPositive = emotions.some(e => positiveEmotions.includes(e.emotion))

  if (hasNegative && hasPositive) {
    sentiment = 'mixed'
  } else if (hasNegative) {
    sentiment = 'negative'
  } else if (hasPositive) {
    sentiment = 'positive'
  }

  // Determine urgency
  let urgency: SentimentResult['urgency'] = 'medium'
  const urgentPatterns = ['urgent', 'asap', 'immediately', 'emergency', 'critical', 'deadline']
  const hasUrgent = urgentPatterns.some(p => lowerMessage.includes(p))

  if (hasUrgent) {
    urgency = 'high'
    if (lowerMessage.includes('emergency') || lowerMessage.includes('critical')) {
      urgency = 'critical'
    }
  }

  return {
    sentiment,
    confidence: 0.6, // Lower confidence for fallback
    emotions,
    urgency,
    keyPoints: [],
    suggestedApproach: 'Address their concerns professionally.'
  }
}

function getDefaultResult(): SentimentResult {
  return {
    sentiment: 'neutral',
    confidence: 0.5,
    emotions: [],
    urgency: 'medium',
    keyPoints: [],
    suggestedApproach: 'Respond professionally and address their concerns.'
  }
}
