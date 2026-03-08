import { writable, derived } from 'svelte/store';
import type { WorldInfo, CorporationData, Region, City, Notification, CorpSummary } from '$lib/wasm/types';

export const initialized = writable(false);
export const worldInfo = writable<WorldInfo>({
	tick: 0,
	speed: 'Paused',
	entity_count: 0,
	region_count: 0,
	city_count: 0,
	corporation_count: 0,
	infra_node_count: 0,
	infra_edge_count: 0,
	player_corp_id: 0,
	cell_spacing_km: 100,
	is_real_earth: false,
	sandbox: false
	});

export const playerCorp = writable<CorporationData | null>(null);
export const regions = writable<Region[]>([]);
export const cities = writable<City[]>([]);
export const notifications = writable<Notification[]>([]);
export const allCorporations = writable<CorpSummary[]>([]);

// History tracking for charts
export interface FinanceSnapshot {
	tick: number;
	revenue: number;
	cost: number;
	cash: number;
}
export const financeHistory = writable<FinanceSnapshot[]>([]);

export function recordSnapshot(tick: number, revenue: number, cost: number, cash: number) {
	financeHistory.update((h) => {
		const entry = { tick, revenue, cost, cash };
		const updated = [...h, entry];
		return updated.length > 200 ? updated.slice(-200) : updated;
	});
}

export const tick = derived(worldInfo, ($info) => $info.tick);
export const speed = derived(worldInfo, ($info) => $info.speed);
export const playerCorpId = derived(worldInfo, ($info) => $info.player_corp_id);

export function formatMoney(amount: number): string {
	if (Math.abs(amount) >= 1_000_000_000) return `$${(amount / 1_000_000_000).toFixed(1)}B`;
	if (Math.abs(amount) >= 1_000_000) return `$${(amount / 1_000_000).toFixed(1)}M`;
	if (Math.abs(amount) >= 1_000) return `$${(amount / 1_000).toFixed(1)}K`;
	return `$${amount}`;
}

export function formatPopulation(pop: number): string {
	if (pop >= 1_000_000) return `${(pop / 1_000_000).toFixed(1)}M`;
	if (pop >= 1_000) return `${(pop / 1_000).toFixed(1)}K`;
	return `${pop}`;
}

// Persistent policy/budget state (survives panel close/reopen within session)
export const policyState = writable({
	maintenanceBudget: 500_000,
	expansionPriority: 'balanced',
	pricingStrategy: 'market',
	hiringPolicy: 'normal',
	researchFocus: 'balanced',
	salaryBand: 'market',
	headcountTarget: 50,
});
