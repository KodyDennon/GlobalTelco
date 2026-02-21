<script lang="ts">
	import { notifications } from "$lib/stores/gameState";
	import { tr } from "$lib/i18n/index";
	import { autoPauseReason } from "$lib/game/GameLoop";
	import { setSpeed } from "$lib/game/GameLoop";

	let expanded = $state(false);

	function getCategory(event: string): string {
		if (event.includes("Disaster")) return "disaster";
		// Check error/warning level in GlobalNotification before generic categories
		if (event.includes('level: "error"') || event.includes('level: "warning"')) return "command-error";
		if (
			event.includes("Construction") ||
			event.includes("Node") ||
			event.includes("Edge") ||
			event.includes("Repair")
		)
			return "infra";
		if (
			event.includes("Revenue") ||
			event.includes("Cost") ||
			event.includes("Loan") ||
			event.includes("Bankruptcy")
		)
			return "finance";
		if (event.includes("Contract")) return "contract";
		if (event.includes("Research")) return "research";
		if (event.includes("Regulation") || event.includes("Market"))
			return "market";
		if (event.startsWith("GlobalNotification")) return "system";
		return "info";
	}

	function getCategoryColor(cat: string): string {
		switch (cat) {
			case "disaster":
				return "var(--red)";
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
			case "system":
				return "#6b7280";
			default:
				return "var(--text-dim)";
		}
	}

	function formatEvent(event: string): string {
		// Handle GlobalNotification specially to keep the message
		if (event.startsWith("GlobalNotification")) {
			try {
				const msgMatch = event.match(/message:\s*"([^"]+)"/);
				if (msgMatch) return msgMatch[1];
			} catch (e) {
				return event;
			}
		}

		// Clean up Rust debug format for other events
		return event
			.replace(/\{[^}]*\}/g, "")
			.replace(/([A-Z])/g, " $1")
			.trim();
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
		if (cat === "command-error" || cat === "disaster") {
			const text = formatEvent(latest.event);
			const id = ++toastId;
			const level = latest.event.includes('"error"') ? "error" : "warning";
			toasts = [...toasts, { id, text, level }];
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
		<button class="resume-btn" onclick={() => setSpeed(1)}>Resume</button>
	</div>
{/if}

{#if hasNotifs}
	<div class="feed" class:expanded role="log" aria-live="polite">
		<button class="feed-header" onclick={() => (expanded = !expanded)}>
			<span class="feed-title">{$tr("game.events")}</span>
			<span class="feed-count">{$notifications.length}</span>
			<span class="toggle">{expanded ? "v" : "^"}</span>
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
