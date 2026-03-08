<script lang="ts">
	import { untrack } from 'svelte';
	import { notifications } from "$lib/stores/gameState";
	import { tr } from "$lib/i18n/index";
	import { autoPauseReason } from "$lib/game/GameLoop";
	import { setSpeed } from "$lib/game/GameLoop";
	import { eventType, eventData } from "$lib/wasm/types";
	import type { GameEvent } from "$lib/wasm/types";
	import { tooltip } from "$lib/ui/tooltip";

	let expanded = $state(false);

	// Event categories mapped from GameEvent variant names
	const CATEGORY_MAP: Record<string, string> = {
		ConstructionStarted: 'infra',
		ConstructionCompleted: 'infra',
		NodeBuilt: 'infra',
		EdgeBuilt: 'infra',
		NodeDestroyed: 'infra',
		RepairStarted: 'infra',
		RepairCompleted: 'infra',
		RevenueEarned: 'finance',
		CostIncurred: 'finance',
		LoanTaken: 'finance',
		Bankruptcy: 'finance',
		InsolvencyWarning: 'finance',
		BailoutTaken: 'finance',
		BankruptcyDeclared: 'finance',
		ContractProposed: 'contract',
		ContractAccepted: 'contract',
		ContractExpired: 'contract',
		ResearchStarted: 'research',
		ResearchCompleted: 'research',
		RegulationChanged: 'market',
		MarketShiftOccurred: 'market',
		MarketUpdate: 'market',
		AuctionStarted: 'market',
		AuctionBidPlaced: 'market',
		AuctionWon: 'market',
		AuctionCancelled: 'market',
		AcquisitionProposed: 'market',
		AcquisitionAccepted: 'market',
		AcquisitionRejected: 'market',
		MergerCompleted: 'market',
		EspionageCompleted: 'covert',
		SabotageCompleted: 'covert',
		EspionageDetected: 'covert',
		SabotageDetected: 'covert',
		SecurityUpgraded: 'covert',
		LobbyingStarted: 'covert',
		LobbyingSucceeded: 'covert',
		LobbyingFailed: 'covert',
		ScandalOccurred: 'covert',
		AchievementUnlocked: 'info',
		VictoryAchieved: 'info',
		InsurancePurchased: 'finance',
		InsurancePayout: 'finance',
		BuyoutCompleted: 'market',
		CoOwnershipEstablished: 'infra',
		SubsidiaryCreated: 'finance',
		UpgradeVotePassed: 'infra',
		UpgradeVoteRejected: 'infra',
		CorporationFounded: 'info',
		CorporationMerged: 'market',
	};

	function getCategory(event: GameEvent): string {
		const type = eventType(event);
		if (type === 'GlobalNotification') {
			const data = eventData(event);
			if (data.level === 'error' || data.level === 'warning') return 'command-error';
			return 'system';
		}
		return CATEGORY_MAP[type] ?? 'info';
	}

	function getCategoryColor(cat: string): string {
		switch (cat) {
			case "command-error":
				return "#f87171";
			case "infra":
				return "var(--blue)";
			case "finance":
				return "var(--green)";
			case "contract":
				return "var(--amber)";
			case "research":
				return "#8b5cf6";
			case "market":
				return "#ec4899";
			case "covert":
				return "#f59e0b";
			case "system":
				return "#6b7280";
			default:
				return "var(--text-dim)";
		}
	}

	// Human-readable labels for event types
	const EVENT_LABELS: Record<string, string> = {
		ConstructionStarted: 'Construction started',
		ConstructionCompleted: 'Construction completed',
		NodeBuilt: 'Node built',
		EdgeBuilt: 'Link built',
		NodeDestroyed: 'Node destroyed',
		RevenueEarned: 'Revenue earned',
		CostIncurred: 'Cost incurred',
		LoanTaken: 'Loan taken',
		Bankruptcy: 'Bankruptcy',
		CorporationFounded: 'Corporation founded',
		CorporationMerged: 'Corporation merged',
		ContractProposed: 'Contract proposed',
		ContractAccepted: 'Contract accepted',
		ContractExpired: 'Contract expired',
		ResearchStarted: 'Research started',
		ResearchCompleted: 'Research completed',
		RepairStarted: 'Repair started',
		RepairCompleted: 'Repair completed',
		InsurancePurchased: 'Insurance purchased',
		InsurancePayout: 'Insurance payout',
		RegulationChanged: 'Regulation changed',
		MarketShiftOccurred: 'Market shift',
		MarketUpdate: 'Market update',
		InsolvencyWarning: 'Insolvency warning',
		BailoutTaken: 'Bailout taken',
		BankruptcyDeclared: 'Bankruptcy declared',
		AuctionStarted: 'Auction started',
		AuctionBidPlaced: 'Bid placed',
		AuctionWon: 'Auction won',
		AuctionCancelled: 'Auction cancelled',
		AcquisitionProposed: 'Acquisition proposed',
		AcquisitionAccepted: 'Acquisition accepted',
		AcquisitionRejected: 'Acquisition rejected',
		MergerCompleted: 'Merger completed',
		EspionageCompleted: 'Espionage completed',
		SabotageCompleted: 'Sabotage completed',
		EspionageDetected: 'Espionage detected',
		SabotageDetected: 'Sabotage detected',
		SecurityUpgraded: 'Security upgraded',
		LobbyingStarted: 'Lobbying started',
		LobbyingSucceeded: 'Lobbying succeeded',
		LobbyingFailed: 'Lobbying failed',
		ScandalOccurred: 'Scandal!',
		AchievementUnlocked: 'Achievement unlocked',
		VictoryAchieved: 'Victory!',
		BuyoutCompleted: 'Buyout completed',
		CoOwnershipEstablished: 'Co-ownership established',
		SubsidiaryCreated: 'Subsidiary created',
		UpgradeVotePassed: 'Upgrade vote passed',
		UpgradeVoteRejected: 'Upgrade vote rejected',
	};

	function formatEvent(event: GameEvent): string {
		const type = eventType(event);
		const data = eventData(event);

		if (type === 'GlobalNotification') {
			return (data.message as string) ?? 'System notification';
		}

		const label = EVENT_LABELS[type] ?? type.replace(/([A-Z])/g, ' $1').trim();

		// Add context from event data
		if (data.tech) return `${label}: ${data.tech}`;
		if (data.disaster_type) return `${label}: ${data.disaster_type}`;
		if (data.description) return `${label}: ${data.description}`;
		if (data.name) return `${label}: ${data.name}`;
		if (data.achievement) return `${label}: ${data.achievement}`;
		if (data.effect) return `${label}: ${data.effect}`;
		if (data.policy) return `${label}: ${data.policy}`;
		if (data.victory_type) return `${label}: ${data.victory_type}`;

		return label;
	}

	let recentNotifs = $derived($notifications.slice(0, expanded ? 20 : 3));
	let hasNotifs = $derived($notifications.length > 0);

	// Command toast: briefly shows error/warning notifications at center-top of screen
	let toasts: Array<{ id: number; text: string; level: string }> = $state([]);
	let toastId = 0;

	$effect(() => {
		// Watch for new notifications that are command errors/warnings
		const latest = $notifications[0];
		if (!latest) return;
		const cat = getCategory(latest.event);
		if (cat === "command-error") {
			const text = formatEvent(latest.event);
			const id = ++toastId;
			const type = eventType(latest.event);
			const data = eventData(latest.event);
			const level = (type === 'GlobalNotification' && data.level === 'error') ? 'error' : 'warning';
			// Use untrack to avoid tracking `toasts` read — prevents infinite reactive loop
			toasts = [...untrack(() => toasts), { id, text, level }];
			setTimeout(() => {
				toasts = toasts.filter((t) => t.id !== id);
			}, 4000);
		}
	});
