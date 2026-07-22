<script lang="ts">
  export let todos: any[] = [];
  export let todoText: string = "";
  export let todoSuggestions: string[] = [];
  export let selectedRunId: string = "";

  export let onAddTodo: (text: string) => void = () => {};
  export let onToggleTodo: (id: string) => void = () => {};
  export let onRemoveTodo: (id: string) => void = () => {};

  let localTodoText = "";

  $: localTodoText = todoText;

  function addTodo() {
    if (localTodoText.trim()) {
      onAddTodo(localTodoText);
      localTodoText = "";
    }
  }
</script>

<h2>待办</h2>
<div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center;margin-bottom:8px">
  <input bind:value={localTodoText} placeholder="新增待办…" />
  <button on:click={addTodo} disabled={!selectedRunId}>
    添加
  </button>
</div>

{#if todoSuggestions.length > 0}
  <div style="margin:8px 0;padding:8px;border:1px solid var(--border);background:var(--bg-canvas)">
    <strong>建议（来自输出）</strong>
    <ul>
      {#each todoSuggestions as s (s)}
        <li>
          <button on:click={() => onAddTodo(s)} disabled={!selectedRunId} style="margin-right:8px">添加</button>
          {s}
        </li>
      {/each}
    </ul>
  </div>
{/if}

{#if todos.length > 0}
  <ul>
    {#each todos as t (t.id)}
      <li>
        <label style="display:flex;gap:8px;align-items:center">
          <input type="checkbox" checked={t.done} on:change={() => onToggleTodo(t.id)} />
          <span style={t.done ? "text-decoration:line-through;color:#6b7280" : ""}>{t.text}</span>
        </label>
        <button on:click={() => onRemoveTodo(t.id)} style="margin-left:8px">移除</button>
      </li>
    {/each}
  </ul>
{/if}
