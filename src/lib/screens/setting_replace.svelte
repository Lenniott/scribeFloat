<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@lib/components/Button.svelte";
	import IconButton from "@lib/components/IconButton.svelte";
	import LabeledTextField from "@lib/components/form/LabeledTextField.svelte";
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

	const selectClass =
		"h-10 rounded-md border border-rim bg-panel px-2 text-body-md text-fg font-sans cursor-pointer focus:outline-none focus:ring-2 focus:ring-focus focus:ring-offset-2 focus:ring-offset-canvas";
</script>

<section class="flex flex-col gap-4">
	<div class="flex items-start justify-between gap-4">
		<div class="flex flex-col gap-1">
			<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">
				Word replacements
			</span>
			<p class="text-body-md text-fg/50">
				Spoken trigger words are substituted in transcripts and dictated text. Matching is case-insensitive and whole-word only.
			</p>
		</div>
		{#if !showAddForm}
			<Button variant="normal" size="small" onclick={startAdd}>Add rule</Button>
		{/if}
	</div>

	{#if rules.length > 0}
		<div class="flex flex-col divide-y divide-rim/40 rounded-md border border-rim/40 overflow-hidden">
			{#each rules as rule, i (i)}
				<div class="flex items-center gap-2 px-3 py-2 bg-card">
					<span class="font-mono text-label-md text-fg min-w-0 truncate">{rule.trigger}</span>
					<span class="text-label-sm text-fg/50 shrink-0">{ruleDescription(rule)}</span>
					<div class="flex items-center gap-1 ml-auto shrink-0">
						{#if rule.scope === "both" || rule.scope === "transcripts"}
							<span class="sf-label-sm shrink-0 rounded-sm border border-brand bg-brand/10 px-1 py-px text-brand">transcripts</span>
						{/if}
						{#if rule.scope === "both" || rule.scope === "dictate"}
							<span class="sf-label-sm shrink-0 rounded-sm border border-focus bg-focus/15 px-1 py-px text-focus">dictate</span>
						{/if}
					</div>
					<IconButton icon={Pencil} size="small" variant="normal" aria-label="Edit rule" onclick={() => startEdit(i)} />
					<IconButton icon={Trash2} size="small" variant="destructive" aria-label="Delete rule" onclick={() => deleteRule(i)} />
				</div>
			{/each}
		</div>
	{:else if !showAddForm}
		<p class="text-body-md text-fg/40">No replacement rules yet.</p>
	{/if}

	{#if showAddForm}
		<div class="flex flex-col gap-3 border-t border-rim/30 pt-3 mt-1">
			<div class="grid grid-cols-[1fr_auto_auto] gap-2 items-end">
				<LabeledTextField
					label="Trigger word or phrase"
					bind:value={form.trigger}
					placeholder="e.g. close bracket"
				/>
				<div class="flex flex-col gap-1">
					<span class="text-label-sm text-fg/80">Type</span>
					<select class={selectClass} bind:value={form.type}>
						<option value="simple">Replace</option>
						<option value="newline">New line</option>
						<option value="wrap">Wrap</option>
					</select>
				</div>
				<div class="flex flex-col gap-1">
					<span class="text-label-sm text-fg/80">Apply to</span>
					<select class={selectClass} bind:value={form.scope}>
						<option value="both">Both</option>
						<option value="transcripts">Transcripts</option>
						<option value="dictate">Dictate</option>
					</select>
				</div>
			</div>

			<LabeledTextField
				label="Also matches (comma-separated)"
				bind:value={aliasesText}
				placeholder="e.g. closed bracket, shut bracket"
			/>

			{#if form.type === "simple"}
				<LabeledTextField
					label="Replace with"
					bind:value={form.output}
					placeholder="e.g. ]"
					multiline={true}
				/>
			{:else if form.type === "wrap"}
				<div class="grid grid-cols-[1fr_1fr_auto] gap-2 items-end">
					<LabeledTextField label="Prefix" bind:value={form.prefix} placeholder="e.g. #" />
					<LabeledTextField label="Suffix" bind:value={form.suffix} placeholder="" />
					<div class="flex flex-col gap-1">
						<span class="text-label-sm text-fg/80">Transform</span>
						<select class={selectClass} bind:value={form.transform}>
							<option value="none">None</option>
							<option value="lower">lowercase</option>
							<option value="upper">UPPERCASE</option>
							<option value="sentence">Sentence</option>
						</select>
					</div>
				</div>
			{/if}

			<div class="flex items-center gap-2">
				<Button variant="primary" size="small" onclick={saveForm}>
					{editingIndex !== null ? "Update" : "Add rule"}
				</Button>
				<Button variant="ghost" size="small" onclick={cancelForm}>Cancel</Button>
				{#if message}
					<span class="text-label-sm text-fg/70">{message}</span>
				{/if}
			</div>
		</div>
	{/if}
</section>
