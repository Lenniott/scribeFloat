<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@lib/components/ui/controls/Button.svelte";
	import Chip from "@lib/components/primitives/display/Chip.svelte";
	import IconButton from "@lib/components/ui/controls/IconButton.svelte";
	import FieldRow from "@lib/components/primitives/form/FieldRow.svelte";
	import TextField from "@lib/components/primitives/form/TextField.svelte";
	import SettingsList from "@lib/components/sections/SettingList.svelte";
	import SettingsRow from "@lib/components/ui/cards/SettingRow.svelte";
	import SettingsSection from "@lib/components/primitives/form/SettingsSection.svelte";
	import { Pencil, Trash2 } from "lucide-svelte";

	type RuleType = "simple" | "newline" | "wrap";
	type Scope = "both" | "transcripts" | "dictate";
	type Transform = "none" | "lower" | "upper" | "sentence";

	interface ReplacementRule {
		trigger: string;
		aliases: string[];
		type: RuleType;
		output: string;
		scope: Scope;
		prefix: string;
		suffix: string;
		transform: Transform;
	}

	let rules = $state<ReplacementRule[]>([]);
	let globalPrefix = $state("float");
	let prefixMessage = $state("");
	let message = $state("");
	let editingIndex = $state<number | null>(null);
	let showAddForm = $state(false);
	let aliasesText = $state("");

	const emptyRule = (): ReplacementRule => ({
		trigger: "",
		aliases: [],
		type: "simple",
		output: "",
		scope: "both",
		prefix: "",
		suffix: "",
		transform: "none",
	});

	let form = $state<ReplacementRule>(emptyRule());

	async function refresh() {
		rules = await invoke<ReplacementRule[]>("settings_get_replacement_rules").catch(() => []);
		globalPrefix = await invoke<string>("settings_get_replacement_prefix").catch(() => "float");
	}

	async function savePrefix() {
		prefixMessage = "";
		try {
			await invoke("settings_set_replacement_prefix", { prefix: globalPrefix.trim() });
			prefixMessage = "Saved";
			setTimeout(() => { prefixMessage = ""; }, 2500);
		} catch (e) {
			prefixMessage = String(e);
		}
	}

	onMount(refresh);

	function startAdd() {
		form = emptyRule();
		aliasesText = "";
		editingIndex = null;
		showAddForm = true;
	}

	function startEdit(index: number) {
		form = { ...rules[index] };
		aliasesText = rules[index].aliases.join(", ");
		editingIndex = index;
		showAddForm = true;
	}

	function cancelForm() {
		showAddForm = false;
		editingIndex = null;
		form = emptyRule();
		aliasesText = "";
	}

	function showMessage(msg: string) {
		message = msg;
		setTimeout(() => { message = ""; }, 2500);
	}

	async function saveForm() {
		const rule = { ...form };
		rule.aliases = aliasesText
			.split(",")
			.map((s) => s.trim())
			.filter((s) => s.length > 0);
		if (rule.type !== "wrap") {
			rule.prefix = "";
			rule.suffix = "";
			rule.transform = "none";
		}
		if (rule.type !== "simple") {
			rule.output = "";
		}
		try {
			if (editingIndex !== null) {
				await invoke("settings_update_replacement_rule", { index: editingIndex, rule });
			} else {
				await invoke("settings_add_replacement_rule", { rule });
			}
			await refresh();
			cancelForm();
		} catch (e) {
			showMessage(String(e));
		}
	}

	async function deleteRule(index: number) {
		try {
			await invoke("settings_delete_replacement_rule", { index });
			await refresh();
		} catch (e) {
			showMessage(String(e));
		}
	}

	function ruleDescription(rule: ReplacementRule): string {
		if (rule.type === "simple") return `→ "${rule.output}"`;
		if (rule.type === "newline") return "→ line break";
		return `→ ${rule.prefix}word${rule.suffix}${rule.transform !== "none" ? ` (${rule.transform})` : ""}`;
	}

	const ruleTypeOptions = [
		{ value: "simple", label: "Replace" },
		{ value: "newline", label: "New line" },
		{ value: "wrap", label: "Wrap" },
	];
	const scopeOptions = [
		{ value: "both", label: "Both" },
		{ value: "transcripts", label: "Transcripts" },
		{ value: "dictate", label: "Dictate" },
	];
	const transformOptions = [
		{ value: "none", label: "None" },
		{ value: "lower", label: "lowercase" },
		{ value: "upper", label: "UPPERCASE" },
		{ value: "sentence", label: "Sentence" },
	];
</script>

