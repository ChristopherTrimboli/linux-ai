<script lang="ts">
  interface Props {
    name: string;
    summary?: string;
    output?: string | null;
    isError?: boolean;
    running?: boolean;
  }
  let { name, summary = "", output = null, isError = false, running = false }: Props =
    $props();
</script>

<div class="tool">
  <div class="tool-head">
    <span class="dot" class:running class:error={isError}></span>
    <span class="name">{name}</span>
    <span class="summary">{summary}</span>
    <span class="status">{running ? "running…" : isError ? "error" : "done"}</span>
  </div>
  {#if output}
    <pre class="tool-out" class:error={isError}>{output}</pre>
  {/if}
</div>
