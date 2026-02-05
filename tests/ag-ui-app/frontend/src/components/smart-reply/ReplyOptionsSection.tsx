import { MessageSquare, Sparkles } from 'lucide-react'
import { ReplyCard, ReplyCardSkeleton } from './ReplyCard'
import type { Reply, Tone } from '@/lib/api'

interface ReplyOptionsSectionProps {
  replies: Reply[]
  isLoading: boolean
  tone: Tone
  copiedId: string | null
  onCopy: (id: string, content: string) => void
  onSelect: (id: string, content: string) => void
  streamedContent?: string
}

/**
 * ReplyOptionsSection Component
 * Section containing reply cards, loading skeletons, or empty state
 */
export function ReplyOptionsSection({
  replies,
  isLoading,
  tone,
  copiedId,
  onCopy,
  onSelect,
  streamedContent,
}: ReplyOptionsSectionProps) {
  // Loading state with skeleton cards
  if (isLoading) {
    return (
      <section aria-live="polite" aria-busy="true" className="animate-fade-in-up">
        {/* Section header */}
        <div className="flex items-center gap-2 mb-4">
          <Sparkles className="w-5 h-5 text-cyan-400 animate-pulse" />
          <h2 className="text-xl font-semibold text-slate-100">
            Generating replies...
          </h2>
        </div>

        {/* Streaming preview */}
        {streamedContent && (
          <div className="mb-4 p-4 bg-slate-800/50 rounded-xl border border-cyan-500/20">
            <p className="text-sm text-slate-400 mb-2">AI is thinking...</p>
            <p className="text-slate-300 text-sm font-mono whitespace-pre-wrap">
              {streamedContent}
              <span className="inline-block w-2 h-4 bg-cyan-400 ml-1 animate-pulse" />
            </p>
          </div>
        )}

        {/* Skeleton cards */}
        <div className="flex flex-col gap-4">
          {[0, 1, 2].map((index) => (
            <ReplyCardSkeleton key={index} delay={index * 100} />
          ))}
        </div>
      </section>
    )
  }

  // Empty state
  if (replies.length === 0) {
    return (
      <section className="flex flex-col items-center justify-center py-16 text-center animate-fade-in-up">
        {/* Icon container */}
        <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-white/5 to-white/[0.02] border border-white/10 flex items-center justify-center mb-5">
          <MessageSquare className="w-10 h-10 text-slate-500" />
        </div>

        {/* Title */}
        <h3 className="text-xl font-medium text-slate-300 mb-2">
          No replies yet
        </h3>

        {/* Description */}
        <p className="text-sm text-slate-500 max-w-sm leading-relaxed">
          Paste a message above, select your preferred tone, and click
          <span className="text-cyan-400 font-medium"> "Generate Replies" </span>
          to get AI-powered suggestions.
        </p>

        {/* Decorative dots */}
        <div className="flex gap-2 mt-6">
          {[0, 1, 2].map((i) => (
            <div
              key={i}
              className="w-2 h-2 rounded-full bg-slate-700"
              style={{ opacity: 0.3 + i * 0.2 }}
            />
          ))}
        </div>
      </section>
    )
  }

  // Reply cards
  return (
    <section aria-live="polite">
      {/* Section header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Sparkles className="w-5 h-5 text-cyan-400" />
          <h2 className="text-xl font-semibold text-slate-100">
            {replies.length} reply suggestion{replies.length !== 1 ? 's' : ''}
          </h2>
        </div>

        <span className="text-xs text-slate-500 bg-white/5 px-3 py-1 rounded-full">
          Click to copy
        </span>
      </div>

      {/* Reply cards */}
      <div className="flex flex-col gap-4">
        {replies.map((reply, index) => (
          <ReplyCard
            key={reply.id}
            reply={reply}
            tone={tone}
            isCopied={copiedId === reply.id}
            onCopy={() => onCopy(reply.id, reply.content)}
            onSelect={() => onSelect(reply.id, reply.content)}
            animationDelay={index * 100}
          />
        ))}
      </div>
    </section>
  )
}
