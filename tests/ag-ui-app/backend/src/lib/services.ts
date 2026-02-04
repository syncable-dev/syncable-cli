/**
 * Service Client Helpers
 * Query the microservices for enhanced reply generation
 */

const SENTIMENT_URL = process.env.SENTIMENT_SERVICE_URL || 'http://localhost:3002'
const CONTACTS_URL = process.env.CONTACTS_SERVICE_URL || 'http://localhost:3003'
const STYLE_URL = process.env.STYLE_SERVICE_URL || 'http://localhost:3004'

// Timeout for service calls (don't block reply generation for too long)
const SERVICE_TIMEOUT = 3000

export interface SentimentResult {
  sentiment: 'positive' | 'negative' | 'neutral' | 'mixed'
  confidence: number
  emotions: Array<{ emotion: string; score: number }>
  urgency: 'low' | 'medium' | 'high' | 'critical'
  keyPoints: string[]
  suggestedApproach: string
}

export interface ContactMatchResult {
  matchedContact: {
    id: string
    name: string
    relationship: string
    company: string | null
    formality: string
    use_emojis: boolean
    preferred_tone: string | null
  } | null
  confidence: number
  relationshipContext: string
}

export interface StyleProfile {
  totalSamples: number
  averageSentenceLength: number
  commonGreetings: string[]
  commonSignoffs: string[]
  vocabularyLevel: 'professional' | 'casual' | 'mixed'
  usesEmojis: boolean
  commonPhrases: string[]
  punctuationStyle: {
    exclamationFrequency: number
    questionFrequency: number
  }
}

/**
 * Fetch with timeout
 */
async function fetchWithTimeout(url: string, options: RequestInit = {}): Promise<Response> {
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), SERVICE_TIMEOUT)

  try {
    const response = await fetch(url, {
      ...options,
      signal: controller.signal
    })
    return response
  } finally {
    clearTimeout(timeout)
  }
}

/**
 * Analyze sentiment of a message
 */
export async function analyzeSentiment(message: string): Promise<SentimentResult | null> {
  try {
    const response = await fetchWithTimeout(`${SENTIMENT_URL}/api/analyze`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message })
    })

    if (!response.ok) {
      console.warn('[Services] Sentiment analysis failed:', response.status)
      return null
    }

    return await response.json() as SentimentResult
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      console.warn('[Services] Sentiment analysis timed out')
    } else {
      console.warn('[Services] Sentiment analysis error:', error)
    }
    return null
  }
}

/**
 * Match message to a known contact
 */
export async function matchContact(message: string): Promise<ContactMatchResult | null> {
  try {
    const response = await fetchWithTimeout(`${CONTACTS_URL}/api/contacts/match`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message })
    })

    if (!response.ok) {
      console.warn('[Services] Contact matching failed:', response.status)
      return null
    }

    return await response.json() as ContactMatchResult
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      console.warn('[Services] Contact matching timed out')
    } else {
      console.warn('[Services] Contact matching error:', error)
    }
    return null
  }
}

/**
 * Get the user's writing style profile
 */
export async function getStyleProfile(): Promise<StyleProfile | null> {
  try {
    const response = await fetchWithTimeout(`${STYLE_URL}/api/profile`)

    if (!response.ok) {
      console.warn('[Services] Style profile fetch failed:', response.status)
      return null
    }

    return await response.json() as StyleProfile
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      console.warn('[Services] Style profile timed out')
    } else {
      console.warn('[Services] Style profile error:', error)
    }
    return null
  }
}

/**
 * Learn from a selected/edited reply
 */
export async function learnFromReply(content: string, type: 'selected_reply' | 'custom_edit' | 'sent_message'): Promise<boolean> {
  try {
    const response = await fetchWithTimeout(`${STYLE_URL}/api/learn`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ content, type })
    })

    return response.ok
  } catch (error) {
    console.warn('[Services] Learn from reply error:', error)
    return false
  }
}

/**
 * Check health of all services
 */
export async function checkServicesHealth(): Promise<{
  sentiment: boolean
  contacts: boolean
  style: boolean
}> {
  const results = await Promise.allSettled([
    fetchWithTimeout(`${SENTIMENT_URL}/health`),
    fetchWithTimeout(`${CONTACTS_URL}/health`),
    fetchWithTimeout(`${STYLE_URL}/health`)
  ])

  return {
    sentiment: results[0].status === 'fulfilled' && results[0].value.ok,
    contacts: results[1].status === 'fulfilled' && results[1].value.ok,
    style: results[2].status === 'fulfilled' && results[2].value.ok
  }
}

/**
 * Build enhanced context from all services
 */
export async function buildEnhancedContext(message: string): Promise<string> {
  // Query all services in parallel
  const [sentiment, contact, style] = await Promise.all([
    analyzeSentiment(message),
    matchContact(message),
    getStyleProfile()
  ])

  const contextParts: string[] = []

  // Add sentiment context
  if (sentiment) {
    const emotionList = sentiment.emotions
      .slice(0, 3)
      .map(e => `${e.emotion} (${Math.round(e.score * 100)}%)`)
      .join(', ')

    contextParts.push(`SENTIMENT ANALYSIS:
- Overall sentiment: ${sentiment.sentiment} (confidence: ${Math.round(sentiment.confidence * 100)}%)
- Detected emotions: ${emotionList || 'none detected'}
- Urgency level: ${sentiment.urgency}
${sentiment.keyPoints.length > 0 ? `- Key points: ${sentiment.keyPoints.join(', ')}` : ''}
- Suggested approach: ${sentiment.suggestedApproach}`)
  }

  // Add contact context
  if (contact && contact.matchedContact) {
    contextParts.push(`RELATIONSHIP CONTEXT:
${contact.relationshipContext}`)
  }

  // Add style context
  if (style && style.totalSamples >= 3) {
    const styleHints: string[] = []

    if (style.commonGreetings.length > 0) {
      styleHints.push(`Preferred greetings: ${style.commonGreetings.slice(0, 3).join(', ')}`)
    }
    if (style.commonSignoffs.length > 0) {
      styleHints.push(`Preferred sign-offs: ${style.commonSignoffs.slice(0, 3).join(', ')}`)
    }
    if (style.vocabularyLevel !== 'mixed') {
      styleHints.push(`Writing style: ${style.vocabularyLevel}`)
    }
    if (style.usesEmojis) {
      styleHints.push('User likes to use emojis')
    }
    if (style.commonPhrases.length > 0) {
      styleHints.push(`Common phrases: "${style.commonPhrases.slice(0, 2).join('", "')}"`)
    }

    if (styleHints.length > 0) {
      contextParts.push(`USER'S WRITING STYLE:
${styleHints.join('\n')}`)
    }
  }

  return contextParts.join('\n\n')
}
