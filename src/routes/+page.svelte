<script lang="ts">
	import { browser } from '$app/environment';
	import DictateScreen from '@lib/screens/dictate.svelte';
	import AppShell from '@lib/screens/app-shell.svelte';
	import OnboardingScreen from '@lib/screens/onboarding.svelte';

	const viewParam = browser ? new URLSearchParams(window.location.search).get('view') : null;
	const standaloneDictate = viewParam === 'dictate';
	const standaloneOnboarding = viewParam === 'onboarding';
</script>

{#if standaloneOnboarding}
	<OnboardingScreen />
{:else if standaloneDictate}
	<DictateScreen />
{:else}
	<AppShell />
{/if}
