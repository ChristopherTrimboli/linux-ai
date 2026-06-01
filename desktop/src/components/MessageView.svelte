<script lang="ts">
  import type { Message } from "../lib/types";
  import { markdown } from "../lib/markdown";
  import ToolCard from "./ToolCard.svelte";

  interface Props {
    message: Message;
  }
  let { message }: Props = $props();

  function summarizeInput(input: unknown): string {
    try {
      const s = JSON.stringify(input);
      return s.length > 80 ? s.slice(0, 80) + "…" : s;
    } catch {
      return "";
    }
  }

  const isUser = $derived(message.role === "user");
  const avatar = $derived(message.role === "user" ? "you" : "ai");

  // Plain-text version of the bubble for copying (text blocks + tool results).
  const copyText = $derived(
    message.content
      .map((b) =>
        b.type === "text" ? b.text : b.type === "tool_result" ? b.content : "",
      )
      .filter(Boolean)
      .join("\n\n")
      .trim(),
  );

  let copied = $state(false);
  let resetTimer: ReturnType<typeof setTimeout> | undefined;

  async function copy() {
    const text = copyText;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // Fallback for webviews without async clipboard access.
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      try {
        document.execCommand("copy");
      } catch {
        /* ignore */
      }
      document.body.removeChild(ta);
    }
    copied = true;
    clearTimeout(resetTimer);
    resetTimer = setTimeout(() => (copied = false), 1500);
  }
</script>

{#snippet copyButton()}
  {#if copyText}
    <button
      class="copy-btn"
      class:copied
      onclick={copy}
      title="Copy message"
      aria-label="Copy message"
    >
      {copied ? "Copied" : "Copy"}
    </button>
  {/if}
{/snippet}

{#if message.role !== "tool"}
  <div class="msg" class:user={isUser} class:assistant={!isUser}>
    <div class="avatar">{avatar}</div>
    <div class="body">
      {#each message.content as block}
        {#if block.type === "text"}
          {#if isUser}
            <p style="white-space: pre-wrap">{block.text}</p>
          {:else}
            <div use:markdown={block.text}></div>
          {/if}
        {:else if block.type === "tool_use"}
          <ToolCard name={block.name} summary={summarizeInput(block.input)} />
        {/if}
      {/each}
    </div>
    {@render copyButton()}
  </div>
{:else}
  <!-- tool result message: render each result as a card aligned with the assistant -->
  <div class="msg assistant">
    <div class="avatar" style="visibility: hidden">ai</div>
    <div class="body">
      {#each message.content as block}
        {#if block.type === "tool_result"}
          <ToolCard
            name="result"
            output={block.content}
            isError={block.is_error}
          />
        {/if}
      {/each}
    </div>
    {@render copyButton()}
  </div>
{/if}
