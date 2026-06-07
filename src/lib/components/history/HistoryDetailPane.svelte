<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import {
    X,
    Copy,
    SquareArrowOutUpRight,
    FileDown,
    ChevronLeft,
    ChevronRight,
    Trash2,
  } from "lucide-svelte";
  import PanelHeader from "@lib/components/layout/PanelHeader.svelte";
  import PanelFooter from "@lib/components/layout/PanelFooter.svelte";
  import ScrollablePanel from "@lib/components/accordion/ScrollablePanel.svelte";
  import IconButton from "@lib/components/IconButton.svelte";
  import Chip from "@lib/components/Chip.svelte";
  import Toast from "@lib/components/Toast.svelte";
  import type { ToastState } from "@lib/components/Toast.svelte";
  import type { HistoryListItem } from "@lib/services/historyActions";
  import { loadTranscriptPreview } from "@lib/services/historyTranscript";
  import {
    exportHistoryMarkdown,
    openHistoryMarkdown,
  } from "@lib/services/historyActions";

  type HistoryDetail = {
    format_version: number;
    id: string;
    kind: string;
    created_at: string;
    title: string;
    model: string;
    segments: { start_ms: number; end_ms: number; text: string }[];
    notes: { id: string; text: string; recorded_at_ms: number }[];
    duration_ms: number;
    word_count: number;
    speaker_capture: boolean;
    dual_source: boolean;
    source_path?: string;
    markdown_path?: string;
    session_dir?: string;
    audio_path?: string;
    deleted: boolean;
  };

  let {
    item,
    onclose,
    onrefresh,
    ondelete,
    canGoPrev = false,
    canGoNext = false,
    onprev,
    onnext,
  }: {
    item: HistoryListItem;
    onclose: () => void;
    onrefresh: () => void;
    ondelete?: () => void;
    canGoPrev?: boolean;
    canGoNext?: boolean;
    onprev?: () => void;
    onnext?: () => void;
  } = $props();

  let bodyText = $state("");
  let loadError = $state("");
  let detail = $state<HistoryDetail | null>(null);
  let loadingBody = $state(true);

  let toastMessage = $state("");
  let toastState = $state<ToastState>("normal");
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;

  const showExport = $derived(
    item.source === "store" && item.kind !== "dictate" && !item.has_markdown,
  );
  const showOpenMd = $derived(item.has_markdown && !!item.markdown_path);

  function showToast(msg: string, state: ToastState = "normal") {
    if (toastTimeout) clearTimeout(toastTimeout);
    toastMessage = msg;
    toastState = state;
    toastTimeout = setTimeout(() => {
      toastMessage = "";
      toastTimeout = null;
    }, 2500);
  }

  function formatDuration(ms: number): string {
    const totalSec = Math.round(ms / 1000);
    const mins = Math.floor(totalSec / 60);
    const secs = totalSec % 60;
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  }

  async function loadContent() {
    loadingBody = true;
    loadError = "";
    detail = null;
    try {
      bodyText = await loadTranscriptPreview(item.id);
    } catch (e) {
      bodyText = "";
      loadError = String(e);
    } finally {
      loadingBody = false;
    }

    if (item.source === "store") {
      try {
        detail = await invoke<HistoryDetail>("history_get_detail", {
          id: item.id,
        });
      } catch {
        detail = null;
      }
    }
  }

  async function copyContent() {
    let content = bodyText;
    if (detail?.notes?.length) {
      const lines = detail.notes
        .map((n) => `[${formatDuration(n.recorded_at_ms)}] ${n.text}`)
        .join("\n");
      content += `\n\nNotes:\n${lines}`;
    }
    try {
      await writeText(content);
      showToast("Copied", "success");
    } catch {
      showToast("Copy failed", "error");
    }
  }

  async function exportMarkdown() {
    try {
      await exportHistoryMarkdown(item.id);
      onrefresh();
      showToast("Exported", "success");
    } catch (e) {
      showToast("Export failed: " + String(e), "error");
    }
  }

  async function openMarkdown() {
    if (!item.markdown_path) return;
    try {
      await openHistoryMarkdown(item.markdown_path);
    } catch {
      showToast("Could not open file", "error");
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
    if (e.key === "ArrowLeft" && canGoPrev) {
      e.preventDefault();
      onprev?.();
    } else if (e.key === "ArrowRight" && canGoNext) {
      e.preventDefault();
      onnext?.();
    }
  }

  $effect(() => {
    void item;
    void loadContent();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex min-h-0 flex-1 flex-col overflow-hidden">
  <PanelHeader>
    {#snippet left()}
      <p
        class="truncate font-mono text-label-md tracking-stamped text-fg/80 uppercase"
      >
        {item.title || "Detail"}
      </p>
    {/snippet}
    {#snippet right()}
      <div class="flex items-center gap-1">
        <IconButton
          aria-label="Previous item"
          icon={ChevronLeft}
          size="small"
          variant="normal"
          disabled={!canGoPrev}
          onclick={() => onprev?.()}
        />
        <IconButton
          aria-label="Next item"
          icon={ChevronRight}
          size="small"
          variant="normal"
          disabled={!canGoNext}
          onclick={() => onnext?.()}
        />
      </div>
    {/snippet}
  </PanelHeader>

  <div
    class="flex shrink-0 flex-wrap items-center gap-2 border-b border-card/60 px-4 py-2"
  >
    {#if item.model}
      <Chip variant="muted">{item.model}</Chip>
    {/if}
    {#if item.duration_ms > 0}
      <Chip variant="muted">{formatDuration(item.duration_ms)}</Chip>
    {/if}
    {#if item.word_count > 0}
      <Chip variant="muted">{item.word_count} words</Chip>
    {/if}
    {#if detail?.dual_source}
      <Chip variant="muted">Dual source</Chip>
    {:else if detail?.speaker_capture}
      <Chip variant="muted">Speaker capture</Chip>
    {/if}
    {#if item.source !== "store"}
      <span
        class="font-mono text-label-sm tracking-stamped text-fg/45 uppercase"
        >Legacy</span
      >
    {/if}
  </div>

  <ScrollablePanel class="min-h-0 flex-1 px-4 py-3">
    {#if loadingBody}
      <p class="text-label-md text-fg/45">Loading…</p>
    {:else if loadError}
      <p class="text-label-md text-destructive">Could not load transcript.</p>
      <p class="mt-1 text-label-sm text-fg/45">{loadError}</p>
    {:else if bodyText}
      <div class="flex flex-col gap-3 text-body-md text-fg/90">
        {#each bodyText.split('\n\n') as para, i (i)}
          <p class="wrap-break-word">
            {#each para.split('\n') as line, j (j)}{#if j > 0}<br />{/if}{line}{/each}
          </p>
        {/each}
      </div>
    {:else}
      <p class="text-label-md text-fg/45">No content available.</p>
    {/if}
    {#if detail?.notes?.length}
      <div class="mt-4 flex flex-col gap-1.5 border-t border-rim/30 pt-3">
        <span
          class="font-mono text-label-sm tracking-stamped text-fg/50 uppercase"
          >Notes</span
        >
        {#each detail.notes as note (note.id)}
          <div class="flex gap-2 text-body-md text-fg/80">
            <span class="shrink-0 pt-0.5 font-mono text-label-sm text-fg/40">
              {formatDuration(note.recorded_at_ms)}
            </span>
            <span class="wrap-break-word">{note.text}</span>
          </div>
        {/each}
      </div>
    {/if}
  </ScrollablePanel>

  <PanelFooter>
    <!-- Fixed-width slots so Copy/Close stay put when Export/Open vary per item -->
    <div class="flex w-full">
	{#if item.source === "store"}
      <IconButton
        aria-label="Delete"
        icon={Trash2}
        size="small"
        variant="destructive"
        onclick={() => ondelete?.()}
      />
    {/if}
	</div>
    {#if showExport}
      <IconButton
        aria-label="Export to Markdown"
        icon={FileDown}
        size="small"
        variant="normal"
        onclick={exportMarkdown}
      />
    {/if}
    {#if showOpenMd}
      <IconButton
        aria-label="Open Markdown file"
        icon={SquareArrowOutUpRight}
        size="small"
        variant="normal"
        onclick={openMarkdown}
      />
    {/if}
    <IconButton
      aria-label="Copy transcript"
      icon={Copy}
      size="small"
      variant="normal"
      onclick={copyContent}
    />
    <IconButton
      aria-label="Close detail"
      icon={X}
      size="small"
      variant="normal"
      onclick={onclose}
    />
  </PanelFooter>
</div>

<Toast message={toastMessage} state={toastState} position="bottom-center" />
