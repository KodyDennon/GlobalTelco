<script lang="ts">
	import '../app.css';
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
</style>
