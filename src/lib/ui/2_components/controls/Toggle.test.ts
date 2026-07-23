import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Toggle from './Toggle.svelte';

describe('Toggle', () => {
	it('fires onchange with toggled value', async () => {
		const onchange = vi.fn();
		render(Toggle, { props: { checked: false, 'aria-label': 'Test toggle', onchange } });

		await fireEvent.click(screen.getByRole('switch', { name: 'Test toggle' }));

		expect(onchange).toHaveBeenCalledWith(true);
	});

	it('does not fire onchange when disabled', async () => {
		const onchange = vi.fn();
		render(Toggle, {
			props: { checked: false, disabled: true, 'aria-label': 'Test toggle', onchange },
		});

		await fireEvent.click(screen.getByRole('switch', { name: 'Test toggle' }));

		expect(onchange).not.toHaveBeenCalled();
	});
});
