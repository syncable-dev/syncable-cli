import { MessageSquare, History, Lightbulb } from 'lucide-react'

interface MessageInputCardProps {
  message: string
  context: string
  intent: string
  onMessageChange: (value: string) => void
  onContextChange: (value: string) => void
  onIntentChange: (value: string) => void
  disabled?: boolean
}

export function MessageInputCard({
  message,
  context,
  intent,
  onMessageChange,
  onContextChange,
  onIntentChange,
  disabled = false,
}: MessageInputCardProps) {
  return (
    <div className="space-y-5 animate-glass-reveal">
      {/* Conversation Context */}
      <div className="bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-5 transition-all duration-300 hover:bg-white/[0.07] hover:border-white/15">
        <label
          htmlFor="context-input"
          className="flex items-center gap-2 text-sm font-medium text-slate-300 mb-3"
        >
          <History className="w-4 h-4 text-violet-400" />
          Conversation context
          <span className="text-slate-500 font-normal">(optional)</span>
        </label>

        <textarea
          id="context-input"
          value={context}
          onChange={(e) => onContextChange(e.target.value)}
          disabled={disabled}
          placeholder="Provide background or previous messages in the conversation...

e.g., 'We've been discussing the Q3 budget. They initially proposed $50k but I countered with $35k. This is their response to my counter-offer.'"
          rows={3}
          className="
            w-full bg-slate-800/60 backdrop-blur-sm border border-white/10 rounded-xl p-4
            text-slate-100 text-[15px] leading-relaxed placeholder:text-slate-500 resize-none
            focus:outline-none focus:border-violet-500/50 focus:shadow-[0_0_30px_rgba(139,92,246,0.1)]
            transition-all duration-200
            disabled:opacity-50 disabled:cursor-not-allowed
          "
        />
      </div>

      {/* Received Message */}
      <div className="bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-5 transition-all duration-300 hover:bg-white/[0.07] hover:border-white/15">
        <label
          htmlFor="message-input"
          className="flex items-center gap-2 text-sm font-medium text-slate-300 mb-3"
        >
          <MessageSquare className="w-4 h-4 text-cyan-400" />
          Message you received
          <span className="text-rose-400">*</span>
        </label>

        <textarea
          id="message-input"
          value={message}
          onChange={(e) => onMessageChange(e.target.value)}
          disabled={disabled}
          placeholder="Paste the message you need to reply to...

e.g., 'Hi, I wanted to follow up on our meeting yesterday. When would be a good time to reschedule?'"
          rows={4}
          className="
            w-full bg-slate-800/60 backdrop-blur-sm border border-white/10 rounded-xl p-4
            text-slate-100 text-[15px] leading-relaxed placeholder:text-slate-500 resize-none
            focus:outline-none focus:border-cyan-500/50 focus:shadow-[0_0_30px_rgba(34,211,238,0.1)]
            transition-all duration-200
            disabled:opacity-50 disabled:cursor-not-allowed
          "
        />
      </div>

      {/* Intent */}
      <div className="bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-5 transition-all duration-300 hover:bg-white/[0.07] hover:border-white/15">
        <label
          htmlFor="intent-input"
          className="flex items-center gap-2 text-sm font-medium text-slate-300 mb-3"
        >
          <Lightbulb className="w-4 h-4 text-amber-400" />
          What you want to say
          <span className="text-slate-500 font-normal">(optional)</span>
        </label>

        <textarea
          id="intent-input"
          value={intent}
          onChange={(e) => onIntentChange(e.target.value)}
          disabled={disabled}
          placeholder="Describe what you want to communicate in your reply...

e.g., 'I want to accept their offer but negotiate a faster timeline. Also mention I need the contract by Friday.'"
          rows={3}
          className="
            w-full bg-slate-800/60 backdrop-blur-sm border border-white/10 rounded-xl p-4
            text-slate-100 text-[15px] leading-relaxed placeholder:text-slate-500 resize-none
            focus:outline-none focus:border-amber-500/50 focus:shadow-[0_0_30px_rgba(245,158,11,0.1)]
            transition-all duration-200
            disabled:opacity-50 disabled:cursor-not-allowed
          "
        />
      </div>
    </div>
  )
}
