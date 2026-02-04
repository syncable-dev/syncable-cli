import { dbOps, type Contact, type Formality, type Relationship } from './db/index.js'

export interface MatchResult {
  matchedContact: Contact | null
  confidence: number
  relationshipContext: string
}

// Relationship descriptions for context
const relationshipDescriptions: Record<Relationship, string> = {
  manager: 'your manager',
  colleague: 'your colleague',
  client: 'a client',
  vendor: 'a vendor/supplier',
  friend: 'a friend',
  family: 'a family member',
  other: 'a contact'
}

// Formality suggestions
const formalitySuggestions: Record<Formality, string> = {
  formal: 'Use formal, professional language.',
  casual: 'You can use casual, friendly language.',
  adaptive: 'Match the tone of their message.'
}

/**
 * Try to match a message to a known contact
 */
export function matchMessageToContact(message: string): MatchResult {
  const contacts = dbOps.getAllContacts()

  if (contacts.length === 0) {
    return {
      matchedContact: null,
      confidence: 0,
      relationshipContext: 'No contacts in database.'
    }
  }

  // Extract potential names/emails from the message
  const lowerMessage = message.toLowerCase()

  let bestMatch: Contact | null = null
  let bestScore = 0

  for (const contact of contacts) {
    let score = 0

    // Check for name match
    const nameParts = contact.name.toLowerCase().split(' ')
    const fullName = contact.name.toLowerCase()

    // Exact full name match
    if (lowerMessage.includes(fullName)) {
      score += 1.0
    } else {
      // Check individual name parts
      for (const part of nameParts) {
        if (part.length >= 3 && lowerMessage.includes(part)) {
          score += 0.5
        }
      }
    }

    // Check for email match
    if (contact.email && lowerMessage.includes(contact.email.toLowerCase())) {
      score += 0.8
    }

    // Check for company match
    if (contact.company && lowerMessage.includes(contact.company.toLowerCase())) {
      score += 0.3
    }

    if (score > bestScore) {
      bestScore = score
      bestMatch = contact
    }
  }

  // Normalize confidence (cap at 1.0)
  const confidence = Math.min(1.0, bestScore)

  if (bestMatch && confidence >= 0.3) {
    const context = buildRelationshipContext(bestMatch)
    return {
      matchedContact: bestMatch,
      confidence,
      relationshipContext: context
    }
  }

  return {
    matchedContact: null,
    confidence: 0,
    relationshipContext: 'No matching contact found. Respond professionally.'
  }
}

/**
 * Build a human-readable context string for the relationship
 */
function buildRelationshipContext(contact: Contact): string {
  const parts: string[] = []

  // Basic relationship
  const relDesc = relationshipDescriptions[contact.relationship]
  if (contact.company) {
    parts.push(`This is ${relDesc} at ${contact.company}.`)
  } else {
    parts.push(`This is ${relDesc}.`)
  }

  // Communication preferences
  parts.push(formalitySuggestions[contact.formality])

  if (contact.use_emojis) {
    parts.push('They appreciate emoji use.')
  }

  if (contact.preferred_tone) {
    parts.push(`Preferred tone: ${contact.preferred_tone}.`)
  }

  if (contact.notes) {
    parts.push(`Note: ${contact.notes}`)
  }

  return parts.join(' ')
}

/**
 * Get communication suggestions based on a contact
 */
export function getContactSuggestions(contact: Contact): string[] {
  const suggestions: string[] = []

  switch (contact.relationship) {
    case 'manager':
      suggestions.push('Be respectful and concise')
      suggestions.push('Focus on solutions, not problems')
      break
    case 'client':
      suggestions.push('Be professional and helpful')
      suggestions.push('Acknowledge their concerns')
      break
    case 'colleague':
      suggestions.push('Be collaborative')
      suggestions.push('Offer assistance if appropriate')
      break
    case 'friend':
    case 'family':
      suggestions.push('Be warm and personal')
      suggestions.push('Show genuine interest')
      break
    case 'vendor':
      suggestions.push('Be clear about expectations')
      suggestions.push('Keep communication professional')
      break
    default:
      suggestions.push('Be professional and courteous')
  }

  if (contact.formality === 'formal') {
    suggestions.push('Use proper salutations and sign-offs')
  }

  return suggestions
}
