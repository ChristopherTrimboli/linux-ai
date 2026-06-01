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
