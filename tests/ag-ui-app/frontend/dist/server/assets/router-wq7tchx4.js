import { createRootRoute, HeadContent, Link, Scripts, createFileRoute, lazyRouteComponent, createRouter } from "@tanstack/react-router";
import { jsx, Fragment, jsxs } from "react/jsx-runtime";
import { useContext, createContext, useState, useEffect } from "react";
const appCss = "/assets/styles-DnGqQgkE.css";
const STORAGE_KEY = "syncable-agent-settings";
const DEFAULT_SETTINGS = {
  provider: "openai",
  model: "gpt-5.2",
  apiKey: "",
  awsRegion: "us-east-1"
};
const MODELS_BY_PROVIDER = {
  openai: [
    { id: "gpt-5.2", name: "GPT-5.2 - Latest reasoning model (Dec 2025)" },
    { id: "gpt-5.2-mini", name: "GPT-5.2 Mini - Fast and affordable" },
    { id: "gpt-4o", name: "GPT-4o - Multimodal workhorse" },
    { id: "o1-preview", name: "o1-preview - Advanced reasoning" }
  ],
  anthropic: [
    { id: "claude-opus-4-5-20251101", name: "Claude Opus 4.5 - Most capable (Nov 2025)" },
    { id: "claude-sonnet-4-5-20250929", name: "Claude Sonnet 4.5 - Balanced (Sep 2025)" },
    { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5 - Fast (Oct 2025)" },
    { id: "claude-sonnet-4-20250514", name: "Claude Sonnet 4 - Previous gen" }
  ],
  bedrock: [
    { id: "global.anthropic.claude-opus-4-5-20251101-v1:0", name: "Claude Opus 4.5 - Most capable (Nov 2025)" },
    { id: "global.anthropic.claude-sonnet-4-5-20250929-v1:0", name: "Claude Sonnet 4.5 - Balanced (Sep 2025)" },
    { id: "global.anthropic.claude-haiku-4-5-20251001-v1:0", name: "Claude Haiku 4.5 - Fast (Oct 2025)" },
    { id: "global.anthropic.claude-sonnet-4-20250514-v1:0", name: "Claude Sonnet 4 - Previous gen" }
  ]
};
const AgentSettingsContext = createContext(null);
function AgentSettingsProvider({ children }) {
  const [settings, setSettings] = useState(DEFAULT_SETTINGS);
  useEffect(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        setSettings({ ...DEFAULT_SETTINGS, ...parsed });
      }
    } catch (e) {
      console.error("Failed to load agent settings:", e);
    }
  }, []);
  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    } catch (e) {
      console.error("Failed to save agent settings:", e);
    }
  }, [settings]);
  const setProvider = (provider) => {
    setSettings((prev) => ({
      ...prev,
      provider,
      model: MODELS_BY_PROVIDER[provider][0].id
      // Reset to first model
    }));
  };
  const setModel = (model) => {
    setSettings((prev) => ({ ...prev, model }));
  };
  const setApiKey = (apiKey) => {
    setSettings((prev) => ({ ...prev, apiKey }));
  };
  const setAwsRegion = (awsRegion) => {
    setSettings((prev) => ({ ...prev, awsRegion }));
  };
  const availableModels = MODELS_BY_PROVIDER[settings.provider];
  return /* @__PURE__ */ jsx(
    AgentSettingsContext.Provider,
    {
      value: {
        settings,
        setProvider,
        setModel,
        setApiKey,
        setAwsRegion,
        availableModels
      },
      children
    }
  );
}
function useAgentSettings() {
  const context = useContext(AgentSettingsContext);
  if (!context) {
    throw new Error("useAgentSettings must be used within AgentSettingsProvider");
  }
  return context;
}
const AGENT_URL = typeof window !== "undefined" ? "http://localhost:9090" : "http://localhost:9090";
function CopilotKitInner({ children }) {
  const [CopilotKit, setCopilotKit] = useState(null);
  const { settings } = useAgentSettings();
  useEffect(() => {
    Promise.all([
      import("@copilotkit/react-core"),
      import("@copilotkit/react-ui/styles.css")
    ]).then(([mod]) => {
      setCopilotKit(() => mod.CopilotKit);
    });
  }, []);
  if (!CopilotKit) {
    return /* @__PURE__ */ jsx(Fragment, { children });
  }
  const forwardedProps = {
    provider: settings.provider,
    model: settings.model,
    apiKey: settings.apiKey,
    awsRegion: settings.awsRegion
  };
  return /* @__PURE__ */ jsx(
    CopilotKit,
    {
      runtimeUrl: AGENT_URL,
      properties: forwardedProps,
      agent: "syncable",
      children
    }
  );
}
function CopilotKitWrapper({ children }) {
  return /* @__PURE__ */ jsx(AgentSettingsProvider, { children: /* @__PURE__ */ jsx(CopilotKitInner, { children }) });
}
const Route$2 = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: "Smart Reply Generator" },
      { name: "description", content: "AI-powered reply suggestions for your messages" }
    ],
    links: [
      { rel: "stylesheet", href: appCss },
      { rel: "icon", href: "/favicon.ico" }
    ]
  }),
  shellComponent: RootDocument
});
function RootDocument({ children }) {
  return /* @__PURE__ */ jsxs("html", { lang: "en", className: "dark", children: [
    /* @__PURE__ */ jsx("head", { children: /* @__PURE__ */ jsx(HeadContent, {}) }),
    /* @__PURE__ */ jsxs("body", { className: "bg-slate-950 antialiased", children: [
      /* @__PURE__ */ jsxs("nav", { className: "fixed top-4 right-4 z-50 flex gap-2", children: [
        /* @__PURE__ */ jsx(
          Link,
          {
            to: "/",
            className: "px-3 py-1.5 text-xs font-medium rounded-lg bg-slate-800/80 text-slate-300 hover:bg-slate-700 hover:text-white border border-slate-700 backdrop-blur-sm transition-all",
            activeProps: { className: "bg-cyan-600/20 text-cyan-400 border-cyan-500/30" },
            children: "Smart Reply"
          }
        ),
        /* @__PURE__ */ jsx(
          Link,
          {
            to: "/agent",
            className: "px-3 py-1.5 text-xs font-medium rounded-lg bg-slate-800/80 text-slate-300 hover:bg-slate-700 hover:text-white border border-slate-700 backdrop-blur-sm transition-all",
            activeProps: { className: "bg-emerald-600/20 text-emerald-400 border-emerald-500/30" },
            children: "Agent Chat"
          }
        )
      ] }),
      /* @__PURE__ */ jsx(CopilotKitWrapper, { children }),
      /* @__PURE__ */ jsx(Scripts, {})
    ] })
  ] });
}
const $$splitComponentImporter$1 = () => import("./agent-M-s1KgCM.js");
const Route$1 = createFileRoute("/agent")({
  component: lazyRouteComponent($$splitComponentImporter$1, "component")
});
const $$splitComponentImporter = () => import("./index-Cv7xvle-.js");
const Route = createFileRoute("/")({
  component: lazyRouteComponent($$splitComponentImporter, "component")
});
const AgentRoute = Route$1.update({
  id: "/agent",
  path: "/agent",
  getParentRoute: () => Route$2
});
const IndexRoute = Route.update({
  id: "/",
  path: "/",
  getParentRoute: () => Route$2
});
const rootRouteChildren = {
  IndexRoute,
  AgentRoute
};
const routeTree = Route$2._addFileChildren(rootRouteChildren)._addFileTypes();
const getRouter = () => {
  const router2 = createRouter({
    routeTree,
    context: {},
    scrollRestoration: true,
    defaultPreloadStaleTime: 0
  });
  return router2;
};
const router = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  getRouter
}, Symbol.toStringTag, { value: "Module" }));
export {
  router as r,
  useAgentSettings as u
};
