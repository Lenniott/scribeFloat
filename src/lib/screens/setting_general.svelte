<script>
  import ConfigField from "@lib/components/form/ConfigField.svelte";
  import DeviceSelect from "@lib/components/form/DeviceSelect.svelte";
  import LabeledTextField from "@lib/components/form/LabeledTextField.svelte";
  import OptionGroup from "@lib/components/form/OptionGroup.svelte";
  import PathSelectorField from "@lib/components/form/PathSelectorField.svelte";
  import ToggleSwitch from "@lib/components/form/ToggleSwitch.svelte";

</script>

Settings general tab

DEFAULT SAVE FOLDER
~/transcripts Browse {PathSelectorField}

DEFAULT MICROPHONE
MacBook Pro Microphone {OptionGroup}

Delete WAV FILES AFTER TRANSCRIPTION
OFF {ToggleSwitch}


DICTATION AUTO-ENTER AFTER PASTE
OFF {ToggleSwitch}

START ON LOGIN
OFF {ToggleSwitch}


OPEN TRANSCRIPTS WITH
cursor Change {DeviceSelect}

SCRIBE LABELS
Mic {LabeledTextField}
Speaker {LabeledTextField}


KEYBOARD SHORTCUTS

Open Scribe ^ T l Change  {ConfigField}

Dictate trigger key Left Ctrl Change {ConfigField}

DICTATE MODES (ALWAYS BOTH ACTIVE)
Double-tap → toggle recording
Hold → record while held