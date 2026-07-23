import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import AccordionHarness from './Accordion.test.harness.svelte';

describe('Accordion', () => {
	it('opens only one row at a time', async () => {
		render(AccordionHarness);

		const first = screen.getByRole('button', { name: 'First' });
		const second = screen.getByRole('button', { name: 'Second' });

		expect(first).toHaveAttribute('aria-expanded', 'false');
		expect(second).toHaveAttribute('aria-expanded', 'false');

		await fireEvent.click(first);
		expect(first).toHaveAttribute('aria-expanded', 'true');
		expect(second).toHaveAttribute('aria-expanded', 'false');

		await fireEvent.click(second);
		expect(first).toHaveAttribute('aria-expanded', 'false');
		expect(second).toHaveAttribute('aria-expanded', 'true');
	});

	it('closes an open row when clicked again', async () => {
		render(AccordionHarness);

		const first = screen.getByRole('button', { name: 'First' });
		await fireEvent.click(first);
		expect(first).toHaveAttribute('aria-expanded', 'true');

		await fireEvent.click(first);
		expect(first).toHaveAttribute('aria-expanded', 'false');
	});
});
