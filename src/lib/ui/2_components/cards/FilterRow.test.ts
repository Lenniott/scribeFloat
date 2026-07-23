import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import FilterRow from './FilterRow.svelte';

describe('FilterRow', () => {
	it('renders label and count', () => {
		render(FilterRow, { props: { label: 'work', count: 3, checked: false } });

		expect(screen.getByText('work')).toBeInTheDocument();
		expect(screen.getByText('3')).toBeInTheDocument();
	});

	it('fires onchange with tag and checked value', async () => {
		const onchange = vi.fn();
		render(FilterRow, { props: { label: 'work', count: 3, checked: false, onchange } });

		await fireEvent.click(screen.getByRole('checkbox', { name: 'work' }));

		expect(onchange).toHaveBeenCalledWith(true);
	});
});
