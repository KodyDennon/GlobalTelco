<script lang="ts">
	let { onNewGame, onSettings }: { onNewGame: () => void; onSettings: () => void } = $props();
</script>

<div class="main-menu">
	<div class="menu-content">
		<h1 class="title">GlobalTelco</h1>
		<p class="subtitle">Build your telecom empire</p>

		<div class="menu-buttons">
			<button class="menu-btn primary" onclick={onNewGame}>New Game</button>
			<button class="menu-btn" disabled>Load Game</button>
			<button class="menu-btn" onclick={onSettings}>Settings</button>
		</div>
	</div>

	<div class="menu-footer">
		<span>v0.1.0 - Early Development</span>
	</div>
</div>

<style>
	.main-menu {
		width: 100vw;
		height: 100vh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, #0a0e17 0%, #111827 50%, #0a0e17 100%);
		font-family: system-ui, sans-serif;
		color: #f3f4f6;
	}

	.menu-content {
		text-align: center;
	}

	.title {
		font-size: 48px;
		font-weight: 800;
		letter-spacing: -1px;
		margin: 0;
		background: linear-gradient(90deg, #10b981, #3b82f6);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.subtitle {
		color: #6b7280;
		font-size: 16px;
		margin: 8px 0 48px;
	}

	.menu-buttons {
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: 280px;
		margin: 0 auto;
	}

	.menu-btn {
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		color: #d1d5db;
		padding: 14px 24px;
		font-size: 16px;
		font-family: system-ui, sans-serif;
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.menu-btn:hover:not(:disabled) {
		background: rgba(55, 65, 81, 0.8);
		color: #f3f4f6;
		border-color: rgba(75, 85, 101, 0.5);
	}

	.menu-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.menu-btn.primary {
		background: rgba(16, 185, 129, 0.2);
		border-color: rgba(16, 185, 129, 0.3);
		color: #10b981;
	}

	.menu-btn.primary:hover {
		background: rgba(16, 185, 129, 0.3);
		color: #34d399;
	}

	.menu-footer {
		position: absolute;
		bottom: 24px;
		color: #4b5563;
		font-size: 12px;
	}
</style>
