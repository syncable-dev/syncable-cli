/**
 * Agent Chat Route
 *
 * Demonstrates CopilotKit integration with syncable-cli AG-UI server.
 * Uses CopilotKit's built-in chat components for agent conversations.
 */
import { createFileRoute } from "@tanstack/react-router";
import { useState, useCallback, FormEvent } from "react";
import { useCopilotChat } from "@copilotkit/react-core";
import { MessageCircle, Send, Loader2, Bot, User, Terminal } from "lucide-react";

export const Route = createFileRoute("/agent")({
  component: AgentChat,
});

function AgentChat() {
  const [input, setInput] = useState("");

  const {
    visibleMessages,
    appendMessage,
    isLoading,
  } = useCopilotChat();

  const handleSubmit = useCallback(
    async (e: FormEvent) => {
      e.preventDefault();
      if (!input.trim() || isLoading) return;

      const message = input.trim();
      setInput("");

      // Append user message and trigger agent response
      await appendMessage({
        id: crypto.randomUUID(),
        role: "user",
        content: message,
      });
    },
    [input, isLoading, appendMessage]
  );

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
            <h1 className="text-4xl font-bold tracking-tight bg-gradient-to-r from-emerald-400 via-cyan-400 to-blue-400 bg-clip-text text-transparent">
              Agent Chat
            </h1>
          </div>
          <p className="text-slate-400 max-w-md text-base leading-relaxed">
            Chat with the Syncable agent via AG-UI protocol.
            Messages are processed by the AG-UI server and streamed back in real-time.
          </p>
        </header>

        {/* Chat Container */}
        <div className="bg-slate-900/50 border border-slate-800 rounded-2xl overflow-hidden">
          {/* Messages Area */}
          <div className="h-[500px] overflow-y-auto p-4 space-y-4">
            {visibleMessages.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-slate-500">
                <MessageCircle className="w-12 h-12 mb-4 opacity-50" />
                <p className="text-sm">No messages yet. Start a conversation!</p>
              </div>
            ) : (
              visibleMessages.map((message) => (
                <div
                  key={message.id}
                  className={`flex gap-3 ${
                    message.role === "user" ? "justify-end" : "justify-start"
                  }`}
                >
                  {message.role !== "user" && (
                    <div className="flex-shrink-0 w-8 h-8 rounded-lg bg-emerald-500/20 flex items-center justify-center">
                      <Bot className="w-4 h-4 text-emerald-400" />
                    </div>
                  )}
                  <div
                    className={`max-w-[80%] rounded-2xl px-4 py-3 ${
                      message.role === "user"
                        ? "bg-cyan-600/20 border border-cyan-500/30 text-cyan-100"
                        : "bg-slate-800/50 border border-slate-700/50 text-slate-200"
                    }`}
                  >
                    <p className="text-sm whitespace-pre-wrap">{message.content}</p>
                  </div>
                  {message.role === "user" && (
                    <div className="flex-shrink-0 w-8 h-8 rounded-lg bg-cyan-500/20 flex items-center justify-center">
                      <User className="w-4 h-4 text-cyan-400" />
                    </div>
                  )}
                </div>
              ))
            )}

            {/* Loading indicator */}
            {isLoading && (
              <div className="flex gap-3 justify-start">
                <div className="flex-shrink-0 w-8 h-8 rounded-lg bg-emerald-500/20 flex items-center justify-center">
                  <Bot className="w-4 h-4 text-emerald-400" />
                </div>
                <div className="bg-slate-800/50 border border-slate-700/50 rounded-2xl px-4 py-3">
                  <div className="flex items-center gap-2 text-slate-400">
                    <Loader2 className="w-4 h-4 animate-spin" />
                    <span className="text-sm">Agent is thinking...</span>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Input Area */}
          <form onSubmit={handleSubmit} className="border-t border-slate-800 p-4">
            <div className="flex gap-3">
              <input
                type="text"
                value={input}
                onChange={(e) => setInput(e.target.value)}
                placeholder="Type your message..."
                className="flex-1 bg-slate-800/50 border border-slate-700 rounded-xl px-4 py-3 text-slate-100 placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-cyan-500/50 focus:border-cyan-500/50 transition-all"
                disabled={isLoading}
              />
              <button
                type="submit"
                disabled={!input.trim() || isLoading}
                className="px-6 py-3 rounded-xl bg-gradient-to-r from-emerald-500 to-cyan-500 text-white font-medium shadow-lg shadow-emerald-500/25 hover:shadow-emerald-500/40 hover:scale-105 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 transition-all duration-200 flex items-center gap-2"
              >
                {isLoading ? (
                  <Loader2 className="w-5 h-5 animate-spin" />
                ) : (
                  <Send className="w-5 h-5" />
                )}
              </button>
            </div>
          </form>
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
