<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Button from "@components/controls/Button.svelte";
  import SettingsList from "@sections/SettingList.svelte";
  import SettingsRow from "@components/cards/SettingRow.svelte";
  import SettingsSection from "@primitives/form/SettingsSection.svelte";
  import { CircleCheckBig } from "lucide-svelte";
  import type { PermissionStatus } from '@utils/types';

  let {
    micOnly = false,
    onReadyChange,
  }: {
    micOnly?: boolean;
    onReadyChange?: (ready: boolean) => void;
  } = $props();

  let statuses = $state<PermissionStatus[]>([]);
  const visibleStatuses = $derived(
    micOnly ? statuses.filter((s) => s.kind === "microphone") : statuses,
  );
  function formatKindLabel(kind: string): string {
    if (kind === "speaker_capture") return "speaker capture";
    return kind.replace(/_/g, " ");
  }

  function permissionDescription(status: PermissionStatus): string | undefined {
    let description: string | undefined;
    if (status.granted || !status.can_request) return status.hint || undefined;
    if (status.kind === "microphone" && micOnly) {
      description = "Windows will show a microphone consent dialog, or open Settings → Privacy → Microphone if access was previously denied.";
    } else if (status.kind !== "microphone") {
      description = status.kind === "accessibility"
        ? "System Settings will open → Privacy & Security → Accessibility. Enable the toggle next to this app."
        : "System Settings will open → Privacy & Security → Input Monitoring. Enable the toggle next to this app.";
    }
    return [description, status.hint].filter(Boolean).join(" ") || undefined;
  }

  let requestingKind = $state<string | null>(null);

  async function refresh() {
    const next = await invoke<PermissionStatus[]>(
      "settings_permissions_status",
    ).catch(() => []);
    statuses = next;
    onReadyChange?.(
      next.find((status) => status.kind === "microphone")?.granted ?? false,
    );
  }

  async function grantPermission(kind: string) {
    requestingKind = kind;
    await invoke("settings_permissions_request", { kind }).catch(() => {});
    await refresh();
    requestingKind = null;
  }

  let pollId: ReturnType<typeof setInterval>;
  let unlistenFocus: (() => void) | undefined;

  onMount(async () => {
    await refresh();
    pollId = setInterval(refresh, 30000);
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

<section class="space-y-5 h-full">
  <h2 class="sf-headline-sm text-fg">Permissions</h2>
  <SettingsSection title="System access">
    <SettingsList>
      {#each visibleStatuses as status (status.kind)}
        <SettingsRow
          title={formatKindLabel(status.kind)}
          description={permissionDescription(status)}
        >
          {#snippet control()}
            {#if status.granted}
              <div class="flex gap-2 items-center text-success">
                <CircleCheckBig class="size-4" />
                <span class="sf-label-sm">Granted</span>
              </div>
            {:else if status.can_request}
              <div class="flex gap-2">
                <span class="size-4 shrink-0 rounded-full border-2 border-rim"
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
                <span class="sf-label-sm text-fg-dim">Not supported</span>
                <span class="size-4 shrink-0 rounded-full border-2 border-fill"
                ></span>
              </div>
            {/if}
          {/snippet}
        </SettingsRow>
      {/each}
    </SettingsList>
  </SettingsSection>
</section>