</script>

<!-- Command error toasts (center top) -->
{#if toasts.length > 0}
	<div class="toast-container" role="alert" aria-live="assertive">
		{#each toasts as toast (toast.id)}
			<div class="toast" class:error={toast.level === "error"} class:warning={toast.level === "warning"}>
				{toast.text}
			</div>
		{/each}
	</div>
{/if}

{#if $autoPauseReason}
	<div class="pause-banner" role="alert">
		<span class="pause-icon">||</span>
		<span class="pause-text">PAUSED: {$autoPauseReason}</span>
		<button class="resume-btn" onclick={() => setSpeed(1)} aria-label="Resume game at normal speed" use:tooltip={'Resume game at normal speed (1x)'}>Resume</button>
	</div>
{/if}

{#if hasNotifs}
	<div class="feed" class:expanded role="log" aria-label="Game event feed" aria-live="polite">
		<button class="feed-header" onclick={() => (expanded = !expanded)} aria-expanded={expanded} aria-label="Toggle event feed, {$notifications.length} events" use:tooltip={expanded ? 'Click to collapse event feed' : 'Click to expand and see more events'}>
			<span class="feed-title">{$tr("game.events")}</span>
			<span class="feed-count" aria-hidden="true">{$notifications.length}</span>
			<span class="toggle" aria-hidden="true">{expanded ? "v" : "^"}</span>
		</button>
		<div class="feed-list">
			{#each recentNotifs as notif}
				{@const cat = getCategory(notif.event)}
				<div class="notif-row">
					<span
						class="dot"
						style="background: {getCategoryColor(cat)}"
					></span>
					<span class="notif-tick">T{notif.tick}</span>
					<span class="notif-text">{formatEvent(notif.event)}</span>
				</div>
			{/each}
		</div>
	</div>
{/if}

<style>
	.feed {
		position: absolute;
		bottom: 8px;
		right: 8px;
		width: 320px;
		max-height: 52px;
		overflow: hidden;
		background: rgba(17, 24, 39, 0.92);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		z-index: 12;
		transition: max-height 0.2s ease;
		font-family: var(--font-sans);
	}

	.feed.expanded {
		max-height: 400px;
		overflow-y: auto;
	}

	.feed-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		cursor: pointer;
		border: none;
		border-bottom: 1px solid var(--border);
		background: transparent;
		width: 100%;
		font-size: 11px;
		color: inherit;
	}

	.feed-title {
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.feed-count {
		background: var(--bg-surface);
		padding: 1px 6px;
		border-radius: 8px;
		font-size: 10px;
		color: var(--text-dim);
		font-family: var(--font-mono);
	}

	.toggle {
		margin-left: auto;
		color: var(--text-dim);
		font-size: 10px;
	}

	.feed-list {
		padding: 2px 0;
	}

	.notif-row {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 3px 10px;
		font-size: 11px;
		color: var(--text-muted);
		border-bottom: 1px solid rgba(55, 65, 81, 0.15);
	}

	.dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.notif-tick {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--text-dim);
		min-width: 40px;
	}

	.notif-text {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* Command error toasts */
	.toast-container {
		position: absolute;
		top: 56px;
		left: 50%;
		transform: translateX(-50%);
		z-index: 30;
		display: flex;
		flex-direction: column;
		gap: 6px;
		pointer-events: none;
	}

	.toast {
		padding: 8px 16px;
		border-radius: var(--radius-md);
		font-family: var(--font-sans);
		font-size: 13px;
		font-weight: 500;
		white-space: nowrap;
		animation: toast-in 0.2s ease-out;
	}

	.toast.error {
		background: rgba(239, 68, 68, 0.9);
		color: #fff;
		border: 1px solid rgba(239, 68, 68, 0.5);
	}

	.toast.warning {
		background: rgba(245, 158, 11, 0.9);
		color: #111827;
		border: 1px solid rgba(245, 158, 11, 0.5);
	}

	@keyframes toast-in {
		from {
			opacity: 0;
			transform: translateY(-8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	/* Auto-pause banner */
	.pause-banner {
		position: absolute;
		bottom: 70px;
		right: 8px;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 14px;
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.4);
		border-radius: var(--radius-md);
		z-index: 13;
		font-family: var(--font-sans);
		font-size: 12px;
		animation: toast-in 0.2s ease-out;
	}

	.pause-icon {
		color: var(--red);
		font-weight: 900;
		font-size: 14px;
	}

	.pause-text {
		color: #fca5a5;
		font-weight: 500;
	}

	.resume-btn {
		padding: 4px 12px;
		background: rgba(239, 68, 68, 0.3);
		border: 1px solid rgba(239, 68, 68, 0.5);
		color: #fca5a5;
		font-size: 11px;
		font-family: var(--font-sans);
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.15s;
	}

	.resume-btn:hover {
		background: rgba(239, 68, 68, 0.4);
		color: #fff;
	}
</style>
