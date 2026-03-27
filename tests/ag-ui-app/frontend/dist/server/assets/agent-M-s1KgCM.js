import { jsxs, jsx } from "react/jsx-runtime";
import { Bot, Settings, Loader2, Terminal, X, ChevronDown, EyeOff, Eye, Wrench, CheckCircle2, Circle, Database, ChevronRight, Shield, AlertTriangle, Server, FolderTree, Package, GitBranch, FileCode, Code2 } from "lucide-react";
import { useState, useEffect } from "react";
import { u as useAgentSettings } from "./router-wq7tchx4.js";
import "@tanstack/react-router";
function SettingsPanel({
  onClose
}) {
  const {
    settings,
    setProvider,
    setModel,
    setApiKey,
    setAwsRegion,
    availableModels
  } = useAgentSettings();
  const [showApiKey, setShowApiKey] = useState(false);
  return /* @__PURE__ */ jsx("div", { className: "absolute inset-0 bg-slate-950/90 backdrop-blur-sm z-20 flex items-center justify-center p-4", children: /* @__PURE__ */ jsxs("div", { className: "bg-slate-900 border border-slate-700 rounded-2xl p-6 w-full max-w-md shadow-2xl", children: [
    /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between mb-6", children: [
      /* @__PURE__ */ jsxs("h2", { className: "text-lg font-semibold text-white flex items-center gap-2", children: [
        /* @__PURE__ */ jsx(Settings, { className: "w-5 h-5 text-emerald-400" }),
        "Agent Settings"
      ] }),
      /* @__PURE__ */ jsx("button", { onClick: onClose, className: "p-1 rounded-lg hover:bg-slate-800 text-slate-400 hover:text-white transition-colors", children: /* @__PURE__ */ jsx(X, { className: "w-5 h-5" }) })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "mb-4", children: [
      /* @__PURE__ */ jsx("label", { className: "block text-sm font-medium text-slate-300 mb-2", children: "Provider" }),
      /* @__PURE__ */ jsxs("div", { className: "relative", children: [
        /* @__PURE__ */ jsxs("select", { value: settings.provider, onChange: (e) => setProvider(e.target.value), className: "w-full px-4 py-2.5 bg-slate-800 border border-slate-600 rounded-lg text-white appearance-none cursor-pointer focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent", children: [
          /* @__PURE__ */ jsx("option", { value: "openai", children: "OpenAI" }),
          /* @__PURE__ */ jsx("option", { value: "anthropic", children: "Anthropic" }),
          /* @__PURE__ */ jsx("option", { value: "bedrock", children: "AWS Bedrock" })
        ] }),
        /* @__PURE__ */ jsx(ChevronDown, { className: "absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none" })
      ] })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "mb-4", children: [
      /* @__PURE__ */ jsx("label", { className: "block text-sm font-medium text-slate-300 mb-2", children: "Model" }),
      /* @__PURE__ */ jsxs("div", { className: "relative", children: [
        /* @__PURE__ */ jsx("select", { value: settings.model, onChange: (e) => setModel(e.target.value), className: "w-full px-4 py-2.5 bg-slate-800 border border-slate-600 rounded-lg text-white appearance-none cursor-pointer focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent", children: availableModels.map((m) => /* @__PURE__ */ jsx("option", { value: m.id, children: m.name }, m.id)) }),
        /* @__PURE__ */ jsx(ChevronDown, { className: "absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none" })
      ] })
    ] }),
    settings.provider !== "bedrock" && /* @__PURE__ */ jsxs("div", { className: "mb-4", children: [
      /* @__PURE__ */ jsx("label", { className: "block text-sm font-medium text-slate-300 mb-2", children: "API Key" }),
      /* @__PURE__ */ jsxs("div", { className: "relative", children: [
        /* @__PURE__ */ jsx("input", { type: showApiKey ? "text" : "password", value: settings.apiKey, onChange: (e) => setApiKey(e.target.value), placeholder: settings.provider === "openai" ? "sk-..." : "sk-ant-...", className: "w-full px-4 py-2.5 pr-10 bg-slate-800 border border-slate-600 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent" }),
        /* @__PURE__ */ jsx("button", { type: "button", onClick: () => setShowApiKey(!showApiKey), className: "absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-white", children: showApiKey ? /* @__PURE__ */ jsx(EyeOff, { className: "w-4 h-4" }) : /* @__PURE__ */ jsx(Eye, { className: "w-4 h-4" }) })
      ] }),
      /* @__PURE__ */ jsx("p", { className: "mt-1.5 text-xs text-slate-500", children: settings.provider === "openai" ? "Get your API key from platform.openai.com" : "Get your API key from console.anthropic.com" })
    ] }),
    settings.provider === "bedrock" && /* @__PURE__ */ jsxs("div", { className: "mb-4", children: [
      /* @__PURE__ */ jsx("label", { className: "block text-sm font-medium text-slate-300 mb-2", children: "AWS Region" }),
      /* @__PURE__ */ jsxs("div", { className: "relative", children: [
        /* @__PURE__ */ jsxs("select", { value: settings.awsRegion || "us-east-1", onChange: (e) => setAwsRegion(e.target.value), className: "w-full px-4 py-2.5 bg-slate-800 border border-slate-600 rounded-lg text-white appearance-none cursor-pointer focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent", children: [
          /* @__PURE__ */ jsx("option", { value: "us-east-1", children: "US East (N. Virginia)" }),
          /* @__PURE__ */ jsx("option", { value: "us-west-2", children: "US West (Oregon)" }),
          /* @__PURE__ */ jsx("option", { value: "eu-west-1", children: "EU (Ireland)" }),
          /* @__PURE__ */ jsx("option", { value: "eu-central-1", children: "EU (Frankfurt)" }),
          /* @__PURE__ */ jsx("option", { value: "ap-northeast-1", children: "Asia Pacific (Tokyo)" })
        ] }),
        /* @__PURE__ */ jsx(ChevronDown, { className: "absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none" })
      ] }),
      /* @__PURE__ */ jsx("p", { className: "mt-1.5 text-xs text-slate-500", children: "Uses AWS credentials from ~/.aws/credentials or environment variables" })
    ] }),
    /* @__PURE__ */ jsx("button", { onClick: onClose, className: "w-full mt-2 px-4 py-2.5 bg-emerald-600 hover:bg-emerald-500 text-white font-medium rounded-lg transition-colors", children: "Save Settings" })
  ] }) });
}
function parseToolResult(result) {
  if (typeof result === "object" && result !== null && !Array.isArray(result)) {
    const obj = result;
    if ("raw" in obj && typeof obj.raw === "string") {
      try {
        return JSON.parse(obj.raw);
      } catch {
        return obj;
      }
    }
    return obj;
  }
  if (typeof result === "string") {
    try {
      return JSON.parse(result);
    } catch {
      return {
        raw: result
      };
    }
  }
  return {};
}
function AnalyzeProjectCard({
  result
}) {
  const data = parseToolResult(result);
  console.log("AnalyzeProjectCard data:", data);
  const isMonorepo = data.is_monorepo;
  const projectCount = data.project_count || (data.project_names || []).length || 1;
  const projectNames = data.project_names || [];
  const rootPath = data.root_path || "";
  const status = data.status;
  const frameworksDetected = data.frameworks_detected || [];
  const languagesDetected = data.languages_detected || [];
  const displayName = rootPath ? rootPath.split("/").pop() || "Project" : "Project Analysis";
  return /* @__PURE__ */ jsxs("div", { className: "bg-gradient-to-br from-slate-800/90 to-slate-900/90 border border-emerald-500/40 rounded-xl overflow-hidden shadow-lg", children: [
    /* @__PURE__ */ jsx("div", { className: "px-4 py-3 bg-emerald-500/10 border-b border-emerald-500/20", children: /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-3", children: [
      /* @__PURE__ */ jsx("div", { className: "p-2.5 bg-emerald-500/20 rounded-xl", children: /* @__PURE__ */ jsx(FolderTree, { className: "w-6 h-6 text-emerald-400" }) }),
      /* @__PURE__ */ jsxs("div", { children: [
        /* @__PURE__ */ jsx("h3", { className: "font-semibold text-white text-lg", children: displayName }),
        /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2 mt-0.5", children: [
          isMonorepo && /* @__PURE__ */ jsx("span", { className: "px-2 py-0.5 bg-purple-500/20 border border-purple-500/30 rounded text-xs text-purple-300", children: "Monorepo" }),
          /* @__PURE__ */ jsxs("span", { className: "text-xs text-slate-400", children: [
            projectCount,
            " project",
            projectCount !== 1 ? "s" : ""
          ] }),
          status === "ANALYSIS_COMPLETE" && /* @__PURE__ */ jsx(CheckCircle2, { className: "w-3.5 h-3.5 text-emerald-400" })
        ] })
      ] })
    ] }) }),
    /* @__PURE__ */ jsxs("div", { className: "p-4 space-y-4", children: [
      languagesDetected.length > 0 && /* @__PURE__ */ jsxs("div", { children: [
        /* @__PURE__ */ jsxs("h4", { className: "text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 flex items-center gap-1.5", children: [
          /* @__PURE__ */ jsx(Code2, { className: "w-3.5 h-3.5" }),
          "Languages"
        ] }),
        /* @__PURE__ */ jsx("div", { className: "flex flex-wrap gap-2", children: languagesDetected.map((lang, i) => /* @__PURE__ */ jsx("span", { className: "px-3 py-1.5 bg-blue-500/15 border border-blue-500/30 rounded-lg text-sm text-blue-300 font-medium", children: lang }, i)) })
      ] }),
      frameworksDetected.length > 0 && /* @__PURE__ */ jsxs("div", { children: [
        /* @__PURE__ */ jsxs("h4", { className: "text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 flex items-center gap-1.5", children: [
          /* @__PURE__ */ jsx(Package, { className: "w-3.5 h-3.5" }),
          "Frameworks & Libraries (",
          frameworksDetected.length,
          ")"
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "flex flex-wrap gap-1.5", children: [
          frameworksDetected.slice(0, 20).map((fw, i) => /* @__PURE__ */ jsx("span", { className: "px-2 py-1 bg-cyan-500/10 border border-cyan-500/25 rounded text-xs text-cyan-300", children: fw }, i)),
          frameworksDetected.length > 20 && /* @__PURE__ */ jsxs("span", { className: "px-2 py-1 text-xs text-slate-500", children: [
            "+",
            frameworksDetected.length - 20,
            " more"
          ] })
        ] })
      ] }),
      projectNames.length > 1 && /* @__PURE__ */ jsxs("div", { children: [
        /* @__PURE__ */ jsxs("h4", { className: "text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 flex items-center gap-1.5", children: [
          /* @__PURE__ */ jsx(GitBranch, { className: "w-3.5 h-3.5" }),
          "Projects (",
          projectNames.length,
          ")"
        ] }),
        /* @__PURE__ */ jsx("div", { className: "grid grid-cols-2 sm:grid-cols-3 gap-1.5 max-h-[120px] overflow-y-auto", children: projectNames.map((name, i) => /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-1.5 px-2 py-1.5 bg-slate-800/60 rounded text-xs text-slate-300 truncate", children: [
          /* @__PURE__ */ jsx(FileCode, { className: "w-3 h-3 text-slate-500 shrink-0" }),
          /* @__PURE__ */ jsx("span", { className: "truncate", children: name })
        ] }, i)) })
      ] }),
      rootPath && /* @__PURE__ */ jsx("div", { className: "pt-2 border-t border-slate-700/50", children: /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2 text-xs text-slate-500", children: [
        /* @__PURE__ */ jsx(Terminal, { className: "w-3.5 h-3.5" }),
        /* @__PURE__ */ jsx("code", { className: "font-mono", children: rootPath })
      ] }) })
    ] })
  ] });
}
function SecurityScanCard({
  result
}) {
  const [expanded, setExpanded] = useState(true);
  const data = parseToolResult(result);
  const vulnerabilities = data.vulnerabilities || [];
  const findings = data.findings || [];
  const score = data.security_score || data.score;
  const criticalCount = vulnerabilities.filter((v) => v.severity === "critical" || v.severity === "CRITICAL").length;
  const highCount = vulnerabilities.filter((v) => v.severity === "high" || v.severity === "HIGH").length;
  return /* @__PURE__ */ jsxs("div", { className: "bg-gradient-to-br from-slate-800/80 to-slate-900/80 border border-orange-500/30 rounded-xl overflow-hidden", children: [
    /* @__PURE__ */ jsxs("button", { onClick: () => setExpanded(!expanded), className: "w-full px-4 py-3 flex items-center justify-between hover:bg-slate-700/30 transition-colors", children: [
      /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-3", children: [
        /* @__PURE__ */ jsx("div", { className: "p-2 bg-orange-500/20 rounded-lg", children: /* @__PURE__ */ jsx(Shield, { className: "w-5 h-5 text-orange-400" }) }),
        /* @__PURE__ */ jsxs("div", { className: "text-left", children: [
          /* @__PURE__ */ jsx("h3", { className: "font-semibold text-white", children: "Security Scan" }),
          /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2 text-xs", children: [
            criticalCount > 0 && /* @__PURE__ */ jsxs("span", { className: "text-red-400", children: [
              criticalCount,
              " Critical"
            ] }),
            highCount > 0 && /* @__PURE__ */ jsxs("span", { className: "text-orange-400", children: [
              highCount,
              " High"
            ] }),
            score !== void 0 && /* @__PURE__ */ jsxs("span", { className: "text-emerald-400", children: [
              "Score: ",
              score
            ] })
          ] })
        ] })
      ] }),
      /* @__PURE__ */ jsx(ChevronRight, { className: `w-5 h-5 text-slate-400 transition-transform ${expanded ? "rotate-90" : ""}` })
    ] }),
    expanded && (vulnerabilities.length > 0 || findings.length > 0) && /* @__PURE__ */ jsx("div", { className: "px-4 pb-4", children: /* @__PURE__ */ jsxs("div", { className: "space-y-2 max-h-[200px] overflow-y-auto", children: [
      vulnerabilities.slice(0, 5).map((vuln, i) => /* @__PURE__ */ jsxs("div", { className: `p-3 rounded-lg border ${vuln.severity === "critical" || vuln.severity === "CRITICAL" ? "bg-red-500/10 border-red-500/30" : vuln.severity === "high" || vuln.severity === "HIGH" ? "bg-orange-500/10 border-orange-500/30" : "bg-yellow-500/10 border-yellow-500/30"}`, children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ jsx(AlertTriangle, { className: `w-4 h-4 ${vuln.severity === "critical" || vuln.severity === "CRITICAL" ? "text-red-400" : vuln.severity === "high" || vuln.severity === "HIGH" ? "text-orange-400" : "text-yellow-400"}` }),
          /* @__PURE__ */ jsx("span", { className: "text-sm font-medium text-white", children: vuln.title })
        ] }),
        vuln.description && /* @__PURE__ */ jsx("p", { className: "mt-1 text-xs text-slate-400 line-clamp-2", children: vuln.description })
      ] }, i)),
      findings.slice(0, 5).map((finding, i) => /* @__PURE__ */ jsx("div", { className: "p-3 rounded-lg bg-slate-700/30 border border-slate-600/30", children: /* @__PURE__ */ jsx("span", { className: "text-sm text-slate-300", children: finding.message }) }, i))
    ] }) })
  ] });
}
function RetrieveOutputCard({
  result,
  args
}) {
  const data = parseToolResult(result);
  args.query || "";
  const projects = data.projects || [];
  if (projects.length > 0) {
    return /* @__PURE__ */ jsxs("div", { className: "bg-gradient-to-br from-slate-800/80 to-slate-900/80 border border-violet-500/30 rounded-xl overflow-hidden", children: [
      /* @__PURE__ */ jsx("div", { className: "px-4 py-3 bg-violet-500/10 border-b border-violet-500/20", children: /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ jsx(Server, { className: "w-5 h-5 text-violet-400" }),
        /* @__PURE__ */ jsx("h3", { className: "font-semibold text-white", children: "Project Details" }),
        /* @__PURE__ */ jsxs("span", { className: "text-xs text-slate-400", children: [
          "(",
          projects.length,
          " projects)"
        ] })
      ] }) }),
      /* @__PURE__ */ jsx("div", { className: "p-3 space-y-2 max-h-[300px] overflow-y-auto", children: projects.map((proj, i) => /* @__PURE__ */ jsxs("div", { className: "p-3 bg-slate-800/50 rounded-lg border border-slate-700/50", children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between mb-2", children: [
          /* @__PURE__ */ jsx("span", { className: "font-medium text-white", children: proj.name }),
          /* @__PURE__ */ jsx("span", { className: `px-2 py-0.5 rounded text-xs ${proj.category === "Frontend" ? "bg-blue-500/20 text-blue-300" : proj.category === "Backend" ? "bg-green-500/20 text-green-300" : proj.category === "Tool" ? "bg-orange-500/20 text-orange-300" : "bg-slate-500/20 text-slate-300"}`, children: proj.category })
        ] }),
        proj.languages.length > 0 && /* @__PURE__ */ jsx("div", { className: "flex flex-wrap gap-1 mb-1", children: proj.languages.map((lang, j) => /* @__PURE__ */ jsx("span", { className: "px-1.5 py-0.5 bg-blue-500/10 rounded text-xs text-blue-300", children: lang }, j)) }),
        proj.frameworks.length > 0 && /* @__PURE__ */ jsxs("div", { className: "flex flex-wrap gap-1", children: [
          proj.frameworks.slice(0, 5).map((fw, j) => /* @__PURE__ */ jsx("span", { className: "px-1.5 py-0.5 bg-cyan-500/10 rounded text-xs text-cyan-300", children: fw }, j)),
          proj.frameworks.length > 5 && /* @__PURE__ */ jsxs("span", { className: "text-xs text-slate-500", children: [
            "+",
            proj.frameworks.length - 5
          ] })
        ] }),
        proj.path && /* @__PURE__ */ jsx("div", { className: "mt-1.5 text-xs text-slate-500 font-mono truncate", children: proj.path || "./" })
      ] }, i)) })
    ] });
  }
  const isMonorepo = data.is_monorepo;
  const projectCount = data.project_count || 0;
  const projectNames = data.project_names || [];
  const rootPath = data.root_path || "";
  if (projectNames.length > 0) {
    const displayName = rootPath ? rootPath.split("/").pop() || "Workspace" : "Workspace Analysis";
    return /* @__PURE__ */ jsxs("div", { className: "bg-gradient-to-br from-slate-800/90 to-slate-900/90 border border-indigo-500/40 rounded-xl overflow-hidden shadow-lg", children: [
      /* @__PURE__ */ jsx("div", { className: "px-4 py-3 bg-indigo-500/10 border-b border-indigo-500/20", children: /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-3", children: [
        /* @__PURE__ */ jsx("div", { className: "p-2.5 bg-indigo-500/20 rounded-xl", children: /* @__PURE__ */ jsx(FolderTree, { className: "w-6 h-6 text-indigo-400" }) }),
        /* @__PURE__ */ jsxs("div", { children: [
          /* @__PURE__ */ jsx("h3", { className: "font-semibold text-white text-lg", children: displayName }),
          /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2 mt-0.5", children: [
            isMonorepo && /* @__PURE__ */ jsx("span", { className: "px-2 py-0.5 bg-purple-500/20 border border-purple-500/30 rounded text-xs text-purple-300", children: "Monorepo" }),
            /* @__PURE__ */ jsxs("span", { className: "text-xs text-slate-400", children: [
              projectCount || projectNames.length,
              " projects detected"
            ] })
          ] })
        ] })
      ] }) }),
      /* @__PURE__ */ jsxs("div", { className: "p-4 space-y-4", children: [
        /* @__PURE__ */ jsx("div", { className: "flex items-center gap-4 text-sm", children: /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ jsx("div", { className: "w-8 h-8 rounded-lg bg-emerald-500/20 flex items-center justify-center", children: /* @__PURE__ */ jsx(Package, { className: "w-4 h-4 text-emerald-400" }) }),
          /* @__PURE__ */ jsxs("div", { children: [
            /* @__PURE__ */ jsx("div", { className: "text-white font-semibold", children: projectNames.length }),
            /* @__PURE__ */ jsx("div", { className: "text-xs text-slate-500", children: "Projects" })
          ] })
        ] }) }),
        /* @__PURE__ */ jsxs("div", { children: [
          /* @__PURE__ */ jsxs("h4", { className: "text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 flex items-center gap-1.5", children: [
            /* @__PURE__ */ jsx(GitBranch, { className: "w-3.5 h-3.5" }),
            "Projects"
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "grid grid-cols-2 sm:grid-cols-3 gap-1.5 max-h-[200px] overflow-y-auto", children: [
            projectNames.slice(0, 30).map((name, i) => /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-1.5 px-2 py-1.5 bg-slate-800/60 rounded text-xs text-slate-300 truncate", children: [
              /* @__PURE__ */ jsx(FileCode, { className: "w-3 h-3 text-slate-500 shrink-0" }),
              /* @__PURE__ */ jsx("span", { className: "truncate", children: name })
            ] }, i)),
            projectNames.length > 30 && /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-center px-2 py-1.5 bg-slate-800/40 rounded text-xs text-slate-500", children: [
              "+",
              projectNames.length - 30,
              " more"
            ] })
          ] })
        ] }),
        rootPath && /* @__PURE__ */ jsx("div", { className: "pt-2 border-t border-slate-700/50", children: /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2 text-xs text-slate-500", children: [
          /* @__PURE__ */ jsx(Terminal, { className: "w-3.5 h-3.5" }),
          /* @__PURE__ */ jsx("code", { className: "font-mono truncate", children: rootPath })
        ] }) })
      ] })
    ] });
  }
  return null;
}
function GenericToolCard({
  toolName,
  result,
  isError
}) {
  const [expanded, setExpanded] = useState(false);
  const displayData = parseToolResult(result);
  const isRawString = "raw" in displayData && typeof displayData.raw === "string" && Object.keys(displayData).length === 1;
  const getIcon = () => {
    if (toolName.includes("k8s") || toolName.includes("kube")) return /* @__PURE__ */ jsx(Server, { className: "w-5 h-5" });
    if (toolName.includes("terraform")) return /* @__PURE__ */ jsx(Code2, { className: "w-5 h-5" });
    if (toolName.includes("docker") || toolName.includes("hadolint")) return /* @__PURE__ */ jsx(Package, { className: "w-5 h-5" });
    if (toolName.includes("git")) return /* @__PURE__ */ jsx(GitBranch, { className: "w-5 h-5" });
    if (toolName.includes("file") || toolName.includes("read")) return /* @__PURE__ */ jsx(FileCode, { className: "w-5 h-5" });
    if (toolName.includes("security") || toolName.includes("vuln")) return /* @__PURE__ */ jsx(Shield, { className: "w-5 h-5" });
    if (toolName.includes("list") || toolName.includes("directory")) return /* @__PURE__ */ jsx(FolderTree, { className: "w-5 h-5" });
    return /* @__PURE__ */ jsx(Database, { className: "w-5 h-5" });
  };
  const hasActualError = isError && typeof displayData === "object" && displayData !== null && "error" in displayData;
  const borderColor = hasActualError ? "border-red-500/30" : "border-slate-600/50";
  const iconBg = hasActualError ? "bg-red-500/20" : "bg-slate-700/50";
  const iconColor = hasActualError ? "text-red-400" : "text-slate-400";
  return /* @__PURE__ */ jsxs("div", { className: `bg-gradient-to-br from-slate-800/60 to-slate-900/60 border ${borderColor} rounded-xl overflow-hidden`, children: [
    /* @__PURE__ */ jsxs("button", { onClick: () => setExpanded(!expanded), className: "w-full px-4 py-3 flex items-center justify-between hover:bg-slate-700/30 transition-colors", children: [
      /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-3", children: [
        /* @__PURE__ */ jsx("div", { className: `p-2 ${iconBg} rounded-lg ${iconColor}`, children: getIcon() }),
        /* @__PURE__ */ jsxs("div", { className: "text-left", children: [
          /* @__PURE__ */ jsx("h3", { className: "font-medium text-white", children: toolName.replace(/_/g, " ") }),
          hasActualError && /* @__PURE__ */ jsx("span", { className: "text-xs text-red-400", children: "Error" })
        ] })
      ] }),
      /* @__PURE__ */ jsx(ChevronRight, { className: `w-5 h-5 text-slate-400 transition-transform ${expanded ? "rotate-90" : ""}` })
    ] }),
    expanded && /* @__PURE__ */ jsx("div", { className: "px-4 pb-4", children: /* @__PURE__ */ jsx("pre", { className: "p-3 bg-slate-900/80 rounded-lg text-xs text-slate-300 overflow-x-auto max-h-[300px] overflow-y-auto whitespace-pre-wrap break-words", children: isRawString ? displayData.raw : JSON.stringify(displayData, null, 2) }) })
  ] });
}
function canRenderRetrieveOutput(result) {
  const data = parseToolResult(result);
  const projects = data.projects || [];
  const projectNames = data.project_names || [];
  return projects.length > 0 || projectNames.length > 0;
}
function ToolResultCard({
  toolResult
}) {
  const {
    tool_name,
    args,
    result,
    is_error
  } = toolResult;
  switch (tool_name) {
    case "analyze_project":
      return /* @__PURE__ */ jsx(AnalyzeProjectCard, { result });
    case "retrieve_output": {
      if (canRenderRetrieveOutput(result)) {
        return /* @__PURE__ */ jsx(RetrieveOutputCard, { result, args });
      }
      return /* @__PURE__ */ jsx(GenericToolCard, { toolName: tool_name, result, isError: is_error });
    }
    case "security_scan":
    case "check_vulnerabilities":
      return /* @__PURE__ */ jsx(SecurityScanCard, { result });
    default:
      return /* @__PURE__ */ jsx(GenericToolCard, { toolName: tool_name, result, isError: is_error });
  }
}
function renderAgentProgress(state) {
  const hasSteps = state?.steps && state.steps.length > 0;
  const hasToolResults = state?.tool_results && state.tool_results.length > 0;
  if (!hasSteps && !hasToolResults) {
    return null;
  }
  const completedCount = state?.steps?.filter((s) => s.status === "completed").length || 0;
  const totalSteps = state?.steps?.length || 0;
  const progressPercent = totalSteps > 0 ? completedCount / totalSteps * 100 : 0;
  const isComplete = totalSteps > 0 && completedCount === totalSteps;
  return /* @__PURE__ */ jsxs("div", { className: "space-y-4 mb-4", children: [
    hasSteps && !isComplete && /* @__PURE__ */ jsxs("div", { className: "p-4 bg-slate-900/80 border border-slate-700/50 rounded-xl backdrop-blur-sm", children: [
      /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between mb-3", children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ jsx(Wrench, { className: "w-4 h-4 text-cyan-400 animate-pulse" }),
          /* @__PURE__ */ jsx("span", { className: "text-sm font-medium text-slate-200", children: "Agent Progress" })
        ] }),
        /* @__PURE__ */ jsxs("span", { className: "text-xs text-slate-400", children: [
          completedCount,
          "/",
          totalSteps,
          " complete"
        ] })
      ] }),
      /* @__PURE__ */ jsx("div", { className: "h-1.5 bg-slate-700 rounded-full overflow-hidden mb-3", children: /* @__PURE__ */ jsx("div", { className: "h-full bg-gradient-to-r from-emerald-500 to-cyan-500 rounded-full transition-all duration-500 ease-out", style: {
        width: `${progressPercent}%`
      } }) }),
      /* @__PURE__ */ jsx("div", { className: "space-y-2 max-h-[150px] overflow-y-auto", children: state.steps?.map((step, index) => {
        const isCompleted = step.status === "completed";
        const isPending = step.status === "pending";
        const isCurrentPending = isPending && index === state.steps?.findIndex((s) => s.status === "pending");
        return /* @__PURE__ */ jsxs("div", { className: `flex items-center gap-2 px-3 py-2 rounded-lg transition-all duration-300 ${isCompleted ? "bg-emerald-500/10 border border-emerald-500/20" : isCurrentPending ? "bg-cyan-500/10 border border-cyan-500/30" : "bg-slate-800/50 border border-slate-700/30"}`, children: [
          isCompleted ? /* @__PURE__ */ jsx(CheckCircle2, { className: "w-4 h-4 text-emerald-400 shrink-0" }) : isCurrentPending ? /* @__PURE__ */ jsx(Loader2, { className: "w-4 h-4 text-cyan-400 animate-spin shrink-0" }) : /* @__PURE__ */ jsx(Circle, { className: "w-4 h-4 text-slate-500 shrink-0" }),
          /* @__PURE__ */ jsx("span", { className: `text-sm truncate ${isCompleted ? "text-emerald-300" : isCurrentPending ? "text-cyan-300" : "text-slate-400"}`, children: step.description })
        ] }, index);
      }) }),
      state.current_tool && /* @__PURE__ */ jsx("div", { className: "mt-3 pt-3 border-t border-slate-700/50", children: /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2 text-xs text-slate-400", children: [
        /* @__PURE__ */ jsx("span", { className: "w-2 h-2 bg-cyan-400 rounded-full animate-pulse" }),
        /* @__PURE__ */ jsxs("span", { children: [
          "Running: ",
          /* @__PURE__ */ jsx("code", { className: "text-cyan-400", children: state.current_tool })
        ] })
      ] }) })
    ] }),
    hasToolResults && /* @__PURE__ */ jsxs("div", { className: "space-y-3", children: [
      /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2 text-xs text-slate-400 uppercase tracking-wider font-medium px-1", children: [
        /* @__PURE__ */ jsx(Database, { className: "w-3.5 h-3.5" }),
        /* @__PURE__ */ jsx("span", { children: "Tool Results" })
      ] }),
      state.tool_results.map((toolResult, index) => /* @__PURE__ */ jsx(ToolResultCard, { toolResult }, index))
    ] })
  ] });
}
function GenerativeUIRenderer({
  useCoAgentStateRender,
  CopilotChat
}) {
  useCoAgentStateRender({
    name: "syncable",
    render: ({
      state
    }) => renderAgentProgress(state)
  });
  return /* @__PURE__ */ jsx(CopilotChat, { className: "h-full", labels: {
    title: "Syncable Agent",
    initial: "Hi! I'm the Syncable agent. How can I help you today?",
    placeholder: "Type your message..."
  } });
}
function ChatWithGenerativeUI({
  CopilotChat
}) {
  const [useCoAgentStateRender, setUseCoAgentStateRender] = useState(null);
  useEffect(() => {
    import("@copilotkit/react-core").then((mod) => {
      setUseCoAgentStateRender(() => mod.useCoAgentStateRender);
    });
  }, []);
  if (!useCoAgentStateRender) {
    return /* @__PURE__ */ jsx(CopilotChat, { className: "h-full", labels: {
      title: "Syncable Agent",
      initial: "Hi! I'm the Syncable agent. How can I help you today?",
      placeholder: "Type your message..."
    } });
  }
  return /* @__PURE__ */ jsx(GenerativeUIRenderer, { useCoAgentStateRender, CopilotChat });
}
function AgentChat() {
  const [CopilotChat, setCopilotChat] = useState(null);
  const [showSettings, setShowSettings] = useState(false);
  const {
    settings
  } = useAgentSettings();
  useEffect(() => {
    import("@copilotkit/react-ui").then((mod) => {
      setCopilotChat(() => mod.CopilotChat);
    });
  }, []);
  return /* @__PURE__ */ jsxs("main", { className: "min-h-screen bg-slate-950 relative overflow-hidden", children: [
    /* @__PURE__ */ jsx("div", { className: "absolute inset-0 bg-linear-to-br from-slate-950 via-slate-900 to-slate-950" }),
    /* @__PURE__ */ jsx("div", { className: "absolute inset-0 bg-[radial-gradient(ellipse_at_top,rgba(34,211,238,0.1),transparent_50%)]" }),
    /* @__PURE__ */ jsxs("div", { className: "relative z-10 max-w-5xl mx-auto px-4 sm:px-6 py-8", children: [
      /* @__PURE__ */ jsxs("header", { className: "flex flex-col items-center text-center mb-6", children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-3 mb-4", children: [
          /* @__PURE__ */ jsx("div", { className: "p-3 rounded-2xl bg-linear-to-br from-emerald-500/20 to-cyan-600/20 border border-emerald-500/30 shadow-[0_0_30px_rgba(16,185,129,0.15)]", children: /* @__PURE__ */ jsx(Bot, { className: "w-8 h-8 text-emerald-400" }) }),
          /* @__PURE__ */ jsx("h1", { className: "text-4xl font-bold tracking-tight bg-linear-to-r from-emerald-400 via-cyan-400 to-blue-400 bg-clip-text text-transparent", children: "Agent Chat" })
        ] }),
        /* @__PURE__ */ jsx("p", { className: "text-slate-400 max-w-md text-base leading-relaxed", children: "Chat with the Syncable agent via AG-UI protocol. Messages are processed by the AG-UI server and streamed back in real-time." })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "mb-4 flex items-center justify-between", children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2 text-sm text-slate-400", children: [
          /* @__PURE__ */ jsx("span", { className: "px-2 py-1 bg-slate-800/50 rounded text-xs", children: settings.provider === "openai" ? "OpenAI" : settings.provider === "anthropic" ? "Anthropic" : "Bedrock" }),
          /* @__PURE__ */ jsx("span", { className: "px-2 py-1 bg-slate-800/50 rounded text-xs font-mono truncate max-w-[200px]", children: settings.model.split("/").pop()?.split(":")[0] || settings.model })
        ] }),
        /* @__PURE__ */ jsxs("button", { onClick: () => setShowSettings(true), className: "flex items-center gap-2 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-sm text-slate-300 hover:text-white transition-colors", children: [
          /* @__PURE__ */ jsx(Settings, { className: "w-4 h-4" }),
          "Settings"
        ] })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "bg-slate-900/50 border border-slate-800 rounded-2xl overflow-hidden h-[calc(100vh-280px)] min-h-[500px] relative", children: [
        showSettings && /* @__PURE__ */ jsx(SettingsPanel, { onClose: () => setShowSettings(false) }),
        CopilotChat ? /* @__PURE__ */ jsx(ChatWithGenerativeUI, { CopilotChat }) : /* @__PURE__ */ jsx("div", { className: "h-full flex items-center justify-center", children: /* @__PURE__ */ jsx(Loader2, { className: "w-8 h-8 text-emerald-400 animate-spin" }) })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "mt-6 p-4 bg-slate-900/30 border border-slate-800/50 rounded-xl", children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2 text-slate-400 text-sm", children: [
          /* @__PURE__ */ jsx(Terminal, { className: "w-4 h-4" }),
          /* @__PURE__ */ jsx("span", { children: "AG-UI Server: " }),
          /* @__PURE__ */ jsx("code", { className: "px-2 py-0.5 bg-slate-800 rounded text-emerald-400 text-xs", children: "http://localhost:9090" })
        ] }),
        /* @__PURE__ */ jsx("p", { className: "mt-2 text-xs text-slate-500", children: "Messages are sent via POST /message and responses streamed via SSE/WebSocket." })
      ] })
    ] })
  ] });
}
export {
  AgentChat as component
};
