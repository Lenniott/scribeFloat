<script lang="ts">
	import Accordion from './Accordion.svelte';
	import AccordionRow from '@components/nav/AccordionRow.svelte';
</script>

<Accordion>
	<AccordionRow id="first" title="First">
		<p>First panel</p>
	</AccordionRow>
	<AccordionRow id="second" title="Second">
		<p>Second panel</p>
	</AccordionRow>
</Accordion>
