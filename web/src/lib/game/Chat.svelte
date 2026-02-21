<script lang="ts">
	import { chatMessages } from '$lib/stores/multiplayerState';
	import { tr } from '$lib/i18n/index';
	import * as wsClient from '$lib/multiplayer/WebSocketClient';

	let collapsed = $state(false);
	let inputText = $state('');
	let chatContainer: HTMLDivElement | undefined = $state();

	function sendMessage() {
		const text = inputText.trim();
		if (!text) return;
		wsClient.sendChat(text);
		inputText = '';
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			sendMessage();
		}
	}

	$effect(() => {
		// Auto-scroll to bottom when new messages arrive
		if (chatContainer && $chatMessages.length > 0) {
			chatContainer.scrollTop = chatContainer.scrollHeight;
		}
	});
</script>

<div class="chat-overlay" class:collapsed>
	<button class="chat-header" type="button" onclick={() => (collapsed = !collapsed)}>
		<span>{$tr('game.chat')}</span>
		<span class="toggle">{collapsed ? '+' : '-'}</span>
	</button>

	{#if !collapsed}
		<div class="chat-messages" bind:this={chatContainer} role="log" aria-live="polite">
			{#each $chatMessages as msg}
				<div class="chat-msg">
					<span class="sender">{msg.sender}:</span>
					<span class="text">{msg.message}</span>
				</div>
			{/each}
			{#if $chatMessages.length === 0}
				<div class="chat-empty">{$tr('game.no_messages')}</div>
			{/if}
		</div>

		<div class="chat-input">
			<input
				type="text"
				bind:value={inputText}
				placeholder={$tr('game.type_message')}
				onkeydown={handleKeyDown}
			/>
			<button onclick={sendMessage}>{$tr('game.send')}</button>
		</div>
	{/if}
</div>

<style>
	.chat-overlay {
		position: fixed;
		bottom: 16px;
		left: 16px;
		width: 320px;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 8px;
		font-family: system-ui, sans-serif;
		z-index: 100;
		display: flex;
		flex-direction: column;
	}

	.chat-overlay.collapsed {
		width: auto;
	}

	.chat-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		width: 100%;
		padding: 8px 12px;
		background: transparent;
		border: none;
		color: #d1d5db;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
		font-family: inherit;
	}

	.toggle {
		color: #6b7280;
		font-size: 16px;
	}

	.chat-messages {
		height: 200px;
		overflow-y: auto;
		padding: 8px 12px;
	}

	.chat-msg {
		font-size: 12px;
		margin-bottom: 4px;
		color: #d1d5db;
		word-break: break-word;
	}

	.sender {
		color: #60a5fa;
		font-weight: 600;
		margin-right: 4px;
	}

	.chat-empty {
		color: #4b5563;
		font-size: 12px;
		text-align: center;
		padding: 20px;
	}

	.chat-input {
		display: flex;
		gap: 4px;
		padding: 8px;
		border-top: 1px solid rgba(55, 65, 81, 0.3);
	}

	.chat-input input {
		flex: 1;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.4);
		color: #f3f4f6;
		padding: 6px 8px;
		border-radius: 4px;
		font-size: 12px;
		font-family: system-ui, sans-serif;
	}

	.chat-input button {
		background: rgba(59, 130, 246, 0.3);
		border: 1px solid rgba(59, 130, 246, 0.4);
		color: #60a5fa;
		padding: 6px 12px;
		border-radius: 4px;
		cursor: pointer;
		font-size: 12px;
		font-family: system-ui, sans-serif;
	}

	.chat-input button:hover {
		background: rgba(59, 130, 246, 0.4);
	}
</style>
