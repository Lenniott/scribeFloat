<script lang="ts">
	import type {
		HelpBlock,
		HelpConditionKey,
		HelpContext,
		HelpContextKey,
		HelpInline,
	} from '@lib/content/helpContent.types';
	import HelpContentRenderer from './HelpContentRenderer.svelte';

	let {
		blocks,
		context,
		nested = false,
	}: { blocks: HelpBlock[]; context: HelpContext; nested?: boolean } = $props();

	function whenVisible(when: HelpConditionKey): boolean {
		return Boolean(context[when]);
	}

	function resolveVar(key: HelpContextKey): string {
		const value = context[key];
		return typeof value === 'string' ? value : '';
	}
</script>

{#snippet inlineSegments(segments: HelpInline[])}
	{#each segments as segment (segment)}
		{#if segment.type === 'text'}
			{segment.value}
		{:else if segment.type === 'strong'}
			<strong>{segment.value}</strong>
		{:else if segment.type === 'code'}
			<code class="sf-meta-sm rounded bg-fill px-1">{segment.value}</code>
		{:else if segment.type === 'var'}
			{#if segment.strong}
				<strong>{resolveVar(segment.value)}</strong>
			{:else}
				{resolveVar(segment.value)}
			{/if}
		{/if}
	{/each}
{/snippet}

{#snippet blockList()}
	{#each blocks as block, index (index)}
		{#if block.type === 'section'}
			<div class="space-y-2">
				<HelpContentRenderer blocks={block.blocks} {context} nested />
			</div>
		{:else if block.type === 'conditional'}
			{#if whenVisible(block.when)}
				<HelpContentRenderer blocks={block.blocks} {context} nested />
			{/if}
		{:else if block.type === 'heading'}
			{#if block.level === 2}
				<h2 class="sf-headline-sm text-fg">{block.text}</h2>
			{:else}
				<h3 class="sf-section-label text-fg-dim">{block.text}</h3>
			{/if}
		{:else if block.type === 'paragraph'}
			<p class="sf-body-md text-fg">
				{@render inlineSegments(block.inline)}
			</p>
		{:else if block.type === 'list'}
			<ul class="list-disc space-y-1 pl-5 sf-body-md text-fg">
				{#each block.items as item, itemIndex (itemIndex)}
					<li>{@render inlineSegments(item)}</li>
				{/each}
			</ul>
		{:else if block.type === 'table'}
			<div class="overflow-hidden rounded-md border border-card">
				<table class="w-full">
					<thead class="bg-fill">
						<tr>
							{#each block.headers as header (header)}
								<th class="px-3 py-2 text-left sf-label-sm text-fg-dim">{header}</th>
							{/each}
						</tr>
					</thead>
					<tbody class="divide-y divide-card">
						{#each block.rows as row, rowIndex (rowIndex)}
							<tr>
								{#each row as cell, cellIndex (cellIndex)}
									<td
										class="px-3 py-2 sf-body-md {cellIndex === 0
											? 'text-fg'
											: 'text-fg-dim'}"
									>
										{@render inlineSegments(cell)}
									</td>
								{/each}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{:else if block.type === 'link'}
			<a class="sf-body-md text-brand underline" href={block.href} target="_blank" rel="noreferrer">
				{block.text}
			</a>
		{/if}
	{/each}
{/snippet}

{#if nested}
	{@render blockList()}
{:else}
	<div class="space-y-4 w-full">
		{@render blockList()}
	</div>
{/if}
