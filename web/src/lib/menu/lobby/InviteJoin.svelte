<script lang="ts">
	import { lookupInviteCode } from '$lib/multiplayer/lobbyApi';
	import type { WorldListEntry } from '$lib/multiplayer/lobbyApi';

	let { onJoin }: { onJoin: (worldId: string) => void } = $props();

	let code = $state('');
	let loading = $state(false);
	let error = $state('');
	let preview = $state<WorldListEntry | null>(null);

	async function handleLookup() {
		if (!code.trim() || code.trim().length < 4) return;
		loading = true;
		error = '';
		preview = null;
		try {
			const result = await lookupInviteCode(code.trim().toUpperCase());
			if (!result) {
				error = 'No world found with that invite code';
			} else {
				preview = result;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Lookup failed';
		} finally {
			loading = false;
		}
	}
</script>

<div class="invite-container">
	<h3>Join by Invite Code</h3>
	<p class="description">Enter an 8-character invite code to join a world.</p>

	<div class="code-row">
		<input
			type="text"
			class="code-input"
			bind:value={code}
			placeholder="Enter invite code..."
			maxlength={8}
		/>
		<button class="btn-lookup" onclick={handleLookup} disabled={loading || code.trim().length < 4}>
			{loading ? 'Looking up...' : 'Find World'}
		</button>
	</div>

	{#if error}
		<div class="error">{error}</div>
	{/if}

	{#if preview}
		<div class="world-preview">
			<h4>{preview.name}</h4>
			<div class="preview-details">
				<span>{preview.player_count}/{preview.max_players} players</span>
				<span>&middot; {preview.era}</span>
				<span>&middot; {preview.speed}</span>
				<span>&middot; Tick {preview.tick}</span>
			</div>
			<button
				class="btn-join"
				onclick={() => onJoin(preview!.id)}
				disabled={preview.player_count >= preview.max_players}
			>
				{preview.player_count >= preview.max_players ? 'World Full' : 'Join World'}
			</button>
		</div>
	{/if}
</div>

<style>
	.invite-container {
		max-width: 480px;
	}

	h3 {
		font-size: 18px;
		margin: 0 0 4px;
	}

	.description {
		font-size: 13px;
		color: #9ca3af;
		margin-bottom: 16px;
	}

	.code-row {
		display: flex;
		gap: 8px;
	}

	.code-input {
		flex: 1;
		padding: 10px 14px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #f3f4f6;
		font-size: 16px;
		font-family: monospace;
		text-transform: uppercase;
		letter-spacing: 2px;
	}

	.code-input:focus {
		outline: none;
		border-color: #10b981;
	}

	.btn-lookup {
		background: rgba(59, 130, 246, 0.2);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: #60a5fa;
		padding: 10px 20px;
		border-radius: 6px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
		white-space: nowrap;
	}

	.btn-lookup:hover:not(:disabled) {
		background: rgba(59, 130, 246, 0.3);
	}

	.btn-lookup:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.error {
		color: #ef4444;
		font-size: 13px;
		margin-top: 12px;
	}

	.world-preview {
		margin-top: 16px;
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 8px;
		padding: 16px;
	}

	.world-preview h4 {
		margin: 0 0 6px;
		font-size: 16px;
	}

	.preview-details {
		font-size: 13px;
		color: #9ca3af;
		margin-bottom: 12px;
	}

	.btn-join {
		width: 100%;
		background: rgba(16, 185, 129, 0.2);
		border: 1px solid rgba(16, 185, 129, 0.3);
		color: #10b981;
		padding: 10px;
		border-radius: 6px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
		font-size: 14px;
	}

	.btn-join:hover:not(:disabled) {
		background: rgba(16, 185, 129, 0.3);
	}

	.btn-join:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
