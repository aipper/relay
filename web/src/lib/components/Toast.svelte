<script lang="ts">
  export let text: string = "";
</script>

{#if text}
  <div class="toast" role="status" aria-live="polite">{text}</div>
{/if}

<style>
  .toast {
    position: fixed;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    padding: 10px 12px;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.18);
    background: rgba(17, 24, 39, 0.86);
    color: rgba(255, 255, 255, 0.95);
    font-size: 13px;
    font-weight: 800;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.12),
      0 10px 30px rgba(0, 0, 0, 0.22);
    z-index: 50;
  }
</style>
