import { jsxs, jsx, Fragment } from "react/jsx-runtime";
import { useState, useCallback, useEffect } from "react";
import { History, MessageSquare, Lightbulb, Briefcase, Smile, Heart, Zap, Minus, Loader2, Square, Send, Check, Copy, ChevronRight, Sparkles, Trash2, Clock, X, RefreshCw } from "lucide-react";
import { T as TSS_SERVER_FUNCTION, g as getServerFnById, a as createServerFn } from "../server.js";
import "@tanstack/history";
import "@tanstack/router-core/ssr/client";
import "@tanstack/router-core";
import "node:async_hooks";
import "@tanstack/router-core/ssr/server";
import "h3-v2";
import "tiny-invariant";
import "seroval";
import "@tanstack/react-router/ssr/server";
import "@tanstack/react-router";
function BackgroundEffects() {
  return /* @__PURE__ */ jsxs("div", { className: "absolute inset-0 pointer-events-none overflow-hidden", children: [
    /* @__PURE__ */ jsx(
      "div",
      {
        className: "absolute -top-48 -left-48 w-96 h-96 bg-cyan-500/10 rounded-full blur-3xl animate-slow-pulse",
        "aria-hidden": "true"
      }
    ),
    /* @__PURE__ */ jsx(
      "div",
      {
        className: "absolute -bottom-48 -right-48 w-96 h-96 bg-violet-500/10 rounded-full blur-3xl animate-slow-pulse",
        style: { animationDelay: "4s" },
        "aria-hidden": "true"
      }
    ),
    /* @__PURE__ */ jsx(
      "div",
      {
        className: "absolute top-1/2 -left-24 w-64 h-64 bg-teal-500/5 rounded-full blur-3xl animate-slow-pulse",
        style: { animationDelay: "2s" },
        "aria-hidden": "true"
      }
    ),
    /* @__PURE__ */ jsx(
      "div",
      {
        className: "absolute -top-24 right-1/4 w-48 h-48 bg-fuchsia-500/5 rounded-full blur-3xl animate-slow-pulse",
        style: { animationDelay: "6s" },
        "aria-hidden": "true"
      }
    ),
    /* @__PURE__ */ jsx(
      "div",
      {
        className: "absolute inset-0 opacity-30",
        style: {
          backgroundImage: `
            linear-gradient(rgba(255, 255, 255, 0.02) 1px, transparent 1px),
            linear-gradient(90deg, rgba(255, 255, 255, 0.02) 1px, transparent 1px)
          `,
          backgroundSize: "60px 60px"
        },
        "aria-hidden": "true"
      }
    ),
    /* @__PURE__ */ jsx(
      "div",
      {
        className: "absolute inset-0 bg-gradient-to-b from-transparent via-slate-950/50 to-slate-950",
        "aria-hidden": "true"
      }
    )
  ] });
}
function MessageInputCard({
  message,
  context,
  intent,
  onMessageChange,
  onContextChange,
  onIntentChange,
  disabled = false
}) {
  return /* @__PURE__ */ jsxs("div", { className: "space-y-5 animate-glass-reveal", children: [
    /* @__PURE__ */ jsxs("div", { className: "bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-5 transition-all duration-300 hover:bg-white/[0.07] hover:border-white/15", children: [
      /* @__PURE__ */ jsxs(
        "label",
        {
          htmlFor: "context-input",
          className: "flex items-center gap-2 text-sm font-medium text-slate-300 mb-3",
          children: [
            /* @__PURE__ */ jsx(History, { className: "w-4 h-4 text-violet-400" }),
            "Conversation context",
            /* @__PURE__ */ jsx("span", { className: "text-slate-500 font-normal", children: "(optional)" })
          ]
        }
      ),
      /* @__PURE__ */ jsx(
        "textarea",
        {
          id: "context-input",
          value: context,
          onChange: (e) => onContextChange(e.target.value),
          disabled,
          placeholder: "Provide background or previous messages in the conversation...\n\ne.g., 'We've been discussing the Q3 budget. They initially proposed $50k but I countered with $35k. This is their response to my counter-offer.'",
          rows: 3,
          className: "\n            w-full bg-slate-800/60 backdrop-blur-sm border border-white/10 rounded-xl p-4\n            text-slate-100 text-[15px] leading-relaxed placeholder:text-slate-500 resize-none\n            focus:outline-none focus:border-violet-500/50 focus:shadow-[0_0_30px_rgba(139,92,246,0.1)]\n            transition-all duration-200\n            disabled:opacity-50 disabled:cursor-not-allowed\n          "
        }
      )
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-5 transition-all duration-300 hover:bg-white/[0.07] hover:border-white/15", children: [
      /* @__PURE__ */ jsxs(
        "label",
        {
          htmlFor: "message-input",
          className: "flex items-center gap-2 text-sm font-medium text-slate-300 mb-3",
          children: [
            /* @__PURE__ */ jsx(MessageSquare, { className: "w-4 h-4 text-cyan-400" }),
            "Message you received",
            /* @__PURE__ */ jsx("span", { className: "text-rose-400", children: "*" })
          ]
        }
      ),
      /* @__PURE__ */ jsx(
        "textarea",
        {
          id: "message-input",
          value: message,
          onChange: (e) => onMessageChange(e.target.value),
          disabled,
          placeholder: "Paste the message you need to reply to...\n\ne.g., 'Hi, I wanted to follow up on our meeting yesterday. When would be a good time to reschedule?'",
          rows: 4,
          className: "\n            w-full bg-slate-800/60 backdrop-blur-sm border border-white/10 rounded-xl p-4\n            text-slate-100 text-[15px] leading-relaxed placeholder:text-slate-500 resize-none\n            focus:outline-none focus:border-cyan-500/50 focus:shadow-[0_0_30px_rgba(34,211,238,0.1)]\n            transition-all duration-200\n            disabled:opacity-50 disabled:cursor-not-allowed\n          "
        }
      )
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-5 transition-all duration-300 hover:bg-white/[0.07] hover:border-white/15", children: [
      /* @__PURE__ */ jsxs(
        "label",
        {
          htmlFor: "intent-input",
          className: "flex items-center gap-2 text-sm font-medium text-slate-300 mb-3",
          children: [
            /* @__PURE__ */ jsx(Lightbulb, { className: "w-4 h-4 text-amber-400" }),
            "What you want to say",
            /* @__PURE__ */ jsx("span", { className: "text-slate-500 font-normal", children: "(optional)" })
          ]
        }
      ),
      /* @__PURE__ */ jsx(
        "textarea",
        {
          id: "intent-input",
          value: intent,
          onChange: (e) => onIntentChange(e.target.value),
          disabled,
          placeholder: "Describe what you want to communicate in your reply...\n\ne.g., 'I want to accept their offer but negotiate a faster timeline. Also mention I need the contract by Friday.'",
          rows: 3,
          className: "\n            w-full bg-slate-800/60 backdrop-blur-sm border border-white/10 rounded-xl p-4\n            text-slate-100 text-[15px] leading-relaxed placeholder:text-slate-500 resize-none\n            focus:outline-none focus:border-amber-500/50 focus:shadow-[0_0_30px_rgba(245,158,11,0.1)]\n            transition-all duration-200\n            disabled:opacity-50 disabled:cursor-not-allowed\n          "
        }
      )
    ] })
  ] });
}
const TONES = [
  {
    value: "professional",
    label: "Professional",
    icon: /* @__PURE__ */ jsx(Briefcase, { className: "w-4 h-4" }),
    description: "Formal, business-appropriate"
  },
  {
    value: "friendly",
    label: "Friendly",
    icon: /* @__PURE__ */ jsx(Smile, { className: "w-4 h-4" }),
    description: "Warm and personable"
  },
  {
    value: "apologetic",
    label: "Apologetic",
    icon: /* @__PURE__ */ jsx(Heart, { className: "w-4 h-4" }),
    description: "Sincere and understanding"
  },
  {
    value: "assertive",
    label: "Assertive",
    icon: /* @__PURE__ */ jsx(Zap, { className: "w-4 h-4" }),
    description: "Direct and confident"
  },
  {
    value: "neutral",
    label: "Neutral",
    icon: /* @__PURE__ */ jsx(Minus, { className: "w-4 h-4" }),
    description: "Balanced and objective"
  }
];
function ToneSelector({ selected, onChange, disabled = false }) {
  return /* @__PURE__ */ jsxs("div", { className: "space-y-3 animate-fade-in-up", style: { animationDelay: "100ms" }, children: [
    /* @__PURE__ */ jsx("label", { className: "text-sm font-medium text-slate-300", children: "Select tone" }),
    /* @__PURE__ */ jsx(
      "div",
      {
        className: "flex flex-wrap gap-2",
        role: "radiogroup",
        "aria-label": "Reply tone selection",
        children: TONES.map((tone) => {
          const isSelected = selected === tone.value;
          return /* @__PURE__ */ jsxs(
            "button",
            {
              onClick: () => onChange(tone.value),
              disabled,
              role: "radio",
              "aria-checked": isSelected,
              title: tone.description,
              className: `
                group flex items-center gap-2 px-4 py-2.5 rounded-xl text-sm font-medium
                transition-all duration-200 border cursor-pointer
                ${isSelected ? "bg-gradient-to-r from-cyan-500/20 to-violet-500/20 border-cyan-500/40 text-cyan-300 shadow-[0_0_20px_rgba(34,211,238,0.15)]" : "bg-white/5 border-white/10 text-slate-400 hover:bg-white/10 hover:border-white/20 hover:text-slate-200"}
                disabled:opacity-50 disabled:cursor-not-allowed
                focus:outline-none focus:ring-2 focus:ring-cyan-500/50 focus:ring-offset-2 focus:ring-offset-slate-950
              `,
              children: [
                /* @__PURE__ */ jsx(
                  "span",
                  {
                    className: `transition-all duration-200 ${isSelected ? "text-cyan-400 scale-110" : "text-slate-500 group-hover:text-slate-300 group-hover:scale-110"}`,
                    children: tone.icon
                  }
                ),
                /* @__PURE__ */ jsx("span", { children: tone.label })
              ]
            },
            tone.value
          );
        })
      }
    )
  ] });
}
function GenerateButton({
  onClick,
  onStop,
  isLoading = false,
  disabled = false
}) {
  const handleClick = () => {
    if (isLoading && onStop) {
      onStop();
    } else {
      onClick();
    }
  };
  return /* @__PURE__ */ jsxs(
    "button",
    {
      onClick: handleClick,
      disabled: disabled && !isLoading,
      className: `
        group relative flex items-center justify-center gap-3 px-8 py-4 rounded-2xl font-semibold text-base
        transition-all duration-300 overflow-hidden cursor-pointer
        ${isLoading ? "bg-slate-800/80 border border-cyan-500/40 text-cyan-300 animate-glow-pulse" : "bg-gradient-to-r from-cyan-500 to-violet-600 text-white shadow-lg shadow-cyan-500/25 hover:shadow-[0_0_40px_rgba(34,211,238,0.4)] hover:from-cyan-400 hover:to-violet-500"}
        ${!isLoading && !disabled ? "transform hover:scale-[1.03] active:scale-[0.98]" : ""}
        disabled:opacity-40 disabled:cursor-not-allowed disabled:transform-none disabled:shadow-none
        focus:outline-none focus:ring-2 focus:ring-cyan-500/50 focus:ring-offset-2 focus:ring-offset-slate-950
      `,
      "aria-label": isLoading ? "Stop generating" : "Generate reply suggestions",
      children: [
        !isLoading && !disabled && /* @__PURE__ */ jsx("div", { className: "absolute inset-0 -translate-x-full group-hover:translate-x-full transition-transform duration-700 bg-gradient-to-r from-transparent via-white/20 to-transparent" }),
        /* @__PURE__ */ jsx("span", { className: "relative flex items-center gap-3", children: isLoading ? /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx(Loader2, { className: "w-5 h-5 animate-spin" }),
          /* @__PURE__ */ jsx("span", { children: "Generating..." }),
          onStop && /* @__PURE__ */ jsx(Square, { className: "w-4 h-4 ml-1 opacity-70" })
        ] }) : /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx(Send, { className: "w-5 h-5 transition-transform duration-200 group-hover:translate-x-0.5" }),
          /* @__PURE__ */ jsx("span", { children: "Generate Replies" })
        ] }) })
      ]
    }
  );
}
const TONE_BADGE_STYLES = {
  professional: "bg-blue-500/10 text-blue-300 border-blue-500/20",
  friendly: "bg-emerald-500/10 text-emerald-300 border-emerald-500/20",
  apologetic: "bg-amber-500/10 text-amber-300 border-amber-500/20",
  assertive: "bg-rose-500/10 text-rose-300 border-rose-500/20",
  neutral: "bg-slate-500/10 text-slate-300 border-slate-500/20"
};
function ReplyCard({
  reply,
  tone,
  isCopied,
  onCopy,
  onSelect,
  animationDelay = 0
}) {
  return /* @__PURE__ */ jsxs(
    "div",
    {
      className: "group bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-6\n        hover:bg-white/[0.08] hover:border-cyan-500/30 hover:shadow-[0_0_40px_rgba(34,211,238,0.1)]\n        cursor-pointer transition-all duration-300 animate-fade-in-up",
      style: {
        animationDelay: `${animationDelay}ms`,
        animationFillMode: "both"
      },
      onClick: onSelect,
      role: "button",
      tabIndex: 0,
      onKeyDown: (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect();
        }
      },
      "aria-label": `Reply option ${reply.reply_index + 1}. ${reply.content}`,
      children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between mb-4", children: [
          /* @__PURE__ */ jsxs(
            "span",
            {
              className: `px-3 py-1.5 rounded-full text-xs font-medium border ${TONE_BADGE_STYLES[tone]}`,
              children: [
                "Option ",
                reply.reply_index + 1
              ]
            }
          ),
          /* @__PURE__ */ jsx(
            "button",
            {
              onClick: (e) => {
                e.stopPropagation();
                onCopy();
              },
              className: "flex items-center gap-2 px-3 py-1.5 rounded-lg cursor-pointer\n            bg-white/5 hover:bg-white/10\n            text-slate-400 hover:text-slate-200\n            opacity-0 group-hover:opacity-100\n            transition-all duration-200\n            focus:outline-none focus:ring-2 focus:ring-cyan-500/50 focus:opacity-100",
              "aria-label": isCopied ? "Copied to clipboard" : "Copy to clipboard",
              children: isCopied ? /* @__PURE__ */ jsxs(Fragment, { children: [
                /* @__PURE__ */ jsx(Check, { className: "w-4 h-4 text-emerald-400" }),
                /* @__PURE__ */ jsx("span", { className: "text-xs font-medium text-emerald-400", children: "Copied!" })
              ] }) : /* @__PURE__ */ jsxs(Fragment, { children: [
                /* @__PURE__ */ jsx(Copy, { className: "w-4 h-4" }),
                /* @__PURE__ */ jsx("span", { className: "text-xs font-medium", children: "Copy" })
              ] })
            }
          )
        ] }),
        /* @__PURE__ */ jsx("p", { className: "text-slate-100 leading-relaxed mb-5 whitespace-pre-wrap text-[15px]", children: reply.content }),
        /* @__PURE__ */ jsx("div", { className: "flex justify-end", children: /* @__PURE__ */ jsxs(
          "button",
          {
            onClick: (e) => {
              e.stopPropagation();
              onSelect();
            },
            className: "flex items-center gap-1.5 text-sm font-medium text-cyan-400 hover:text-cyan-300\n            transition-all duration-200 cursor-pointer group/btn",
            children: [
              /* @__PURE__ */ jsx("span", { children: "Use this reply" }),
              /* @__PURE__ */ jsx(ChevronRight, { className: "w-4 h-4 transition-transform duration-200 group-hover/btn:translate-x-1" })
            ]
          }
        ) })
      ]
    }
  );
}
function ReplyCardSkeleton({ delay = 0 }) {
  return /* @__PURE__ */ jsxs(
    "div",
    {
      className: "bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-6 animate-fade-in-up",
      style: { animationDelay: `${delay}ms`, animationFillMode: "both" },
      children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between mb-4", children: [
          /* @__PURE__ */ jsx("div", { className: "h-7 w-20 bg-slate-700/50 rounded-full animate-shimmer" }),
          /* @__PURE__ */ jsx("div", { className: "h-8 w-16 bg-slate-700/50 rounded-lg animate-shimmer" })
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "space-y-3 mb-5", children: [
          /* @__PURE__ */ jsx("div", { className: "h-4 bg-slate-700/50 rounded-lg w-full animate-shimmer" }),
          /* @__PURE__ */ jsx("div", { className: "h-4 bg-slate-700/50 rounded-lg w-11/12 animate-shimmer" }),
          /* @__PURE__ */ jsx("div", { className: "h-4 bg-slate-700/50 rounded-lg w-4/5 animate-shimmer" })
        ] }),
        /* @__PURE__ */ jsx("div", { className: "flex justify-end", children: /* @__PURE__ */ jsx("div", { className: "h-5 w-28 bg-slate-700/50 rounded-lg animate-shimmer" }) })
      ]
    }
  );
}
function ReplyOptionsSection({
  replies,
  isLoading,
  tone,
  copiedId,
  onCopy,
  onSelect,
  streamedContent
}) {
  if (isLoading) {
    return /* @__PURE__ */ jsxs("section", { "aria-live": "polite", "aria-busy": "true", className: "animate-fade-in-up", children: [
      /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2 mb-4", children: [
        /* @__PURE__ */ jsx(Sparkles, { className: "w-5 h-5 text-cyan-400 animate-pulse" }),
        /* @__PURE__ */ jsx("h2", { className: "text-xl font-semibold text-slate-100", children: "Generating replies..." })
      ] }),
      streamedContent && /* @__PURE__ */ jsxs("div", { className: "mb-4 p-4 bg-slate-800/50 rounded-xl border border-cyan-500/20", children: [
        /* @__PURE__ */ jsx("p", { className: "text-sm text-slate-400 mb-2", children: "AI is thinking..." }),
        /* @__PURE__ */ jsxs("p", { className: "text-slate-300 text-sm font-mono whitespace-pre-wrap", children: [
          streamedContent,
          /* @__PURE__ */ jsx("span", { className: "inline-block w-2 h-4 bg-cyan-400 ml-1 animate-pulse" })
        ] })
      ] }),
      /* @__PURE__ */ jsx("div", { className: "flex flex-col gap-4", children: [0, 1, 2].map((index) => /* @__PURE__ */ jsx(ReplyCardSkeleton, { delay: index * 100 }, index)) })
    ] });
  }
  if (replies.length === 0) {
    return /* @__PURE__ */ jsxs("section", { className: "flex flex-col items-center justify-center py-16 text-center animate-fade-in-up", children: [
      /* @__PURE__ */ jsx("div", { className: "w-20 h-20 rounded-2xl bg-gradient-to-br from-white/5 to-white/[0.02] border border-white/10 flex items-center justify-center mb-5", children: /* @__PURE__ */ jsx(MessageSquare, { className: "w-10 h-10 text-slate-500" }) }),
      /* @__PURE__ */ jsx("h3", { className: "text-xl font-medium text-slate-300 mb-2", children: "No replies yet" }),
      /* @__PURE__ */ jsxs("p", { className: "text-sm text-slate-500 max-w-sm leading-relaxed", children: [
        "Paste a message above, select your preferred tone, and click",
        /* @__PURE__ */ jsx("span", { className: "text-cyan-400 font-medium", children: ' "Generate Replies" ' }),
        "to get AI-powered suggestions."
      ] }),
      /* @__PURE__ */ jsx("div", { className: "flex gap-2 mt-6", children: [0, 1, 2].map((i) => /* @__PURE__ */ jsx(
        "div",
        {
          className: "w-2 h-2 rounded-full bg-slate-700",
          style: { opacity: 0.3 + i * 0.2 }
        },
        i
      )) })
    ] });
  }
  return /* @__PURE__ */ jsxs("section", { "aria-live": "polite", children: [
    /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between mb-4", children: [
      /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ jsx(Sparkles, { className: "w-5 h-5 text-cyan-400" }),
        /* @__PURE__ */ jsxs("h2", { className: "text-xl font-semibold text-slate-100", children: [
          replies.length,
          " reply suggestion",
          replies.length !== 1 ? "s" : ""
        ] })
      ] }),
      /* @__PURE__ */ jsx("span", { className: "text-xs text-slate-500 bg-white/5 px-3 py-1 rounded-full", children: "Click to copy" })
    ] }),
    /* @__PURE__ */ jsx("div", { className: "flex flex-col gap-4", children: replies.map((reply, index) => /* @__PURE__ */ jsx(
      ReplyCard,
      {
        reply,
        tone,
        isCopied: copiedId === reply.id,
        onCopy: () => onCopy(reply.id, reply.content),
        onSelect: () => onSelect(reply.id, reply.content),
        animationDelay: index * 100
      },
      reply.id
    )) })
  ] });
}
const TONE_COLORS = {
  professional: "bg-blue-500/20 text-blue-300",
  friendly: "bg-emerald-500/20 text-emerald-300",
  apologetic: "bg-amber-500/20 text-amber-300",
  assertive: "bg-rose-500/20 text-rose-300",
  neutral: "bg-slate-500/20 text-slate-300"
};
function formatRelativeTime(dateString) {
  const date = new Date(dateString);
  const now = /* @__PURE__ */ new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 6e4);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);
  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}
