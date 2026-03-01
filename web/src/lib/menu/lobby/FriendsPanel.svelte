<script lang="ts">
	import {
		fetchFriends,
		sendFriendRequest,
		fetchFriendRequests,
		acceptFriendRequest,
		rejectFriendRequest,
		removeFriend,
		searchUsers
	} from '$lib/multiplayer/socialApi';
	import { friends, friendRequests } from '$lib/stores/socialState';
	import type { UserProfile } from '$lib/stores/accountState';
	import { onMount } from 'svelte';

	let { visible }: { visible: boolean } = $props();

	let tab = $state<'friends' | 'requests' | 'search'>('friends');
	let searchQuery = $state('');
	let searchResults = $state<UserProfile[]>([]);
	let searching = $state(false);
	let addError = $state('');
	let loading = $state(true);

	onMount(async () => {
		try {
			const [f, r] = await Promise.all([fetchFriends(), fetchFriendRequests()]);
			friends.set(f);
			friendRequests.set(r);
		} catch {
			// Silently fail — friends feature may not be available
		} finally {
			loading = false;
		}
	});

	// Listen for real-time presence updates
	$effect(() => {
		function handlePresence(e: Event) {
			const detail = (e as CustomEvent).detail;
			friends.update((list) =>
				list.map((f) =>
					f.id === detail.friend_id
						? { ...f, online: detail.online, world_id: detail.world_id, world_name: detail.world_name }
						: f
				)
			);
		}

		function handleFriendRequest(e: Event) {
			const detail = (e as CustomEvent).detail;
			friendRequests.update((r) => ({
				...r,
				incoming: [
					...r.incoming,
					{
						id: '', // Will be fetched on refresh
						from_id: detail.from_id,
						from_username: detail.from_username,
						to_id: '',
						to_username: '',
						status: 'pending',
						created_at: new Date().toISOString()
					}
				]
			}));
		}

		window.addEventListener('mp-friend-presence', handlePresence);
		window.addEventListener('mp-friend-request', handleFriendRequest);

		return () => {
			window.removeEventListener('mp-friend-presence', handlePresence);
			window.removeEventListener('mp-friend-request', handleFriendRequest);
		};
	});

	async function handleSearch() {
		if (!searchQuery.trim() || searchQuery.trim().length < 2) return;
		searching = true;
		addError = '';
		try {
			searchResults = await searchUsers(searchQuery.trim());
		} catch {
			addError = 'Search failed';
		} finally {
			searching = false;
		}
	}

	async function handleSendRequest(username: string) {
		addError = '';
		try {
			await sendFriendRequest(username);
			searchResults = searchResults.filter((u) => u.username !== username);
		} catch (e) {
			addError = e instanceof Error ? e.message : 'Failed';
		}
	}

	async function handleAccept(requestId: string) {
		try {
			await acceptFriendRequest(requestId);
			const [f, r] = await Promise.all([fetchFriends(), fetchFriendRequests()]);
			friends.set(f);
			friendRequests.set(r);
		} catch {
			// ignore
		}
	}

	async function handleReject(requestId: string) {
		try {
			await rejectFriendRequest(requestId);
			friendRequests.update((r) => ({
				...r,
				incoming: r.incoming.filter((req) => req.id !== requestId)
			}));
		} catch {
			// ignore
		}
	}

	async function handleRemove(friendId: string) {
		try {
			await removeFriend(friendId);
			friends.update((list) => list.filter((f) => f.id !== friendId));
		} catch {
			// ignore
		}
	}

	let pendingCount = $derived($friendRequests.incoming.length);
</script>

