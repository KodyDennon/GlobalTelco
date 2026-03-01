<script lang="ts">
	import { pendingInvites, type WorldInvite } from '$lib/stores/socialState';
	import { onMount } from 'svelte';

	let { onJoin }: { onJoin: (worldId: string) => void } = $props();

	let invites = $state<WorldInvite[]>([]);

	// Listen for world invite events
	onMount(() => {
		function handleInvite(e: Event) {
			const detail = (e as CustomEvent).detail;
			const invite: WorldInvite = {
				from_username: detail.from_username,
				world_id: detail.world_id,
				world_name: detail.world_name,
				invite_code: detail.invite_code,
				received_at: Date.now()
			};
			invites = [...invites, invite];
			pendingInvites.update((p) => [...p, invite]);

			// Auto-dismiss after 30 seconds
			setTimeout(() => {
				dismiss(invite);
			}, 30000);
		}

		window.addEventListener('mp-world-invite', handleInvite);
		return () => window.removeEventListener('mp-world-invite', handleInvite);
	});

	function dismiss(invite: WorldInvite) {
		invites = invites.filter((i) => i !== invite);
		pendingInvites.update((p) => p.filter((i) => i.received_at !== invite.received_at));
	}

	function accept(invite: WorldInvite) {
		onJoin(invite.world_id);
		dismiss(invite);
	}
</script>

{#if invites.length > 0}
	<div class="invite-toasts">
		{#each invites as invite}
			<div class="invite-toast">
				<div class="invite-info">
					<span class="invite-from">{invite.from_username}</span>
					<span class="invite-text">invites you to</span>
					<span class="invite-world">{invite.world_name}</span>
				</div>
				<div class="invite-actions">
					<button class="btn-accept" onclick={() => accept(invite)}>Join</button>
					<button class="btn-dismiss" onclick={() => dismiss(invite)}>Dismiss</button>
				</div>
			</div>
		{/each}
	</div>
{/if}

<style>
	.invite-toasts {
		position: fixed;
		top: 20px;
		right: 20px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		z-index: 1000;
	}

	.invite-toast {
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(59, 130, 246, 0.4);
		border-radius: 8px;
		padding: 12px 16px;
		min-width: 280px;
		animation: slideIn 0.3s ease-out;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
	}

	@keyframes slideIn {
		from {
			transform: translateX(100%);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}

	.invite-info {
		font-size: 13px;
		color: #d1d5db;
		margin-bottom: 8px;
	}

	.invite-from {
		color: #60a5fa;
		font-weight: 600;
	}

	.invite-text {
		color: #9ca3af;
	}

	.invite-world {
		color: #10b981;
		font-weight: 600;
	}

	.invite-actions {
		display: flex;
		gap: 8px;
	}

	.btn-accept {
		background: rgba(16, 185, 129, 0.2);
		border: 1px solid rgba(16, 185, 129, 0.3);
		color: #10b981;
		padding: 5px 16px;
		border-radius: 5px;
		cursor: pointer;
		font-size: 12px;
		font-family: system-ui, sans-serif;
	}

	.btn-accept:hover {
		background: rgba(16, 185, 129, 0.3);
	}

	.btn-dismiss {
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		color: #9ca3af;
		padding: 5px 16px;
		border-radius: 5px;
		cursor: pointer;
		font-size: 12px;
		font-family: system-ui, sans-serif;
	}

	.btn-dismiss:hover {
		color: #d1d5db;
	}
</style>
