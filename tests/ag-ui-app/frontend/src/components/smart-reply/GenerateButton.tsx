import { Send, Loader2, Square } from 'lucide-react'

interface GenerateButtonProps {
  onClick: () => void
  onStop?: () => void
  isLoading?: boolean
  disabled?: boolean
}

export function GenerateButton({
  onClick,
  onStop,
  isLoading = false,
  disabled = false,
}: GenerateButtonProps) {
  const handleClick = () => {
    if (isLoading && onStop) {
      onStop()
    } else {
      onClick()
    }
  }

  return (
    <button
      onClick={handleClick}
      disabled={disabled && !isLoading}
      className={`
        group relative flex items-center justify-center gap-3 px-8 py-4 rounded-2xl font-semibold text-base
        transition-all duration-300 overflow-hidden cursor-pointer
        ${
          isLoading
            ? 'bg-slate-800/80 border border-cyan-500/40 text-cyan-300 animate-glow-pulse'
            : 'bg-gradient-to-r from-cyan-500 to-violet-600 text-white shadow-lg shadow-cyan-500/25 hover:shadow-[0_0_40px_rgba(34,211,238,0.4)] hover:from-cyan-400 hover:to-violet-500'
        }
        ${!isLoading && !disabled ? 'transform hover:scale-[1.03] active:scale-[0.98]' : ''}
        disabled:opacity-40 disabled:cursor-not-allowed disabled:transform-none disabled:shadow-none
        focus:outline-none focus:ring-2 focus:ring-cyan-500/50 focus:ring-offset-2 focus:ring-offset-slate-950
      `}
      aria-label={isLoading ? 'Stop generating' : 'Generate reply suggestions'}
    >
      {/* Shimmer effect on hover */}
      {!isLoading && !disabled && (
        <div className="absolute inset-0 -translate-x-full group-hover:translate-x-full transition-transform duration-700 bg-gradient-to-r from-transparent via-white/20 to-transparent" />
      )}

      {/* Button content */}
      <span className="relative flex items-center gap-3">
        {isLoading ? (
          <>
            <Loader2 className="w-5 h-5 animate-spin" />
            <span>Generating...</span>
            {onStop && (
              <Square className="w-4 h-4 ml-1 opacity-70" />
            )}
          </>
        ) : (
          <>
            <Send className="w-5 h-5 transition-transform duration-200 group-hover:translate-x-0.5" />
            <span>Generate Replies</span>
          </>
        )}
      </span>
    </button>
  )
}
