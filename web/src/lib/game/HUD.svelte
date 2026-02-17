<script lang="ts">
	import { worldInfo, playerCorp, formatMoney } from '$lib/stores/gameState';
	import SpeedControls from './SpeedControls.svelte';
</script>

<div class="hud">
	<div class="hud-left">
		<span class="corp-name">{$playerCorp?.name ?? 'Loading...'}</span>
		<span class="cash" class:negative={($playerCorp?.cash ?? 0) < 0}>
			{formatMoney($playerCorp?.cash ?? 0)}
		</span>
		<span class="profit" class:loss={($playerCorp?.profit_per_tick ?? 0) < 0}>
			{($playerCorp?.profit_per_tick ?? 0) >= 0 ? '+' : ''}{formatMoney($playerCorp?.profit_per_tick ?? 0)}/tick
		</span>
	</div>

	<div class="hud-center">
		<SpeedControls />
	</div>

	<div class="hud-right">
		<span class="tick">Tick {$worldInfo.tick}</span>
		<span class="rating">{$playerCorp?.credit_rating ?? '---'}</span>
		<span class="infra">{$playerCorp?.infrastructure_count ?? 0} nodes</span>
	</div>
</div>

<style>
	.hud {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 48px;
		background: rgba(17, 24, 39, 0.95);
		border-bottom: 1px solid rgba(55, 65, 81, 0.5);
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 16px;
		z-index: 10;
		font-family: 'SF Mono', 'Fira Code', monospace;
		font-size: 13px;
		color: #d1d5db;
	}

	.hud-left, .hud-right {
		display: flex;
		align-items: center;
		gap: 16px;
	}

	.hud-center {
		display: flex;
		align-items: center;
	}

	.corp-name {
		font-weight: 600;
		color: #f3f4f6;
	}

	.cash {
		color: #10b981;
		font-weight: 600;
	}

	.cash.negative {
		color: #ef4444;
	}

	.profit {
		color: #10b981;
		font-size: 11px;
	}

	.profit.loss {
		color: #ef4444;
	}

	.tick {
		color: #9ca3af;
	}

	.rating {
		color: #f59e0b;
		font-weight: 600;
	}

	.infra {
		color: #3b82f6;
	}
</style>
