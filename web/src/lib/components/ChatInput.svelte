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

  const quickPrompts = [
    { key: "#解释", label: "解释代码", desc: "解释选中的代码", text: "请解释这段代码的作用和设计意图" },
    { key: "#测试", label: "写测试", desc: "为函数补单元测试", text: "为这段代码编写单元测试，覆盖主要分支和边界情况" },
    { key: "#重构", label: "重构", desc: "提升可读性", text: "重构这段代码，提升可读性和可维护性，保持行为不变" },
    { key: "#审查", label: "代码审查", desc: "指出问题", text: "审查这段代码，指出潜在 bug、风格问题和改进建议" },
    { key: "#修复", label: "修复 bug", desc: "定位并修复", text: "找出这段代码里的 bug 并给出最小修复" },
    { key: "#文档", label: "补文档", desc: "加注释/文档", text: "为这个文件补充文档注释，说明公开接口的用法" },
    { key: "#优化", label: "性能优化", desc: "提速/降耗", text: "分析这段代码的性能瓶颈并给出优化方案" },
    { key: "#格式化", label: "格式化", desc: "统一风格", text: "按项目规范格式化这段代码" },
  ];

  let localText = $state("");
  let textareaEl: HTMLTextAreaElement;
  let showCommands = $state(false);
  let filteredCommands: typeof commands = $state([]);
  let selectedIdx = $state(0);
  let showPrompts = $state(false);
  let filteredPrompts: typeof quickPrompts = $state([]);
  let selectedPromptIdx = $state(0);

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

  function updatePrompts() {
    const text = localText;
    const match = text.match(/^#(\S*)$/);
    if (match) {
      const partial = match[1] ?? "";
      filteredPrompts = quickPrompts.filter((p) => partial.length === 0 || p.key.slice(1).startsWith(partial));
      selectedPromptIdx = 0;
      showPrompts = filteredPrompts.length > 0;
    } else {
      showPrompts = false;
    }
  }

  // Phase A (paseo-absorb-v1): exact-match local slash commands.
  // Mirrors paseo client-slash-commands semantics: intercept only on
  // exact match, no-args, no-attachments. ChatInput has no attachments
  // prop, so the no-attachments condition is vacuously true here.
  const LOCAL_COMMANDS = new Set(["/quit", "/clear"]);

  function isLocalCommand(text: string): boolean {
    return LOCAL_COMMANDS.has(text.trim());
  }

  function handleLocalCommand(_cmd: string): void {
    // /clear: drop the draft locally; /quit: no-op in the PWA (no PTY to quit).
    // Intentionally never calls onSendChatInput so nothing reaches run.send_input.
    localText = "";
    showCommands = false;
    showPrompts = false;
  }

  function handleSend() {
    if (!localText.trim()) return;
    if (isLocalCommand(localText)) {
      handleLocalCommand(localText.trim());
      return;
    }
    onSendChatInput(localText);
    localText = "";
    showCommands = false;
    showPrompts = false;
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
    if (showPrompts) {
      if (e.key === "ArrowDown") { e.preventDefault(); selectedPromptIdx = (selectedPromptIdx + 1) % filteredPrompts.length; return; }
      if (e.key === "ArrowUp") { e.preventDefault(); selectedPromptIdx = (selectedPromptIdx - 1 + filteredPrompts.length) % filteredPrompts.length; return; }
      if (e.key === "Tab" || e.key === "Enter") {
        const p = filteredPrompts[selectedPromptIdx];
        if (p) {
          e.preventDefault();
          localText = p.text;
          showPrompts = false;
          tick().then(() => textareaEl?.focus());
          return;
        }
      }
      if (e.key === "Escape") { showPrompts = false; return; }
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

  function selectPrompt(p: { text: string }) {
    localText = p.text;
    showPrompts = false;
    tick().then(() => textareaEl?.focus());
  }

  function handleInput() {
    updateCommands();
    updatePrompts();
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
  {#if showPrompts}
    <div class="cmd-popup prompt-popup" role="listbox">
      {#each filteredPrompts as p, i (p.key)}
        <button
          class="cmd-item"
          class:selected={i === selectedPromptIdx}
          role="option"
          aria-selected={i === selectedPromptIdx}
          onclick={() => selectPrompt(p)}
          onmouseenter={() => (selectedPromptIdx = i)}
          type="button"
        >
          <span class="cmd-key">{p.key}</span>
          <span class="cmd-label">{p.label}</span>
          <span class="cmd-desc">{p.desc}</span>
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
          : "输入消息（/ 命令，# 快速 prompt，Enter 发送）"
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
