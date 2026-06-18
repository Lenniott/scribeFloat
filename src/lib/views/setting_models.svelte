<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { createModelDownloadStore } from "$lib/stores/modelDownload.svelte";
  import Toast from "@lib/components/ui/indicators/Toast.svelte";
  import type { ToastState } from "@lib/components/ui/indicators/Toast.svelte";
  import FieldRow from "@lib/components/primitives/form/FieldRow.svelte";
  import IconButton from "@lib/components/ui/controls/IconButton.svelte";
  import Chip from "@lib/components/primitives/display/Chip.svelte";
  import SettingsList from "@lib/components/sections/SettingList.svelte";
  import SettingsRow from "@lib/components/ui/cards/SettingRow.svelte";
  import SettingsSection from "@lib/components/primitives/form/SettingsSection.svelte";
  import ScrollablePanel from "@lib/components/primitives/layout/ScrollBody.svelte";
  import { Download, Trash2 } from "lucide-svelte";

  let {
    ready = $bindable(false),
  }: {
    ready?: boolean;
  } = $props();

  type ToastConfig = {
    message: string;
    state: ToastState;
  };

  const modelStore = createModelDownloadStore();
  const emptyToast: ToastConfig = { message: "", state: "normal" };
  const toastMessages = {
    modelSelected: { message: "Model selected", state: "success" },
    modelsRefreshed: { message: "Models refreshed", state: "success" },
    modelRemoved: { message: "Model removed", state: "success" },
  } satisfies Record<string, ToastConfig>;

  let unlisteners: (() => void)[] = [];
  let toast = $state<ToastConfig>({ ...emptyToast });
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;
  let readyHydrated = $state(false);

  const selectedModel = $derived(modelStore.models.find((m) => m.selected));
  const selectedId = $derived(selectedModel?.id ?? "");
  const downloadedModels = $derived(
    modelStore.models.filter((m) => m.downloaded),
  );
  const hasReadyModel = $derived(
    modelStore.models.some((m) => m.selected && m.downloaded),
  );

  let dictateModelId = $state<string | null>(null);

  let vadDownloaded = $state(false);
  let vadDownloading = $state(false);

  $effect(() => {
    if (!readyHydrated) return;
    ready = hasReadyModel;
  });

  $effect(() => {
    const progress = modelStore.progressByModel["vad"] ?? 0;
    if (progress >= 1 && vadDownloading) {
      vadDownloading = false;
      vadDownloaded = true;
    }
  });

  onMount(async () => {
    unlisteners = await modelStore.subscribe();
    await modelStore.refresh();
    dictateModelId = await invoke<string | null>(
      "settings_get_dictate_model_id",
    ).catch(() => null);
    vadDownloaded = await invoke<boolean>("model_vad_status").catch(
      () => false,
    );
    readyHydrated = true;
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
    if (toastTimeout) clearTimeout(toastTimeout);
  });

  async function downloadModel(modelId: string) {
    clearToast();
    await modelStore.download(modelId);
  }

  async function selectModel(modelId: string) {
    clearToast();
    await modelStore.select(modelId);
    if (!modelStore.error) showToastMessage(toastMessages.modelSelected);
  }

  async function removeModel(modelId: string) {
    clearToast();
    await modelStore.remove(modelId);
    if (!modelStore.error) showToastMessage(toastMessages.modelRemoved);
  }

  async function onDictateModelChange(value: string) {
    const modelId = value || null;
    dictateModelId = modelId;
    try {
      await invoke("settings_set_dictate_model_id", { modelId });
      showToastMessage(toastMessages.modelSelected);
    } catch (e) {
      modelStore.error = String(e);
    }
  }

  function clearToast() {
    toast = { ...emptyToast };
  }

  function showToastMessage(nextToast: ToastConfig) {
    if (toastTimeout) clearTimeout(toastTimeout);
    toast = nextToast;
    toastTimeout = setTimeout(() => {
      clearToast();
      toastTimeout = null;
    }, 2000);
  }

  async function downloadVad() {
    clearToast();
    vadDownloading = true;
    try {
      await invoke("model_vad_download");
    } catch (e) {
      modelStore.error = String(e);
      vadDownloading = false;
    }
  }

  async function removeVad() {
    clearToast();
    try {
      await invoke("model_vad_remove");
      vadDownloaded = false;
    } catch (e) {
      modelStore.error = String(e);
    }
  }

  function progressPct(modelId: string): number {
    return Math.round((modelStore.progressByModel[modelId] ?? 0) * 100);
  }

  function rowDownloading(modelId: string): boolean {
    return !!modelStore.downloadingByModel[modelId];
  }
</script>

