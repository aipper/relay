<script lang="ts">
  export let show: boolean = false;
  export let text: string = "";
  export let selectedRunId: string = "";
  export let status: string = "";

  export let onClose: () => void = () => {};
  export let onSend: () => void = () => {};
  export let onQuickInput: (text: string) => void = () => {};

  let textareaEl: HTMLTextAreaElement;
  let localText = "";

  $: localText = text;

  function handleSend() {
    onSend();
  }
</script>

{#if show}
  <div class="modal-overlay" role="dialog" aria-modal="true">
    <div class="modal">
      <div class="modal-head">
        <div class="modal-title">输入</div>
        <button class="secondary" on:click={onClose} type="button">关闭</button>
      </div>
      <div class="modal-body">
        <textarea
          class="input-textarea"
          bind:this={textareaEl}
          bind:value={localText}
          placeholder=""
          rows="3"
        ></textarea>
        <div class="quick-inputs">
          <button class="secondary" on:click={() => onQuickInput("y\n")} type="button">y</button>
          <button class="secondary" on:click={() => onQuickInput("n\n")} type="button">n</button>
          <button class="secondary" on:click={() => onQuickInput("continue\n")} type="button">continue</button>
        </div>
      </div>
      <div class="modal-actions">
        <button class="secondary" on:click={onClose} type="button">取消</button>
        <button on:click={handleSend} disabled={!selectedRunId || status !== "connected"} type="button">发送</button>
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

  .input-textarea {
    width: 100%;
    min-height: 96px;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: rgba(255, 255, 255, 0.9);
    font: inherit;
    font-size: 13px;
    color: var(--text-strong);
    box-sizing: border-box;
    resize: vertical;
  }

  .quick-inputs {
    margin-top: 10px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
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
</style>
