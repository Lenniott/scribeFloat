export const ACCORDION_KEY = Symbol("accordion");

export type AccordionContextState = {
	openId: string | null;
	toggle: (id: string) => void;
};
