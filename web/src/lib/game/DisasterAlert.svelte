<script lang="ts">
	import type { ActiveDisaster, ForecastDisaster } from './WeatherLayer';
	import { DISASTER_DISPLAY_DURATION } from './WeatherLayer';
	import { worldInfo, playerCorpId, formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { tooltip } from '$lib/ui/tooltip';
	import { openPanelGroup } from '$lib/stores/uiState';

	interface Props {
		disasters: ActiveDisaster[];
		forecasts?: ForecastDisaster[];
	}

	let { disasters, forecasts = [] }: Props = $props();

	// Filter to only show disasters that haven't expired
	let visibleDisasters = $derived(
		disasters.filter(d => {
			const elapsed = $worldInfo.tick - d.startTick;
			return elapsed < DISASTER_DISPLAY_DURATION;
		})
	);

	// Limit forecasts to top 3 highest probability
	let visibleForecasts = $derived(
		forecasts.slice(0, 3)
	);

	/** Map disaster/weather type to a display icon character. */
	function disasterIcon(type: string): string {
		const lower = type.toLowerCase();
		if (lower.includes('earthquake')) return '/\\';
		if (lower.includes('hurricane') || lower.includes('typhoon') || lower.includes('cyclone')) return '()';
		if (lower.includes('flood')) return '~~';
		if (lower.includes('ice') || lower.includes('blizzard')) return '**';
		if (lower.includes('storm') || lower.includes('thunder') || lower.includes('landslide')) return '//';
		if (lower.includes('heat') || lower.includes('heatwave')) return '^^';
		if (lower.includes('cyber')) return '<!>';
		return '!!';
	}

	/** Severity as label. */
	function severityLabel(severity: number): string {
		if (severity >= 0.5) return 'CRITICAL';
		if (severity >= 0.3) return 'SEVERE';
		if (severity >= 0.15) return 'MODERATE';
		return 'MINOR';
	}

	/** Severity CSS class. */
	function severityClass(severity: number): string {
		if (severity >= 0.5) return 'critical';
		if (severity >= 0.3) return 'severe';
		return 'moderate';
	}

	/** Remaining ticks before display expires. */
	function remainingTicks(startTick: number): number {
		return Math.max(0, DISASTER_DISPLAY_DURATION - ($worldInfo.tick - startTick));
	}

	/** Emergency repair all damaged nodes for player. */
	function emergencyRepairAll() {
		if (!bridge.isInitialized()) return;
		const corpId = $playerCorpId;
		const damaged = bridge.getDamagedNodes(corpId);
		for (const node of damaged) {
			bridge.processCommand({
				EmergencyRepair: { entity_id: node.id },
			});
		}
	}

	/** Fly to disaster location. */
	function viewAffected(disaster: ActiveDisaster) {
		window.dispatchEvent(new CustomEvent('map-fly-to', {
			detail: { lon: disaster.lon, lat: disaster.lat, zoom: 5 },
		}));
	}

	/** Fly to forecast region and open infrastructure panel for preparation. */
	function prepareForecast(forecast: ForecastDisaster) {
		window.dispatchEvent(new CustomEvent('map-fly-to', {
			detail: { lon: forecast.lon, lat: forecast.lat, zoom: 5 },
		}));
		openPanelGroup('operations', 'infrastructure');
	}

	/** Format probability as a percentage string. */
	function formatProbability(p: number): string {
		return `${Math.round(p * 100)}%`;
	}

	/** Probability risk level label. */
	function riskLabel(probability: number): string {
		if (probability >= 0.6) return 'HIGH';
		if (probability >= 0.4) return 'ELEVATED';
		return 'WATCH';
	}

	/** Probability risk class for styling. */
	function riskClass(probability: number): string {
		if (probability >= 0.6) return 'risk-high';
		if (probability >= 0.4) return 'risk-elevated';
		return 'risk-watch';
	}
</script>

{#if visibleDisasters.length > 0 || visibleForecasts.length > 0}
	<div class="disaster-alert-container" role="alert" aria-live="assertive">
		{#each visibleDisasters as disaster (disaster.id)}
			{@const remaining = remainingTicks(disaster.startTick)}
			{@const sevClass = severityClass(disaster.severity)}
			<div class="disaster-alert {sevClass}">
				<div class="alert-icon">{disasterIcon(disaster.disasterType)}</div>
				<div class="alert-body">
					<div class="alert-header">
						<span class="alert-type">{disaster.disasterType}</span>
						<span class="alert-sep">|</span>
						<span class="alert-region">{disaster.regionName}</span>
						<span class="alert-sep">|</span>
						<span class="alert-severity {sevClass}">{severityLabel(disaster.severity)}</span>
					</div>
					<div class="alert-details">
						<span class="alert-affected">{disaster.affectedCount} asset{disaster.affectedCount !== 1 ? 's' : ''} affected</span>
						<span class="alert-sep">|</span>
						<span class="alert-timer">{remaining} ticks remaining</span>
					</div>
				</div>
				<div class="alert-actions">
					<button
						class="alert-btn repair"
						onclick={emergencyRepairAll}
						aria-label="Emergency repair all damaged infrastructure"
						use:tooltip={'Emergency repair all damaged infrastructure (costs extra)'}
					>
						Repair All
					</button>
					<button
						class="alert-btn view"
						onclick={() => viewAffected(disaster)}
						aria-label="Zoom to {disaster.regionName} disaster area"
						use:tooltip={'Zoom to the affected region'}
					>
						View
					</button>
				</div>
			</div>
		{/each}

		{#if visibleForecasts.length > 0 && visibleDisasters.length === 0}
			<div class="forecast-header">FORECAST</div>
		{/if}
		{#each visibleForecasts as forecast (forecast.id)}
			{@const rClass = riskClass(forecast.probability)}
			<div class="forecast-alert {rClass}">
				<div class="forecast-icon">{disasterIcon(forecast.disasterType)}</div>
				<div class="alert-body">
					<div class="alert-header">
						<span class="forecast-type">{forecast.disasterType}</span>
						<span class="alert-sep">|</span>
						<span class="alert-region">{forecast.regionName}</span>
						<span class="alert-sep">|</span>
						<span class="forecast-risk {rClass}">{riskLabel(forecast.probability)}</span>
					</div>
					<div class="alert-details">
						<span class="forecast-prob">{formatProbability(forecast.probability)} probability</span>
						<span class="alert-sep">|</span>
						<span class="forecast-eta">~{forecast.estimatedTicks} ticks</span>
					</div>
				</div>
				<div class="alert-actions">
					<button
						class="alert-btn prepare"
						onclick={() => prepareForecast(forecast)}
						aria-label="Prepare for {forecast.disasterType} in {forecast.regionName}"
						use:tooltip={'Zoom to region and open infrastructure panel to prepare'}
					>
						Prepare
					</button>
				</div>
			</div>
		{/each}
	</div>
{/if}

<style>
	.disaster-alert-container {
		position: absolute;
		top: 84px;
		left: 50%;
		transform: translateX(-50%);
		z-index: 25;
		display: flex;
		flex-direction: column;
		gap: 4px;
		pointer-events: auto;
		max-width: 600px;
		width: 90vw;
	}

	.disaster-alert {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 14px;
		background: rgba(17, 24, 39, 0.96);
		border-radius: 6px;
		font-family: var(--font-mono);
		font-size: 11px;
		animation: alert-slide-in 0.25s ease-out;
	}

	.disaster-alert.moderate {
		border: 1px solid rgba(245, 158, 11, 0.5);
		border-left: 3px solid #f59e0b;
	}

	.disaster-alert.severe {
		border: 1px solid rgba(239, 68, 68, 0.5);
		border-left: 3px solid #ef4444;
	}

	.disaster-alert.critical {
		border: 1px solid rgba(239, 68, 68, 0.7);
		border-left: 3px solid #ef4444;
		animation: alert-slide-in 0.25s ease-out, alert-pulse 2s infinite;
	}

	@keyframes alert-slide-in {
		from {
			opacity: 0;
			transform: translateY(-12px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes alert-pulse {
		0%, 100% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0); }
		50% { box-shadow: 0 0 12px 2px rgba(239, 68, 68, 0.2); }
	}

	.alert-icon {
		font-size: 14px;
		font-weight: 900;
		color: #f59e0b;
		min-width: 24px;
		text-align: center;
		flex-shrink: 0;
	}

	.severe .alert-icon,
	.critical .alert-icon {
		color: #ef4444;
	}

	.alert-body {
		flex: 1;
		min-width: 0;
	}

	.alert-header {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: nowrap;
	}

	.alert-type {
		font-weight: 700;
		color: #f3f4f6;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.alert-region {
		color: #9ca3af;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.alert-severity {
		font-weight: 800;
		letter-spacing: 0.06em;
		font-size: 10px;
		padding: 1px 5px;
		border-radius: 3px;
	}

	.alert-severity.moderate {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}

	.alert-severity.severe {
		background: rgba(239, 68, 68, 0.15);
		color: #f87171;
	}

	.alert-severity.critical {
		background: rgba(239, 68, 68, 0.25);
		color: #fca5a5;
	}

	.alert-sep {
		color: rgba(75, 85, 99, 0.6);
		flex-shrink: 0;
	}

	.alert-details {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-top: 2px;
		color: #6b7280;
		font-size: 10px;
	}

	.alert-affected {
		color: #9ca3af;
	}

	.alert-timer {
		color: #6b7280;
		font-variant-numeric: tabular-nums;
	}

	.alert-actions {
		display: flex;
		gap: 4px;
		flex-shrink: 0;
	}

	.alert-btn {
		padding: 4px 10px;
		border-radius: 4px;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.12s;
		border: 1px solid;
		white-space: nowrap;
	}

	.alert-btn.repair {
		background: rgba(239, 68, 68, 0.15);
		border-color: rgba(239, 68, 68, 0.4);
		color: #f87171;
	}

	.alert-btn.repair:hover {
		background: rgba(239, 68, 68, 0.25);
		color: #fca5a5;
	}

	.alert-btn.view {
		background: rgba(59, 130, 246, 0.15);
		border-color: rgba(59, 130, 246, 0.4);
		color: #60a5fa;
	}

	.alert-btn.view:hover {
		background: rgba(59, 130, 246, 0.25);
		color: #93c5fd;
	}

	/* ── Forecast entries ──────────────────────────────────────────────── */

	.forecast-header {
		font-family: var(--font-mono);
		font-size: 9px;
		font-weight: 700;
		letter-spacing: 0.1em;
		color: #6b7280;
		padding: 4px 14px 0;
	}

	.forecast-alert {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 14px;
		background: rgba(17, 24, 39, 0.90);
		border-radius: 6px;
		font-family: var(--font-mono);
		font-size: 11px;
		animation: alert-slide-in 0.25s ease-out;
	}

	.forecast-alert.risk-watch {
		border: 1px solid rgba(245, 158, 11, 0.25);
		border-left: 3px solid rgba(245, 158, 11, 0.5);
	}

	.forecast-alert.risk-elevated {
		border: 1px solid rgba(245, 158, 11, 0.4);
		border-left: 3px solid #f59e0b;
	}

	.forecast-alert.risk-high {
		border: 1px solid rgba(239, 68, 68, 0.35);
		border-left: 3px solid rgba(239, 68, 68, 0.7);
		animation: alert-slide-in 0.25s ease-out, forecast-pulse 3s infinite;
	}

	@keyframes forecast-pulse {
		0%, 100% { box-shadow: 0 0 0 0 rgba(245, 158, 11, 0); }
		50% { box-shadow: 0 0 8px 1px rgba(245, 158, 11, 0.12); }
	}

	.forecast-icon {
		font-size: 12px;
		font-weight: 900;
		color: rgba(245, 158, 11, 0.6);
		min-width: 24px;
		text-align: center;
		flex-shrink: 0;
	}

	.risk-high .forecast-icon {
		color: rgba(239, 68, 68, 0.7);
	}

	.forecast-type {
		font-weight: 600;
		color: #d1d5db;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		font-size: 10px;
	}

	.forecast-risk {
		font-weight: 800;
		letter-spacing: 0.06em;
		font-size: 9px;
		padding: 1px 5px;
		border-radius: 3px;
	}

	.forecast-risk.risk-watch {
		background: rgba(245, 158, 11, 0.1);
		color: rgba(245, 158, 11, 0.7);
	}

	.forecast-risk.risk-elevated {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}

	.forecast-risk.risk-high {
		background: rgba(239, 68, 68, 0.15);
		color: #f87171;
	}

	.forecast-prob {
		color: #9ca3af;
		font-variant-numeric: tabular-nums;
	}

	.forecast-eta {
		color: #6b7280;
		font-variant-numeric: tabular-nums;
	}

	.alert-btn.prepare {
		background: rgba(245, 158, 11, 0.12);
		border-color: rgba(245, 158, 11, 0.35);
		color: #f59e0b;
	}

	.alert-btn.prepare:hover {
		background: rgba(245, 158, 11, 0.22);
		color: #fbbf24;
	}
</style>
