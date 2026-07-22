<script lang="ts">
  import { onMount } from "svelte";
  import { relay } from "./lib/stores/relay-store.svelte";
  import LoginPage from "./lib/pages/LoginPage.svelte";
  import SessionsPage from "./lib/pages/SessionsPage.svelte";
  import LaunchPage from "./lib/pages/LaunchPage.svelte";
  import SettingsPage from "./lib/pages/SettingsPage.svelte";
  import PageShell from "./lib/pages/PageShell.svelte";
  import InputModal from "./lib/components/InputModal.svelte";
  import ApprovalModal from "./lib/components/ApprovalModal.svelte";
  import StopConfirmModal from "./lib/components/StopConfirmModal.svelte";

  let lastSeenApprovalRequest: Record<string, string> = {};
  let lastSeenPromptRequest: Record<string, string> = {};

  onMount(() => {
    if (typeof window === "undefined") return;
    const mq = window.matchMedia("(max-width: 640px)");
    const apply = () => { relay.isMobile = mq.matches; };
    apply();
    mq.addEventListener("change", apply);

    let stopped = false;
    let inFlight = false;
    const timer = setInterval(async () => {
      if (stopped || inFlight) return;
      if (!relay.token || relay.status === "connected" || relay.status === "checking" || relay.status === "connecting") return;
      inFlight = true;
      try { await Promise.all([relay.refreshHosts(), relay.refreshRuns()]);
      } finally { inFlight = false; }
    }, 10000);

    if (relay.token) relay.resumeFromStoredToken();

    return () => { stopped = true; clearInterval(timer); mq.removeEventListener("change", apply); };
  });

  $effect(() => {
    if (relay.selectedRunId && relay.token && relay.status === "connected") {
      relay.subscribeToRun(relay.selectedRunId);
    }
  });

  $effect(() => {
    const a = relay.selectedAwaiting;
    if (!relay.selectedRunId || !a) return;

    if (relay.awaitingIsApproval(a) && !relay.approvalModalOpen) {
      const key = (a.request_id ?? a.op_tool ?? "").trim();
      if (key && lastSeenApprovalRequest[relay.selectedRunId] !== key) {
        lastSeenApprovalRequest = { ...lastSeenApprovalRequest, [relay.selectedRunId]: key };
        relay.approvalModalShowArgs = false;
        relay.approvalModalOpen = true;
      }
    }

    if (relay.awaitingIsPrompt(a) && !relay.inputModalOpen) {
      const key = (a.request_id ?? "").trim();
      if (key && lastSeenPromptRequest[relay.selectedRunId] !== key) {
        lastSeenPromptRequest = { ...lastSeenPromptRequest, [relay.selectedRunId]: key };
        relay.inputModalText = "";
        relay.inputModalOpen = true;
      }
    }
  });

  $effect(() => {
    if (relay.approvalModalOpen && (!relay.selectedAwaiting || !relay.awaitingIsApproval(relay.selectedAwaiting))) {
      relay.approvalModalOpen = false;
    }
  });

  const showLogin = $derived(!relay.token);
  const currentView = $derived(relay.token ? relay.view : "login");
</script>

{#if showLogin}
  <LoginPage />
{:else}
  <PageShell>
    {#if currentView === "sessions"}
      <SessionsPage />
    {:else if currentView === "start"}
      <LaunchPage />
    {:else if currentView === "settings"}
      <SettingsPage />
    {/if}
  </PageShell>
{/if}

<InputModal
  show={relay.inputModalOpen}
  bind:text={relay.inputModalText}
  selectedRunId={relay.selectedRunId}
  status={relay.status}
  onClose={() => relay.closeInputModal()}
  onSend={() => relay.sendInputModalText()}
  onQuickInput={(text: string) => relay.sendQuickInput(text)}
/>

<ApprovalModal
  show={relay.approvalModalOpen}
  selectedRunId={relay.selectedRunId}
  status={relay.status}
  awaiting={relay.selectedAwaiting}
  runTool={relay.selectedRun?.tool ?? ""}
  approvalForSession={relay.approvalForSession}
  bind:approvalAnswersJson={relay.approvalAnswersJson}
  showArgs={relay.approvalModalShowArgs}
  riskForOpTool={(name: string | null | undefined) => relay.riskForOpTool(name)}
  onClose={() => { relay.approvalModalOpen = false; relay.approvalModalShowArgs = false; }}
  onSendDecision={(d: string) => { relay.sendDecision(d); relay.approvalModalOpen = false; relay.approvalModalShowArgs = false; }}
  onToggleApprovalForSession={(v: boolean) => (relay.approvalForSession = v)}
/>

<StopConfirmModal
  show={relay.stopConfirmOpen}
  runId={relay.selectedRunId}
  status={relay.status}
  onClose={() => (relay.stopConfirmOpen = false)}
  onStop={(signal: string) => { relay.sendStop(signal); relay.stopConfirmOpen = false; }}
/>
