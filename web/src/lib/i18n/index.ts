import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import en from './en.json';

type TranslationDict = Record<string, string>;

function persistentStore<T>(key: string, initial: T) {
	const stored = browser ? localStorage.getItem(key) : null;
	const value = stored ? JSON.parse(stored) : initial;
	const store = writable<T>(value);
	if (browser) {
		store.subscribe((v) => localStorage.setItem(key, JSON.stringify(v)));
	}
	return store;
}

export const locale = persistentStore<string>('gt_locale', 'en');

const translations = writable<TranslationDict>(flattenTranslations(en));

function flattenTranslations(obj: Record<string, unknown>, prefix = ''): TranslationDict {
	const result: TranslationDict = {};
	for (const [key, value] of Object.entries(obj)) {
		const fullKey = prefix ? `${prefix}.${key}` : key;
		if (typeof value === 'string') {
			result[fullKey] = value;
		} else if (typeof value === 'object' && value !== null) {
			Object.assign(result, flattenTranslations(value as Record<string, unknown>, fullKey));
		}
	}
	return result;
}

export async function loadLocale(loc: string): Promise<void> {
	if (loc === 'en') {
		translations.set(flattenTranslations(en));
	} else {
		try {
			const mod = await import(`./locales/${loc}.json`);
			translations.set(flattenTranslations(mod.default));
		} catch {
			console.warn(`Locale "${loc}" not found, falling back to English`);
			translations.set(flattenTranslations(en));
		}
	}
	locale.set(loc);
}

export function t(key: string, params?: Record<string, string | number>): string {
	const dict = get(translations);
	let str = dict[key];
	if (!str) return key;
	if (params) {
		for (const [k, v] of Object.entries(params)) {
			str = str.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
		}
	}
	return str;
}

export const tr = derived(translations, ($translations) => {
	return (key: string, params?: Record<string, string | number>): string => {
		let str = $translations[key];
		if (!str) return key;
		if (params) {
			for (const [k, v] of Object.entries(params)) {
				str = str.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
			}
		}
		return str;
	};
});

export function formatNumber(n: number): string {
	const loc = get(locale);
	return new Intl.NumberFormat(loc).format(n);
}

export function formatCurrency(n: number): string {
	const loc = get(locale);
	if (Math.abs(n) >= 1_000_000_000) {
		return new Intl.NumberFormat(loc, { style: 'currency', currency: 'USD', notation: 'compact', maximumFractionDigits: 1 }).format(n);
	}
	if (Math.abs(n) >= 1_000_000) {
		return new Intl.NumberFormat(loc, { style: 'currency', currency: 'USD', notation: 'compact', maximumFractionDigits: 1 }).format(n);
	}
	return new Intl.NumberFormat(loc, { style: 'currency', currency: 'USD', maximumFractionDigits: 0 }).format(n);
}

export function formatDate(d: Date): string {
	const loc = get(locale);
	return new Intl.DateTimeFormat(loc, { dateStyle: 'medium', timeStyle: 'short' }).format(d);
}

export function formatPercent(n: number): string {
	const loc = get(locale);
	return new Intl.NumberFormat(loc, { style: 'percent', maximumFractionDigits: 0 }).format(n);
}
