import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ButtonHarness from './Button.test.harness.svelte';

describe('Button', () => {
	it('fires onclick when clicked', async () => {
		const onclick = vi.fn();
		render(ButtonHarness, { props: { onclick } });

		await fireEvent.click(screen.getByRole('button', { name: 'Click me' }));
		expect(onclick).toHaveBeenCalledOnce();
	});

	it('marks the button disabled when the disabled prop is set', () => {
		render(ButtonHarness, { props: { onclick: vi.fn(), disabled: true } });
		expect(screen.getByRole('button', { name: 'Click me' })).toBeDisabled();
	});
});
