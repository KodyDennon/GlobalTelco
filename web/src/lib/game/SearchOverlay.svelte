<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { cities, regions } from '$lib/stores/gameState';
	import { searchOverlayVisible } from './KeyboardManager';

	let inputEl: HTMLInputElement | undefined = $state();
	let query = $state('');
	let selectedIndex = $state(0);

	interface SearchResult {
		type: 'city' | 'region';
		name: string;
		id: number;
		lon: number;
		lat: number;
		population: number;
		detail: string;
	}

	let results = $derived.by((): SearchResult[] => {
		const q = query.trim().toLowerCase();
		if (q.length === 0) return [];

		const matches: SearchResult[] = [];

		// Search cities
		for (const city of $cities) {
			if (city.name.toLowerCase().includes(q)) {
				matches.push({
					type: 'city',
					name: city.name,
					id: city.id,
					lon: city.x,
					lat: city.y,
					population: city.population,
					detail: formatPop(city.population),
				});
			}
			if (matches.length >= 20) break; // Pre-filter cap
		}

		// Search regions
		for (const region of $regions) {
			if (region.name.toLowerCase().includes(q)) {
				matches.push({
					type: 'region',
					name: region.name,
					id: region.id,
					lon: region.center_lon,
					lat: region.center_lat,
					population: region.population,
					detail: formatPop(region.population),
				});
			}
			if (matches.length >= 30) break; // Pre-filter cap
		}

		// Sort by: exact prefix first, then by population descending
		matches.sort((a, b) => {
			const aPrefix = a.name.toLowerCase().startsWith(q) ? 0 : 1;
			const bPrefix = b.name.toLowerCase().startsWith(q) ? 0 : 1;
			if (aPrefix !== bPrefix) return aPrefix - bPrefix;
			return b.population - a.population;
		});

		return matches.slice(0, 8);
	});

	function formatPop(pop: number): string {
		if (pop >= 1_000_000) return `${(pop / 1_000_000).toFixed(1)}M pop`;
		if (pop >= 1_000) return `${(pop / 1_000).toFixed(0)}K pop`;
		return `${pop} pop`;
	}

	function close() {
		searchOverlayVisible.set(false);
		query = '';
		selectedIndex = 0;
	}

	function navigateTo(result: SearchResult) {
		// Dispatch fly-to event — zoom depends on type
		const zoom = result.type === 'city' ? 6 : 4;
		window.dispatchEvent(
			new CustomEvent('map-fly-to', {
				detail: { lon: result.lon, lat: result.lat, zoom }
			})
		);
		close();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			e.stopPropagation();
			close();
			return;
		}

		if (e.key === 'ArrowDown') {
			e.preventDefault();
			if (results.length > 0) {
				selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
			}
			return;
		}

		if (e.key === 'ArrowUp') {
			e.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, 0);
			return;
		}

		if (e.key === 'Enter') {
			e.preventDefault();
			if (results.length > 0 && selectedIndex < results.length) {
				navigateTo(results[selectedIndex]);
			}
			return;
		}
	}

	// Reset selected index when results change
	$effect(() => {
		void results;
		selectedIndex = 0;
	});

	// Focus input when visible
	$effect(() => {
		if ($searchOverlayVisible && inputEl) {
			// Defer focus to next frame so the element is rendered
			requestAnimationFrame(() => inputEl?.focus());
		}
	});

	// Close on click outside
	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			close();
		}
	}
</script>

{#if $searchOverlayVisible}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="search-backdrop" onclick={handleBackdropClick}>
		<div class="search-container" role="search">
			<div class="search-input-row">
				<svg class="search-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
					<circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/>
					<path d="M11 11L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
				</svg>
				<input
					bind:this={inputEl}
					bind:value={query}
					onkeydown={handleKeydown}
					type="text"
					placeholder="Search cities, regions..."
					class="search-input"
					spellcheck="false"
					autocomplete="off"
				/>
				<kbd class="search-hint">ESC</kbd>
			</div>

			{#if results.length > 0}
				<div class="search-results">
					{#each results as result, i}
						<button
							class="search-result"
							class:selected={i === selectedIndex}
							onclick={() => navigateTo(result)}
							onmouseenter={() => selectedIndex = i}
						>
							<span class="result-type-badge" class:city={result.type === 'city'} class:region={result.type === 'region'}>
								{result.type === 'city' ? 'CITY' : 'REGION'}
							</span>
							<span class="result-name">{result.name}</span>
							<span class="result-detail">{result.detail}</span>
						</button>
					{/each}
				</div>
			{:else if query.trim().length > 0}
				<div class="search-empty">No results found</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.search-backdrop {
		position: fixed;
		inset: 0;
		z-index: 95;
		display: flex;
		justify-content: center;
		padding-top: 100px;
		background: rgba(0, 0, 0, 0.3);
		animation: search-fade-in 0.1s ease;
	}

	@keyframes search-fade-in {
		from { opacity: 0; }
		to { opacity: 1; }
	}

	.search-container {
		width: 420px;
		max-width: 90vw;
		background: rgba(13, 18, 30, 0.98);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 10px;
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.03);
		overflow: hidden;
		height: fit-content;
	}

	.search-input-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px 16px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.search-icon {
		color: rgba(156, 163, 175, 0.6);
		flex-shrink: 0;
	}

	.search-input {
		flex: 1;
		background: transparent;
		border: none;
		outline: none;
		color: #f3f4f6;
		font-size: 15px;
		font-family: 'Inter', sans-serif;
		caret-color: #3b82f6;
	}

	.search-input::placeholder {
		color: rgba(107, 114, 128, 0.7);
	}

	.search-hint {
		display: inline-block;
		background: rgba(55, 65, 81, 0.4);
		border: 1px solid rgba(75, 85, 99, 0.4);
		border-radius: 4px;
		padding: 1px 6px;
		font-family: monospace;
		font-size: 10px;
		color: rgba(156, 163, 175, 0.5);
		flex-shrink: 0;
	}

	.search-results {
		padding: 4px 0;
		max-height: 320px;
		overflow-y: auto;
	}

	.search-result {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 8px 16px;
		background: transparent;
		border: none;
		color: #d1d5db;
		font-size: 13px;
		font-family: 'Inter', sans-serif;
		cursor: pointer;
		text-align: left;
		transition: background 0.08s;
	}

	.search-result:hover,
	.search-result.selected {
		background: rgba(59, 130, 246, 0.12);
	}

	.search-result.selected {
		background: rgba(59, 130, 246, 0.18);
	}

	.result-type-badge {
		font-size: 8px;
		font-weight: 800;
		letter-spacing: 0.08em;
		padding: 2px 5px;
		border-radius: 3px;
		flex-shrink: 0;
		min-width: 44px;
		text-align: center;
	}

	.result-type-badge.city {
		background: rgba(245, 158, 11, 0.2);
		color: #fbbf24;
	}

	.result-type-badge.region {
		background: rgba(59, 130, 246, 0.2);
		color: #60a5fa;
	}

	.result-name {
		flex: 1;
		color: #f3f4f6;
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.result-detail {
		font-size: 11px;
		color: rgba(156, 163, 175, 0.6);
		font-family: var(--font-mono, monospace);
		flex-shrink: 0;
	}

	.search-empty {
		padding: 16px;
		text-align: center;
		color: rgba(107, 114, 128, 0.6);
		font-size: 13px;
	}
</style>
