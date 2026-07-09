<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { emit } from "@tauri-apps/api/event";
  import Toast from "@components/indicators/Toast.svelte";
  import TextField from "@primitives/form/TextField.svelte";
  import PathPicker from "@components/controls/PathPicker.svelte";
  import ToggleSwitch from "@components/controls/Toggle.svelte";
  import SettingsList from "@sections/SettingList.svelte";
  import SettingsRow from "@components/cards/SettingRow.svelte";
  import SettingsSection from "@primitives/form/SettingsSection.svelte";
  import { appErrorMessage } from '@utils/types';
  import { createToast } from '@stores/toast.svelte';

  let {
    savedSpeakerDeviceName = "",
    blackholeDetected = false,
    speakerCaptureRequiresDeviceName = false,
    onSpeakerConfigSaved,
  }: {
    savedSpeakerDeviceName?: string;
    blackholeDetected?: boolean;
    speakerCaptureRequiresDeviceName?: boolean;
    onSpeakerConfigSaved?: (name: string) => void;
  } = $props();

  let outputPath = $state("");
  let preferredInputDevice = $state("");
  let preferredSpeakerDevice = $state("");
  let scribeCaptureSpeaker = $state(false);
  let dictateAutoEnter = $state(false);
  let actionError = $state("");
  let loadError = $state("");
  const toast = createToast();

  const speakerCaptureAvailable = $derived(
    !speakerCaptureRequiresDeviceName ||
      (blackholeDetected && savedSpeakerDeviceName.trim().length > 0),
  );

  async function refresh() {
    loadError = "";
    try {
      const [
        nextOutputPath,
        [preferredInput, preferredSpeaker],
        nextScribeCaptureSpeaker,
        nextDictateAutoEnter,
      ] = await Promise.all([
        invoke<string>("settings_get_output_path"),
        invoke<[string | null, string | null]>(
          "settings_get_preferred_audio_devices",
        ),
        invoke<boolean>("settings_get_scribe_capture_speaker"),
        invoke<boolean>("settings_get_dictate_auto_enter"),
      ]);

      outputPath = nextOutputPath;
      preferredInputDevice = preferredInput ?? "";
      preferredSpeakerDevice = preferredSpeaker ?? "";
      scribeCaptureSpeaker = nextScribeCaptureSpeaker;
      dictateAutoEnter = nextDictateAutoEnter;
    } catch (e) {
      loadError = `Could not load settings: ${appErrorMessage(e)}`;
    }
  }

  async function saveOutputPath(path: string) {
    actionError = "";
    try {
      await invoke("settings_set_output_path", { path });
      toast.show("Saved", "success");
    } catch (e) {
      actionError = `Could not save save folder: ${appErrorMessage(e)}`;
      await refresh();
    }
  }

  async function saveSpeakerDeviceName() {
    const trimmed = preferredSpeakerDevice.trim();
    actionError = "";
    try {
      await invoke("settings_set_preferred_audio_devices", {
        preferredInputDevice: preferredInputDevice.trim() || null,
        preferredSpeakerDevice: trimmed || null,
      });
      preferredSpeakerDevice = trimmed;
      onSpeakerConfigSaved?.(trimmed);
      await emit("settings://speaker-capture-saved");
      toast.show("Saved", "success");
    } catch (e) {
      actionError = `Could not save speaker device: ${appErrorMessage(e)}`;
    }
  }

  async function setScribeCaptureSpeaker(enabled: boolean) {
    const previous = scribeCaptureSpeaker;
    scribeCaptureSpeaker = enabled;
    actionError = "";
    try {
      await invoke("settings_set_scribe_capture_speaker", { enabled });
      toast.show("Saved", "success");
    } catch (e) {
      scribeCaptureSpeaker = previous;
      actionError = `Could not save speaker capture setting: ${appErrorMessage(e)}`;
    }
  }

  async function setDictateAutoEnter(enabled: boolean) {
    const previous = dictateAutoEnter;
    dictateAutoEnter = enabled;
    actionError = "";
    try {
      await invoke("settings_set_dictate_auto_enter", { enabled });
      toast.show("Saved", "success");
    } catch (e) {
      dictateAutoEnter = previous;
      actionError = `Could not save dictate setting: ${appErrorMessage(e)}`;
    }
  }

  onMount(refresh);
  onDestroy(() => toast.dismiss());
</script>

<section class="space-y-5">
  <h2 class="sf-headline-sm text-fg">General settings</h2>

  {#if loadError}
    <p
      class="rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive"
    >
      {loadError}
    </p>
  {/if}
  {#if actionError}
    <p
      class="rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive"
    >
      {actionError}
    </p>
  {/if}

  <SettingsSection title="Scribe">
    <SettingsList>
      <SettingsRow title="Default save folder">
        <PathPicker
          label="Default save folder"
          labelHidden={true}
          bind:path={outputPath}
          onChange={(next) => void saveOutputPath(next)}
        />
      </SettingsRow>

      {#if speakerCaptureRequiresDeviceName}
        <SettingsRow
          title="Speaker capture device name"
          description="Type the exact Audio MIDI device name."
        >
          <TextField
            label="Speaker capture device name"
            labelHidden={true}
            bind:value={preferredSpeakerDevice}
            placeholder="Type the exact Audio MIDI device name"
            onblur={() => void saveSpeakerDeviceName()}
          />
        </SettingsRow>
      {/if}

      {#if speakerCaptureAvailable}
        <SettingsRow title="Capture speaker by default">
          {#snippet control()}
            <ToggleSwitch
              checked={scribeCaptureSpeaker}
              onchange={(next) => void setScribeCaptureSpeaker(next)}
              aria-label="Capture speaker by default"
            />
          {/snippet}
        </SettingsRow>
      {/if}
    </SettingsList>
  </SettingsSection>

  <SettingsSection title="Dictate">
    <SettingsList>
      <SettingsRow title="Press Enter after dictate">
        {#snippet control()}
          <ToggleSwitch
            checked={dictateAutoEnter}
            onchange={(next) => void setDictateAutoEnter(next)}
            aria-label="Press Enter after dictate"
          />
        {/snippet}
      </SettingsRow>
    </SettingsList>
  </SettingsSection>
</section>

<Toast message={toast.message} state={toast.state} position="bottom-center" />
