<script lang="ts">
	import { userProfile, avatarList } from '$lib/stores/accountState';
	import { isAuthenticated, playerUsername } from '$lib/stores/multiplayerState';
	import { fetchProfile, updateProfile, fetchAvatars, deleteAccount } from '$lib/multiplayer/accountApi';
	import { resetMultiplayerState } from '$lib/stores/multiplayerState';
	import { resetAccountState } from '$lib/stores/accountState';
	import { onMount } from 'svelte';

	let { onBack }: { onBack: () => void } = $props();

	let displayName = $state('');
	let selectedAvatar = $state('tower_01');
	let avatars: string[] = $state([]);
	let saving = $state(false);
	let saveMessage = $state('');
	let deleteConfirm = $state(false);
	let deleting = $state(false);
	let loading = $state(true);

	onMount(async () => {
		try {
			const [profile] = await Promise.all([fetchProfile(), fetchAvatars()]);
			displayName = profile.display_name || '';
			selectedAvatar = profile.avatar_id || 'tower_01';
			avatars = $avatarList;
		} catch (e) {
			console.error('Failed to load profile:', e);
		} finally {
			loading = false;
		}
	});

	async function handleSave() {
		saving = true;
		saveMessage = '';
		try {
			await updateProfile(displayName, selectedAvatar);
			saveMessage = 'Profile updated';
			setTimeout(() => (saveMessage = ''), 3000);
		} catch (e) {
			saveMessage = 'Failed to save';
		} finally {
			saving = false;
		}
	}

	async function handleDelete() {
		deleting = true;
		try {
			await deleteAccount();
			resetMultiplayerState();
			resetAccountState();
			onBack();
		} catch (e) {
			console.error('Failed to delete account:', e);
			deleting = false;
			deleteConfirm = false;
		}
	}
</script>

<div class="profile-page">
	<div class="profile-container">
		<div class="header">
			<button class="back-btn" onclick={onBack}>Back</button>
			<h1>Profile</h1>
		</div>

		{#if loading}
			<div class="loading">Loading profile...</div>
		{:else}
			<div class="section">
				<span class="field-label">Username</span>
				<div class="readonly-field">{$userProfile?.username || $playerUsername || 'Unknown'}</div>
			</div>

			<div class="section">
				<label class="field-label" for="display-name">Display Name</label>
				<input id="display-name" type="text" class="text-input" bind:value={displayName} maxlength={64} placeholder="Set a display name..." />
			</div>

			<div class="section">
				<span class="field-label">Avatar</span>
				<div class="avatar-grid">
					{#each avatars as avatar}
						<button class="avatar-btn" class:selected={selectedAvatar === avatar} onclick={() => (selectedAvatar = avatar)} title={avatar}>
							<span class="avatar-icon">{avatar.replace(/_/g, ' ')}</span>
						</button>
					{/each}
				</div>
			</div>

			<div class="section">
				<button class="save-btn" onclick={handleSave} disabled={saving}>
					{saving ? 'Saving...' : 'Save Changes'}
				</button>
				{#if saveMessage}
					<span class="save-message" class:error={saveMessage.includes('Failed')}>{saveMessage}</span>
				{/if}
			</div>

			<div class="section info">
				<span class="field-label">Account Info</span>
				<div class="info-row">
					<span class="info-label">Provider:</span>
					<span class="info-value">{$userProfile?.auth_provider || 'local'}</span>
				</div>
				<div class="info-row">
					<span class="info-label">Created:</span>
					<span class="info-value">{$userProfile?.created_at ? new Date($userProfile.created_at).toLocaleDateString() : '-'}</span>
				</div>
			</div>

			<div class="section danger">
				{#if deleteConfirm}
					<p class="danger-text">Are you sure? This cannot be undone.</p>
					<div class="danger-actions">
						<button class="delete-btn confirm" onclick={handleDelete} disabled={deleting}>
							{deleting ? 'Deleting...' : 'Yes, Delete My Account'}
						</button>
						<button class="cancel-btn" onclick={() => (deleteConfirm = false)}>Cancel</button>
					</div>
				{:else}
					<button class="delete-btn" onclick={() => (deleteConfirm = true)}>Delete Account</button>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.profile-page {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, #0a0e17 0%, #111827 50%, #0a0e17 100%);
		font-family: system-ui, sans-serif;
		color: #f3f4f6;
	}

	.profile-container {
		width: 480px;
		max-height: 90vh;
		overflow-y: auto;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 12px;
		padding: 32px;
	}

	.header {
		display: flex;
		align-items: center;
		gap: 16px;
		margin-bottom: 24px;
	}

	.header h1 {
		font-size: 24px;
		font-weight: 700;
		margin: 0;
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

	.section {
		margin-bottom: 20px;
	}

	.field-label {
		display: block;
		font-size: 12px;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 6px;
	}

	.readonly-field {
		padding: 10px 14px;
		background: rgba(31, 41, 55, 0.5);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 6px;
		color: #9ca3af;
		font-size: 14px;
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
	}

	.text-input:focus {
		outline: none;
		border-color: #10b981;
	}

	.avatar-grid {
		display: grid;
		grid-template-columns: repeat(6, 1fr);
		gap: 6px;
	}

	.avatar-btn {
		padding: 8px 4px;
		background: rgba(31, 41, 55, 0.6);
		border: 2px solid rgba(55, 65, 81, 0.3);
		border-radius: 6px;
		cursor: pointer;
		color: #9ca3af;
		font-size: 9px;
		text-align: center;
		transition: all 0.15s;
	}

	.avatar-btn:hover {
		border-color: rgba(55, 65, 81, 0.8);
		color: #d1d5db;
	}

	.avatar-btn.selected {
		border-color: #10b981;
		background: rgba(16, 185, 129, 0.1);
		color: #10b981;
	}

	.avatar-icon {
		display: block;
		line-height: 1.3;
		word-break: break-all;
	}

	.save-btn {
		background: rgba(16, 185, 129, 0.2);
		border: 1px solid rgba(16, 185, 129, 0.3);
		color: #10b981;
		padding: 10px 24px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 14px;
		font-family: system-ui, sans-serif;
	}

	.save-btn:hover:not(:disabled) {
		background: rgba(16, 185, 129, 0.3);
	}

	.save-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.save-message {
		margin-left: 12px;
		font-size: 13px;
		color: #10b981;
	}

	.save-message.error {
		color: #ef4444;
	}

	.info-row {
		display: flex;
		gap: 8px;
		font-size: 13px;
		padding: 4px 0;
	}

	.info-label {
		color: #6b7280;
	}

	.info-value {
		color: #d1d5db;
	}

	.danger {
		margin-top: 32px;
		padding-top: 20px;
		border-top: 1px solid rgba(55, 65, 81, 0.3);
	}

	.danger-text {
		color: #ef4444;
		font-size: 13px;
		margin-bottom: 12px;
	}

	.danger-actions {
		display: flex;
		gap: 8px;
	}

	.delete-btn {
		background: rgba(127, 29, 29, 0.3);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #ef4444;
		padding: 8px 16px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 13px;
		font-family: system-ui, sans-serif;
	}

	.delete-btn:hover:not(:disabled) {
		background: rgba(127, 29, 29, 0.5);
	}

	.delete-btn.confirm {
		background: rgba(239, 68, 68, 0.2);
	}

	.cancel-btn {
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		color: #9ca3af;
		padding: 8px 16px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 13px;
	}

	.loading {
		text-align: center;
		color: #6b7280;
		padding: 40px;
	}
</style>
