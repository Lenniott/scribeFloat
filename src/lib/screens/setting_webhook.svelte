<script lang="ts">
	import SettingsList from "@lib/components/settings/SettingsList.svelte";
	import SettingsRow from "@lib/components/settings/SettingsRow.svelte";
	import SettingsSection from "@lib/components/settings/SettingsSection.svelte";
</script>

<section class="space-y-4">
	<h2 class="sf-headline-sm text-fg">Webhook</h2>
	<SettingsSection title="Output">
		<SettingsList>
			<SettingsRow
				title="Webhook output"
				description="Webhook output is not yet available. Transcripts are saved locally only."
			/>
		</SettingsList>
	</SettingsSection>
</section>
