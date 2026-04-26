<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Button from "@lib/components/Button.svelte";
  import {CircleCheckBig} from "lucide-svelte"; 
  import type { PermissionStatus } from "$lib/types";

  let statuses = $state<PermissionStatus[]>([]);
  let requestingKind = $state<string | null>(null);

  async function refresh() {
    statuses = await invoke<PermissionStatus[]>(
      "settings_permissions_status",
    ).catch(() => []);
  }

  async function grantPermission(kind: string) {
    requestingKind = kind;
    if (kind === "microphone") {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({
          audio: true,
        });
        stream.getTracks().forEach((t) => t.stop());
      } catch {
        // Denied or unavailable — status refresh below reflects reality.
      }
    } else {
      await invoke("settings_permissions_open", { kind }).catch(() => {});
    }
    await refresh();
    requestingKind = null;
  }

  let pollId: ReturnType<typeof setInterval>;
  let unlistenFocus: (() => void) | undefined;

  onMount(async () => {
    await refresh();
    pollId = setInterval(refresh, 3000);
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

<section class="space-y-3">
  <h2 class="text-title-sm font-semibold">Permissions</h2>
  {#each statuses as status (status.kind)}
    <div
      class={`rounded-md border px-3 py-2.5 transition ${
        status.granted
          ? "border-green-500/30 bg-green-500/5"
          : status.can_request
            ? "border-amber-500/30 bg-amber-500/5"
            : "border-surface-container bg-surface"
      }`}
    >
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <div>
            <p class="text-body-sm capitalize">
              {status.kind.replace(/_/g, " ")}
            </p>
          </div>
        </div>
        {#if status.granted}
          <div class="flex gap-2 items-center text-green brightness-120">
			<CircleCheckBig class='size-4'/>
            <span class="text-label-sm font-medium">Granted</span
            >
          </div>
        {:else if status.can_request}
          <div class="flex gap-2">
            <span
              class="size-4 shrink-0 rounded-full border-2 border-secondary bg-amber-secondary/10"
            ></span>
            <Button
              variant="secondary"
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
            <span class="text-label-sm text-on-surface/50">Not supported</span>
            <span
              class="size-4 shrink-0 rounded-full border-2 border-surface-container-high"
            ></span>
          </div>
        {/if}
      </div>
      {#if !status.granted && status.can_request && status.kind !== "microphone"}
        <p class="mt-1.5 text-label-sm text-on-surface/50">
          {status.kind === "accessibility"
            ? "System Settings will open → Privacy & Security → Accessibility. Enable the toggle next to this app."
            : "System Settings will open → Privacy & Security → Input Monitoring. Enable the toggle next to this app."}
        </p>
      {/if}
    </div>
  {/each}
</section>
