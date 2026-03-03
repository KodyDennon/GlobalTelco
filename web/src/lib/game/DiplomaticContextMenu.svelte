<script lang="ts">
	import { diploMenu } from '$lib/stores/uiState';
	import { openPanelGroup } from '$lib/stores/uiState';
	import { allCorporations, formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import { onMount, onDestroy } from 'svelte';

	let menuEl: HTMLElement | undefined = $state(undefined);
	let contracts: any[] = $state([]);

	// Derive corp info from store
	let corpInfo = $derived.by(() => {
		const menu = $diploMenu;
		if (!menu) return null;
		const corp = $allCorporations.find(c => c.id === menu.corpId);
		return corp ?? null;
	});

	// Load contracts with this corp when menu opens
	$effect(() => {
		const menu = $diploMenu;
		if (menu?.visible && bridge.isInitialized()) {
			const playerId = bridge.getPlayerCorpId();
			const allContracts = bridge.getContracts(playerId);
			contracts = allContracts.filter(
				(c: any) => c.from === menu.corpId || c.to === menu.corpId
			);
		} else {
			contracts = [];
		}
	});

	function proposeTransit() {
		const menu = $diploMenu;
		if (!menu) return;
		const playerId = bridge.getPlayerCorpId();
		gameCommand({
			ProposeContract: {
				from: playerId,
				to: menu.corpId,
				terms: 'type:Transit,bandwidth:1000,price:50000,duration:100',
			},
		});
		diploMenu.set(null);
	}

	function proposePeering() {
		const menu = $diploMenu;
		if (!menu) return;
		const playerId = bridge.getPlayerCorpId();
		gameCommand({
			ProposeContract: {
				from: playerId,
				to: menu.corpId,
				terms: 'type:Peering,bandwidth:5000,price:0,duration:200',
			},
		});
		diploMenu.set(null);
	}

	function proposeAlliance() {
		const menu = $diploMenu;
		if (!menu) return;
		gameCommand({
			ProposeAlliance: {
				target_corp: menu.corpId,
				name: `Alliance-${Date.now()}`,
				revenue_share: 0.1,
			},
		});
		diploMenu.set(null);
	}

	function viewContracts() {
		openPanelGroup('market', 'contracts');
		diploMenu.set(null);
	}

	function dismiss() {
		diploMenu.set(null);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') dismiss();
	}

	function handleClickOutside(e: MouseEvent) {
		if (menuEl && !menuEl.contains(e.target as Node)) {
			dismiss();
		}
	}

	onMount(() => {
		// Delay attaching to avoid catching the click that opened the menu
		const timer = setTimeout(() => {
			window.addEventListener('click', handleClickOutside, true);
		}, 100);
		return () => {
			clearTimeout(timer);
			window.removeEventListener('click', handleClickOutside, true);
		};
	});

	// Clamp to viewport
	let menuStyle = $derived.by(() => {
		const menu = $diploMenu;
		if (!menu) return '';
		const x = Math.min(menu.x, window.innerWidth - 240);
		const y = Math.min(menu.y, window.innerHeight - 300);
		return `left: ${Math.max(0, x)}px; top: ${Math.max(0, y)}px;`;
	});

	let isPeeringNode = $derived(
		$diploMenu?.nodeType === 'ExchangePoint' || $diploMenu?.nodeType === 'InternetExchangePoint'
	);
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $diploMenu?.visible}
	<div class="diplo-menu" style={menuStyle} bind:this={menuEl} role="menu" aria-label="Diplomatic actions">
		<div class="diplo-header">
			{#if corpInfo}
				<span class="corp-dot" style="background: hsl({(corpInfo.id * 67) % 360}, 70%, 55%)"></span>
				<span class="corp-name">{corpInfo.name}</span>
			{:else}
				<span class="corp-name">{$diploMenu.corpName || `Corp #${$diploMenu.corpId}`}</span>
			{/if}
		</div>

		{#if contracts.length > 0}
			<div class="contract-badge">
				{contracts.length} active contract{contracts.length > 1 ? 's' : ''}
			</div>
		{/if}

		<div class="diplo-actions">
			<button class="diplo-btn transit" onclick={proposeTransit}>
				<span class="btn-icon">T</span>
				Propose Transit
			</button>
			{#if isPeeringNode}
				<button class="diplo-btn peering" onclick={proposePeering}>
					<span class="btn-icon">P</span>
					Propose Peering
				</button>
			{/if}
			<button class="diplo-btn alliance" onclick={proposeAlliance}>
				<span class="btn-icon">A</span>
				Propose Alliance
			</button>
			<button class="diplo-btn contracts" onclick={viewContracts}>
				<span class="btn-icon">C</span>
				View Contracts
			</button>
		</div>
	</div>
{/if}

<style>
	.diplo-menu {
		position: fixed;
		z-index: 60;
		min-width: 220px;
		background: rgba(13, 18, 30, 0.96);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 8px;
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.03);
		backdrop-filter: blur(12px);
		animation: diplo-fade-in 0.12s ease;
		font-family: system-ui, sans-serif;
	}

	@keyframes diplo-fade-in {
		from { opacity: 0; transform: translateY(4px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.diplo-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 14px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.5);
	}

	.corp-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.corp-name {
		font-size: 13px;
		font-weight: 600;
		color: #f3f4f6;
		flex: 1;
	}

	.contract-badge {
		padding: 4px 14px;
		font-size: 11px;
		color: #10b981;
		background: rgba(16, 185, 129, 0.08);
		border-bottom: 1px solid rgba(55, 65, 81, 0.4);
	}

	.diplo-actions {
		padding: 6px;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.diplo-btn {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 8px 10px;
		border: none;
		border-radius: 5px;
		background: transparent;
		color: #d1d5db;
		font-size: 12px;
		font-family: system-ui, sans-serif;
		font-weight: 500;
		cursor: pointer;
		transition: background 0.12s;
		text-align: left;
	}

	.diplo-btn:hover {
		background: rgba(55, 65, 81, 0.4);
		color: #f3f4f6;
	}

	.btn-icon {
		width: 22px;
		height: 22px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		font-size: 11px;
		font-weight: 700;
		font-family: 'SF Mono', 'Fira Code', monospace;
		flex-shrink: 0;
	}

	.diplo-btn.transit .btn-icon {
		background: rgba(59, 130, 246, 0.15);
		color: #60a5fa;
	}
	.diplo-btn.transit:hover { background: rgba(59, 130, 246, 0.1); }

	.diplo-btn.peering .btn-icon {
		background: rgba(16, 185, 129, 0.15);
		color: #34d399;
	}
	.diplo-btn.peering:hover { background: rgba(16, 185, 129, 0.1); }

	.diplo-btn.alliance .btn-icon {
		background: rgba(139, 92, 246, 0.15);
		color: #a78bfa;
	}
	.diplo-btn.alliance:hover { background: rgba(139, 92, 246, 0.1); }

	.diplo-btn.contracts .btn-icon {
		background: rgba(156, 163, 175, 0.15);
		color: #9ca3af;
	}
	.diplo-btn.contracts:hover { background: rgba(156, 163, 175, 0.1); }
</style>
