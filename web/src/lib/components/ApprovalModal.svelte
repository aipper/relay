<script lang="ts">
  export let show: boolean = false;
  export let awaiting: any = null;
  export let runTool: string = "";
  export let showArgs: boolean = false;
  export let approvalForSession: boolean = false;
  export let approvalAnswersJson: string = "";
  export let selectedRunId: string = "";
  export let status: string = "";

  export let riskForOpTool: (tool: string | null) => { kind: string; label: string } | null = () => null;

  export let onClose: () => void = () => {};
  export let onSendDecision: (decision: string) => void = () => {};
  export let onToggleApprovalForSession: (v: boolean) => void = () => {};
</script>

{#if show && awaiting}
  {@const risk = riskForOpTool(awaiting.op_tool)}
  <div class="modal-overlay" role="dialog" aria-modal="true">
    <div class="modal">
      <div class="modal-head">
        <div class="modal-title">待审批</div>
        <button class="secondary" on:click={onClose} type="button">关闭</button>
      </div>
      <div class="modal-body">
        <div class="approval-meta">
          <span class="meta-pill">
            <span class="meta-k">tool</span>
            <span class="meta-v">{runTool}</span>
          </span>
          {#if awaiting.op_tool}
            <span class="meta-pill">
              <span class="meta-k">op</span>
              <span class="meta-v"><code>{awaiting.op_tool}</code></span>
            </span>
          {/if}
          {#if risk}
            <span class="risk-pill" data-kind={risk.kind}>{risk.label}</span>
          {/if}
        </div>

        {#if awaiting.op_args_summary}
          <div class="approval-summary"><code>{awaiting.op_args_summary}</code></div>
        {/if}

        {#if awaiting.prompt}
          <div class="approval-prompt">{awaiting.prompt}</div>
        {/if}

        <div class="approval-extra">
          <label class="checkbox">
            <input type="checkbox" checked={approvalForSession} on:change={onToggleApprovalForSession} disabled={!awaiting.op_tool} />
            本会话允许{#if awaiting.op_tool}（<code>{awaiting.op_tool}</code>）{/if}
          </label>
        </div>

        {#if awaiting.questions !== undefined && awaiting.questions !== null}
          <div class="approval-questions">
            <div class="approval-questions-title">需要回答</div>
            <pre class="approval-questions-json">{JSON.stringify(awaiting.questions, null, 2)}</pre>
            <div class="approval-answers-label">answers（JSON）</div>
            <textarea class="approval-answers" rows="5" bind:value={approvalAnswersJson} placeholder={"{}"}></textarea>
          </div>
        {/if}

        {#if awaiting.op_args !== undefined && awaiting.op_args !== null}
          <details class="approval-details" open={showArgs}>
            <summary>参数</summary>
            <pre>{JSON.stringify(awaiting.op_args, null, 2)}</pre>
          </details>
        {/if}
      </div>
      <div class="modal-actions">
        <button class="secondary" on:click={onClose} type="button">取消</button>
        <button on:click={() => { onSendDecision("deny"); onClose(); }} disabled={!selectedRunId || status !== "connected"} type="button">拒绝</button>
        <button on:click={() => { onSendDecision("approve"); onClose(); }} disabled={!selectedRunId || status !== "connected"} type="button">同意</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(17, 24, 39, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    z-index: 100;
  }

  .modal {
    width: 100%;
    max-width: 520px;
    border-radius: 18px;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    padding: 14px;
    box-shadow:
      0 2px 12px rgba(0, 0, 0, 0.4),
      0 18px 50px rgba(0, 0, 0, 0.5);
  }

  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .modal-title {
    font-weight: 900;
    font-size: 14px;
  }

  .modal-body {
    margin-top: 10px;
    color: var(--muted);
  }

  .approval-meta {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }

  .risk-pill {
    display: inline-flex;
    align-items: center;
    padding: 4px 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    font-weight: 900;
    font-size: 12px;
    text-transform: uppercase;
  }

  .risk-pill[data-kind="read"] {
    background: color-mix(in srgb, #38bdf8 12%, transparent);
    border-color: color-mix(in srgb, #38bdf8 25%, transparent);
    color: #38bdf8;
  }

  .risk-pill[data-kind="write"] {
    background: color-mix(in srgb, var(--warning) 12%, transparent);
    border-color: color-mix(in srgb, var(--warning) 25%, transparent);
    color: var(--warning);
  }

  .risk-pill[data-kind="exec"] {
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    border-color: color-mix(in srgb, var(--danger) 25%, transparent);
    color: var(--danger);
  }

  .risk-pill[data-kind="other"] {
    background: color-mix(in srgb, var(--muted) 12%, transparent);
    border-color: color-mix(in srgb, var(--muted) 25%, transparent);
    color: var(--text-secondary);
  }

  .approval-summary {
    margin-top: 10px;
    color: var(--text-strong);
  }

  .approval-summary code {
    font-size: 12px;
  }

  .approval-prompt {
    margin-top: 10px;
    font-size: 13px;
    color: var(--text-strong);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .approval-extra {
    margin-top: 10px;
  }

  .approval-details {
    margin-top: 10px;
  }

  .approval-details summary {
    cursor: pointer;
    font-weight: 900;
    color: var(--text-secondary);
  }

  .modal-actions {
    margin-top: 12px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .meta-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    max-width: 100%;
    min-width: 0;
  }

  .meta-k {
    color: var(--muted);
    font-weight: 900;
  }

  .meta-v {
    color: var(--text-strong);
    font-weight: 900;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .modal-actions .secondary {
    background: var(--bg-surface);
  }
</style>
