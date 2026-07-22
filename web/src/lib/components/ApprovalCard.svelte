<script lang="ts">
  export let awaiting: any = null;
  export let runTool: string = "";
  export let status: string = "";
  export let approvalForSession: boolean = false;
  export let approvalAnswersJson: string = "";

  export let onSendDecision: (decision: string) => void = () => {};
  export let onToggleApprovalForSession: () => void = () => {};
</script>

{#if awaiting}
  <div class="approval-card">
    <div class="approval-card-top">
      <span class="meta-pill">
        <span class="meta-k">tool</span>
        <span class="meta-v">{runTool}</span>
      </span>
      {#if awaiting.op_tool}
        <span class="session-op">{awaiting.op_tool}</span>
      {/if}
      {#if awaiting.op_args_summary}
        <span class="session-op-args">{awaiting.op_args_summary}</span>
      {/if}
    </div>
    {#if awaiting.prompt}
      <div class="approval-card-prompt">{awaiting.prompt}</div>
    {/if}

    <div class="approval-card-footer">
      <label class="checkbox">
        <input type="checkbox" checked={approvalForSession} on:change={onToggleApprovalForSession} disabled={!awaiting.op_tool || status !== "connected"} />
        本会话允许{#if awaiting.op_tool}（<code>{awaiting.op_tool}</code>）{/if}
      </label>

      {#if awaiting.questions !== undefined && awaiting.questions !== null}
        <div class="approval-questions">
          <div class="approval-questions-title">需要回答</div>
          <pre class="approval-questions-json">{JSON.stringify(awaiting.questions, null, 2)}</pre>
          <div class="approval-answers-label">answers（JSON）</div>
          <textarea
            class="approval-answers"
            rows="5"
            bind:value={approvalAnswersJson}
            placeholder={"{}"}
            disabled={status !== "connected"}
          ></textarea>
        </div>
      {/if}

      <div class="approval-card-actions">
        <button class="secondary" type="button" disabled={status !== "connected"} on:click={() => onSendDecision("deny")}>拒绝</button>
        <button type="button" disabled={status !== "connected"} on:click={() => onSendDecision("approve")}>同意</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .approval-card {
    width: 100%;
    border-radius: var(--radius-lg);
    border: 1px solid rgba(249, 115, 22, 0.28);
    background: linear-gradient(165deg, rgba(255, 247, 237, 0.95), rgba(254, 243, 199, 0.74));
    padding: 10px 12px;
  }

  .approval-card-top {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }

  .approval-card-prompt {
    margin-top: 8px;
    font-size: 12px;
    color: var(--warning);
    word-break: break-word;
    white-space: pre-wrap;
  }

  .approval-card-footer {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    text-align: left;
  }

  .approval-card-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  .approval-questions-title {
    font-size: 12px;
    color: var(--warning);
    font-weight: 800;
    margin-bottom: 6px;
  }

  .approval-questions-json {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid rgba(249, 115, 22, 0.25);
    background: var(--bg-canvas);
    font-size: 12px;
    white-space: pre-wrap;
    word-break: break-word;
    overflow: auto;
    max-height: 240px;
  }

  .approval-answers-label {
    display: block;
    margin-top: 10px;
    font-size: 12px;
    color: var(--warning);
    font-weight: 800;
  }

  .approval-answers {
    width: 100%;
    margin-top: 6px;
    font-size: 12px;
  }

  .session-op,
  .session-op-args {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    font-size: 12px;
    background: var(--bg-canvas);
    border: 1px solid color-mix(in srgb, var(--accent) 25%, transparent);
    padding: 2px 6px;
    border-radius: 8px;
  }

  .approval-card-actions .secondary {
    background: var(--bg-surface);
  }
</style>
