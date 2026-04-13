<script lang="ts">
  export let awaiting: any = null;
  export let status: string = "";

  export let onSendInput: (text: string) => void = () => {};

  let inlineAwaitingText = "";
</script>

{#if awaiting}
  <div class="awaiting-card">
    {#if awaiting.prompt}
      <div class="awaiting-card-prompt">{awaiting.prompt}</div>
    {/if}
    <div class="awaiting-card-footer">
      <textarea
        class="awaiting-card-input"
        rows="2"
        bind:value={inlineAwaitingText}
        disabled={status !== "connected"}
        on:keydown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            const text = (inlineAwaitingText || "").trimEnd();
            if (!text.trim()) return;
            onSendInput(text.endsWith("\n") ? text : `${text}\n`);
            inlineAwaitingText = "";
          }
        }}
      ></textarea>
      <div class="awaiting-card-actions">
        <button class="secondary" type="button" disabled={status !== "connected"} on:click={() => onSendInput("y\n")}>y</button>
        <button class="secondary" type="button" disabled={status !== "connected"} on:click={() => onSendInput("n\n")}>n</button>
        <button class="secondary" type="button" disabled={status !== "connected"} on:click={() => onSendInput("continue\n")}>continue</button>
        <button
          type="button"
          disabled={status !== "connected" || !inlineAwaitingText.trim()}
          on:click={() => {
            const text = (inlineAwaitingText || "").trimEnd();
            if (!text.trim()) return;
            onSendInput(text.endsWith("\n") ? text : `${text}\n`);
            inlineAwaitingText = "";
          }}
        >
          发送
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .awaiting-card {
    width: 100%;
    border-radius: var(--radius-lg);
    border: 1px solid rgba(249, 115, 22, 0.28);
    background: linear-gradient(165deg, rgba(255, 247, 237, 0.95), rgba(254, 243, 199, 0.72));
    padding: 10px 12px;
  }

  .awaiting-card-prompt {
    font-size: 12px;
    color: #9a3412;
    word-break: break-word;
    white-space: pre-wrap;
  }

  .awaiting-card-footer {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    text-align: left;
  }

  .awaiting-card-input {
    width: 100%;
    font-size: 12px;
  }

  .awaiting-card-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .awaiting-card-actions .secondary {
    background: linear-gradient(145deg, rgba(255, 255, 255, 0.92), rgba(238, 246, 255, 0.86));
  }
</style>