{#if visible}
	<div class="friends-panel">
		<div class="panel-header">
			<h3>Friends</h3>
		</div>

		<div class="panel-tabs">
			<button class:active={tab === 'friends'} onclick={() => (tab = 'friends')}>
				Friends ({$friends.length})
			</button>
			<button class:active={tab === 'requests'} onclick={() => (tab = 'requests')}>
				Requests
				{#if pendingCount > 0}
					<span class="badge">{pendingCount}</span>
				{/if}
			</button>
			<button class:active={tab === 'search'} onclick={() => (tab = 'search')}>Add</button>
		</div>

		<div class="panel-content">
			{#if loading}
				<div class="loading">Loading...</div>
			{:else if tab === 'friends'}
				{#if $friends.length === 0}
					<div class="empty-state">No friends yet. Use the Add tab to find players.</div>
				{:else}
					{#each $friends as friend}
						<div class="friend-row">
							<span class="presence-dot" class:online={friend.online}></span>
							<div class="friend-info">
								<span class="friend-name">{friend.display_name || friend.username}</span>
								{#if friend.online && friend.world_name}
									<span class="friend-world">In: {friend.world_name}</span>
								{:else if !friend.online}
									<span class="friend-offline">Offline</span>
								{/if}
							</div>
							<button class="btn-remove" onclick={() => handleRemove(friend.id)} title="Remove friend">x</button>
						</div>
					{/each}
				{/if}
			{:else if tab === 'requests'}
				{#if $friendRequests.incoming.length === 0 && $friendRequests.outgoing.length === 0}
					<div class="empty-state">No pending requests.</div>
				{/if}
				{#if $friendRequests.incoming.length > 0}
					<div class="request-section">
						<span class="section-label">Incoming</span>
						{#each $friendRequests.incoming as req}
							<div class="request-row">
								<span class="request-name">{req.from_username}</span>
								<div class="request-actions">
									<button class="btn-accept" onclick={() => handleAccept(req.id)}>Accept</button>
									<button class="btn-reject" onclick={() => handleReject(req.id)}>Reject</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
				{#if $friendRequests.outgoing.length > 0}
					<div class="request-section">
						<span class="section-label">Sent</span>
						{#each $friendRequests.outgoing as req}
							<div class="request-row">
								<span class="request-name">{req.to_username || req.to_id}</span>
								<span class="request-pending">Pending</span>
							</div>
						{/each}
					</div>
				{/if}
			{:else if tab === 'search'}
				<div class="search-row">
					<input
						type="text"
						class="search-input"
						bind:value={searchQuery}
						placeholder="Search username..."
						onkeydown={(e) => e.key === 'Enter' && handleSearch()}
					/>
					<button class="btn-search" onclick={handleSearch} disabled={searching}>
						{searching ? '...' : 'Search'}
					</button>
				</div>
				{#if addError}
					<div class="error">{addError}</div>
				{/if}
				{#each searchResults as user}
					<div class="search-result">
						<span>{user.display_name || user.username}</span>
						<button class="btn-add" onclick={() => handleSendRequest(user.username)}>Add</button>
					</div>
				{/each}
			{/if}
		</div>
	</div>
{/if}

<style>
	.friends-panel {
		width: 280px;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 10px;
		display: flex;
		flex-direction: column;
		max-height: 500px;
		overflow: hidden;
	}

	.panel-header {
		padding: 12px 16px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.panel-header h3 {
		margin: 0;
		font-size: 15px;
		font-weight: 600;
	}

	.panel-tabs {
		display: flex;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.panel-tabs button {
		flex: 1;
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		color: #9ca3af;
		padding: 8px 4px;
		font-size: 12px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
		position: relative;
	}

	.panel-tabs button.active {
		color: #60a5fa;
		border-bottom-color: #60a5fa;
	}

	.badge {
		background: #ef4444;
		color: white;
		font-size: 10px;
		padding: 1px 5px;
		border-radius: 8px;
		margin-left: 4px;
	}

	.panel-content {
		flex: 1;
		overflow-y: auto;
		padding: 8px;
	}

	.loading,
	.empty-state {
		text-align: center;
		color: #6b7280;
		font-size: 12px;
		padding: 20px 8px;
	}

	.friend-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-radius: 6px;
	}

	.friend-row:hover {
		background: rgba(31, 41, 55, 0.5);
	}

	.presence-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #4b5563;
		flex-shrink: 0;
	}

	.presence-dot.online {
		background: #10b981;
		box-shadow: 0 0 4px #10b981;
	}

	.friend-info {
		flex: 1;
		min-width: 0;
	}

	.friend-name {
		display: block;
		font-size: 13px;
		color: #d1d5db;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.friend-world {
		display: block;
		font-size: 11px;
		color: #10b981;
	}

	.friend-offline {
		font-size: 11px;
		color: #6b7280;
	}

	.btn-remove {
		background: none;
		border: none;
		color: #6b7280;
		cursor: pointer;
		font-size: 14px;
		padding: 2px 6px;
	}

	.btn-remove:hover {
		color: #ef4444;
	}

	.section-label {
		display: block;
		font-size: 11px;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		padding: 8px 8px 4px;
	}

	.request-section {
		margin-bottom: 8px;
	}

	.request-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 8px;
	}

	.request-name {
		font-size: 13px;
		color: #d1d5db;
	}

	.request-actions {
		display: flex;
		gap: 4px;
	}

	.btn-accept {
		background: rgba(16, 185, 129, 0.2);
		border: 1px solid rgba(16, 185, 129, 0.3);
		color: #10b981;
		padding: 3px 10px;
		border-radius: 4px;
		cursor: pointer;
		font-size: 11px;
		font-family: system-ui, sans-serif;
	}

	.btn-reject {
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #ef4444;
		padding: 3px 10px;
		border-radius: 4px;
		cursor: pointer;
		font-size: 11px;
		font-family: system-ui, sans-serif;
	}

	.request-pending {
		font-size: 11px;
		color: #6b7280;
	}

	.search-row {
		display: flex;
		gap: 6px;
		margin-bottom: 8px;
	}

	.search-input {
		flex: 1;
		padding: 6px 10px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 5px;
		color: #f3f4f6;
		font-size: 13px;
		font-family: system-ui, sans-serif;
	}

	.search-input:focus {
		outline: none;
		border-color: #10b981;
	}

	.btn-search {
		background: rgba(59, 130, 246, 0.2);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: #60a5fa;
		padding: 6px 12px;
		border-radius: 5px;
		cursor: pointer;
		font-size: 12px;
		font-family: system-ui, sans-serif;
	}

	.btn-search:disabled {
		opacity: 0.5;
	}

	.error {
		color: #ef4444;
		font-size: 12px;
		padding: 4px 0;
	}

	.search-result {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 8px;
		font-size: 13px;
		color: #d1d5db;
	}

	.btn-add {
		background: rgba(16, 185, 129, 0.2);
		border: 1px solid rgba(16, 185, 129, 0.3);
		color: #10b981;
		padding: 3px 10px;
		border-radius: 4px;
		cursor: pointer;
		font-size: 11px;
		font-family: system-ui, sans-serif;
	}
</style>
