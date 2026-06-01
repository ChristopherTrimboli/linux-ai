<script lang="ts">
  import { untrack } from "svelte";
  import type { Config, ProviderStatus } from "../lib/types";
  import * as api from "../lib/api";

  interface Props {
    config: Config;
    providers: ProviderStatus[];
    onclose: () => void;
    onsaved: () => void;
  }
  let { config, providers, onclose, onsaved }: Props = $props();

  // Editable working copies, snapshotted once from the incoming props.
  let draft = $state<Config>(untrack(() => structuredClone($state.snapshot(config))));
  let keyInputs = $state<Record<string, string>>({});
  let status = $state<ProviderStatus[]>(
    untrack(() => structuredClone($state.snapshot(providers))),
  );
  let rootsText = $state(draft.tools.file_roots.join("\n"));
  let saving = $state(false);
  let message = $state("");

  // Older config files may predate the [stt] section.
  if (!draft.stt) draft.stt = { provider: "openai", model: "gpt-4o-transcribe" };

  // Suggested transcription models per provider for the /audio/transcriptions
  // endpoint. The field stays free-text (local servers expose their own names).
  const sttModelsByProvider: Record<string, string[]> = {
    openai: ["gpt-4o-transcribe", "gpt-4o-mini-transcribe", "whisper-1"],
    // OpenRouter requires the fully-qualified slug (e.g. `openai/whisper-1`).
    openrouter: [
      "openai/gpt-4o-transcribe",
      "openai/gpt-4o-mini-transcribe",
      "openai/whisper-1",
      "google/chirp-3",
    ],
    local: ["whisper-large-v3", "whisper-large-v3-turbo", "whisper-1"],
  };
  const sttModelSuggestions = $derived(
    sttModelsByProvider[draft.stt.provider] ?? [
      "gpt-4o-transcribe",
      "whisper-1",
      "whisper-large-v3",
    ],
  );

  function onSttProviderChange() {
    // When switching provider, snap to its first suggested model unless the
    // current one is already a known option for it.
    if (!sttModelSuggestions.includes(draft.stt.model)) {
      draft.stt.model = sttModelSuggestions[0];
    }
  }

  const providerNames = $derived(Object.keys(draft.providers));
  const selectedModels = $derived(draft.providers[draft.default_provider]?.models ?? []);

  async function saveKey(name: string) {
    const key = keyInputs[name]?.trim();
    if (!key) return;
    await api.setApiKey(name, key);
    keyInputs[name] = "";
    status = await api.providerStatus();
    message = `Saved key for ${name}.`;
  }

  async function save() {
    saving = true;
    try {
      draft.tools.file_roots = rootsText
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean);
      draft.max_tokens = Number(draft.max_tokens) || 4096;
      await api.setConfig($state.snapshot(draft));
      onsaved();
      onclose();
    } finally {
      saving = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />
<div class="overlay" onclick={onclose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
    <h2>Settings</h2>
    <p style="color: var(--text-dim); margin-top: 4px">
      Keys are stored in your OS keyring. Config lives in ~/.config/linux-ai/config.toml.
    </p>

    <h3>Default model</h3>
    <div class="field">
      <label for="prov">Provider</label>
      <select id="prov" bind:value={draft.default_provider}>
        {#each providerNames as name}
          <option value={name}>{name}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="model">Model</label>
      {#if selectedModels.length}
        <select id="model" bind:value={draft.default_model}>
          {#each selectedModels as m}
            <option value={m}>{m}</option>
          {/each}
        </select>
      {:else}
        <input id="model" bind:value={draft.default_model} />
      {/if}
    </div>
    <div class="field">
      <label for="maxtok">Max tokens</label>
      <input id="maxtok" type="number" bind:value={draft.max_tokens} />
    </div>

    <h3>Providers &amp; keys</h3>
    {#each status as p (p.name)}
      <div class="provider-row">
        <div class="ph">
          <strong>{p.name}</strong>
          <span class="badge">{p.kind}</span>
          <span class="badge" class:ok={p.has_key}>{p.has_key ? "key set" : "no key"}</span>
        </div>
        <div class="key-row">
          <input
            type="password"
            placeholder="Paste API key…"
            bind:value={keyInputs[p.name]}
          />
          <button onclick={() => saveKey(p.name)}>Save key</button>
        </div>
      </div>
    {/each}

    <h3>Tool access</h3>
    <div class="field inline">
      <input id="auto" type="checkbox" bind:checked={draft.tools.auto_approve} />
      <label for="auto" style="margin: 0">
        Auto-approve tool actions (skip confirmation prompts)
      </label>
    </div>
    <div class="field">
      <label for="roots">Allowed filesystem roots (one per line; empty = home)</label>
      <textarea id="roots" rows="3" bind:value={rootsText}></textarea>
    </div>

    <h3>Voice input (speech-to-text)</h3>
    <p style="color: var(--text-dim); margin-top: 4px">
      Audio is recorded locally and sent to this provider's
      <code>/audio/transcriptions</code> endpoint. Use any OpenAI-compatible STT server.
    </p>
    <div class="field">
      <label for="stt-prov">Provider</label>
      <select id="stt-prov" bind:value={draft.stt.provider} onchange={onSttProviderChange}>
        {#each providerNames as name}
          <option value={name}>{name}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="stt-model">Model</label>
      <input
        id="stt-model"
        list="stt-models"
        bind:value={draft.stt.model}
        placeholder="gpt-4o-transcribe"
      />
      <datalist id="stt-models">
        {#each sttModelSuggestions as m}
          <option value={m}></option>
        {/each}
      </datalist>
    </div>

    {#if message}
      <p style="color: var(--accent-2)">{message}</p>
    {/if}

    <div class="modal-actions">
      <button onclick={onclose}>Cancel</button>
      <button class="primary" onclick={save} disabled={saving}>
        {saving ? "Saving…" : "Save"}
      </button>
    </div>
  </div>
</div>
