import { Copy, Check, ChevronRight } from 'lucide-react'
import type { Reply, Tone } from '@/lib/api'

interface ReplyCardProps {
  reply: Reply
  tone: Tone
  isCopied: boolean
  onCopy: () => void
  onSelect: () => void
  animationDelay?: number
}

const TONE_BADGE_STYLES: Record<Tone, string> = {
  professional: 'bg-blue-500/10 text-blue-300 border-blue-500/20',
  friendly: 'bg-emerald-500/10 text-emerald-300 border-emerald-500/20',
  apologetic: 'bg-amber-500/10 text-amber-300 border-amber-500/20',
  assertive: 'bg-rose-500/10 text-rose-300 border-rose-500/20',
  neutral: 'bg-slate-500/10 text-slate-300 border-slate-500/20',
}

export function ReplyCard({
  reply,
  tone,
  isCopied,
  onCopy,
  onSelect,
  animationDelay = 0,
}: ReplyCardProps) {
  return (
    <div
      className="group bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-6
        hover:bg-white/[0.08] hover:border-cyan-500/30 hover:shadow-[0_0_40px_rgba(34,211,238,0.1)]
        cursor-pointer transition-all duration-300 animate-fade-in-up"
      style={{
        animationDelay: `${animationDelay}ms`,
        animationFillMode: 'both',
      }}
      onClick={onSelect}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          onSelect()
        }
      }}
      aria-label={`Reply option ${reply.reply_index + 1}. ${reply.content}`}
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <span
          className={`px-3 py-1.5 rounded-full text-xs font-medium border ${TONE_BADGE_STYLES[tone]}`}
        >
          Option {reply.reply_index + 1}
        </span>

        <button
          onClick={(e) => {
            e.stopPropagation()
            onCopy()
          }}
          className="flex items-center gap-2 px-3 py-1.5 rounded-lg cursor-pointer
            bg-white/5 hover:bg-white/10
            text-slate-400 hover:text-slate-200
            opacity-0 group-hover:opacity-100
            transition-all duration-200
            focus:outline-none focus:ring-2 focus:ring-cyan-500/50 focus:opacity-100"
          aria-label={isCopied ? 'Copied to clipboard' : 'Copy to clipboard'}
        >
          {isCopied ? (
            <>
              <Check className="w-4 h-4 text-emerald-400" />
              <span className="text-xs font-medium text-emerald-400">Copied!</span>
            </>
          ) : (
            <>
              <Copy className="w-4 h-4" />
              <span className="text-xs font-medium">Copy</span>
            </>
          )}
        </button>
      </div>

      {/* Content */}
      <p className="text-slate-100 leading-relaxed mb-5 whitespace-pre-wrap text-[15px]">
        {reply.content}
      </p>

      {/* Action */}
      <div className="flex justify-end">
        <button
          onClick={(e) => {
            e.stopPropagation()
            onSelect()
          }}
          className="flex items-center gap-1.5 text-sm font-medium text-cyan-400 hover:text-cyan-300
            transition-all duration-200 cursor-pointer group/btn"
        >
          <span>Use this reply</span>
          <ChevronRight className="w-4 h-4 transition-transform duration-200 group-hover/btn:translate-x-1" />
        </button>
      </div>
    </div>
  )
}

export function ReplyCardSkeleton({ delay = 0 }: { delay?: number }) {
  return (
    <div
      className="bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-6 animate-fade-in-up"
      style={{ animationDelay: `${delay}ms`, animationFillMode: 'both' }}
    >
      <div className="flex items-center justify-between mb-4">
        <div className="h-7 w-20 bg-slate-700/50 rounded-full animate-shimmer" />
        <div className="h-8 w-16 bg-slate-700/50 rounded-lg animate-shimmer" />
      </div>

      <div className="space-y-3 mb-5">
        <div className="h-4 bg-slate-700/50 rounded-lg w-full animate-shimmer" />
        <div className="h-4 bg-slate-700/50 rounded-lg w-11/12 animate-shimmer" />
        <div className="h-4 bg-slate-700/50 rounded-lg w-4/5 animate-shimmer" />
      </div>

      <div className="flex justify-end">
        <div className="h-5 w-28 bg-slate-700/50 rounded-lg animate-shimmer" />
      </div>
    </div>
  )
}
