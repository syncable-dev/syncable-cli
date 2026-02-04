/**
 * CopilotKit Provider Wrapper
 *
 * Configures CopilotKit to connect to the syncable-cli AG-UI server.
 * The agent endpoint can be customized via VITE_AGENT_URL environment variable.
 *
 * Note: CopilotKit is loaded client-side only to avoid SSR CSS import issues.
 */
import { ReactNode, useEffect, useState, type ComponentType } from "react";
import { AgentSettingsProvider, useAgentSettings } from "./agent-settings";

/**
 * AG-UI server endpoint.
 * Default: http://localhost:9090 (local development)
 * Override with VITE_AGENT_URL environment variable.
 */
const AGENT_URL = typeof window !== 'undefined'
  ? (import.meta.env.VITE_AGENT_URL || "http://localhost:9090")
  : "http://localhost:9090";

interface CopilotKitWrapperProps {
  children: ReactNode;
}

/**
 * Inner wrapper that uses agent settings context
 */
function CopilotKitInner({ children }: { children: ReactNode }) {
  const [CopilotKit, setCopilotKit] = useState<ComponentType<any> | null>(null);
  const { settings } = useAgentSettings();

  useEffect(() => {
    // Import CopilotKit and styles only on client side
    Promise.all([
      import("@copilotkit/react-core"),
      import("@copilotkit/react-ui/styles.css"),
    ]).then(([mod]) => {
      setCopilotKit(() => mod.CopilotKit);
    });
  }, []);

  // On server or before CopilotKit loads, just render children
  if (!CopilotKit) {
    return <>{children}</>;
  }

  // Build properties to forward to the agent
  const forwardedProps = {
    provider: settings.provider,
    model: settings.model,
    apiKey: settings.apiKey,
    awsRegion: settings.awsRegion,
  };

  return (
    <CopilotKit
      runtimeUrl={AGENT_URL}
      properties={forwardedProps}
      agent="syncable"
    >
      {children}
    </CopilotKit>
  );
}

/**
 * Wraps the application with CopilotKit provider configured for AG-UI server.
 * Only renders on client-side to avoid SSR issues with CSS imports.
 *
 * Usage:
 * ```tsx
 * <CopilotKitWrapper>
 *   <App />
 * </CopilotKitWrapper>
 * ```
 */
export function CopilotKitWrapper({ children }: CopilotKitWrapperProps) {
  return (
    <AgentSettingsProvider>
      <CopilotKitInner>{children}</CopilotKitInner>
    </AgentSettingsProvider>
  );
}