<section class="flex flex-col gap-5">
	<SettingsSection title="Trigger prefix">
		<SettingsList>
			<SettingsRow
				title="Trigger prefix"
				description={`Triggers replacements (e.g. "float dash → -").\nLeave empty for no trigger prefix.`}
			>
				{#snippet control()}
					<div class="flex w-full items-end gap-2 sm:w-72">
						<div class="min-w-0 flex-1">
							<TextField
								label="Trigger prefix"
								labelHidden={true}
								bind:value={globalPrefix}
								placeholder="e.g. float"
							/>
						</div>
						<Button variant="normal" onclick={savePrefix}>Save</Button>
					</div>
				{/snippet}
				{#if prefixMessage}
					<span class="sf-label-sm text-fg-dim">{prefixMessage}</span>
				{/if}
			</SettingsRow>
		</SettingsList>
	</SettingsSection>

	<SettingsSection
		title="Word replacements"
		description="Spoken trigger words are substituted in transcripts (case-insensitive)."
	>
		{#snippet action()}
			{#if !showAddForm}
				<Button variant="normal" onclick={startAdd}>Add rule</Button>
			{/if}
		{/snippet}
	</SettingsSection>
	{#if showAddForm}
		<SettingsSection title={editingIndex !== null ? "Edit rule" : "Add rule"}>
			<SettingsList>
				<SettingsRow title="Trigger word or phrase">
					<div class="grid gap-2 sm:grid-cols-[minmax(0,1fr)_10rem_10rem] sm:items-end">
						<TextField
							label="Trigger word or phrase"
							labelHidden={true}
							bind:value={form.trigger}
							placeholder="e.g. close bracket"
						/>
						<FieldRow
							label="Type"
							value={form.type}
							options={ruleTypeOptions}
							onchange={(value) => (form.type = value as RuleType)}
						/>
						<FieldRow
							label="Apply to"
							value={form.scope}
							options={scopeOptions}
							onchange={(value) => (form.scope = value as Scope)}
						/>
					</div>
				</SettingsRow>

				<SettingsRow title="Also matches">
					<TextField
						label="Also matches (comma-separated)"
						labelHidden={true}
						bind:value={aliasesText}
						placeholder="e.g. closed bracket, shut bracket"
					/>
				</SettingsRow>

				{#if form.type === "simple"}
					<SettingsRow title="Replace with">
						<TextField
							label="Replace with"
							labelHidden={true}
							bind:value={form.output}
							placeholder="e.g. ]"
							multiline={true}
						/>
					</SettingsRow>
				{:else if form.type === "wrap"}
					<SettingsRow title="Wrap output">
						<div class="grid gap-2 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_10rem] sm:items-end">
							<TextField label="Prefix" bind:value={form.prefix} placeholder="e.g. #" />
							<TextField label="Suffix" bind:value={form.suffix} placeholder="" />
							<FieldRow
								label="Transform"
								value={form.transform}
								options={transformOptions}
								onchange={(value) => (form.transform = value as Transform)}
							/>
						</div>
					</SettingsRow>
				{/if}
			</SettingsList>

			<div class="flex items-center gap-2">
				<Button variant="primary" size="small" onclick={saveForm}>
					{editingIndex !== null ? "Update" : "Add rule"}
				</Button>
				<Button variant="ghost" size="small" onclick={cancelForm}>Cancel</Button>
				{#if message}
					<span class="sf-label-sm text-fg-dim">{message}</span>
				{/if}
			</div>
		</SettingsSection>
	{/if}
	{#if rules.length > 0}
		<SettingsList>
			{#each rules as rule, i (i)}
				<div class="flex items-center gap-2 px-3 py-2.5">
					<span class="sf-label-md text-fg min-w-0 truncate">{globalPrefix ? `${globalPrefix} ` : ""}{rule.trigger}</span>
					<span class="sf-label-sm text-fg-dim shrink-0">{ruleDescription(rule)}</span>
					<div class="flex items-center gap-2 ml-auto shrink-0">
						{#if rule.scope === "both" || rule.scope === "transcripts"}
							<Chip variant="brand">transcripts</Chip>
						{/if}
						{#if rule.scope === "both" || rule.scope === "dictate"}
							<Chip variant="focus">dictate</Chip>
						{/if}
					</div>
					<IconButton icon={Pencil} size="small" variant="normal" aria-label="Edit rule" onclick={() => startEdit(i)} />
					<IconButton icon={Trash2} size="small" variant="destructive" aria-label="Delete rule" onclick={() => deleteRule(i)} />
				</div>
			{/each}
		</SettingsList>
	{:else if !showAddForm}
		<p class="sf-body-md text-fg-muted">No replacement rules yet.</p>
	{/if}


</section>
