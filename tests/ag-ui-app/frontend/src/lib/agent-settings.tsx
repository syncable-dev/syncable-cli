/**
 * Agent Settings Context
 *
 * Provides provider, model, and API key configuration for the agent.
 * Settings are persisted to localStorage.
 */
import { createContext, useContext, useState, useEffect, ReactNode } from 'react';

export type Provider = 'openai' | 'anthropic' | 'bedrock';

export interface AgentSettings {
  provider: Provider;
  model: string;
  apiKey: string;
  awsRegion?: string;
}

interface AgentSettingsContextType {
  settings: AgentSettings;
  setProvider: (provider: Provider) => void;
  setModel: (model: string) => void;
  setApiKey: (apiKey: string) => void;
  setAwsRegion: (region: string) => void;
  availableModels: { id: string; name: string }[];
}

const STORAGE_KEY = 'syncable-agent-settings';

const DEFAULT_SETTINGS: AgentSettings = {
  provider: 'openai',
  model: 'gpt-5.2',
  apiKey: '',
  awsRegion: 'us-east-1',
};

const MODELS_BY_PROVIDER: Record<Provider, { id: string; name: string }[]> = {
  openai: [
    { id: 'gpt-5.2', name: 'GPT-5.2 - Latest reasoning model (Dec 2025)' },
    { id: 'gpt-5.2-mini', name: 'GPT-5.2 Mini - Fast and affordable' },
    { id: 'gpt-4o', name: 'GPT-4o - Multimodal workhorse' },
    { id: 'o1-preview', name: 'o1-preview - Advanced reasoning' },
  ],
  anthropic: [
    { id: 'claude-opus-4-5-20251101', name: 'Claude Opus 4.5 - Most capable (Nov 2025)' },
    { id: 'claude-sonnet-4-5-20250929', name: 'Claude Sonnet 4.5 - Balanced (Sep 2025)' },
    { id: 'claude-haiku-4-5-20251001', name: 'Claude Haiku 4.5 - Fast (Oct 2025)' },
    { id: 'claude-sonnet-4-20250514', name: 'Claude Sonnet 4 - Previous gen' },
  ],
  bedrock: [
    { id: 'global.anthropic.claude-opus-4-5-20251101-v1:0', name: 'Claude Opus 4.5 - Most capable (Nov 2025)' },
    { id: 'global.anthropic.claude-sonnet-4-5-20250929-v1:0', name: 'Claude Sonnet 4.5 - Balanced (Sep 2025)' },
    { id: 'global.anthropic.claude-haiku-4-5-20251001-v1:0', name: 'Claude Haiku 4.5 - Fast (Oct 2025)' },
    { id: 'global.anthropic.claude-sonnet-4-20250514-v1:0', name: 'Claude Sonnet 4 - Previous gen' },
  ],
};

const AgentSettingsContext = createContext<AgentSettingsContextType | null>(null);

export function AgentSettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<AgentSettings>(DEFAULT_SETTINGS);

  // Load from localStorage on mount
  useEffect(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        setSettings({ ...DEFAULT_SETTINGS, ...parsed });
      }
    } catch (e) {
      console.error('Failed to load agent settings:', e);
    }
  }, []);

  // Save to localStorage on change
  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    } catch (e) {
      console.error('Failed to save agent settings:', e);
    }
  }, [settings]);

  const setProvider = (provider: Provider) => {
    setSettings(prev => ({
      ...prev,
      provider,
      model: MODELS_BY_PROVIDER[provider][0].id, // Reset to first model
    }));
  };

  const setModel = (model: string) => {
    setSettings(prev => ({ ...prev, model }));
  };

  const setApiKey = (apiKey: string) => {
    setSettings(prev => ({ ...prev, apiKey }));
  };

  const setAwsRegion = (awsRegion: string) => {
    setSettings(prev => ({ ...prev, awsRegion }));
  };

  const availableModels = MODELS_BY_PROVIDER[settings.provider];

  return (
    <AgentSettingsContext.Provider
      value={{
        settings,
        setProvider,
        setModel,
        setApiKey,
        setAwsRegion,
        availableModels,
      }}
    >
      {children}
    </AgentSettingsContext.Provider>
  );
}

export function useAgentSettings() {
  const context = useContext(AgentSettingsContext);
  if (!context) {
    throw new Error('useAgentSettings must be used within AgentSettingsProvider');
  }
  return context;
}
