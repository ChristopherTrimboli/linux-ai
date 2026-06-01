<script lang="ts">
  import * as api from "./lib/api";
  import type { AgentEvent, Config, Conversation, Message, ProviderStatus } from "./lib/types";
  import { markdown } from "./lib/markdown";
  import Sidebar from "./components/Sidebar.svelte";
  import MessageView from "./components/MessageView.svelte";
  import ToolCard from "./components/ToolCard.svelte";
  import Settings from "./components/Settings.svelte";

  type LiveTool = {
    id: string;
    name: string;
    summary: string;
    output: string | null;
    isError: boolean;
    running: boolean;
  };

  let conversations = $state<Conversation[]>([]);
  let activeId = $state<string | null>(null);
  let messages = $state<Message[]>([]);

  let config = $state<Config | null>(null);
  let providers = $state<ProviderStatus[]>([]);
  let selProvider = $state("");
  let selModel = $state("");

  let input = $state("");
  let sending = $state(false);
  let errorMsg = $state("");

  let recording = $state(false);
  let transcribing = $state(false);

  let liveText = $state("");
  let liveTools = $state<LiveTool[]>([]);
  let approvals = $state<{ id: string; tool: string; summary: string }[]>([]);

  let showSettings = $state(false);
  let thread = $state<HTMLDivElement | null>(null);
  let composerEl = $state<HTMLTextAreaElement | null>(null);

  $effect(() => {
    // auto-grow the composer to fit its content (and shrink back when cleared)
    void input;
    if (composerEl) {
      composerEl.style.height = "auto";
      composerEl.style.height = Math.min(composerEl.scrollHeight, 200) + "px";
    }
  });

  const activeConvo = $derived(conversations.find((c) => c.id === activeId) ?? null);
  const modelOptions = $derived(providers.find((p) => p.name === selProvider)?.models ?? []);

  $effect(() => {
    // autoscroll on new content
    void messages.length;
    void liveText;
    void liveTools.length;
    if (thread) thread.scrollTop = thread.scrollHeight;
  });

  async function boot() {
    config = await api.getConfig();
    providers = await api.providerStatus();
    selProvider = config.default_provider;
    selModel = config.default_model;
    await refreshConversations();
    if (conversations.length) {
      await selectConversation(conversations[0].id);
    }
  }
  boot();

  async function refreshConversations() {
    conversations = await api.listConversations();
  }

  async function selectConversation(id: string) {
    activeId = id;
    messages = await api.loadMessages(id);
    liveText = "";
    liveTools = [];
    approvals = [];
  }

  async function newChat() {
    const convo = await api.createConversation();
    await refreshConversations();
    await selectConversation(convo.id);
  }

  async function removeChat(id: string) {
    await api.deleteConversation(id);
    if (activeId === id) {
      activeId = null;
      messages = [];
    }
    await refreshConversations();
  }

  function onProviderChange() {
    const models = providers.find((p) => p.name === selProvider)?.models ?? [];
    if (models.length && !models.includes(selModel)) selModel = models[0];
  }

  function handleEvent(ev: AgentEvent) {
    switch (ev.type) {
      case "text_delta":
        liveText += ev.data;
        break;
      case "tool_started":
        liveTools = [
          ...liveTools,
          {
            id: ev.data.id,
            name: ev.data.name,
            summary: ev.data.summary,
            output: null,
            isError: false,
            running: true,
          },
        ];
        break;
      case "tool_completed":
        liveTools = liveTools.map((t) =>
          t.id === ev.data.id
            ? { ...t, output: ev.data.output, isError: ev.data.is_error, running: false }
            : t,
        );
        break;
      case "approval_required":
        approvals = [...approvals, ev.data];
        break;
      case "error":
        errorMsg = ev.data;
        break;
      case "done":
      case "usage":
        break;
    }
  }

  async function decide(id: string, approved: boolean) {
    await api.respondApproval(id, approved);
    approvals = approvals.filter((a) => a.id !== id);
  }

  async function send() {
    const text = input.trim();
    if (!text || sending) return;

    if (!activeId) {
      const convo = await api.createConversation();
      await refreshConversations();
      activeId = convo.id;
    }
    const convoId = activeId!;

    // optimistic user bubble
    messages = [...messages, { role: "user", content: [{ type: "text", text }] }];
    input = "";
    errorMsg = "";
    sending = true;
    liveText = "";
    liveTools = [];
    approvals = [];

    // name a fresh conversation from the first message
    if (activeConvo && activeConvo.title === "New chat") {
      await api.renameConversation(convoId, text.slice(0, 60));
    }

    try {
      await api.sendMessage(convoId, text, selProvider, selModel, handleEvent);
    } catch (e) {
      errorMsg = String(e);
    }

    messages = await api.loadMessages(convoId);
    liveText = "";
    liveTools = [];
    approvals = [];
    sending = false;
    await refreshConversations();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  async function toggleRecording() {
    if (transcribing) return;
    if (recording) {
      recording = false;
      transcribing = true;
      try {
        const text = await api.stopRecording();
        if (text) {
          input = input.trim() ? `${input.trim()} ${text}` : text;
          composerEl?.focus();
        }
      } catch (e) {
        errorMsg = `Transcription failed: ${e}`;
      } finally {
        transcribing = false;
      }
    } else {
      errorMsg = "";
      try {
        await api.startRecording();
        recording = true;
      } catch (e) {
        errorMsg = `Could not start recording: ${e}`;
      }
    }
  }

  async function onSettingsSaved() {
    config = await api.getConfig();
    providers = await api.providerStatus();
  }
</script>

<div class="app">
  <Sidebar
    {conversations}
    {activeId}
    onselect={selectConversation}
    onnew={newChat}
    ondelete={removeChat}
    onsettings={() => (showSettings = true)}
  />

  <main class="main">
    <div class="topbar">
      <select bind:value={selProvider} onchange={onProviderChange}>
        {#each providers as p}
          <option value={p.name}>{p.name}</option>
        {/each}
      </select>
      {#if modelOptions.length}
        <select bind:value={selModel}>
          {#each modelOptions as m}
            <option value={m}>{m}</option>
          {/each}
        </select>
      {:else}
        <input style="width: 200px" bind:value={selModel} placeholder="model" />
      {/if}
      <span class="spacer"></span>
      {#if !providers.find((p) => p.name === selProvider)?.has_key}
        <span style="color: var(--text-dim)">no API key — open Settings</span>
      {/if}
    </div>

    {#if !activeId && messages.length === 0}
      <div class="empty">
        <div>
          <div class="big">Linux AI</div>
          <div>Ask anything, or let it act on your computer with your approval.</div>
        </div>
      </div>
    {:else}
      <div class="messages" bind:this={thread}>
        <div class="thread">
          {#each messages as message, i (i)}
            <MessageView {message} />
          {/each}

          {#if sending || liveText || liveTools.length || approvals.length}
            <div class="msg assistant">
              <div class="avatar">ai</div>
              <div class="body">
                {#if liveText}
                  <div use:markdown={liveText}></div>
                {:else if !liveTools.length && !approvals.length}
                  <p style="color: var(--text-dim)">thinking…</p>
                {/if}

                {#each liveTools as t (t.id)}
                  <ToolCard
                    name={t.name}
                    summary={t.summary}
                    output={t.output}
                    isError={t.isError}
                    running={t.running}
                  />
                {/each}

                {#each approvals as a (a.id)}
                  <div class="approval">
                    <div class="q">
                      Allow <b>{a.tool}</b>: <code>{a.summary}</code>?
                    </div>
                    <div class="actions">
                      <button class="primary" onclick={() => decide(a.id, true)}>
                        Approve
                      </button>
                      <button class="danger" onclick={() => decide(a.id, false)}>Deny</button>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if errorMsg}
            <div class="msg assistant">
              <div class="avatar" style="background: var(--danger)">!</div>
              <div class="body"><p style="color: var(--danger)">{errorMsg}</p></div>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <div class="composer">
      <div class="row">
        <button
          class="mic"
          class:recording
          onclick={toggleRecording}
          disabled={transcribing}
          title={recording ? "Stop & transcribe" : "Record voice"}
          aria-label={recording ? "Stop recording and transcribe" : "Record voice"}
        >
          {#if transcribing}
            <span class="spin">◌</span>
          {:else if recording}
            ■
          {:else}
            ●
          {/if}
        </button>
        <textarea
          rows="1"
          placeholder={recording
            ? "Listening… click ■ to transcribe"
            : transcribing
              ? "Transcribing…"
              : "Message Linux AI…  (Enter to send, Shift+Enter for newline)"}
          bind:value={input}
          bind:this={composerEl}
          onkeydown={onKey}
        ></textarea>
        <button class="primary" onclick={send} disabled={sending || !input.trim()}>
          {sending ? "…" : "Send"}
        </button>
      </div>
    </div>
  </main>

  {#if showSettings && config}
    <Settings
      {config}
      {providers}
      onclose={() => (showSettings = false)}
      onsaved={onSettingsSaved}
    />
  {/if}
</div>
