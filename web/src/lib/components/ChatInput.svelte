<script lang="ts">
  import { tick } from "svelte";

  let {
    selectedRunId = "",
    status = "",
    selectedRunReady = false,
    selectedAwaiting = null as any,
    awaitingIsApproval = ((a: any) => false) as (a: any) => boolean,
    awaitingWantsYesNo = ((a: any) => false) as (a: any) => boolean,
    onSendChatInput = ((text: string) => {}) as (text: string) => void,
    onOpenInputModal = ((text: string) => {}) as (text: string) => void,
  } = $props();

  const commands = [
    { cmd: "/new", label: "新对话", desc: "开始新的对话" },
    { cmd: "/reset", label: "重置", desc: "重置当前对话" },
    { cmd: "/model", label: "切换模型", desc: "更换 LLM 模型" },
    { cmd: "/session", label: "切换会话", desc: "切换到指定会话" },
    { cmd: "/skills", label: "浏览 Skills", desc: "列出可用 skills" },
    { cmd: "/retry", label: "重试", desc: "重试上一次回复" },
    { cmd: "/undo", label: "撤销", desc: "撤销上一步" },
    { cmd: "/compress", label: "压缩", desc: "压缩上下文节省 token" },
    { cmd: "/usage", label: "用量", desc: "查看 token 用量统计" },
    { cmd: "/insights", label: "洞察", desc: "查看 AI 行为洞察" },
    { cmd: "/stop", label: "中断", desc: "中断当前处理" },
  ];

  let localText = $state("");
  let textareaEl: HTMLTextAreaElement;
  let showCommands = $state(false);
  let filteredCommands: typeof commands = $state([]);
  let selectedIdx = $state(0);

  function updateCommands() {
    const text = localText;
    const match = text.match(/^\/([a-z]*)$/i);
    if (match) {
      const partial = match[1]!.toLowerCase();
      filteredCommands = commands.filter((c) => partial.length === 0 || c.cmd.slice(1).startsWith(partial));
      selectedIdx = 0;
      showCommands = filteredCommands.length > 0;
    } else {
      showCommands = false;
    }
  }

  function handleSend() {
    if (!localText.trim()) return;
    onSendChatInput(localText);
    localText = "";
    showCommands = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (showCommands) {
      if (e.key === "ArrowDown") { e.preventDefault(); selectedIdx = (selectedIdx + 1) % filteredCommands.length; return; }
      if (e.key === "ArrowUp") { e.preventDefault(); selectedIdx = (selectedIdx - 1 + filteredCommands.length) % filteredCommands.length; return; }
      if (e.key === "Tab" || e.key === "Enter") {
        const cmd = filteredCommands[selectedIdx];
        if (cmd) {
          e.preventDefault();
          localText = cmd.cmd + " ";
          showCommands = false;
          tick().then(() => textareaEl?.focus());
          return;
        }
      }
      if (e.key === "Escape") { showCommands = false; return; }
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  function selectCommand(cmd: string) {
    localText = cmd + " ";
    showCommands = false;
    tick().then(() => textareaEl?.focus());
  }

  function handleInput() {
    updateCommands();
  }
</script>

<div class="chat-input-wrapper">
  {#if showCommands}
    <div class="cmd-popup" role="listbox">
      {#each filteredCommands as cmd, i (cmd.cmd)}
        <button
          class="cmd-item"
          class:selected={i === selectedIdx}
          role="option"
          aria-selected={i === selectedIdx}
          onclick={() => selectCommand(cmd.cmd)}
          onmouseenter={() => (selectedIdx = i)}
          type="button"
        >
          <span class="cmd-key">{cmd.cmd}</span>
          <span class="cmd-label">{cmd.label}</span>
          <span class="cmd-desc">{cmd.desc}</span>
        </button>
      {/each}
    </div>
  {/if}
  <div class="chat-inputbar">
    <textarea
      class="chat-textarea"
      bind:this={textareaEl}
      bind:value={localText}
      rows="2"
      onkeydown={handleKeydown}
      oninput={handleInput}
      placeholder={
        selectedAwaiting && (awaitingIsApproval(selectedAwaiting) || awaitingWantsYesNo(selectedAwaiting))
          ? "待确认（Proceed?）：输入 y/n 或用上方按钮"
          : "输入消息（/ 查看命令，Enter 发送）"
      }
      disabled={!selectedRunId || status !== "connected"}
    ></textarea>
    <div class="chat-input-actions">
      <button onclick={handleSend} disabled={!selectedRunId || status !== "connected" || !localText.trim()} type="button">
        发送
      </button>
      <button class="secondary" onclick={() => onOpenInputModal(localText)} disabled={!selectedRunId || status !== "connected"} type="button">
        更多
      </button>
    </div>
  </div>
</div>

<style>
  .chat-input-wrapper {
    position: relative;
  }

  .cmd-popup {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    z-index: 30;
    max-height: 240px;
    overflow-y: auto;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    box-shadow: var(--shadow-md);
    margin-bottom: 4px;
    padding: 4px;
  }

  .cmd-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  .cmd-item.selected {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }

  .cmd-key {
    font-family: "Geist Mono", monospace;
    font-weight: 600;
    color: var(--accent);
    min-width: 80px;
  }

  .cmd-label {
    font-weight: 500;
    min-width: 60px;
  }

  .cmd-desc {
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chat-inputbar {
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
