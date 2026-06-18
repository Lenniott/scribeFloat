<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { emit } from "@tauri-apps/api/event";
  import Button from "@components/controls/Button.svelte";
  import TextField from "@primitives/form/TextField.svelte";
  import OptionGroup from "@components/controls/OptionGroup.svelte";
  import PathPicker from "@components/controls/PathPicker.svelte";
  import ToggleSwitch from "@components/controls/Toggle.svelte";
  import SettingsList from "@sections/SettingList.svelte";
  import SettingsRow from "@components/cards/SettingRow.svelte";
  import SettingsSection from "@primitives/form/SettingsSection.svelte";
  import { applyThemeMode, type ThemeMode } from '@utils/theme';
  import { dictateModifierLabel, formatHotkeyForDisplay } from '@utils/platform';
  import { appErrorMessage } from '@utils/types';

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
  let openHotkey = $state("");
  let dictateHotkey = $state("");
  let inputLabel = $state("");
  let outputLabel = $state("");
  let preferredInputDevice = $state("");
  let preferredSpeakerDevice = $state("");
  let scribeCaptureSpeaker = $state(false);
  let dictateAutoEnter = $state(false);
  let keepWav = $state(false);
  let saveTranscriptsAsMarkdown = $state(false);
  let themeMode = $state<ThemeMode>("system");
  let openWithApp = $state("");
  let message = $state("");
  let loadError = $state("");
  let messageClearId: ReturnType<typeof setTimeout> | undefined;

  const speakerCaptureAvailable = $derived(
    !speakerCaptureRequiresDeviceName ||
      (blackholeDetected && savedSpeakerDeviceName.trim().length > 0),
  );

  $effect(() => {
    applyThemeMode(themeMode);
  });

  const themeOptions = [
    { value: "system", label: "System" },
    { value: "dark", label: "Dark" },
    { value: "light", label: "Light" },
  ];

  async function refresh() {
    loadError = "";
    try {
      const [
        nextOutputPath,
        [open, dictate],
        [inLabel, outLabel],
        nextThemeMode,
        nextOpenWithApp,
        [preferredInput, preferredSpeaker],
        nextScribeCaptureSpeaker,
        nextDictateAutoEnter,
        nextKeepWav,
        nextSaveTranscriptsAsMarkdown,
      ] = await Promise.all([
        invoke<string>("settings_get_output_path"),
        invoke<[string, string]>("settings_get_hotkeys"),
        invoke<[string, string]>("settings_get_input_labels"),
        invoke<ThemeMode>("settings_get_theme_mode"),
        invoke<string | null>("settings_get_open_with_app_path"),
        invoke<[string | null, string | null]>(
          "settings_get_preferred_audio_devices",
        ),
        invoke<boolean>("settings_get_scribe_capture_speaker"),
        invoke<boolean>("settings_get_dictate_auto_enter"),
        invoke<boolean>("settings_get_keep_wav"),
        invoke<boolean>("settings_get_save_transcripts_as_markdown"),
      ]);

      outputPath = nextOutputPath;
      openHotkey = open;
      dictateHotkey = dictate;
      inputLabel = inLabel;
      outputLabel = outLabel;
      themeMode = nextThemeMode;
      openWithApp = nextOpenWithApp ?? "";
      preferredInputDevice = preferredInput ?? "";
      preferredSpeakerDevice = preferredSpeaker ?? "";
      scribeCaptureSpeaker = nextScribeCaptureSpeaker;
      dictateAutoEnter = nextDictateAutoEnter;
      keepWav = nextKeepWav;
      saveTranscriptsAsMarkdown = nextSaveTranscriptsAsMarkdown;
    } catch (e) {
      loadError = `Could not load settings: ${appErrorMessage(e)}`;
    }
  }

  async function saveAll() {
    if (messageClearId !== undefined) {
      clearTimeout(messageClearId);
      messageClearId = undefined;
    }
    message = "";
    try {
      const trimmedSpeaker = preferredSpeakerDevice.trim();
      const captureAvailableAfterSave =
        !speakerCaptureRequiresDeviceName ||
        (blackholeDetected && trimmedSpeaker.length > 0);
      const captureDefault = captureAvailableAfterSave && scribeCaptureSpeaker;
      await invoke("settings_save_general", {
        payload: {
          outputPath,
          openHotkey,
          dictateHotkey,
          inputLabel,
          outputLabel,
          preferredInputDevice: preferredInputDevice.trim() || null,
          preferredSpeakerDevice: trimmedSpeaker || null,
          scribeCaptureSpeaker,
          speakerCaptureAvailable: captureAvailableAfterSave,
          dictateAutoEnter,
          keepWav,
          saveTranscriptsAsMarkdown,
          themeMode,
          openWithAppPath: openWithApp.trim() || null,
        },
      });
      scribeCaptureSpeaker = captureDefault;
      preferredSpeakerDevice = trimmedSpeaker;
      onSpeakerConfigSaved?.(trimmedSpeaker);
      await emit("settings://speaker-capture-saved");
      message = "Saved";
      messageClearId = setTimeout(() => {
        message = "";
        messageClearId = undefined;
      }, 3000);
    } catch (e) {
      message = `Failed to save: ${appErrorMessage(e)}`;
    }
  }

  onMount(refresh);
  onDestroy(() => {
    if (messageClearId !== undefined) clearTimeout(messageClearId);
  });
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

  <SettingsSection title="Appearance">
    <SettingsList>
      <SettingsRow title="Theme">
        {#snippet control()}
          <OptionGroup
            name="theme-mode"
            label="Theme"
            labelHidden={true}
            options={themeOptions}
            bind:selected={themeMode}
          />
        {/snippet}
      </SettingsRow>
    </SettingsList>
  </SettingsSection>

  <SettingsSection title="Keyboard shortcuts">
    <SettingsList>
      <SettingsRow title="Scribe keyboard shortcut">
        {#snippet control()}
          <code class="flex justify-center w-34 h-10 items-center rounded-md bg-fill px-2 sf-label-md text-fg">
            {formatHotkeyForDisplay(openHotkey)}
          </code>
        {/snippet}
      </SettingsRow>
      <SettingsRow
        title="Dictate keyboard shortcut"
        description={`Double tap left ${dictateModifierLabel}, or tap and hold.`}
      >        {#snippet control()}
          <code class="flex justify-center w-34 h-10 items-center rounded-md bg-fill px-2 sf-label-md text-fg">
            {formatHotkeyForDisplay(dictateHotkey)}
          </code>
        {/snippet}
		</SettingsRow>
    </SettingsList>
  </SettingsSection>

  <SettingsSection title="Scribe">
    <SettingsList>
      <SettingsRow title="Default save folder">
        <PathPicker
          label="Default save folder"
          labelHidden={true}
          bind:path={outputPath}
        />
      </SettingsRow>

      <SettingsRow title="Save transcripts as Markdown">
        {#snippet control()}
          <ToggleSwitch
            checked={saveTranscriptsAsMarkdown}
            onchange={(next) => (saveTranscriptsAsMarkdown = next)}
            aria-label="Save transcripts as Markdown"
          />
        {/snippet}
      </SettingsRow>

      <SettingsRow title="Keep audio after transcription">
        {#snippet control()}
          <ToggleSwitch
            checked={keepWav}
            onchange={(next) => (keepWav = next)}
            aria-label="Keep audio after transcription"
          />
        {/snippet}
      </SettingsRow>
      
      {#if saveTranscriptsAsMarkdown}
        <SettingsRow title="Open transcripts with">
          <PathPicker
            label="Open transcripts with"
            labelHidden={true}
            bind:path={openWithApp}
            directory={false}
          />
        </SettingsRow>
      {/if}
    </SettingsList>
  </SettingsSection>

  <SettingsSection title="Scribe speaker capture">
    <SettingsList>
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
          />
        </SettingsRow>
      {/if}
      {#if speakerCaptureAvailable}
        <SettingsRow title="Input label">
          <TextField
            label="Input label"
            labelHidden={true}
            bind:value={inputLabel}
          />
        </SettingsRow>
        <SettingsRow title="Output label">
          <TextField
            label="Output label"
            labelHidden={true}
            bind:value={outputLabel}
          />
        </SettingsRow>
        <SettingsRow title="Capture speaker by default">
          {#snippet control()}
            <ToggleSwitch
              checked={scribeCaptureSpeaker}
              onchange={(next) => (scribeCaptureSpeaker = next)}
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
            onchange={(next) => (dictateAutoEnter = next)}
            aria-label="Press Enter after dictate"
          />
        {/snippet}
      </SettingsRow>
    </SettingsList>
  </SettingsSection>

  <div class="flex items-center gap-3 mt-14">
    <Button variant="primary" onclick={saveAll}>Save</Button>
    {#if message}
      <p class="sf-label-sm text-fg-dim">{message}</p>
    {/if}
  </div>
</section>
