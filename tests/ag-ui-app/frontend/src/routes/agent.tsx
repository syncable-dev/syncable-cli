/**
 * Agent Chat Route
 *
 * Demonstrates CopilotKit integration with syncable-cli AG-UI server.
 * Uses CopilotKit's built-in CopilotChat component for conversations.
 * Includes settings panel for provider/model/API key configuration.
 * Tool results are displayed in a separate sidebar panel to avoid duplication issues.
 */
import { createFileRoute } from "@tanstack/react-router";
import { Bot, Terminal, Loader2, Settings, ChevronDown, Eye, EyeOff, X, CheckCircle2, FolderTree, Shield, AlertTriangle, FileCode, Package, Code2, GitBranch, ChevronRight, PanelLeftClose, PanelLeft, Server } from "lucide-react";
import React, { createContext, useContext, useEffect, useState, useCallback, useRef, type ComponentType, type ReactNode } from "react";
import { useAgentSettings, type Provider } from "../lib/agent-settings";

// Type for agent state from backend
interface AgentStep {
  description: string;
  status: "pending" | "completed";
}

interface ToolResult {
  tool_name: string;
  args: Record<string, unknown>;
  result: unknown;
  is_error: boolean;
  timestamp?: number;
  id?: string; // Unique ID for each result
}

interface AgentState {
  steps: AgentStep[];
  current_tool?: string;
  tool_results?: ToolResult[];
}

// Context for tool results - allows capturing them from useCoAgentStateRender
interface ToolResultsContextType {
  toolResults: ToolResult[];
  currentTool: string | null;
  addToolResults: (results: ToolResult[]) => void;
  setCurrentTool: (tool: string | null) => void;
  clearToolResults: () => void;
}

const ToolResultsContext = createContext<ToolResultsContextType | null>(null);

function useToolResults() {
  const context = useContext(ToolResultsContext);
  if (!context) {
    throw new Error("useToolResults must be used within ToolResultsProvider");
  }
  return context;
}

/**
 * Generate a stable content-based key for a tool result.
 * This ensures the same result always gets the same key, preventing duplicates.
 */
function getToolResultKey(r: ToolResult): string {
  const resultStr = typeof r.result === 'string'
    ? r.result
    : JSON.stringify(r.result);
  // Use tool name + first 500 chars of result as key (should be unique enough)
  return `${r.tool_name}:${resultStr.substring(0, 500)}`;
}

function ToolResultsProvider({ children }: { children: ReactNode }) {
  // Use a Map to store results by content-based key - prevents duplicates automatically
  const [toolResultsMap, setToolResultsMap] = useState<Map<string, ToolResult>>(new Map());
  const [currentTool, setCurrentToolState] = useState<string | null>(null);

  // Convert map to array for consumers (maintains insertion order)
  const toolResults = Array.from(toolResultsMap.values());

  const addToolResults = useCallback((results: ToolResult[]) => {
    setToolResultsMap(prev => {
      const newMap = new Map(prev);
      let hasChanges = false;

      for (const r of results) {
        const key = getToolResultKey(r);
        // Only add if we don't already have this exact result
        if (!newMap.has(key)) {
          hasChanges = true;
          newMap.set(key, {
            ...r,
            id: key,
            timestamp: r.timestamp || Date.now()
          });
        }
      }

      // Return same reference if no changes (prevents unnecessary re-renders)
      return hasChanges ? newMap : prev;
    });
  }, []);

  const setCurrentTool = useCallback((tool: string | null) => {
    setCurrentToolState(tool);
  }, []);

  const clearToolResults = useCallback(() => {
    setToolResultsMap(new Map());
    setCurrentToolState(null);
  }, []);

  return (
    <ToolResultsContext.Provider value={{ toolResults, currentTool, addToolResults, setCurrentTool, clearToolResults }}>
      {children}
    </ToolResultsContext.Provider>
  );
}

export const Route = createFileRoute("/agent")({
  component: AgentChat,
});

