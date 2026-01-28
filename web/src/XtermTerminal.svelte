<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount, tick } from "svelte";
  import { Terminal } from "xterm";
  import { FitAddon } from "xterm-addon-fit";
  import "xterm/css/xterm.css";

  export let fontSize = 13;
  export let readOnly = false;
  export let autoFocus = false;
  export let scrollback = 5000;

  const dispatch = createEventDispatcher<{
    data: { data: string };
    resize: { cols: number; rows: number };
    ready: {};
  }>();

  let hostEl: HTMLDivElement | null = null;
  let term: Terminal | null = null;
  let fit: FitAddon | null = null;
  let ro: ResizeObserver | null = null;

  function safeFit() {
    if (!term || !fit) return;
    try {
      fit.fit();
      dispatch("resize", { cols: term.cols, rows: term.rows });
    } catch {
      // ignore
    }
  }

  export function write(data: string) {
    if (!term || !data) return;
    term.write(data);
  }

  export function reset() {
    if (!term) return;
    term.reset();
    term.clear();
  }

  export function focus() {
    term?.focus();
  }

  onMount(async () => {
    await tick();

    term = new Terminal({
      cursorBlink: true,
      fontSize,
      scrollback,
      convertEol: false,
      fontFamily:
        'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
      theme: {
        background: "#0b1020",
        foreground: "#e5e7eb",
        cursor: "#e5e7eb",
        selectionBackground: "rgba(148, 163, 184, 0.35)",
      },
    });
    fit = new FitAddon();
    term.loadAddon(fit);

    if (hostEl) term.open(hostEl);
    safeFit();

    term.onData((data) => {
      if (readOnly) return;
      dispatch("data", { data });
    });

    ro = new ResizeObserver(() => safeFit());
    if (hostEl) ro.observe(hostEl);

    dispatch("ready", {});
    if (autoFocus) {
      setTimeout(() => term?.focus(), 0);
    }
  });

  onDestroy(() => {
    try {
      ro?.disconnect();
    } catch {
      // ignore
    }
    ro = null;
    try {
      term?.dispose();
    } catch {
      // ignore
    }
    term = null;
    fit = null;
  });
</script>

<div class="xterm-host" bind:this={hostEl}></div>

<style>
  .xterm-host {
    width: 100%;
    height: 100%;
    background: #0b1020;
    overflow: hidden;
  }

  :global(.xterm) {
    height: 100%;
  }

  :global(.xterm-viewport) {
    overscroll-behavior: contain;
  }
</style>

