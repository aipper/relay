<script lang="ts">
  export let show: boolean = false;
  export let runId: string = "";
  export let status: string = "";

  export let onClose: () => void = () => {};
  export let onStop: (signal: string) => void = () => {};
</script>

{#if show}
  <div class="modal-overlay" role="dialog" aria-modal="true">
    <div class="modal">
      <div class="modal-head">
        <div class="modal-title">停止会话</div>
        <button class="secondary" on:click={onClose} type="button">关闭</button>
      </div>
      <div class="modal-body">
        <code>{runId}</code>
      </div>
      <div class="modal-actions">
        <button class="secondary" on:click={onClose} type="button">取消</button>
        <button
          on:click={() => { onStop("term"); onClose(); }}
          disabled={!runId || status !== "connected"}
          type="button"
        >
          停止
        </button>
        <button
          class="danger"
          on:click={() => { onStop("kill"); onClose(); }}
          disabled={!runId || status !== "connected"}
          type="button"
        >
          强制停止
        </button>
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
    background: rgba(255, 255, 255, 0.98);
    padding: 14px;
    box-shadow:
      0 2px 12px rgba(0, 0, 0, 0.16),
      0 18px 50px rgba(0, 0, 0, 0.22);
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

  .modal-actions {
    margin-top: 12px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .modal-actions .secondary {
    background: linear-gradient(145deg, rgba(255, 255, 255, 0.92), rgba(238, 246, 255, 0.86));
  }

  button.danger {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.26);
    color: #991b1b;
  }
</style>
