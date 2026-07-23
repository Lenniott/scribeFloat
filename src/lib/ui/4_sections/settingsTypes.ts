export type SettingsTab = 'general' | 'advanced' | 'voice' | 'permissions' | 'help';

export const SETTINGS_TABS: Array<{ id: SettingsTab; label: string }> = [
	{ id: 'general', label: 'General' },
	{ id: 'advanced', label: 'Advanced' },
	{ id: 'voice', label: 'Voices' },
	{ id: 'permissions', label: 'Permissions' },
	{ id: 'help', label: 'Help' },
];
