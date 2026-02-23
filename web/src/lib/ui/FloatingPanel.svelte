<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { tooltip } from '$lib/ui/tooltip';

	interface Props {
		title: string;
		groupId: string;
		tabs: Array<{ key: string; label: string }>;
		activeTab: string;
		onclose: () => void;
		ontabchange: (tab: string) => void;
		children: import('svelte').Snippet;
	}

	let { title, groupId, tabs, activeTab, onclose, ontabchange, children }: Props = $props();

	let panelEl: HTMLDivElement | undefined = $state();
	let dragging = $state(false);
	let dragOffset = { x: 0, y: 0 };
	let posX = $state(0);
	let posY = $state(0);
	let initialized = $state(false);

	let storageKey = $derived(`gt_panel_pos_${groupId}`);

	onMount(() => {
		// Restore saved position or center
		if (browser) {
			const saved = localStorage.getItem(storageKey);
			if (saved) {
				try {
					const { x, y } = JSON.parse(saved);
					posX = x;
					posY = y;
				} catch {
					centerPanel();
				}
			} else {
				centerPanel();
			}
		}
		initialized = true;
	});

	function centerPanel() {
		if (!browser) return;
		posX = Math.max(0, (window.innerWidth - 500) / 2);
		posY = Math.max(88, (window.innerHeight - 450) / 2);
	}

	function onPointerDown(e: PointerEvent) {
		if ((e.target as HTMLElement).closest('.tab-btn') || (e.target as HTMLElement).closest('.close-btn')) return;
		dragging = true;
		dragOffset = { x: e.clientX - posX, y: e.clientY - posY };
		(e.target as HTMLElement).setPointerCapture(e.pointerId);
	}

	function onPointerMove(e: PointerEvent) {
		if (!dragging) return;
		posX = Math.max(0, Math.min(window.innerWidth - 100, e.clientX - dragOffset.x));
		posY = Math.max(0, Math.min(window.innerHeight - 60, e.clientY - dragOffset.y));
	}

	function onPointerUp() {
		if (!dragging) return;
		dragging = false;
		if (browser) {
			localStorage.setItem(storageKey, JSON.stringify({ x: posX, y: posY }));
		}
	}
</script>

{#if initialized}
	<div
		class="floating-panel"
		bind:this={panelEl}
		style="left: {posX}px; top: {posY}px;"
		role="dialog"
		aria-label={title}
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="title-bar"
			onpointerdown={onPointerDown}
			onpointermove={onPointerMove}
			onpointerup={onPointerUp}
		>
			<span class="panel-title">{title}</span>
			<button class="close-btn" onclick={onclose} aria-label="Close panel" use:tooltip={'Close this panel'}>&#x2715;</button>
		</div>

		{#if tabs.length > 1}
			<div class="tab-bar" role="tablist">
				{#each tabs as tab}
					<button
						class="tab-btn"
						class:active={activeTab === tab.key}
						onclick={() => ontabchange(tab.key)}
						role="tab"
						aria-selected={activeTab === tab.key}
					>
						{tab.label}
					</button>
				{/each}
			</div>
		{/if}

		<div class="panel-content">
			{@render children()}
		</div>
	</div>
{/if}

<style>
	.floating-panel {
		position: absolute;
		width: 500px;
		max-height: calc(100vh - 100px);
		background: rgba(17, 24, 39, 0.97);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 8px;
		z-index: 20;
		display: flex;
		flex-direction: column;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
	}

	.title-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 12px;
		background: rgba(31, 41, 55, 0.8);
		border-bottom: 1px solid rgba(55, 65, 81, 0.4);
		border-radius: 8px 8px 0 0;
		cursor: grab;
		user-select: none;
	}

	.title-bar:active {
		cursor: grabbing;
	}

	.panel-title {
		font-family: var(--font-sans, system-ui, sans-serif);
		font-size: 13px;
		font-weight: 600;
		color: #d1d5db;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.close-btn {
		width: 44px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: none;
		color: #6b7280;
		font-size: 16px;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.close-btn:hover {
		background: rgba(239, 68, 68, 0.2);
		color: #ef4444;
	}

	.tab-bar {
		display: flex;
		gap: 0;
		padding: 0 8px;
		background: rgba(17, 24, 39, 0.6);
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
		overflow-x: auto;
	}

	.tab-btn {
		padding: 8px 14px;
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		color: #6b7280;
		font-family: var(--font-sans, system-ui, sans-serif);
		font-size: 12px;
		cursor: pointer;
		white-space: nowrap;
		transition: all 0.15s;
	}

	.tab-btn:hover {
		color: #9ca3af;
		background: rgba(55, 65, 81, 0.2);
	}

	.tab-btn.active {
		color: #10b981;
		border-bottom-color: #10b981;
	}

	.panel-content {
		flex: 1;
		overflow-y: auto;
		max-height: calc(100vh - 200px);
	}
</style>
