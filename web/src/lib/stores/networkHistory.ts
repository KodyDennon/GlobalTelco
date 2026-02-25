import { writable, derived } from 'svelte/store';
import type { TrafficFlows, InfrastructureList } from '$lib/wasm/types';

export interface NetworkSnapshot {
	tick: number;
	served: number;
	demand: number;
	dropped: number;
	avgHealth: number;
	avgUtilization: number;
}

const MAX_HISTORY = 100;

export const networkHistory = writable<NetworkSnapshot[]>([]);

/**
 * Record a network snapshot from the current tick's traffic flow data.
 * Call this from the game loop every N ticks.
 */
export function recordNetworkSnapshot(
	tick: number,
	traffic: TrafficFlows,
	infra: InfrastructureList
): void {
	// Calculate average health and utilization from player edges
	let totalHealth = 0;
	let totalUtil = 0;
	const edgeCount = infra.edges.length;

	for (const edge of infra.edges) {
		totalHealth += edge.health;
		totalUtil += edge.utilization;
	}

	const avgHealth = edgeCount > 0 ? totalHealth / edgeCount : 1;
	const avgUtilization = edgeCount > 0 ? totalUtil / edgeCount : 0;

	const snapshot: NetworkSnapshot = {
		tick,
		served: traffic.total_served,
		demand: traffic.total_demand,
		dropped: traffic.total_dropped,
		avgHealth,
		avgUtilization,
	};

	networkHistory.update((history) => {
		const updated = [...history, snapshot];
		if (updated.length > MAX_HISTORY) {
			return updated.slice(-MAX_HISTORY);
		}
		return updated;
	});
}

/** Last 50 snapshots for the traffic overview chart. */
export const chartHistory = derived(networkHistory, ($history) => {
	return $history.slice(-50);
});
