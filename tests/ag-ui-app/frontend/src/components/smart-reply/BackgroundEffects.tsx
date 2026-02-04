/**
 * BackgroundEffects Component
 * Creates the ambient glassmorphic background with animated orbs and grid pattern
 */
export function BackgroundEffects() {
  return (
    <div className="absolute inset-0 pointer-events-none overflow-hidden">
      {/* Ambient Orb - Cyan (top left) */}
      <div
        className="absolute -top-48 -left-48 w-96 h-96 bg-cyan-500/10 rounded-full blur-3xl animate-slow-pulse"
        aria-hidden="true"
      />

      {/* Ambient Orb - Violet (bottom right) */}
      <div
        className="absolute -bottom-48 -right-48 w-96 h-96 bg-violet-500/10 rounded-full blur-3xl animate-slow-pulse"
        style={{ animationDelay: '4s' }}
        aria-hidden="true"
      />

      {/* Secondary Orb - Teal (center left) */}
      <div
        className="absolute top-1/2 -left-24 w-64 h-64 bg-teal-500/5 rounded-full blur-3xl animate-slow-pulse"
        style={{ animationDelay: '2s' }}
        aria-hidden="true"
      />

      {/* Secondary Orb - Fuchsia (top right) */}
      <div
        className="absolute -top-24 right-1/4 w-48 h-48 bg-fuchsia-500/5 rounded-full blur-3xl animate-slow-pulse"
        style={{ animationDelay: '6s' }}
        aria-hidden="true"
      />

      {/* Grid Pattern Overlay */}
      <div
        className="absolute inset-0 opacity-30"
        style={{
          backgroundImage: `
            linear-gradient(rgba(255, 255, 255, 0.02) 1px, transparent 1px),
            linear-gradient(90deg, rgba(255, 255, 255, 0.02) 1px, transparent 1px)
          `,
          backgroundSize: '60px 60px',
        }}
        aria-hidden="true"
      />

      {/* Radial gradient overlay for depth */}
      <div
        className="absolute inset-0 bg-gradient-to-b from-transparent via-slate-950/50 to-slate-950"
        aria-hidden="true"
      />
    </div>
  )
}
