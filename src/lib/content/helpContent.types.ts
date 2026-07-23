export type HelpContextKey =
	| 'dictateModifierLabel'
	| 'openRecordHotkey'
	| 'isWindows'
	| 'speakerCaptureRequiresDeviceName';

export type HelpContext = Record<HelpContextKey, string | boolean>;

export type HelpConditionKey = 'isWindows' | 'speakerCaptureRequiresDeviceName';

export type HelpInline =
	| { type: 'text'; value: string }
	| { type: 'strong'; value: string }
	| { type: 'code'; value: string }
	| { type: 'var'; value: HelpContextKey; strong?: boolean };

export type HelpTableCell = HelpInline[];
export type HelpTableRow = HelpTableCell[];

export type HelpBlock =
	| { type: 'section'; blocks: HelpBlock[] }
	| { type: 'conditional'; when: HelpConditionKey; blocks: HelpBlock[] }
	| { type: 'heading'; level: 2 | 3; text: string }
	| { type: 'paragraph'; inline: HelpInline[] }
	| { type: 'list'; items: HelpInline[][] }
	| { type: 'table'; headers: string[]; rows: HelpTableRow[] }
	| { type: 'link'; text: string; href: string };

export type HelpContent = {
	blocks: HelpBlock[];
};
