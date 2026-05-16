<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Button from "@lib/components/Button.svelte";
  import {CircleCheckBig} from "lucide-svelte"; 
  import type { PermissionStatus } from "$lib/types";

  let {
    ready = $bindable(false),
  }: {
    ready?: boolean;
  } = $props();

  let statuses = $state<PermissionStatus[]>([]);
  function formatKindLabel(kind: string): string {
    if (kind === "speaker_capture") return "speaker capture";
    return kind.replace(/_/g, " ");
  }

  let requestingKind = $state<string | null>(null);

  async function refresh() {
    const next = await invoke<PermissionStatus[]>(
      "settings_permissions_status",
    ).catch(() => []);
    statuses = next;
    ready =
      next.find((status) => status.kind === "microphone")?.granted ?? false;
  }

  async function grantPermission(kind: string) {
    requestingKind = kind;
    // Use the native Tauri command for all permissions, including microphone.
    // getUserMedia was replaced because it triggers two dialogs (WKWebView
    // browser-level + macOS TCC) and the WKWebView grant doesn't persist across
    // restarts. The native path calls AVCaptureDevice.requestAccessForMediaType:
    // directly, which writes to the system TCC database and is permanent.
    await invoke("settings_permissions_request", { kind }).catch(() => {});
    await refresh();
    requestingKind = null;
  }

  let pollId: ReturnType<typeof setInterval>;
  let unlistenFocus: (() => void) | undefined;

  onMount(async () => {
    await refresh();
    pollId = setInterval(refresh, 10000);
    unlistenFocus = await getCurrentWindow().onFocusChanged(
      ({ payload: focused }) => {
        if (focused) refresh();
      },
    );
  });

  onDestroy(() => {
    clearInterval(pollId);
    unlistenFocus?.();
  });
</script>

<section class="space-y-3 h-full">
  <h2 class="sf-headline-sm">Permissions</h2>
  {#each statuses as status (status.kind)}
    <div class="rounded-md border border-fill px-3 py-2 transition bg-card">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <div>
            <p class="text-label-md font-sans uppercase tracking-stamped">
              {formatKindLabel(status.kind)}
            </p>
          </div>
        </div>
        {#if status.granted}
          <div class="flex gap-2 items-center text-success">
			<CircleCheckBig class='size-4'/>
            <span class="text-label-sm font-medium">Granted</span>
          </div>
        {:else if status.can_request}
          <div class="flex gap-2">
            <span
              class="size-4 shrink-0 rounded-full border-2 border-rim"
            ></span>
            <Button
              variant="normal"
              disabled={requestingKind === status.kind}
              onclick={() => grantPermission(status.kind)}
            >
              {requestingKind === status.kind
                ? "Requesting…"
                : "Grant permission"}
            </Button>
          </div>
        {:else}
          <div class="flex gap-2">
            <span class="text-label-sm text-fg/50">Not supported</span>
            <span
              class="size-4 shrink-0 rounded-full border-2 border-fill"
            ></span>
          </div>
        {/if}
      </div>
      {#if !status.granted && status.can_request && status.kind !== "microphone"}
        <p class="mt-1.5 text-label-sm text-fg/50">
          {status.kind === "accessibility"
            ? "System Settings will open → Privacy & Security → Accessibility. Enable the toggle next to this app."
            : "System Settings will open → Privacy & Security → Input Monitoring. Enable the toggle next to this app."}
        </p>
      {/if}
      {#if status.hint}
        <p class="mt-1.5 text-label-sm text-fg/50">
          {status.hint}
        </p>
      {/if}
    </div>
  {/each}
</section>
