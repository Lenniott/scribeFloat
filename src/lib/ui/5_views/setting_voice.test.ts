import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import SettingVoice from './setting_voice.svelte';

const mockedInvoke = vi.mocked(invoke);

type NameEntry = { name: string; slug: string };

function stubInvoke(
	names: NameEntry[],
	overrides: Record<string, unknown | ((args: unknown) => unknown)> = {},
) {
	mockedInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
		if (cmd in overrides) {
			const value = overrides[cmd];
			return typeof value === 'function' ? (value as (a: unknown) => unknown)(args) : value;
		}
		switch (cmd) {
			case 'speaker_names_list':
				return names;
			case 'speaker_name_save':
				return { name: 'New', slug: 'new' };
			case 'speaker_name_delete':
				return true;
			default:
				return undefined;
		}
	});
}

async function renderView() {
	const result = render(SettingVoice);
	await waitFor(() => {
		expect(mockedInvoke).toHaveBeenCalledWith('speaker_names_list');
	});
	return result;
}

describe('setting_voice speaker names', () => {
	beforeEach(() => {
		mockedInvoke.mockReset();
	});

	it('renders saved names', async () => {
		stubInvoke([
			{ name: 'Ben', slug: 'ben' },
			{ name: 'Sarah', slug: 'sarah' },
		]);
		await renderView();

		expect(screen.getByText('Ben')).toBeInTheDocument();
		expect(screen.getByText('Sarah')).toBeInTheDocument();
	});

	it('adds a name via speaker_name_save', async () => {
		stubInvoke([]);
		await renderView();

		const field = screen.getByLabelText('Speaker name');
		await fireEvent.input(field, { target: { value: 'Ben' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Add name' }));

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('speaker_name_save', { name: 'Ben' });
		});
	});

	it('shows backend errors when adding fails', async () => {
		stubInvoke([{ name: 'Ben', slug: 'ben' }], {
			speaker_name_save: () => {
				throw new Error('a speaker named "Ben" already exists');
			},
		});
		await renderView();

		const field = screen.getByLabelText('Speaker name');
		await fireEvent.input(field, { target: { value: 'Ben' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Add name' }));

		await waitFor(() => {
			expect(screen.getByText(/already exists/)).toBeInTheDocument();
		});
	});

	it('renames a name with previousSlug', async () => {
		stubInvoke([{ name: 'Ben', slug: 'ben' }]);
		await renderView();

		await fireEvent.click(screen.getByRole('button', { name: 'Rename' }));
		const field = screen.getByLabelText('Name');
		await fireEvent.input(field, { target: { value: 'Benjamin' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('speaker_name_save', {
				name: 'Benjamin',
				previousSlug: 'ben',
			});
		});
	});

	it('deletes a name after confirmation', async () => {
		stubInvoke([{ name: 'Ben', slug: 'ben' }]);
		await renderView();

		await fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Remove name' }));

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('speaker_name_delete', { slug: 'ben' });
		});
	});

	it('contains no voiceprint or enrollment language', async () => {
		stubInvoke([{ name: 'Ben', slug: 'ben' }]);
		const { container } = await renderView();

		expect(container.textContent).not.toMatch(/voiceprint|enroll|refine|clip/i);
	});
});