function SettingsPanel({ onClose }: { onClose: () => void }) {
  const { settings, setProvider, setModel, setApiKey, setAwsRegion, availableModels } = useAgentSettings();
  const [showApiKey, setShowApiKey] = useState(false);

  return (
    <div className="absolute inset-0 bg-slate-950/90 backdrop-blur-sm z-20 flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-2xl p-6 w-full max-w-md shadow-2xl">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-lg font-semibold text-white flex items-center gap-2">
            <Settings className="w-5 h-5 text-emerald-400" />
            Agent Settings
          </h2>
          <button
            onClick={onClose}
            className="p-1 rounded-lg hover:bg-slate-800 text-slate-400 hover:text-white transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Provider Selection */}
        <div className="mb-4">
          <label className="block text-sm font-medium text-slate-300 mb-2">
            Provider
          </label>
          <div className="relative">
            <select
              value={settings.provider}
              onChange={(e) => setProvider(e.target.value as Provider)}
              className="w-full px-4 py-2.5 bg-slate-800 border border-slate-600 rounded-lg text-white appearance-none cursor-pointer focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
            >
              <option value="openai">OpenAI</option>
              <option value="anthropic">Anthropic</option>
              <option value="bedrock">AWS Bedrock</option>
            </select>
            <ChevronDown className="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none" />
          </div>
        </div>

        {/* Model Selection */}
        <div className="mb-4">
          <label className="block text-sm font-medium text-slate-300 mb-2">
            Model
          </label>
          <div className="relative">
            <select
              value={settings.model}
              onChange={(e) => setModel(e.target.value)}
              className="w-full px-4 py-2.5 bg-slate-800 border border-slate-600 rounded-lg text-white appearance-none cursor-pointer focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
            >
              {availableModels.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
            <ChevronDown className="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none" />
          </div>
        </div>

        {/* API Key Input */}
        {settings.provider !== 'bedrock' && (
          <div className="mb-4">
            <label className="block text-sm font-medium text-slate-300 mb-2">
              API Key
            </label>
            <div className="relative">
              <input
                type={showApiKey ? "text" : "password"}
                value={settings.apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder={settings.provider === 'openai' ? 'sk-...' : 'sk-ant-...'}
                className="w-full px-4 py-2.5 pr-10 bg-slate-800 border border-slate-600 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
              />
              <button
                type="button"
                onClick={() => setShowApiKey(!showApiKey)}
                className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-white"
              >
                {showApiKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
            </div>
            <p className="mt-1.5 text-xs text-slate-500">
              {settings.provider === 'openai'
                ? 'Get your API key from platform.openai.com'
                : 'Get your API key from console.anthropic.com'}
            </p>
          </div>
        )}

        {/* AWS Region (Bedrock only) */}
        {settings.provider === 'bedrock' && (
          <div className="mb-4">
            <label className="block text-sm font-medium text-slate-300 mb-2">
              AWS Region
            </label>
            <div className="relative">
              <select
                value={settings.awsRegion || 'us-east-1'}
                onChange={(e) => setAwsRegion(e.target.value)}
                className="w-full px-4 py-2.5 bg-slate-800 border border-slate-600 rounded-lg text-white appearance-none cursor-pointer focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
              >
                <option value="us-east-1">US East (N. Virginia)</option>
                <option value="us-west-2">US West (Oregon)</option>
                <option value="eu-west-1">EU (Ireland)</option>
                <option value="eu-central-1">EU (Frankfurt)</option>
                <option value="ap-northeast-1">Asia Pacific (Tokyo)</option>
              </select>
              <ChevronDown className="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none" />
            </div>
            <p className="mt-1.5 text-xs text-slate-500">
              Uses AWS credentials from ~/.aws/credentials or environment variables
            </p>
          </div>
        )}

        {/* Save Button */}
        <button
          onClick={onClose}
          className="w-full mt-2 px-4 py-2.5 bg-emerald-600 hover:bg-emerald-500 text-white font-medium rounded-lg transition-colors"
        >
          Save Settings
        </button>
      </div>
    </div>
  );
}

/**
 * Helper to extract JSON from a string that may have text prefix.
 * E.g., "Note: Large project detected...\n\n{\"foo\": \"bar\"}" -> {"foo": "bar"}
 */
function extractJsonFromString(str: string): Record<string, unknown> | null {
  // First, try parsing the whole string directly
  try {
    return JSON.parse(str);
  } catch {
    // Not valid JSON as-is, try to find embedded JSON
  }

  // Look for JSON object start - find the first '{' that might start valid JSON
  const firstBrace = str.indexOf('{');
  if (firstBrace === -1) {
    return null;
  }

  // Try parsing from the first brace
  const jsonCandidate = str.slice(firstBrace);
  try {
    return JSON.parse(jsonCandidate);
  } catch {
    // Still not valid, try finding the last matching brace
  }

  // More robust: find matching braces
  let depth = 0;
  let start = -1;
  for (let i = 0; i < str.length; i++) {
    if (str[i] === '{') {
      if (depth === 0) start = i;
      depth++;
    } else if (str[i] === '}') {
      depth--;
      if (depth === 0 && start !== -1) {
        const candidate = str.slice(start, i + 1);
        try {
          return JSON.parse(candidate);
        } catch {
          // Continue searching
        }
        start = -1;
      }
    }
  }

  return null;
}

/**
 * Helper to parse tool result which can be a JSON string or object.
 * Handles strings with text prefixes before the JSON (e.g., "Note: ...\n\n{...}").
 */
function parseToolResult(result: unknown): Record<string, unknown> {
  // If it's already a parsed object
  if (typeof result === 'object' && result !== null && !Array.isArray(result)) {
    const obj = result as Record<string, unknown>;
    // Check if it has a 'raw' field that's a JSON string
    if ('raw' in obj && typeof obj.raw === 'string') {
      const extracted = extractJsonFromString(obj.raw);
      if (extracted) return extracted;
      return obj;
    }
    return obj;
  }

  // If it's a string, try to extract JSON (handles text prefixes)
  if (typeof result === 'string') {
    const extracted = extractJsonFromString(result);
    if (extracted) return extracted;
    return { raw: result };
  }

  return {};
}

/**
 * Renders a rich UI card for analyze_project tool results.
 */
function AnalyzeProjectCard({ result }: { result: unknown }) {
  // Parse the result - handles JSON with text prefixes
  const data = parseToolResult(result);

  // Extract fields from the actual analyze_project response format
  const isMonorepo = data.is_monorepo as boolean;
  const projectCount = (data.project_count as number) || (data.project_names as string[] || []).length || 1;
  const projectNames = data.project_names as string[] || [];
  const rootPath = data.root_path as string || '';
  const status = data.status as string;
  const frameworksDetected = data.frameworks_detected as string[] || [];
  const languagesDetected = data.languages_detected as string[] || [];

  // Get display name from root path
  const displayName = rootPath ? rootPath.split('/').pop() || 'Project' : 'Project Analysis';

  return (
    <div className="bg-gradient-to-br from-slate-800/90 to-slate-900/90 border border-emerald-500/40 rounded-xl overflow-hidden shadow-lg">
      {/* Header - Always visible */}
      <div className="px-4 py-3 bg-emerald-500/10 border-b border-emerald-500/20">
        <div className="flex items-center gap-3">
          <div className="p-2.5 bg-emerald-500/20 rounded-xl">
            <FolderTree className="w-6 h-6 text-emerald-400" />
          </div>
          <div>
            <h3 className="font-semibold text-white text-lg">{displayName}</h3>
            <div className="flex items-center gap-2 mt-0.5">
              {isMonorepo && (
                <span className="px-2 py-0.5 bg-purple-500/20 border border-purple-500/30 rounded text-xs text-purple-300">
                  Monorepo
                </span>
              )}
              <span className="text-xs text-slate-400">{projectCount} project{projectCount !== 1 ? 's' : ''}</span>
              {status === 'ANALYSIS_COMPLETE' && (
                <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Content - Always expanded */}
      <div className="p-4 space-y-4">
        {/* Languages */}
        {languagesDetected.length > 0 && (
          <div>
            <h4 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 flex items-center gap-1.5">
              <Code2 className="w-3.5 h-3.5" />
              Languages
            </h4>
            <div className="flex flex-wrap gap-2">
              {languagesDetected.map((lang, i) => (
                <span
                  key={i}
                  className="px-3 py-1.5 bg-blue-500/15 border border-blue-500/30 rounded-lg text-sm text-blue-300 font-medium"
                >
                  {lang}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Frameworks */}
        {frameworksDetected.length > 0 && (
          <div>
            <h4 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 flex items-center gap-1.5">
              <Package className="w-3.5 h-3.5" />
              Frameworks & Libraries ({frameworksDetected.length})
            </h4>
            <div className="flex flex-wrap gap-1.5">
              {frameworksDetected.slice(0, 20).map((fw, i) => (
                <span
                  key={i}
                  className="px-2 py-1 bg-cyan-500/10 border border-cyan-500/25 rounded text-xs text-cyan-300"
                >
                  {fw}
                </span>
              ))}
              {frameworksDetected.length > 20 && (
                <span className="px-2 py-1 text-xs text-slate-500">
                  +{frameworksDetected.length - 20} more
                </span>
              )}
            </div>
          </div>
        )}

        {/* Projects in Monorepo - only show if there are multiple projects */}
        {projectNames.length > 1 && (
          <div>
            <h4 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 flex items-center gap-1.5">
              <GitBranch className="w-3.5 h-3.5" />
              Projects
              {projectCount > projectNames.length ? (
                <span className="text-slate-500">({projectNames.length} of {projectCount} shown)</span>
              ) : (
                <span className="text-slate-500">({projectNames.length})</span>
              )}
            </h4>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5 max-h-[120px] overflow-y-auto">
              {projectNames.map((name, i) => (
                <div
                  key={i}
                  className="flex items-center gap-1.5 px-2 py-1.5 bg-slate-800/60 rounded text-xs text-slate-300 truncate"
                >
                  <FileCode className="w-3 h-3 text-slate-500 shrink-0" />
                  <span className="truncate">{name}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Root Path */}
        {rootPath && (
          <div className="pt-2 border-t border-slate-700/50">
            <div className="flex items-center gap-2 text-xs text-slate-500">
              <Terminal className="w-3.5 h-3.5" />
              <code className="font-mono">{rootPath}</code>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * Renders a rich UI card for security_scan tool results.
 */
function SecurityScanCard({ result }: { result: unknown }) {
  const [expanded, setExpanded] = useState(true);

  const data = parseToolResult(result);
  const vulnerabilities = data.vulnerabilities as Array<{severity: string; title: string; description?: string}> || [];
  const findings = data.findings as Array<{level: string; message: string}> || [];
  const score = data.security_score as number || data.score as number;

  const criticalCount = vulnerabilities.filter(v => v.severity === 'critical' || v.severity === 'CRITICAL').length;
  const highCount = vulnerabilities.filter(v => v.severity === 'high' || v.severity === 'HIGH').length;

  return (
    <div className="bg-gradient-to-br from-slate-800/80 to-slate-900/80 border border-orange-500/30 rounded-xl overflow-hidden">
      {/* Header */}
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full px-4 py-3 flex items-center justify-between hover:bg-slate-700/30 transition-colors"
      >
        <div className="flex items-center gap-3">
          <div className="p-2 bg-orange-500/20 rounded-lg">
            <Shield className="w-5 h-5 text-orange-400" />
          </div>
          <div className="text-left">
            <h3 className="font-semibold text-white">Security Scan</h3>
            <div className="flex items-center gap-2 text-xs">
              {criticalCount > 0 && <span className="text-red-400">{criticalCount} Critical</span>}
              {highCount > 0 && <span className="text-orange-400">{highCount} High</span>}
              {score !== undefined && <span className="text-emerald-400">Score: {score}</span>}
            </div>
          </div>
        </div>
        <ChevronRight className={`w-5 h-5 text-slate-400 transition-transform ${expanded ? 'rotate-90' : ''}`} />
      </button>

      {expanded && (vulnerabilities.length > 0 || findings.length > 0) && (
        <div className="px-4 pb-4">
          <div className="space-y-2 max-h-[200px] overflow-y-auto">
            {vulnerabilities.slice(0, 5).map((vuln, i) => (
              <div
                key={i}
                className={`p-3 rounded-lg border ${
                  vuln.severity === 'critical' || vuln.severity === 'CRITICAL'
                    ? 'bg-red-500/10 border-red-500/30'
                    : vuln.severity === 'high' || vuln.severity === 'HIGH'
                      ? 'bg-orange-500/10 border-orange-500/30'
                      : 'bg-yellow-500/10 border-yellow-500/30'
                }`}
              >
                <div className="flex items-center gap-2">
                  <AlertTriangle className={`w-4 h-4 ${
                    vuln.severity === 'critical' || vuln.severity === 'CRITICAL' ? 'text-red-400' :
                    vuln.severity === 'high' || vuln.severity === 'HIGH' ? 'text-orange-400' : 'text-yellow-400'
                  }`} />
                  <span className="text-sm font-medium text-white">{vuln.title}</span>
                </div>
                {vuln.description && (
                  <p className="mt-1 text-xs text-slate-400 line-clamp-2">{vuln.description}</p>
                )}
              </div>
            ))}
            {findings.slice(0, 5).map((finding, i) => (
              <div key={i} className="p-3 rounded-lg bg-slate-700/30 border border-slate-600/30">
                <span className="text-sm text-slate-300">{finding.message}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/**
 * Renders a card for retrieve_output tool - shows detailed project/section data.
 */
function RetrieveOutputCard({ result }: { result: unknown }) {
  const data = parseToolResult(result);

  const projects = data.projects as Array<{name: string; category: string; frameworks: string[]; languages: string[]; path: string}> || [];

  // If this is a projects section with detailed project info
  if (projects.length > 0) {
    return (
      <div className="bg-gradient-to-br from-slate-800/80 to-slate-900/80 border border-violet-500/30 rounded-xl overflow-hidden">
        <div className="px-4 py-3 bg-violet-500/10 border-b border-violet-500/20">
          <div className="flex items-center gap-2">
            <Server className="w-5 h-5 text-violet-400" />
            <h3 className="font-semibold text-white">Project Details</h3>
            <span className="text-xs text-slate-400">({projects.length} projects)</span>
          </div>
        </div>
        <div className="p-3 space-y-2 max-h-[300px] overflow-y-auto">
          {projects.map((proj, i) => (
            <div key={i} className="p-3 bg-slate-800/50 rounded-lg border border-slate-700/50">
              <div className="flex items-center justify-between mb-2">
                <span className="font-medium text-white">{proj.name}</span>
                <span className={`px-2 py-0.5 rounded text-xs ${
                  proj.category === 'Frontend' ? 'bg-blue-500/20 text-blue-300' :
                  proj.category === 'Backend' ? 'bg-green-500/20 text-green-300' :
                  proj.category === 'Tool' ? 'bg-orange-500/20 text-orange-300' :
                  'bg-slate-500/20 text-slate-300'
                }`}>
                  {proj.category}
                </span>
              </div>
              {proj.languages.length > 0 && (
                <div className="flex flex-wrap gap-1 mb-1">
                  {proj.languages.map((lang, j) => (
                    <span key={j} className="px-1.5 py-0.5 bg-blue-500/10 rounded text-xs text-blue-300">{lang}</span>
                  ))}
                </div>
              )}
              {proj.frameworks.length > 0 && (
                <div className="flex flex-wrap gap-1">
                  {proj.frameworks.slice(0, 5).map((fw, j) => (
                    <span key={j} className="px-1.5 py-0.5 bg-cyan-500/10 rounded text-xs text-cyan-300">{fw}</span>
                  ))}
                  {proj.frameworks.length > 5 && (
                    <span className="text-xs text-slate-500">+{proj.frameworks.length - 5}</span>
                  )}
                </div>
              )}
              {proj.path && (
                <div className="mt-1.5 text-xs text-slate-500 font-mono truncate">{proj.path || './'}</div>
              )}
            </div>
          ))}
        </div>
      </div>
    );
  }

  // For other retrieve_output results, show summary info (multi-repo analysis)
  const isMonorepo = data.is_monorepo as boolean;
  const projectCount = data.project_count as number || 0;
  const projectNames = data.project_names as string[] || [];
  const rootPath = data.root_path as string || '';

  if (projectNames.length > 0) {
    // Get display name from root path
    const displayName = rootPath ? rootPath.split('/').pop() || 'Workspace' : 'Workspace Analysis';

    return (
      <div className="bg-gradient-to-br from-slate-800/90 to-slate-900/90 border border-indigo-500/40 rounded-xl overflow-hidden shadow-lg">
        {/* Header */}
        <div className="px-4 py-3 bg-indigo-500/10 border-b border-indigo-500/20">
          <div className="flex items-center gap-3">
            <div className="p-2.5 bg-indigo-500/20 rounded-xl">
              <FolderTree className="w-6 h-6 text-indigo-400" />
            </div>
            <div>
              <h3 className="font-semibold text-white text-lg">{displayName}</h3>
              <div className="flex items-center gap-2 mt-0.5">
                {isMonorepo && (
                  <span className="px-2 py-0.5 bg-purple-500/20 border border-purple-500/30 rounded text-xs text-purple-300">
                    Monorepo
                  </span>
                )}
                <span className="text-xs text-slate-400">{projectCount || projectNames.length} projects detected</span>
              </div>
            </div>
          </div>
        </div>

        {/* Content */}
        <div className="p-4 space-y-4">
          {/* Project count stats */}
          <div className="flex items-center gap-4 text-sm">
            <div className="flex items-center gap-2">
              <div className="w-8 h-8 rounded-lg bg-emerald-500/20 flex items-center justify-center">
                <Package className="w-4 h-4 text-emerald-400" />
              </div>
              <div>
                <div className="text-white font-semibold">{projectNames.length}</div>
                <div className="text-xs text-slate-500">Projects</div>
              </div>
            </div>
          </div>

          {/* Projects grid */}
          <div>
            <h4 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 flex items-center gap-1.5">
              <GitBranch className="w-3.5 h-3.5" />
              Projects
            </h4>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5 max-h-[200px] overflow-y-auto">
              {projectNames.slice(0, 30).map((name, i) => (
                <div
                  key={i}
                  className="flex items-center gap-1.5 px-2 py-1.5 bg-slate-800/60 rounded text-xs text-slate-300 truncate"
                >
                  <FileCode className="w-3 h-3 text-slate-500 shrink-0" />
                  <span className="truncate">{name}</span>
                </div>
              ))}
              {projectNames.length > 30 && (
                <div className="flex items-center justify-center px-2 py-1.5 bg-slate-800/40 rounded text-xs text-slate-500">
                  +{projectNames.length - 30} more
                </div>
              )}
            </div>
          </div>

          {/* Root Path */}
          {rootPath && (
            <div className="pt-2 border-t border-slate-700/50">
              <div className="flex items-center gap-2 text-xs text-slate-500">
                <Terminal className="w-3.5 h-3.5" />
                <code className="font-mono truncate">{rootPath}</code>
              </div>
            </div>
          )}
        </div>
      </div>
    );
  }

  // Fallback to simple display
  return null;
}

/**
 * Routes tool results to the appropriate card component.
 */
function ToolResultCard({ toolResult }: { toolResult: ToolResult }) {
  const { tool_name, result } = toolResult;

  switch (tool_name) {
    case 'analyze_project':
      return <AnalyzeProjectCard result={result} />;
    case 'retrieve_output': {
      const data = parseToolResult(result);
      const projects = data.projects as Array<unknown> || [];
      const projectNames = data.project_names as string[] || [];
      if (projects.length > 0 || projectNames.length > 0) {
        return <RetrieveOutputCard result={result} />;
      }
      return null;
    }
    case 'security_scan':
    case 'check_vulnerabilities':
      return <SecurityScanCard result={result} />;
    default:
      return null;
  }
}

/**
 * Running tool indicator card
 */
function RunningToolCard({ toolName }: { toolName: string }) {
  // Format tool name for display
  const displayName = toolName.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase());

  return (
    <div className="p-4 bg-gradient-to-br from-cyan-500/10 to-blue-500/10 border border-cyan-500/30 rounded-xl animate-pulse">
      <div className="flex items-center gap-3">
        <div className="p-2 bg-cyan-500/20 rounded-lg">
          <Loader2 className="w-5 h-5 text-cyan-400 animate-spin" />
        </div>
        <div>
          <div className="text-sm font-medium text-cyan-300">Running Tool</div>
          <div className="text-white font-semibold">{displayName}</div>
        </div>
      </div>
      <div className="mt-3 flex items-center gap-2">
        <div className="flex-1 h-1 bg-slate-700 rounded-full overflow-hidden">
          <div className="h-full w-1/2 bg-gradient-to-r from-cyan-500 to-blue-500 rounded-full animate-[shimmer_1.5s_ease-in-out_infinite]" />
        </div>
        <span className="text-xs text-slate-500">Processing...</span>
      </div>
    </div>
  );
}

/**
 * Tool Results Sidebar Panel - displays captured tool results
 */
function ToolResultsPanel({ isOpen, onToggle }: { isOpen: boolean; onToggle: () => void }) {
  const { toolResults, currentTool, clearToolResults } = useToolResults();

  const hasActivity = currentTool || toolResults.length > 0;

  return (
    <div className={`${isOpen ? 'w-[400px]' : 'w-0'} transition-all duration-300 overflow-hidden`}>
      <div className="h-full bg-slate-900/70 border-r border-slate-800 flex flex-col min-w-[400px]">
        {/* Header */}
        <div className="px-4 py-3 border-b border-slate-800 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <FolderTree className="w-5 h-5 text-emerald-400" />
            <h2 className="font-semibold text-white">Tool Activity</h2>
            {toolResults.length > 0 && (
              <span className="px-2 py-0.5 bg-emerald-500/20 rounded-full text-xs text-emerald-300">
                {toolResults.length}
              </span>
            )}
            {currentTool && (
              <span className="px-2 py-0.5 bg-cyan-500/20 rounded-full text-xs text-cyan-300 animate-pulse">
                Running
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {toolResults.length > 0 && (
              <button
                onClick={clearToolResults}
                className="px-2 py-1 text-xs text-slate-400 hover:text-white hover:bg-slate-800 rounded transition-colors"
              >
                Clear
              </button>
            )}
            <button
              onClick={onToggle}
              className="p-1 text-slate-400 hover:text-white hover:bg-slate-800 rounded transition-colors"
            >
              <PanelLeftClose className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Results */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4 custom-scrollbar">
          {/* Currently running tool */}
          {currentTool && (
            <RunningToolCard toolName={currentTool} />
          )}

          {/* Completed tool results */}
          {toolResults.map((toolResult) => (
            <ToolResultCard key={toolResult.id || toolResult.tool_name} toolResult={toolResult} />
          ))}

          {/* Empty state */}
          {!hasActivity && (
            <div className="flex flex-col items-center justify-center h-full text-center py-12">
              <FolderTree className="w-12 h-12 text-slate-700 mb-4" />
              <p className="text-slate-500 text-sm">No tool activity yet</p>
              <p className="text-slate-600 text-xs mt-1">Tool calls will appear here as the agent works</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * Floating toggle button when panel is closed
 */
function ToolResultsToggle({ onClick }: { onClick: () => void }) {
  const { toolResults, currentTool } = useToolResults();

  return (
    <button
      onClick={onClick}
      className={`absolute left-4 top-4 z-10 flex items-center gap-2 px-3 py-2 bg-slate-800 hover:bg-slate-700 border rounded-lg text-sm text-slate-300 hover:text-white transition-colors shadow-lg ${
        currentTool ? 'border-cyan-500/50 animate-pulse' : 'border-slate-700'
      }`}
    >
      <PanelLeft className="w-4 h-4" />
      <span>Tools</span>
      {currentTool && (
        <span className="px-1.5 py-0.5 bg-cyan-500/20 rounded-full text-xs text-cyan-300">
          Running
        </span>
      )}
      {!currentTool && toolResults.length > 0 && (
        <span className="px-1.5 py-0.5 bg-emerald-500/20 rounded-full text-xs text-emerald-300">
          {toolResults.length}
        </span>
      )}
    </button>
  );
}

/**
 * Inner component that registers the generative UI hook and captures tool results.
 */
function GenerativeUIRenderer({
  useCoAgentStateRender,
  CopilotChat
}: {
  useCoAgentStateRender: any;
  CopilotChat: ComponentType<any>;
}) {
  const { addToolResults, setCurrentTool } = useToolResults();
  const lastToolRef = useRef<string | null>(null);

  // Register the generative UI renderer for agent state - captures tool results
  useCoAgentStateRender({
    name: "syncable",
    render: ({ state }: { state: AgentState }) => {
      // Track current running tool
      const currentTool = state?.current_tool || null;
      if (currentTool !== lastToolRef.current) {
        lastToolRef.current = currentTool;
        setTimeout(() => {
          setCurrentTool(currentTool);
        }, 0);
      }

      // Forward all tool results to context - deduplication is handled by the context
      // using content-based keys, so it's safe to call this multiple times
      if (state?.tool_results && state.tool_results.length > 0) {
        // Use setTimeout to avoid calling setState during render
        setTimeout(() => {
          addToolResults(state.tool_results);
          if (!state.current_tool) {
            setCurrentTool(null);
          }
        }, 0);
      }
      // Return null - we don't render in chat, we show in sidebar
      return null;
    },
  });

  return (
    <CopilotChat
      className="h-full"
      labels={{
        title: "Syncable Agent",
        initial: "Hi! I'm the Syncable agent. How can I help you today?",
        placeholder: "Type your message...",
      }}
    />
  );
}

/**
 * Chat component that dynamically loads the CopilotKit hooks.
 */
function ChatWithGenerativeUI({ CopilotChat }: { CopilotChat: ComponentType<any> }) {
  const [useCoAgentStateRender, setUseCoAgentStateRender] = useState<any>(null);

  useEffect(() => {
    import("@copilotkit/react-core").then((mod) => {
      setUseCoAgentStateRender(() => mod.useCoAgentStateRender);
    });
  }, []);

  // While loading hook, show basic CopilotChat
  if (!useCoAgentStateRender) {
    return (
      <CopilotChat
        className="h-full"
        labels={{
          title: "Syncable Agent",
          initial: "Hi! I'm the Syncable agent. How can I help you today?",
          placeholder: "Type your message...",
        }}
      />
    );
  }

  return (
    <GenerativeUIRenderer
      useCoAgentStateRender={useCoAgentStateRender}
      CopilotChat={CopilotChat}
    />
  );
}

/**
 * Inner component that uses the tool results context
 */
function AgentChatInner() {
  // Dynamically load CopilotChat on client side only
  const [CopilotChat, setCopilotChat] = useState<ComponentType<any> | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showToolPanel, setShowToolPanel] = useState(true);
  const { settings } = useAgentSettings();

  useEffect(() => {
    import("@copilotkit/react-ui").then((mod) => {
      setCopilotChat(() => mod.CopilotChat);
    });
  }, []);

  return (
    <main className="min-h-screen bg-slate-950 relative overflow-hidden">
      {/* Background */}
      <div className="absolute inset-0 bg-linear-to-br from-slate-950 via-slate-900 to-slate-950" />
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top,rgba(34,211,238,0.1),transparent_50%)]" />

      <div className="relative z-10 h-screen flex flex-col">
        {/* Compact Header */}
        <header className="px-6 py-4 border-b border-slate-800/50">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-xl bg-linear-to-br from-emerald-500/20 to-cyan-600/20 border border-emerald-500/30">
                <Bot className="w-6 h-6 text-emerald-400" />
              </div>
              <h1 className="text-2xl font-bold tracking-tight bg-linear-to-r from-emerald-400 via-cyan-400 to-blue-400 bg-clip-text text-transparent">
                Agent Chat
              </h1>
            </div>
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-2 text-sm text-slate-400">
                <span className="px-2 py-1 bg-slate-800/50 rounded text-xs">
                  {settings.provider === 'openai' ? 'OpenAI' : settings.provider === 'anthropic' ? 'Anthropic' : 'Bedrock'}
                </span>
                <span className="px-2 py-1 bg-slate-800/50 rounded text-xs font-mono truncate max-w-[200px]">
                  {settings.model.split('/').pop()?.split(':')[0] || settings.model}
                </span>
              </div>
              <button
                onClick={() => setShowSettings(true)}
                className="flex items-center gap-2 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-sm text-slate-300 hover:text-white transition-colors"
              >
                <Settings className="w-4 h-4" />
                Settings
              </button>
            </div>
          </div>
        </header>

        {/* Main Content - Sidebar + Chat */}
        <div className="flex-1 flex overflow-hidden">
          {/* Tool Results Sidebar */}
          <ToolResultsPanel isOpen={showToolPanel} onToggle={() => setShowToolPanel(false)} />

          {/* Chat Area */}
          <div className="flex-1 flex flex-col relative">
            {/* Toggle button when panel is closed */}
            {!showToolPanel && (
              <ToolResultsToggle onClick={() => setShowToolPanel(true)} />
            )}

            {/* Chat Container */}
            <div className="flex-1 m-4 bg-slate-900/50 border border-slate-800 rounded-2xl overflow-hidden relative">
              {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}

              {CopilotChat ? (
                <ChatWithGenerativeUI CopilotChat={CopilotChat} />
              ) : (
                <div className="h-full flex items-center justify-center">
                  <Loader2 className="w-8 h-8 text-emerald-400 animate-spin" />
                </div>
              )}
            </div>

            {/* Connection Info */}
            <div className="mx-4 mb-4 px-4 py-2 bg-slate-900/30 border border-slate-800/50 rounded-xl">
              <div className="flex items-center gap-2 text-slate-400 text-xs">
                <Terminal className="w-3 h-3" />
                <span>AG-UI Server: </span>
                <code className="px-1.5 py-0.5 bg-slate-800 rounded text-emerald-400 text-xs">
                  {import.meta.env.VITE_AGENT_URL || "http://localhost:9090"}
                </code>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}

function AgentChat() {
  return (
    <ToolResultsProvider>
      <AgentChatInner />
    </ToolResultsProvider>
  );
}
