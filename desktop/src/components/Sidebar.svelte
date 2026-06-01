<script lang="ts">
  import type { Conversation } from "../lib/types";

  interface Props {
    conversations: Conversation[];
    activeId: string | null;
    onselect: (id: string) => void;
    onnew: () => void;
    ondelete: (id: string) => void;
    onsettings: () => void;
  }

  let { conversations, activeId, onselect, onnew, ondelete, onsettings }: Props = $props();
</script>

<aside class="sidebar">
  <div class="sidebar-head">
    <span class="logo"><span class="mark">&gt;_</span> Linux AI</span>
  </div>
  <div style="padding: 10px 10px 4px">
    <button class="primary" style="width: 100%" onclick={onnew}>+ New chat</button>
  </div>
  <div class="convos">
    {#each conversations as convo (convo.id)}
      <div
        class="convo"
        class:active={convo.id === activeId}
        onclick={() => onselect(convo.id)}
        role="button"
        tabindex="0"
        onkeydown={(e) => e.key === "Enter" && onselect(convo.id)}
      >
        <span class="title">{convo.title || "Untitled"}</span>
        <button
          class="del"
          title="Delete"
          onclick={(e) => {
            e.stopPropagation();
            ondelete(convo.id);
          }}>✕</button
        >
      </div>
    {/each}
  </div>
  <div class="sidebar-foot">
    <button style="width: 100%" onclick={onsettings}>⚙ Settings</button>
  </div>
</aside>
