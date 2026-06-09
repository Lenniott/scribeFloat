<script lang="ts">
  import Button from "@lib/components/Button.svelte";
  import StepShell from "./StepShell.svelte";
  import { Mic, FileVolume, History, Settings, Wifi } from "lucide-svelte";
  import { isWindows } from "$lib/platform";
  import { onDestroy, onMount } from "svelte";

  let {
    onBack,
    onFinish,
  }: {
    onBack: () => void;
    onFinish: () => void;
  } = $props();

  const features = [
    {
      label: "Scribe",
      description: "Transcribe long-form recordings and take notes.",
      Icon: Mic,
    },
    {
      label: "Transcribe",
      description: "Transcribe pre-recorded audio files.",
      Icon: FileVolume,
    },
    {
      label: "History",
      description: "View and manage all transcriptions and dictations.",
      Icon: History,
    },
    {
      label: "Settings",
      description: "Further customise your setup.",
      Icon: Settings,
    },
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
      <!-- Stylized system bar -->
      <div class="rounded-md overflow-hidden border border-fill">
        <!-- Bar row -->
        <div
          class="bg-[#1a1a1a] flex items-center justify-between px-3 py-1.5 gap-2"
        >
          {#if isWindows}
            <!-- Windows taskbar: app icon left, system icons right -->
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
            <div class="flex items-center gap-2 text-white/70">
              <Wifi class="size-3.5" />
              <span class="text-[11px] tabular-nums leading-none"
                >{timeStr}</span
              >
            </div>
          {:else}
            <!-- macOS menu bar: system icons + clock right, app icon highlighted -->
            <div class="flex items-center gap-0.5">
              <!-- dim dot decorations for left side of menu bar -->
              <div class="w-1.5 h-1.5 rounded-full bg-white/15"></div>
              <div class="w-1.5 h-1.5 rounded-full bg-white/15 ml-1.5"></div>
            </div>
            <div class="flex items-center gap-2.5 text-white/70">
              <!-- App icon in tray, highlighted -->
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
              <div class="text-[11px] tabular-nums leading-none text-white/80">
                {dateStr}
                {timeStr}
              </div>
            </div>
          {/if}
        </div>

        <!-- Feature list inside card -->
        <div class="bg-card px-3 py-2 space-y-0.5">
          {#each features as { label, description, Icon } (label)}
            <div class="flex items-center gap-3 px-1 py-1.5">
              <div
                class="size-6 rounded bg-fill flex items-center justify-center shrink-0"
              >
                <Icon class="size-3.5 text-fg-dim" />
              </div>
              <div>
                <span class="text-body-md text-fg font-medium">{label}</span>
                <span class="text-body-md text-fg-dim ml-1.5"
                  >{description}</span
                >
              </div>
            </div>
          {/each}
        </div>
      </div>

      <!-- Start on login -->
      <div class="rounded-md bg-card border border-fill px-3 py-3 space-y-1">
        <p
          class="text-label-sm font-mono tracking-stamped uppercase text-fg/70"
        >
          Start on login
        </p>
        {#if isWindows}
          <p class="text-body-md text-fg-dim">
            Go to <strong class="text-fg">Settings → Apps → Startup</strong> and
            enable ScribeFloat.
          </p>
        {:else}
          <p class="text-body-md text-fg-dim">
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
