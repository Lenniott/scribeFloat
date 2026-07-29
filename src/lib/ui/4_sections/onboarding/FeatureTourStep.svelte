<script lang="ts">
  import Button from "@components/controls/Button.svelte";
  import StepShell from "@primitives/layout/StepFrame.svelte";
  import { Mic, SquarePen, AppWindow, Settings, Wifi, LogOut } from "lucide-svelte";
  import { isWindows } from '@utils/platform';
  import { onDestroy, onMount } from "svelte";

  let {
    onBack,
    onFinish,
  }: {
    onBack: () => void;
    onFinish: () => void;
  } = $props();

  const features = [
    { label: "Dictate", Icon: Mic },
    { label: "New note", Icon: SquarePen },
    { separatorBefore: true, label: "Open ScribeFloat", Icon: AppWindow },
    { label: "Settings", Icon: Settings },
    { separatorBefore: true, label: "Quit ScribeFloat", Icon: LogOut },
  ];

  let timeStr = $state("");
  let dateStr = $state("");

  function tick() {
    const now = new Date();
    timeStr = now.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
    dateStr = now.toLocaleDateString([], {
      weekday: "short",
      month: "short",
      day: "numeric",
    });
  }

  let tickInterval: ReturnType<typeof setInterval>;
  onMount(() => {
    tick();
    tickInterval = setInterval(tick, 10_000);
  });
  onDestroy(() => clearInterval(tickInterval));
</script>

<StepShell
  title="You're all set"
  subtitle="Access everything from the tray icon."
>
  {#snippet children()}
    <div class="space-y-3">
      <div class="rounded-md overflow-hidden border border-fill">
        <div
          class="bg-canvas flex items-center justify-between px-3 py-1.5 gap-2"
        >
          {#if isWindows}
            <div class="flex items-center gap-1.5">
              <div
                class="w-6 h-6 rounded flex items-center justify-center ring-1 ring-brand/40 bg-brand/10"
                title="ScribeFloat"
              >
                <img
                  src="/favicon.png"
                  alt="ScribeFloat"
                  class="w-4 h-4 object-contain"
                />
              </div>
            </div>
            <div class="flex items-center gap-2 text-fg-dim">
              <Wifi class="size-3.5" />
              <span class="sf-meta-sm leading-none"
                >{timeStr}</span
              >
            </div>
          {:else}
            <div class="flex items-center gap-0.5">
              <div class="w-1.5 h-1.5 rounded-full bg-rim/40"></div>
              <div class="w-1.5 h-1.5 rounded-full bg-rim/40 ml-1.5"></div>
            </div>
            <div class="flex items-center gap-2.5 text-fg-dim">
              <div
                class="w-6 h-6 rounded flex items-center justify-center ring-1 ring-brand/50 bg-brand/15"
                title="ScribeFloat"
              >
                <img
                  src="/icon.ico"
                  alt="ScribeFloat"
                  class="w-4 h-4 object-contain"
                />
              </div>
              <Wifi class="size-3.5" />
              <div class="sf-meta-sm leading-none text-fg-dim">
                {dateStr}
                {timeStr}
              </div>
            </div>
          {/if}
        </div>

        <div class="bg-card px-2 py-1.5">
          {#each features as { label, Icon, separatorBefore } (label)}
            {#if separatorBefore}
              <div class="sf-divider my-1"></div>
            {/if}
            <div class="flex items-center gap-2 px-1.5 py-1">
              <Icon class="size-3.5 text-fg-dim shrink-0" />
              <span class="sf-body-md text-fg">{label}</span>
            </div>
          {/each}
        </div>
      </div>

      <div class="rounded-md bg-card border border-fill px-3 py-2 space-y-0.5">
        <p class="sf-section-label text-fg-dim">
          Start on login
        </p>
        {#if isWindows}
          <p class="sf-body-md text-fg-dim">
            Go to <strong class="text-fg">Settings → Apps → Startup</strong> and
            enable ScribeFloat.
          </p>
        {:else}
          <p class="sf-body-md text-fg-dim">
            Go to <strong class="text-fg"
              >System Settings → General → Login Items</strong
            > and add ScribeFloat.
          </p>
        {/if}
      </div>
    </div>
  {/snippet}

  {#snippet footer()}
    <Button variant="ghost" onclick={onBack}>Back</Button>
    <Button variant="primary" onclick={onFinish}>Done</Button>
  {/snippet}
</StepShell>
