<script lang="ts">
	import { requestPasswordReset } from '$lib/multiplayer/accountApi';

	let { onBack }: { onBack: () => void } = $props();

	let username = $state('');
	let submitting = $state(false);
	let submitted = $state(false);
	let error = $state('');

	async function handleSubmit() {
		if (!username.trim()) return;
		submitting = true;
		error = '';
		try {
			await requestPasswordReset(username.trim());
			submitted = true;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Request failed';
		} finally {
			submitting = false;
		}
	}
</script>

<div class="forgot-page">
	<div class="forgot-container">
		<button class="back-btn" onclick={onBack}>Back</button>
		<h1>Reset Password</h1>

		{#if submitted}
			<div class="success-message">
				<p>If the account exists, a password reset has been queued.</p>
				<p class="hint">An admin will generate a temporary password. Check back later or contact an admin.</p>
				<button class="back-btn" onclick={onBack}>Return to Login</button>
			</div>
		{:else}
			<p class="description">Enter your username to request a password reset. An admin will review your request and generate a temporary password.</p>

			<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
				<label class="field-label" for="username">Username</label>
				<input id="username" type="text" class="text-input" bind:value={username} placeholder="Enter your username" autocomplete="username" />

				{#if error}
					<p class="error-text">{error}</p>
				{/if}

				<button type="submit" class="submit-btn" disabled={submitting || !username.trim()}>
					{submitting ? 'Submitting...' : 'Request Reset'}
				</button>
			</form>
		{/if}
	</div>
</div>

<style>
	.forgot-page {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, #0a0e17 0%, #111827 50%, #0a0e17 100%);
		font-family: system-ui, sans-serif;
		color: #f3f4f6;
	}

	.forgot-container {
		width: 400px;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 12px;
		padding: 32px;
	}

	h1 {
		font-size: 24px;
		font-weight: 700;
		margin: 16px 0;
	}

	.description {
		color: #9ca3af;
		font-size: 14px;
		line-height: 1.5;
		margin-bottom: 20px;
	}

	.back-btn {
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		color: #9ca3af;
		padding: 6px 14px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 13px;
	}

	.back-btn:hover {
		color: #d1d5db;
		border-color: rgba(75, 85, 101, 0.5);
	}

	.field-label {
		display: block;
		font-size: 12px;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 6px;
	}

	.text-input {
		width: 100%;
		padding: 10px 14px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #f3f4f6;
		font-size: 14px;
		font-family: system-ui, sans-serif;
		box-sizing: border-box;
		margin-bottom: 12px;
	}

	.text-input:focus {
		outline: none;
		border-color: #10b981;
	}

	.submit-btn {
		width: 100%;
		background: rgba(16, 185, 129, 0.2);
		border: 1px solid rgba(16, 185, 129, 0.3);
		color: #10b981;
		padding: 12px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 14px;
		font-family: system-ui, sans-serif;
		margin-top: 8px;
	}

	.submit-btn:hover:not(:disabled) {
		background: rgba(16, 185, 129, 0.3);
	}

	.submit-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.error-text {
		color: #ef4444;
		font-size: 13px;
		margin-bottom: 8px;
	}

	.success-message {
		text-align: center;
		padding: 20px 0;
	}

	.success-message p {
		color: #10b981;
		font-size: 15px;
		line-height: 1.5;
	}

	.hint {
		color: #6b7280 !important;
		font-size: 13px !important;
		margin: 12px 0 20px;
	}
</style>