<div class="flex h-full min-h-0 flex-1 flex-col">
  <h2 class="sf-headline-sm shrink-0 p-4 text-fg">Whisper models</h2>

  {#if modelStore.error}
    <p
      class="mx-4 shrink-0 rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-body-md text-destructive"
    >
      {modelStore.error}
    </p>
  {/if}

  <ScrollablePanel class="space-y-5 px-4 pb-4">
    <SettingsSection title="Default models">
      <SettingsList>
        <SettingsRow
          title="Default Scribe transcription model"
          description="Used for recordings created in Scribe."
          disabled={downloadedModels.length === 0}
        >
          {#snippet control()}
            <div class="w-full sm:w-56">
              <FieldRow
                label="Default Scribe transcription model"
                labelHidden={true}
                id="scribe-model-select"
                options={downloadedModels.map((m) => ({ value: m.id, label: m.label }))}
                emptyOption={!selectedId && downloadedModels.length > 0
                  ? { label: 'Choose model…', disabled: true }
                  : undefined}
                value={selectedId}
                onchange={(v: string) => { if (v) selectModel(v); }}
                disabled={downloadedModels.length === 0}
              />
            </div>
          {/snippet}
        </SettingsRow>

        <SettingsRow
          title="Dictate transcription model override"
          description="Leave as Scribe unless Dictate should use a separate model."
          disabled={downloadedModels.length === 0}
        >
          {#snippet control()}
            <div class="w-full sm:w-56">
              <FieldRow
                label="Dictate transcription model override"
                labelHidden={true}
                id="dictate-model-select"
                options={downloadedModels.map((m) => ({ value: m.id, label: m.label }))}
                emptyOption={{ label: 'Same as Scribe' }}
                value={dictateModelId ?? ''}
                onchange={onDictateModelChange}
                disabled={downloadedModels.length === 0}
              />
            </div>
          {/snippet}
        </SettingsRow>
      </SettingsList>
    </SettingsSection>

    <SettingsSection title="Voice activity detection">
      <SettingsList>
        <SettingsRow
          title="Silero VAD · 2 MB"
          description="Skips silence mid-recording. Reduces hallucinations."
        >
          {#snippet control()}
            {#if vadDownloaded}
              <IconButton
                icon={Trash2}
                variant="destructive"
                size="small"
                aria-label="Remove Silero VAD model"
                onclick={() => void removeVad()}
              />
            {:else}
              {@const vadPct = Math.round(
                (modelStore.progressByModel["vad"] ?? 0) * 100,
              )}
              {#if vadDownloading && vadPct > 0 && vadPct < 100}
                <div class="flex items-center gap-2">
                  <div class="h-0.5 w-20 overflow-hidden rounded-sm bg-fill">
                    <div
                      class="h-full rounded-sm bg-active transition-[width]"
                      style={`width:${vadPct}%`}
                    ></div>
                  </div>
                  <p class="sf-meta-sm text-fg-dim">
                    {vadPct}%
                  </p>
                </div>
              {:else if vadDownloading}
                <p class="sf-label-sm text-fg-dim">Starting…</p>
              {:else}
                <IconButton
                  icon={Download}
                  variant="normal"
                  size="small"
                  aria-label="Install Silero VAD model"
                  onclick={() => void downloadVad()}
                />
              {/if}
            {/if}
          {/snippet}
        </SettingsRow>
      </SettingsList>
    </SettingsSection>

    <SettingsSection title="Installed models">

      <SettingsList>
        {#each modelStore.models as model (model.id)}
          <SettingsRow title={model.label} direction="horizontal">
            {#if !model.downloaded && (rowDownloading(model.id) || (modelStore.progressByModel[model.id] ?? 0) > 0)}
              {@const pct = progressPct(model.id)}
              <div class="flex w-full items-center gap-1">
                <div class="h-0.5 min-w-24 flex-1 overflow-hidden rounded-sm bg-fill">
                  <div
                    class="h-full rounded-sm bg-active transition-[width]"
                    style={`width:${pct}%`}
                  ></div>
                </div>
                {#if pct < 100}
                  <p class="sf-meta-sm text-fg-dim">
                    {pct}%
                  </p>
                {:else}
                  <p class="sf-label-sm text-fg-dim">
                    Finalising…
                  </p>
                {/if}
              </div>
            {/if}
            <div class="flex items-center gap-2">
              {#if model.selected && model.downloaded}
                <Chip variant="brand">scribe</Chip>
              {/if}
              {#if model.downloaded && (model.id === dictateModelId || (dictateModelId === null && model.selected))}
                <Chip variant="focus">dictate</Chip>
              {/if}
            </div>

            {#snippet control()}
              <span class="sf-meta-sm w-14 text-left text-fg-dim"
                >{model.size_mb} MB</span
              >
              <div class="w-8 flex items-start justify-end pt-0.5">
                {#if model.downloaded && !rowDownloading(model.id)}
                  <IconButton
                    icon={Trash2}
                    variant="destructive"
                    size="small"
                    aria-label={`Remove ${model.label}`}
                    onclick={() => void removeModel(model.id)}
                  />
                {:else}
                  <IconButton
                    icon={Download}
                    variant="normal"
                    size="small"
                    disabled={!!modelStore.downloadingByModel[model.id]}
                    aria-label={rowDownloading(model.id)
                      ? `Installing ${model.label}`
                      : `Install ${model.label}`}
                    onclick={() => void downloadModel(model.id)}
                  />
                {/if}
              </div>
            {/snippet}
          </SettingsRow>
        {/each}
      </SettingsList>
    </SettingsSection>
  </ScrollablePanel>
</div>

<Toast message={toast.message} state={toast.state} />
