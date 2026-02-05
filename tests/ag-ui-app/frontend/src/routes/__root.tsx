import { HeadContent, Scripts, createRootRoute, Link } from '@tanstack/react-router'
import appCss from '../styles.css?url'
import { CopilotKitWrapper } from '../lib/copilotkit'

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: 'utf-8' },
      { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      { title: 'Smart Reply Generator' },
      { name: 'description', content: 'AI-powered reply suggestions for your messages' },
    ],
    links: [
      { rel: 'stylesheet', href: appCss },
      { rel: 'icon', href: '/favicon.ico' },
    ],
  }),
  shellComponent: RootDocument,
})

function RootDocument({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <head>
        <HeadContent />
      </head>
      <body className="bg-slate-950 antialiased">
        {/* Navigation */}
        <nav className="fixed top-4 right-4 z-50 flex gap-2">
          <Link
            to="/"
            className="px-3 py-1.5 text-xs font-medium rounded-lg bg-slate-800/80 text-slate-300 hover:bg-slate-700 hover:text-white border border-slate-700 backdrop-blur-sm transition-all"
            activeProps={{ className: 'bg-cyan-600/20 text-cyan-400 border-cyan-500/30' }}
          >
            Smart Reply
          </Link>
          <Link
            to="/agent"
            className="px-3 py-1.5 text-xs font-medium rounded-lg bg-slate-800/80 text-slate-300 hover:bg-slate-700 hover:text-white border border-slate-700 backdrop-blur-sm transition-all"
            activeProps={{ className: 'bg-emerald-600/20 text-emerald-400 border-emerald-500/30' }}
          >
            Agent Chat
          </Link>
        </nav>
        <CopilotKitWrapper>
          {children}
        </CopilotKitWrapper>
        <Scripts />
      </body>
    </html>
  )
}
