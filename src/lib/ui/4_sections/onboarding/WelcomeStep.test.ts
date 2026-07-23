import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import WelcomeStep from './WelcomeStep.svelte';

describe('WelcomeStep', () => {
	it('calls onStart when Get started is clicked', async () => {
		const onStart = vi.fn();
		render(WelcomeStep, { props: { onStart, onSkip: vi.fn() } });

		await fireEvent.click(screen.getByRole('button', { name: 'Get started' }));

		expect(onStart).toHaveBeenCalledOnce();
	});

	it('calls onSkip when Skip to Settings is clicked', async () => {
		const onSkip = vi.fn();
		render(WelcomeStep, { props: { onStart: vi.fn(), onSkip } });

		await fireEvent.click(screen.getByRole('button', { name: 'Skip to Settings' }));

		expect(onSkip).toHaveBeenCalledOnce();
	});
});
