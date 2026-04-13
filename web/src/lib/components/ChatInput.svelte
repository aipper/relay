<script lang="ts">
  export let selectedRunId: string = "";
  export let status: string = "";
  export let selectedRunReady: boolean = false;
  export let chatInputText: string = "";
  export let selectedAwaiting: any = null;

  export let awaitingIsApproval: (a: any) => boolean = () => false;
  export let awaitingWantsYesNo: (a: any) => boolean = () => false;

  export let onSendChatInput: () => void = () => {};
  export let onOpenInputModal: (text: string) => void = () => {};
  export let onKeydown: (e: KeyboardEvent) => void = () => {};

  let textareaEl: HTMLTextAreaElement;
  let localText = "";

  $: localText = chatInputText;
  $: {
    if (textareaEl) chatInputEl = textareaEl;
  }
  // We need a way to expose textareaEl to parent
  export let chatInputEl: HTMLTextAreaElement | undefined = undefined;
  $: if (textareaEl) chatInputEl = textareaEl;
</script>

<div class="chat-inputbar">
  {#if selectedRunId && status === "connected" && !selectedRunReady}
    <div class="subtle" style="margin:0 0 6px">终端连接中…（建立输出通道后会开始显示回复）</div>
  {/if}
  <textarea
    class="chat-textarea"
    bind:this={textareaEl}
    bind:value={localText}
    rows="2"
    on:keydown={onKeydown}
    placeholder={
      selectedAwaiting && (awaitingIsApproval(selectedAwaiting) || awaitingWantsYesNo(selectedAwaiting))
        ? "待确认（Proceed?）：输入 y/n 或用上方按钮"
        : selectedRunId && status === "connected" && !selectedRunReady
          ? "终端连接中…"
          : "输入消息（Enter 发送，Shift+Enter 换行）"
    }
    disabled={!selectedRunId || status !== "connected"}
  ></textarea>
  <div class="chat-input-actions">
    <button on:click={onSendChatInput} disabled={!selectedRunId || status !== "connected" || !localText.trim()} type="button">
      发送
    </button>
    <button class="secondary" on:click={() => onOpenInputModal(localText)} disabled={!selectedRunId || status !== "connected"} type="button">
      更多
    </button>
  </div>
</div>

<style>
  .chat-inputbar {
    margin-top: 10px;
    display: flex;
    gap: 8px;
    align-items: flex-end;
  }

  .chat-textarea {
    flex: 1;
    min-height: 44px;
    resize: vertical;
  }

  .chat-input-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: stretch;
  }

  @media (max-width: 640px) {
    .chat-inputbar {
      flex-direction: column;
      align-items: stretch;
    }

    .chat-input-actions {
      flex-direction: row;
      justify-content: flex-end;
    }
  }
</style>
