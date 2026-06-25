export type SettingsTab = 'general' | 'permissions' | 'models' | 'voice' | 'replacements' | 'help';

export const SETTINGS_TABS: Array<{ id: SettingsTab; label: string }> = [
	{ id: 'general', label: 'General' },
	{ id: 'permissions', label: 'Permissions' },
	{ id: 'models', label: 'Models' },
	{ id: 'voice', label: 'Voice' },
	{ id: 'replacements', label: 'Replacements' },
	{ id: 'help', label: 'Help' },
];
