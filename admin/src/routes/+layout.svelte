<script lang="ts">
	import '../app.css';
	import { goto } from '$app/navigation';
	import { adminAuthed, adminKey } from '$lib/stores/auth.js';
	import { validateAdminKey } from '$lib/api/client.js';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import Toast from '$lib/components/Toast.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

	let { children } = $props();

	let keyInput = $state('');
	let authError = $state('');
	let authLoading = $state(false);
	let loginAttempts = $state(0);
	let backoffUntil = $state(0);

	// Command palette state
	let paletteOpen = $state(false);
	let paletteQuery = $state('');
	let paletteIndex = $state(0);
	let paletteInput: HTMLInputElement | undefined = $state(undefined);

	interface PaletteItem {
		label: string;
		href: string;
	}

	const paletteItems: PaletteItem[] = [
		{ label: 'Overview', href: '/overview' },
		{ label: 'Worlds', href: '/worlds' },
		{ label: 'Players', href: '/players' },
		{ label: 'Multiplayer', href: '/multiplayer' },
		{ label: 'Monitoring', href: '/monitoring' },
		{ label: 'Audit Log', href: '/audit' },
		{ label: 'Settings', href: '/settings' },
	];

	const filteredItems = $derived(
		paletteQuery
			? paletteItems.filter(item =>
				item.label.toLowerCase().includes(paletteQuery.toLowerCase())
			)
			: paletteItems
	);

	function openPalette() {
		paletteOpen = true;
		paletteQuery = '';
		paletteIndex = 0;
		// Focus input after DOM update
		requestAnimationFrame(() => paletteInput?.focus());
	}

	function closePalette() {
		paletteOpen = false;
		paletteQuery = '';
		paletteIndex = 0;
	}

	function navigatePalette(item: PaletteItem) {
		closePalette();
		goto(item.href);
	}

	function handlePaletteKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			paletteIndex = Math.min(paletteIndex + 1, filteredItems.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			paletteIndex = Math.max(paletteIndex - 1, 0);
		} else if (e.key === 'Enter' && filteredItems.length > 0) {
			e.preventDefault();
			navigatePalette(filteredItems[paletteIndex]);
		}
	}

	async function handleLogin() {
		if (!keyInput.trim()) {
			authError = 'Please enter an admin key';
			return;
		}

		const now = Date.now();
		if (now < backoffUntil) {
			const waitSec = Math.ceil((backoffUntil - now) / 1000);
			authError = `Too many attempts. Wait ${waitSec}s`;
			return;
		}

		authLoading = true;
		authError = '';

		try {
			const valid = await validateAdminKey(keyInput);
			if (valid) {
				$adminKey = keyInput;
				loginAttempts = 0;
			} else {
				loginAttempts++;
				const delay = Math.min(30000, 1000 * Math.pow(2, loginAttempts - 1));
				backoffUntil = Date.now() + delay;
				authError = `Invalid admin key (attempt ${loginAttempts})`;
			}
		} catch {
			authError = 'Failed to connect to server';
		} finally {
			authLoading = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !$adminAuthed) {
			handleLogin();
			return;
		}

		// Escape closes the command palette
		if (e.key === 'Escape' && paletteOpen) {
			closePalette();
			return;
		}

		// Ctrl+K / Cmd+K opens command palette
		if (e.key === 'k' && (e.ctrlKey || e.metaKey) && $adminAuthed) {
			e.preventDefault();
			if (paletteOpen) closePalette();
			else openPalette();
			return;
		}

		// R key refreshes (when not in an input/textarea/select and palette is closed)
		if (e.key === 'r' && !e.ctrlKey && !e.metaKey && !e.altKey && $adminAuthed && !paletteOpen) {
			const tag = (e.target as HTMLElement)?.tagName?.toLowerCase();
			if (tag !== 'input' && tag !== 'textarea' && tag !== 'select') {
				e.preventDefault();
				window.dispatchEvent(new CustomEvent('gt-admin-refresh'));
			}
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<Toast />
<ConfirmDialog />

{#if $adminAuthed}
	<div class="app-layout">
		<Sidebar />
		<main class="app-content">
			{@render children()}
		</main>
	</div>

	{#if paletteOpen}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="palette-overlay" onclick={closePalette} onkeydown={handlePaletteKeydown}>
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div class="palette-modal" onclick={(e: MouseEvent) => e.stopPropagation()}>
				<input
					bind:this={paletteInput}
					type="text"
					class="palette-input"
					placeholder="Search pages... (Esc to close)"
					bind:value={paletteQuery}
					onkeydown={handlePaletteKeydown}
				/>
				<div class="palette-results">
					{#each filteredItems as item, i}
						<button
							class="palette-item"
							class:selected={i === paletteIndex}
							onclick={() => navigatePalette(item)}
						>
							{item.label}
						</button>
					{/each}
					{#if filteredItems.length === 0}
						<div class="palette-empty">No matching pages</div>
					{/if}
				</div>
			</div>
		</div>
	{/if}
{:else}
	<div class="login-page">
		<div class="login-card">
			<h1 class="login-title">GlobalTelco Admin</h1>
			<p class="login-subtitle">Server administration dashboard</p>

			<div class="login-form">
				<label class="login-label" for="admin-key">Admin Key</label>
				<input
					id="admin-key"
					type="password"
					class="login-input"
					bind:value={keyInput}
					placeholder="Enter admin key"
					disabled={authLoading}
					autocomplete="off"
				/>

				{#if authError}
					<p class="login-error">{authError}</p>
				{/if}

				<button class="login-btn" onclick={handleLogin} disabled={authLoading}>
					{authLoading ? 'Authenticating...' : 'Login'}
				</button>
			</div>

			{#if loginAttempts > 0}
				<p class="attempt-count">{loginAttempts} failed attempt{loginAttempts > 1 ? 's' : ''}</p>
			{/if}
		</div>
	</div>
{/if}

<style>
	.app-layout {
		display: flex;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
	}
	.app-content {
		flex: 1;
		overflow-y: auto;
		padding: 24px;
	}
	.login-page {
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--bg-primary);
	}
	.login-card {
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-radius: var(--radius-xl);
		padding: 40px;
		width: 380px;
		text-align: center;
	}
	.login-title {
		font-size: 22px;
		font-weight: 700;
		color: var(--blue);
		margin-bottom: 4px;
	}
	.login-subtitle {
		font-size: 13px;
		color: var(--text-dim);
		margin-bottom: 28px;
	}
	.login-form {
		display: flex;
		flex-direction: column;
		gap: 10px;
		text-align: left;
	}
	.login-label {
		font-size: 12px;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.login-input {
		padding: 10px 14px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		color: var(--text-primary);
		font-size: 14px;
		font-family: var(--font-mono);
	}
	.login-input:focus {
		border-color: var(--blue);
	}
	.login-error {
		font-size: 12px;
		color: var(--red);
	}
	.login-btn {
		padding: 10px;
		background: var(--blue);
		color: white;
		border: none;
		border-radius: var(--radius-md);
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		margin-top: 4px;
	}
	.login-btn:hover {
		opacity: 0.9;
	}
	.login-btn:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.attempt-count {
		font-size: 11px;
		color: var(--text-faint);
		margin-top: 16px;
	}

	/* Command Palette */
	.palette-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding-top: 20vh;
		z-index: 1000;
	}
	.palette-modal {
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		width: 420px;
		max-width: 90vw;
		overflow: hidden;
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
	}
	.palette-input {
		width: 100%;
		padding: 14px 16px;
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--border);
		color: var(--text-primary);
		font-size: 15px;
		font-family: inherit;
		outline: none;
	}
	.palette-input::placeholder {
		color: var(--text-dim);
	}
	.palette-results {
		max-height: 300px;
		overflow-y: auto;
		padding: 4px;
	}
	.palette-item {
		display: block;
		width: 100%;
		text-align: left;
		padding: 10px 14px;
		background: none;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--text-secondary);
		font-size: 14px;
		cursor: pointer;
		font-family: inherit;
	}
	.palette-item:hover,
	.palette-item.selected {
		background: var(--bg-surface);
		color: var(--text-primary);
	}
	.palette-empty {
		padding: 16px;
		text-align: center;
		color: var(--text-dim);
		font-size: 13px;
	}
</style>
