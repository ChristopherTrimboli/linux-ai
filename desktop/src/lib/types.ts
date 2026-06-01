export type Role = "system" | "user" | "assistant" | "tool";

export type ContentBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; id: string; name: string; input: unknown }
  | { type: "tool_result"; tool_use_id: string; content: string; is_error: boolean };

export interface Message {
  role: Role;
  content: ContentBlock[];
}

export interface Conversation {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
}

export interface ProviderStatus {
  name: string;
  kind: string;
  base_url: string;
  has_key: boolean;
  models: string[];
}

export type ProviderKind = "openai" | "anthropic";

export interface ProviderConfig {
  kind: ProviderKind;
  base_url?: string | null;
  api_key_env?: string | null;
  api_key?: string | null;
  models: string[];
}

export interface ToolPolicy {
  auto_approve: boolean;
  file_roots: string[];
  shell_deny: string[];
}

export interface Config {
  default_provider: string;
  default_model: string;
  max_tokens: number;
  providers: Record<string, ProviderConfig>;
  tools: ToolPolicy;
}

export type AgentEvent =
  | { type: "text_delta"; data: string }
  | { type: "tool_started"; data: { id: string; name: string; summary: string } }
  | {
      type: "tool_completed";
      data: { id: string; name: string; output: string; is_error: boolean };
    }
  | { type: "approval_required"; data: { id: string; tool: string; summary: string } }
  | { type: "usage"; data: { input_tokens: number; output_tokens: number } }
  | { type: "done" }
  | { type: "error"; data: string };