function truncate(text, maxLength) {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength).trim() + "...";
}
function HistoryItem({
  conversation,
  onClick,
  onDelete,
  animationDelay = 0
}) {
  const replyCount = conversation.replies?.length || 0;
  return /* @__PURE__ */ jsxs(
    "div",
    {
      className: "group relative bg-white/5 hover:bg-white/10 border border-white/10 hover:border-white/20\n        rounded-xl p-3 cursor-pointer transition-all duration-200 animate-fade-in-up",
      style: { animationDelay: `${animationDelay}ms`, animationFillMode: "both" },
      onClick,
      role: "button",
      tabIndex: 0,
      onKeyDown: (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick();
        }
      },
      children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between mb-2", children: [
          /* @__PURE__ */ jsx(
            "span",
            {
              className: `px-2 py-0.5 rounded text-xs font-medium ${TONE_COLORS[conversation.tone]}`,
              children: conversation.tone
            }
          ),
          /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2", children: [
            /* @__PURE__ */ jsx("span", { className: "text-xs text-slate-500", children: formatRelativeTime(conversation.created_at) }),
            /* @__PURE__ */ jsx(
              "button",
              {
                onClick: (e) => {
                  e.stopPropagation();
                  onDelete();
                },
                className: "p-1.5 rounded-lg opacity-0 group-hover:opacity-100 hover:bg-rose-500/20\n              text-slate-500 hover:text-rose-400 transition-all duration-200 cursor-pointer\n              focus:outline-none focus:opacity-100 focus:ring-2 focus:ring-rose-500/50",
                "aria-label": "Delete conversation",
                children: /* @__PURE__ */ jsx(Trash2, { className: "w-3.5 h-3.5" })
              }
            )
          ] })
        ] }),
        /* @__PURE__ */ jsx("p", { className: "text-sm text-slate-300 leading-relaxed mb-2", children: truncate(conversation.original_message, 80) }),
        /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-1.5 text-xs text-slate-500", children: [
            /* @__PURE__ */ jsx(MessageSquare, { className: "w-3.5 h-3.5" }),
            /* @__PURE__ */ jsxs("span", { children: [
              replyCount,
              " repl",
              replyCount !== 1 ? "ies" : "y"
            ] })
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-1 text-xs text-cyan-400 opacity-0 group-hover:opacity-100 transition-opacity", children: [
            /* @__PURE__ */ jsx("span", { children: "Load" }),
            /* @__PURE__ */ jsx(ChevronRight, { className: "w-3.5 h-3.5" })
          ] })
        ] })
      ]
    }
  );
}
function HistoryDrawer({
  isOpen,
  onClose,
  history,
  isLoading,
  onSelectConversation,
  onDeleteConversation
}) {
  if (!isOpen) return null;
  return /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsx(
      "div",
      {
        className: "fixed inset-0 bg-black/70 backdrop-blur-sm z-40 animate-backdrop-fade-in cursor-pointer",
        onClick: onClose,
        "aria-hidden": "true"
      }
    ),
    /* @__PURE__ */ jsxs(
      "aside",
      {
        className: "fixed top-0 right-0 h-full w-full sm:w-[400px] bg-slate-900/95 backdrop-blur-xl\n          border-l border-white/10 shadow-2xl z-50 flex flex-col animate-slide-in-right",
        role: "dialog",
        "aria-modal": "true",
        "aria-labelledby": "history-title",
        children: [
          /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between p-5 border-b border-white/10", children: [
            /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-3", children: [
              /* @__PURE__ */ jsx("div", { className: "p-2 rounded-xl bg-cyan-500/10 border border-cyan-500/20", children: /* @__PURE__ */ jsx(Clock, { className: "w-5 h-5 text-cyan-400" }) }),
              /* @__PURE__ */ jsx("h2", { id: "history-title", className: "text-lg font-semibold text-slate-100", children: "History" })
            ] }),
            /* @__PURE__ */ jsx(
              "button",
              {
                onClick: onClose,
                className: "p-2.5 rounded-xl hover:bg-white/10 text-slate-400 hover:text-slate-100\n              transition-all duration-200 cursor-pointer\n              focus:outline-none focus:ring-2 focus:ring-cyan-500/50",
                "aria-label": "Close history",
                children: /* @__PURE__ */ jsx(X, { className: "w-5 h-5" })
              }
            )
          ] }),
          /* @__PURE__ */ jsx("div", { className: "flex-1 overflow-y-auto custom-scrollbar p-4", children: isLoading ? /* @__PURE__ */ jsxs("div", { className: "flex flex-col items-center justify-center h-48 text-slate-400", children: [
            /* @__PURE__ */ jsx(Loader2, { className: "w-8 h-8 animate-spin mb-3 text-cyan-400" }),
            /* @__PURE__ */ jsx("span", { className: "text-sm", children: "Loading history..." })
          ] }) : history.length === 0 ? /* @__PURE__ */ jsxs("div", { className: "flex flex-col items-center justify-center h-48 text-center", children: [
            /* @__PURE__ */ jsx("div", { className: "w-16 h-16 rounded-2xl bg-white/5 border border-white/10 flex items-center justify-center mb-4", children: /* @__PURE__ */ jsx(Clock, { className: "w-8 h-8 text-slate-600" }) }),
            /* @__PURE__ */ jsx("p", { className: "text-slate-300 font-medium", children: "No history yet" }),
            /* @__PURE__ */ jsx("p", { className: "text-slate-500 text-sm mt-1", children: "Your generated replies will appear here" })
          ] }) : /* @__PURE__ */ jsx("div", { className: "space-y-3", children: history.map((conversation, index) => /* @__PURE__ */ jsx(
            HistoryItem,
            {
              conversation,
              onClick: () => onSelectConversation(conversation),
              onDelete: () => onDeleteConversation(conversation.id),
              animationDelay: index * 50
            },
            conversation.id
          )) }) }),
          history.length > 0 && /* @__PURE__ */ jsx("div", { className: "p-4 border-t border-white/10", children: /* @__PURE__ */ jsxs("p", { className: "text-xs text-slate-500 text-center", children: [
            history.length,
            " conversation",
            history.length !== 1 ? "s" : "",
            " saved"
          ] }) })
        ]
      }
    )
  ] });
}
const createSsrRpc = (functionId, importer) => {
  const url = "/_serverFn/" + functionId;
  const fn = async (...args) => {
    const serverFn = await getServerFnById(functionId);
    return serverFn(...args);
  };
  return Object.assign(fn, {
    url,
    functionId,
    [TSS_SERVER_FUNCTION]: true
  });
};
const getApiBase = createServerFn({
  method: "GET"
}).handler(createSsrRpc("cc35041ac8534be1df357d50dcecc9c79d69232d0d2da0a89a0d930737c7ec36"));
const api = {
  /**
   * Generate smart replies with SSE streaming
   * Returns a fetch Response that can be streamed
   */
  generateReplies: async (message, tone, context, intent, signal) => {
    return fetch(`${await getApiBase()}/replies/generate`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        message,
        tone,
        context,
        intent
      }),
      signal
    });
  },
  /**
   * Get conversation history
   */
  getHistory: async (limit = 50) => {
    const res = await fetch(`${await getApiBase()}/history?limit=${limit}`);
    if (!res.ok) {
      throw new Error("Failed to fetch history");
    }
    const data = await res.json();
    return data.data;
  },
  /**
   * Get a single conversation by ID
   */
  getConversation: async (id) => {
    const res = await fetch(`${await getApiBase()}/history/${id}`);
    if (!res.ok) {
      throw new Error("Conversation not found");
    }
    const data = await res.json();
    return data.data;
  },
  /**
   * Delete a conversation
   */
  deleteConversation: async (id) => {
    const res = await fetch(`${await getApiBase()}/history/${id}`, {
      method: "DELETE"
    });
    if (!res.ok) {
      throw new Error("Failed to delete conversation");
    }
  },
  /**
   * Save generated replies to database
   */
  saveReplies: async (conversationId, replies) => {
    const res = await fetch(`${await getApiBase()}/replies/save`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        conversationId,
        replies
      })
    });
    if (!res.ok) {
      throw new Error("Failed to save replies");
    }
  },
  /**
   * Health check
   */
  healthCheck: async () => {
    try {
      const res = await fetch(`${await getApiBase()}.replace('/api', '')}/health`);
      return res.ok;
    } catch {
      return false;
    }
  }
};
function useSmartReply() {
  const [isGenerating, setIsGenerating] = useState(false);
  const [streamedContent, setStreamedContent] = useState("");
  const [replies, setReplies] = useState([]);
  const [conversationId, setConversationId] = useState(null);
  const [error, setError] = useState(null);
  const [abortController, setAbortController] = useState(null);
  const generate = useCallback(async (message, tone, context, intent) => {
    setIsGenerating(true);
    setReplies([]);
    setConversationId(null);
    setStreamedContent("");
    setError(null);
    const controller = new AbortController();
    setAbortController(controller);
    try {
      const response = await api.generateReplies(message, tone, context, intent, controller.signal);
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }
      const convId = response.headers.get("X-Conversation-Id");
      if (convId) {
        setConversationId(convId);
      }
      const reader = response.body?.getReader();
      if (!reader) {
        throw new Error("No response body");
      }
      const decoder = new TextDecoder();
      let accumulated = "";
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        const text = decoder.decode(value, { stream: true });
        const lines = text.split("\n");
        for (const line of lines) {
          if (line.startsWith("data: ")) {
            const data = line.slice(6).trim();
            if (data === "[DONE]") continue;
            try {
              const chunk = JSON.parse(data);
              if (chunk.type === "text" && chunk.content) {
                accumulated += chunk.content;
                setStreamedContent(accumulated);
              } else if (chunk.type === "content" && chunk.content) {
                accumulated = chunk.content;
                setStreamedContent(accumulated);
              } else if (chunk.type === "error") {
                throw new Error(chunk.error?.message || "Stream error");
              }
            } catch (parseErr) {
              if (data && !data.startsWith("{")) {
                accumulated += data;
                setStreamedContent(accumulated);
              }
            }
          }
        }
      }
      const parsedReplies = parseReplies(accumulated);
      setReplies(parsedReplies);
      if (convId && parsedReplies.length > 0) {
        try {
          await api.saveReplies(convId, parsedReplies.map((r) => r.content));
        } catch (e) {
          console.error("Failed to save replies:", e);
        }
      }
    } catch (err) {
      if (err instanceof Error && err.name === "AbortError") {
        return;
      }
      const errorMessage = err instanceof Error ? err.message : "Failed to generate replies";
      setError(errorMessage);
      console.error("Generation error:", err);
    } finally {
      setIsGenerating(false);
      setAbortController(null);
    }
  }, []);
  const stop = useCallback(() => {
    if (abortController) {
      abortController.abort();
      setIsGenerating(false);
    }
  }, [abortController]);
  const reset = useCallback(() => {
    stop();
    setStreamedContent("");
    setReplies([]);
    setConversationId(null);
    setError(null);
  }, [stop]);
  return {
    isGenerating,
    streamedContent,
    replies,
    conversationId,
    error,
    generate,
    stop,
    reset,
    setReplies
  };
}
function parseReplies(content) {
  try {
    const jsonMatch = content.match(/\[[\s\S]*\]/);
    if (jsonMatch) {
      const parsed = JSON.parse(jsonMatch[0]);
      if (Array.isArray(parsed)) {
        return parsed.slice(0, 3).map((text, index) => ({
          id: `reply-${Date.now()}-${index}`,
          content: String(text),
          reply_index: index
        }));
      }
    }
  } catch (e) {
    console.error("Failed to parse replies:", e, content);
  }
  return [];
}
function useHistory() {
  const [history, setHistory] = useState([]);
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);
  const fetchHistory = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await api.getHistory(50);
      setHistory(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to fetch history";
      setError(message);
      console.error("History fetch error:", err);
    } finally {
      setIsLoading(false);
    }
  }, []);
  const loadConversation = useCallback(async (id) => {
    try {
      return await api.getConversation(id);
    } catch (err) {
      console.error("Load conversation error:", err);
      return null;
    }
  }, []);
  const deleteConversation = useCallback(async (id) => {
    try {
      await api.deleteConversation(id);
      setHistory((prev) => prev.filter((c) => c.id !== id));
    } catch (err) {
      console.error("Delete conversation error:", err);
      throw err;
    }
  }, []);
  const refreshHistory = fetchHistory;
  useEffect(() => {
    if (isOpen) {
      fetchHistory();
    }
  }, [isOpen, fetchHistory]);
  return {
    history,
    isOpen,
    isLoading,
    error,
    setIsOpen,
    fetchHistory,
    loadConversation,
    deleteConversation,
    refreshHistory
  };
}
function SmartReplyApp() {
  const [message, setMessage] = useState("");
  const [context, setContext] = useState("");
  const [intent, setIntent] = useState("");
  const [selectedTone, setSelectedTone] = useState("professional");
  const [copiedId, setCopiedId] = useState(null);
  const {
    isGenerating,
    streamedContent,
    replies,
    error,
    generate,
    stop,
    reset,
    setReplies
  } = useSmartReply();
  const {
    history,
    isOpen: isHistoryOpen,
    isLoading: isHistoryLoading,
    setIsOpen: setIsHistoryOpen,
    deleteConversation,
    refreshHistory
  } = useHistory();
  const handleGenerate = useCallback(async () => {
    if (!message.trim() || isGenerating) return;
    await generate(message, selectedTone, context, intent);
    refreshHistory();
  }, [message, selectedTone, context, intent, isGenerating, generate, refreshHistory]);
  const handleCopy = useCallback(async (replyId, content) => {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedId(replyId);
      setTimeout(() => setCopiedId(null), 2e3);
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  }, []);
  const handleSelect = useCallback(async (replyId, content) => {
    await handleCopy(replyId, content);
  }, [handleCopy]);
  const handleLoadConversation = useCallback((conversation) => {
    setMessage(conversation.original_message);
    setContext(conversation.context || "");
    setIntent(conversation.intent || "");
    setSelectedTone(conversation.tone);
    setReplies(conversation.replies);
    setIsHistoryOpen(false);
  }, [setReplies, setIsHistoryOpen]);
  const handleDeleteConversation = useCallback(async (id) => {
    try {
      await deleteConversation(id);
    } catch (err) {
      console.error("Failed to delete:", err);
    }
  }, [deleteConversation]);
  const handleReset = useCallback(() => {
    setMessage("");
    setContext("");
    setIntent("");
    reset();
  }, [reset]);
  return /* @__PURE__ */ jsxs("main", { className: "min-h-screen bg-slate-950 relative overflow-hidden", children: [
    /* @__PURE__ */ jsx(BackgroundEffects, {}),
    /* @__PURE__ */ jsxs("div", { className: "relative z-10 max-w-2xl mx-auto px-4 sm:px-6 py-12 sm:py-16", children: [
      /* @__PURE__ */ jsxs("header", { className: "flex flex-col items-center text-center mb-10 animate-fade-in-up", children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-3 mb-4", children: [
          /* @__PURE__ */ jsx("div", { className: "p-3 rounded-2xl bg-gradient-to-br from-cyan-500/20 to-violet-600/20 border border-cyan-500/30 shadow-[0_0_30px_rgba(34,211,238,0.15)]", children: /* @__PURE__ */ jsx(Sparkles, { className: "w-8 h-8 text-cyan-400" }) }),
          /* @__PURE__ */ jsx("h1", { className: "text-4xl sm:text-5xl font-bold tracking-tight bg-gradient-to-r from-cyan-400 via-blue-400 to-violet-400 bg-clip-text text-transparent", children: "Smart Reply" })
        ] }),
        /* @__PURE__ */ jsx("p", { className: "text-slate-400 max-w-md text-base sm:text-lg leading-relaxed", children: "Provide context and intent to get AI-powered reply suggestions tailored to your needs." }),
        /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-3 mt-6", children: [
          /* @__PURE__ */ jsxs("button", { onClick: () => setIsHistoryOpen(true), className: "flex items-center gap-2 px-4 py-2.5 rounded-xl bg-white/5 border border-white/10\n                text-slate-400 hover:text-slate-100 hover:bg-white/10 hover:border-white/20\n                transition-all duration-200 text-sm font-medium cursor-pointer\n                focus:outline-none focus:ring-2 focus:ring-cyan-500/50", "aria-label": "Open history", children: [
            /* @__PURE__ */ jsx(Clock, { className: "w-4 h-4" }),
            /* @__PURE__ */ jsx("span", { children: "History" })
          ] }),
          (message || context || intent || replies.length > 0) && /* @__PURE__ */ jsxs("button", { onClick: handleReset, className: "flex items-center gap-2 px-4 py-2.5 rounded-xl bg-white/5 border border-white/10\n                  text-slate-400 hover:text-slate-100 hover:bg-white/10 hover:border-white/20\n                  transition-all duration-200 text-sm font-medium cursor-pointer\n                  focus:outline-none focus:ring-2 focus:ring-cyan-500/50", "aria-label": "Reset form", children: [
            /* @__PURE__ */ jsx(RefreshCw, { className: "w-4 h-4" }),
            /* @__PURE__ */ jsx("span", { children: "Reset" })
          ] })
        ] })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "space-y-8", children: [
        /* @__PURE__ */ jsx(MessageInputCard, { message, context, intent, onMessageChange: setMessage, onContextChange: setContext, onIntentChange: setIntent, disabled: isGenerating }),
        /* @__PURE__ */ jsx(ToneSelector, { selected: selectedTone, onChange: setSelectedTone, disabled: isGenerating }),
        /* @__PURE__ */ jsx("div", { className: "flex justify-center pt-4", children: /* @__PURE__ */ jsx(GenerateButton, { onClick: handleGenerate, onStop: stop, isLoading: isGenerating, disabled: !message.trim() }) }),
        error && /* @__PURE__ */ jsx("div", { className: "bg-rose-500/10 border border-rose-500/20 rounded-2xl p-5 animate-fade-in-up", children: /* @__PURE__ */ jsx("p", { className: "text-rose-400 text-center text-sm font-medium", children: error }) }),
        /* @__PURE__ */ jsx(ReplyOptionsSection, { replies, isLoading: isGenerating, tone: selectedTone, copiedId, onCopy: handleCopy, onSelect: handleSelect, streamedContent })
      ] }),
      /* @__PURE__ */ jsx("footer", { className: "mt-16 text-center", children: /* @__PURE__ */ jsx("p", { className: "text-xs text-slate-600", children: "Powered by AI" }) })
    ] }),
    /* @__PURE__ */ jsx(HistoryDrawer, { isOpen: isHistoryOpen, onClose: () => setIsHistoryOpen(false), history, isLoading: isHistoryLoading, onSelectConversation: handleLoadConversation, onDeleteConversation: handleDeleteConversation })
  ] });
}
export {
  SmartReplyApp as component
};
