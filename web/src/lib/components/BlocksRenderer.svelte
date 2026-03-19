<script lang="ts">
  import type { UiBlock } from "../blocks/types";

  export let blocks: UiBlock[];
  export let renderMarkdownBasic: (src: string) => string;
  export let formatAbsTime: (ts: string) => string;
  export let copyText: (text: string) => void | Promise<void>;
  export let runTool: string;

  function doCopy(text: string) {
    void copyText(text);
  }
</script>

{#each blocks as b (b.id)}
  {#if b.type === "tool_pair"}
    <div class="chat-row" data-role="system">
      <details open class="tool-card">
        <summary>
          <code>{b.label}</code>
          {#if b.actor}<code>actor={b.actor}</code>{/if}
          {#if b.ok === true}
            <span style="color:#065f46">ok</span>
          {:else if b.ok === false}
            <span style="color:#b91c1c">error</span>
          {/if}
        </summary>
        <div class="tool-card-body">
          <div class="tool-card-actions">
            <button
              class="secondary"
              type="button"
              on:click={() => doCopy(b.call_json ?? b.call_details ?? "")}
            >
              复制 call
            </button>
            <button
              class="secondary"
              type="button"
              on:click={() => doCopy(b.result_json ?? b.result_details ?? "")}
            >
              复制 result
            </button>
            {#if b.request_id}
              <button class="secondary" type="button" on:click={() => doCopy(b.request_id || "")}>复制 request_id</button>
            {/if}
          </div>

          <div class="tool-card-label">call</div>
          {#if b.call_json}
            <pre class="tool-json">{b.call_json}</pre>
          {:else}
            {@html renderMarkdownBasic(b.call_details || "")}
          {/if}

          <div class="tool-card-label" style="margin-top:10px">result</div>
          {#if b.result_json}
            <pre class="tool-json">{b.result_json}</pre>
          {:else}
            {@html renderMarkdownBasic(b.result_details || "")}
          {/if}
        </div>
      </details>
      <div class="chat-ts">{formatAbsTime(b.ts)}</div>
    </div>
  {:else}
    {#if b.kind === "run.permission_requested"}
      <div class="chat-row" data-role="system">
        <div class="approval-card">
          <div class="approval-card-top">
            <span class="meta-pill">
              <span class="meta-k">tool</span>
              <span class="meta-v">{runTool}</span>
            </span>
          </div>
          {#if b.text}
            <div class="approval-card-prompt">{b.text}</div>
          {/if}
        </div>
        <div class="chat-ts">{formatAbsTime(b.ts)}</div>
      </div>
    {:else if b.kind === "run.awaiting_input"}
      <div class="chat-row" data-role="system">
        <div class="awaiting-card">
          {#if b.text}
            <div class="awaiting-card-prompt">{b.text}</div>
          {/if}
        </div>
        <div class="chat-ts">{formatAbsTime(b.ts)}</div>
      </div>
    {:else}
      <div class="chat-row" data-role={b.role}>
        {#if b.role === "system"}
          <div class="chat-system">
            {@html renderMarkdownBasic(b.text || "")}
          </div>
        {:else}
          <div class="chat-bubble" data-role={b.role}>
            {@html renderMarkdownBasic(b.text || "")}
          </div>
        {/if}
        <div class="chat-ts">{formatAbsTime(b.ts)}</div>
      </div>
    {/if}
  {/if}
{/each}
