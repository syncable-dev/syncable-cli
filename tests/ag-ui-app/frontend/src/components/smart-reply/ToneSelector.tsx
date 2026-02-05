import { Briefcase, Smile, Heart, Zap, Minus } from 'lucide-react'
import type { Tone } from '@/lib/api'

interface ToneSelectorProps {
  selected: Tone
  onChange: (tone: Tone) => void
  disabled?: boolean
}

const TONES: Array<{
  value: Tone
  label: string
  icon: React.ReactNode
  description: string
}> = [
  {
    value: 'professional',
    label: 'Professional',
    icon: <Briefcase className="w-4 h-4" />,
    description: 'Formal, business-appropriate',
  },
  {
    value: 'friendly',
    label: 'Friendly',
    icon: <Smile className="w-4 h-4" />,
    description: 'Warm and personable',
  },
  {
    value: 'apologetic',
    label: 'Apologetic',
    icon: <Heart className="w-4 h-4" />,
    description: 'Sincere and understanding',
  },
  {
    value: 'assertive',
    label: 'Assertive',
    icon: <Zap className="w-4 h-4" />,
    description: 'Direct and confident',
  },
  {
    value: 'neutral',
    label: 'Neutral',
    icon: <Minus className="w-4 h-4" />,
    description: 'Balanced and objective',
  },
]

export function ToneSelector({ selected, onChange, disabled = false }: ToneSelectorProps) {
  return (
    <div className="space-y-3 animate-fade-in-up" style={{ animationDelay: '100ms' }}>
      <label className="text-sm font-medium text-slate-300">Select tone</label>

      <div
        className="flex flex-wrap gap-2"
        role="radiogroup"
        aria-label="Reply tone selection"
      >
        {TONES.map((tone) => {
          const isSelected = selected === tone.value

          return (
            <button
              key={tone.value}
              onClick={() => onChange(tone.value)}
              disabled={disabled}
              role="radio"
              aria-checked={isSelected}
              title={tone.description}
              className={`
                group flex items-center gap-2 px-4 py-2.5 rounded-xl text-sm font-medium
                transition-all duration-200 border cursor-pointer
                ${
                  isSelected
                    ? 'bg-gradient-to-r from-cyan-500/20 to-violet-500/20 border-cyan-500/40 text-cyan-300 shadow-[0_0_20px_rgba(34,211,238,0.15)]'
                    : 'bg-white/5 border-white/10 text-slate-400 hover:bg-white/10 hover:border-white/20 hover:text-slate-200'
                }
                disabled:opacity-50 disabled:cursor-not-allowed
                focus:outline-none focus:ring-2 focus:ring-cyan-500/50 focus:ring-offset-2 focus:ring-offset-slate-950
              `}
            >
              <span
                className={`transition-all duration-200 ${
                  isSelected ? 'text-cyan-400 scale-110' : 'text-slate-500 group-hover:text-slate-300 group-hover:scale-110'
                }`}
              >
                {tone.icon}
              </span>
              <span>{tone.label}</span>
            </button>
          )
        })}
      </div>
    </div>
  )
}
