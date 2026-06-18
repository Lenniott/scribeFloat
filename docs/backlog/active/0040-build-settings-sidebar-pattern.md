---
id: "0040"
title: Build Settings sidebar pattern — sidebar swap on settings navigation
status: done
---

# Build Settings sidebar pattern

When the user navigates to Settings, the app sidebar is replaced by a settings-specific sidebar (with back button). Navigating back restores the previous Area.

## Acceptance criteria

- `SettingsSidebar` component: back button + tab list
- `SettingsPanel` component: renders active settings tab content
- Back button label shows previous Area name
- `settingsTypes.ts` defines the `SettingsTab` union type
- Tabs: General, Models, Permissions, Hotkeys (match existing settings screen content)

## Reference

Exploration: `src/lib/components/settings/` on `release/0.3`.
