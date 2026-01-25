/**
 * CopilotKit Provider Wrapper
 *
 * Configures CopilotKit to connect to the syncable-cli AG-UI server.
 * The agent endpoint can be customized via VITE_AGENT_URL environment variable.
 */
import { CopilotKit } from "@copilotkit/react-core";
import { ReactNode } from "react";

/**
 * AG-UI server endpoint.
 * Default: http://localhost:9090 (local development)
 * Override with VITE_AGENT_URL environment variable.
 */
const AGENT_URL = import.meta.env.VITE_AGENT_URL || "http://localhost:9090";

interface CopilotKitWrapperProps {
  children: ReactNode;
}

/**
 * Wraps the application with CopilotKit provider configured for AG-UI server.
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
    <CopilotKit runtimeUrl={AGENT_URL}>
      {children}
    </CopilotKit>
  );
}
