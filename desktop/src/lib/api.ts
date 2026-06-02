import { invoke, Channel } from "@tauri-apps/api/core";
import type { AgentEvent, Config, Conversation, Message, ProviderStatus } from "./types";

export const listConversations = () => invoke<Conversation[]>("list_conversations");

export const createConversation = () => invoke<Conversation>("create_conversation");

export const loadMessages = (id: string) => invoke<Message[]>("load_messages", { id });

export const renameConversation = (id: string, title: string) =>
  invoke<void>("rename_conversation", { id, title });

export const deleteConversation = (id: string) =>
  invoke<void>("delete_conversation", { id });

export const getConfig = () => invoke<Config>("get_config");

export const setConfig = (config: Config) => invoke<void>("set_config", { config });

export const providerStatus = () => invoke<ProviderStatus[]>("provider_status");

export const setApiKey = (provider: string, key: string) =>
  invoke<void>("set_api_key", { provider, key });

export const respondApproval = (id: string, approved: boolean) =>
  invoke<void>("respond_approval", { id, approved });

export const stopGeneration = () => invoke<void>("stop_generation");

export const startRecording = () => invoke<void>("start_recording");

export const stopRecording = () => invoke<string>("stop_recording");

export const cancelRecording = () => invoke<void>("cancel_recording");

export function sendMessage(
  conversationId: string,
  text: string,
  provider: string | null,
  model: string | null,
  onEvent: (event: AgentEvent) => void,
): Promise<void> {
  const channel = new Channel<AgentEvent>();
  channel.onmessage = onEvent;
  return invoke<void>("send_message", {
    conversationId,
    text,
    provider,
    model,
    onEvent: channel,
  });
}
