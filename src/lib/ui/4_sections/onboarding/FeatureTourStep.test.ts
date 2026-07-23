import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import FeatureTourStep from './FeatureTourStep.svelte';

describe('FeatureTourStep', () => {
	it('calls onFinish when Done is clicked', async () => {
		const onFinish = vi.fn();
		render(FeatureTourStep, { props: { onBack: vi.fn(), onFinish } });

		await fireEvent.click(screen.getByRole('button', { name: 'Done' }));

		expect(onFinish).toHaveBeenCalledOnce();
	});

	it('calls onBack when Back is clicked', async () => {
		const onBack = vi.fn();
		render(FeatureTourStep, { props: { onBack, onFinish: vi.fn() } });

		await fireEvent.click(screen.getByRole('button', { name: 'Back' }));

		expect(onBack).toHaveBeenCalledOnce();
	});
});
