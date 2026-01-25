/**
 * Agent Chat Route
 *
 * Demonstrates CopilotKit integration with syncable-cli AG-UI server.
 * Uses CopilotKit's built-in CopilotChat component for conversations.
 */
import { createFileRoute } from "@tanstack/react-router";
import { CopilotChat } from "@copilotkit/react-ui";
import { Bot, Terminal } from "lucide-react";

// Import CopilotKit styles
import "@copilotkit/react-ui/styles.css";

export const Route = createFileRoute("/agent")({
  component: AgentChat,
});

function AgentChat() {
  return (
    <main className="min-h-screen bg-slate-950 relative overflow-hidden">
      {/* Background */}
      <div className="absolute inset-0 bg-linear-to-br from-slate-950 via-slate-900 to-slate-950" />
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top,rgba(34,211,238,0.1),transparent_50%)]" />

      <div className="relative z-10 max-w-3xl mx-auto px-4 sm:px-6 py-12">
        {/* Header */}
        <header className="flex flex-col items-center text-center mb-8">
          <div className="flex items-center gap-3 mb-4">
            <div className="p-3 rounded-2xl bg-linear-to-br from-emerald-500/20 to-cyan-600/20 border border-emerald-500/30 shadow-[0_0_30px_rgba(16,185,129,0.15)]">
              <Bot className="w-8 h-8 text-emerald-400" />
            </div>
            <h1 className="text-4xl font-bold tracking-tight bg-linear-to-r from-emerald-400 via-cyan-400 to-blue-400 bg-clip-text text-transparent">
              Agent Chat
            </h1>
          </div>
          <p className="text-slate-400 max-w-md text-base leading-relaxed">
            Chat with the Syncable agent via AG-UI protocol.
            Messages are processed by the AG-UI server and streamed back in real-time.
          </p>
        </header>

        {/* Chat Container */}
        <div className="bg-slate-900/50 border border-slate-800 rounded-2xl overflow-hidden h-[500px]">
          <CopilotChat
            className="h-full"
            labels={{
              title: "Syncable Agent",
              initial: "Hi! I'm the Syncable agent. How can I help you today?",
              placeholder: "Type your message...",
            }}
          />
        </div>

        {/* Connection Info */}
        <div className="mt-6 p-4 bg-slate-900/30 border border-slate-800/50 rounded-xl">
          <div className="flex items-center gap-2 text-slate-400 text-sm">
            <Terminal className="w-4 h-4" />
            <span>AG-UI Server: </span>
            <code className="px-2 py-0.5 bg-slate-800 rounded text-emerald-400 text-xs">
              {import.meta.env.VITE_AGENT_URL || "http://localhost:9090"}
            </code>
          </div>
          <p className="mt-2 text-xs text-slate-500">
            Messages are sent via POST /message and responses streamed via SSE/WebSocket.
          </p>
        </div>
      </div>
    </main>
  );
}
