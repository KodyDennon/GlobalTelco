<script lang="ts">
	import * as bridge from '$lib/wasm/bridge';
	import type { CovertOpsInfo, LobbyingInfo } from '$lib/wasm/types';
	import { worldInfo, playerCorp, allCorporations, regions as regionStore, formatMoney } from '$lib/stores/gameState';
	import { tr } from '$lib/i18n/index';
	import { tooltip } from '$lib/ui/tooltip';

	let ops: CovertOpsInfo = $state({ security_level: 0, active_missions: 0, detection_count: 0 });
	let campaigns: LobbyingInfo[] = $state([]);

	let espionageTarget = $state(0);
	let sabotageTarget = $state(0);
	let sabotageNode = $state(0);
	let lobbyRegion = $state(0);
	let lobbyPolicy = $state('');
	let lobbyBudget = $state(500000);

	// Derived from stores — no polling needed
	let playerId = $derived($playerCorp?.id ?? 0);
	let playerCash = $derived($playerCorp?.cash ?? 0);
	let corporations = $derived($allCorporations);
	let regions = $derived($regionStore);

	// Reactive: refresh covert ops data when tick changes
	$effect(() => {
		const _tick = $worldInfo.tick; // subscribe to tick changes
		if (playerId > 0) {
			ops = bridge.getCovertOps(playerId);
			campaigns = bridge.getLobbyingCampaigns(playerId);
		}
	});

	function refresh() {
		if (playerId > 0) {
			ops = bridge.getCovertOps(playerId);
			campaigns = bridge.getLobbyingCampaigns(playerId);
		}
	}

	function launchEspionage() {
		if (!espionageTarget) return;
		bridge.processCommand({ LaunchEspionage: { target: espionageTarget, region: regions[0]?.id || 0 } });
		espionageTarget = 0;
		refresh();
	}

	function launchSabotage() {
		if (!sabotageTarget) return;
		bridge.processCommand({ LaunchSabotage: { target: sabotageTarget, node: sabotageNode || 0 } });
		sabotageTarget = 0;
		refresh();
	}

	function upgradeSecurity() {
		const newLevel = ops.security_level + 1;
		bridge.processCommand({ UpgradeSecurity: { level: newLevel } });
		refresh();
	}

	function startLobbying() {
		if (!lobbyRegion || !lobbyPolicy || lobbyBudget <= 0) return;
		bridge.processCommand({ StartLobbying: { region: lobbyRegion, policy: lobbyPolicy, budget: lobbyBudget } });
		lobbyRegion = 0;
		lobbyPolicy = '';
		refresh();
	}

	function cancelLobbying(id: number) {
		bridge.processCommand({ CancelLobbying: { lobby_id: id } });
		refresh();
	}



	const aiCorps = $derived(corporations.filter((c) => !c.is_player));
	const securityCost = $derived((ops.security_level + 1) * 500000);

	const lobbyPolicyKeys = [
		{ value: 'ReduceTax', key: 'panels.policy_reduce_tax' },
		{ value: 'RelaxZoning', key: 'panels.policy_relax_zoning' },
		{ value: 'FastTrackPermits', key: 'panels.policy_fast_track' },
		{ value: 'IncreasedCompetitorBurden', key: 'panels.policy_burden_competitors' },
		{ value: 'SubsidyRequest', key: 'panels.policy_subsidy' }
	];
</script>

<div class="panel" role="region" aria-label={$tr('panels.intel')}>
	<section>
		<h3>{$tr('panels.security')}</h3>
		<div class="security-row">
			<span>{$tr('panels.security_level')}: <strong>{ops.security_level}</strong></span>
			<span>{$tr('panels.active_missions')}: <strong>{ops.active_missions}</strong></span>
			<button onclick={upgradeSecurity} disabled={playerCash < securityCost} aria-label={$tr('panels.upgrade')} use:tooltip={() => `Upgrade security to level ${ops.security_level + 1}\nCost: ${formatMoney(securityCost)}\nReduces enemy espionage and sabotage success rate`}>
				{$tr('panels.upgrade')} ({formatMoney(securityCost)})
			</button>
		</div>
	</section>

	<section>
		<h3>{$tr('panels.espionage')}</h3>
		<div class="form-row">
			<select bind:value={espionageTarget} aria-label={$tr('panels.select_target')}>
				<option value={0}>{$tr('panels.select_target')}</option>
				{#each aiCorps as corp}
					<option value={corp.id}>{corp.name}</option>
				{/each}
			</select>
			<button onclick={launchEspionage} disabled={!espionageTarget} aria-label={$tr('panels.launch')} use:tooltip={'Launch espionage mission\nReveals target\'s infrastructure, finances, and strategies\nRisk of detection depends on their security level'}>{$tr('panels.launch')}</button>
		</div>
	</section>

	<section>
		<h3>{$tr('panels.sabotage')}</h3>
		<div class="form-row">
			<select bind:value={sabotageTarget} aria-label={$tr('panels.select_target')}>
				<option value={0}>{$tr('panels.select_target')}</option>
				{#each aiCorps as corp}
					<option value={corp.id}>{corp.name}</option>
				{/each}
			</select>
			<button onclick={launchSabotage} disabled={!sabotageTarget} aria-label={$tr('panels.launch')} use:tooltip={'Launch sabotage mission\nDamages a random node of the target corporation\nHigh risk of detection — may trigger retaliation'}>{$tr('panels.launch')}</button>
		</div>
	</section>

	<section>
		<h3>{$tr('panels.lobbying')}</h3>
		<div class="lobby-form">
			<select bind:value={lobbyRegion} aria-label={$tr('panels.select_region')}>
				<option value={0}>{$tr('panels.select_region')}</option>
				{#each regions as region}
					<option value={region.id}>{region.name}</option>
				{/each}
			</select>
			<select bind:value={lobbyPolicy} aria-label={$tr('panels.select_policy')}>
				<option value="">{$tr('panels.select_policy')}</option>
				{#each lobbyPolicyKeys as policy}
					<option value={policy.value}>{$tr(policy.key)}</option>
				{/each}
			</select>
			<input type="number" bind:value={lobbyBudget} placeholder={$tr('panels.budget')} aria-label={$tr('panels.budget')} min="100000" step="100000" />
			<button onclick={startLobbying} disabled={!lobbyRegion || !lobbyPolicy || lobbyBudget > playerCash} aria-label={$tr('panels.start_campaign')} use:tooltip={() => `Start lobbying campaign\nBudget: ${formatMoney(lobbyBudget)}\nInfluence regional policy to benefit your operations`}>
				{$tr('panels.start_campaign')}
			</button>
		</div>
	</section>

	{#if campaigns.length > 0}
		<section>
			<h3>{$tr('panels.active_campaigns')}</h3>
			{#each campaigns as campaign}
				<div class="campaign-card" class:inactive={!campaign.active}>
					<div class="campaign-info">
						<span class="region">{campaign.region_name}</span>
						<span class="policy">{campaign.policy}</span>
					</div>
					<div class="progress-bar" role="progressbar" aria-valuenow={Math.min(campaign.influence / campaign.threshold * 100, 100)} aria-valuemin={0} aria-valuemax={100}>
						<div class="fill" style:width="{Math.min(campaign.influence / campaign.threshold * 100, 100)}%"></div>
					</div>
					<div class="campaign-stats">
						<span>{$tr('panels.spent')}: {formatMoney(campaign.budget_spent)} / {formatMoney(campaign.budget_total)}</span>
						{#if campaign.active}
							<button class="cancel" onclick={() => cancelLobbying(campaign.id)} aria-label={$tr('panels.cancel')}>{$tr('panels.cancel')}</button>
						{/if}
					</div>
				</div>
			{/each}
		</section>
	{/if}
</div>

<style>
	.panel { padding: 16px; color: var(--text-secondary); }
	h3 { font-size: 13px; color: var(--text-muted); margin: 12px 0 8px; text-transform: uppercase; letter-spacing: 0.05em; }
	section { margin-bottom: 16px; }
	.security-row { display: flex; align-items: center; gap: 12px; font-size: 13px; flex-wrap: wrap; }
	.security-row button { background: var(--blue); color: white; border: none; padding: 4px 10px; border-radius: var(--radius-sm); cursor: pointer; font-size: 12px; }
	.form-row { display: flex; gap: 8px; }
	.form-row select { flex: 1; background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-secondary); padding: 6px; border-radius: var(--radius-sm); font-size: 13px; }
	.form-row button, .lobby-form button { background: var(--red); color: white; border: none; padding: 6px 12px; border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; }
	.form-row button:disabled, .lobby-form button:disabled, .security-row button:disabled { opacity: 0.5; cursor: not-allowed; }
	.lobby-form { display: flex; flex-direction: column; gap: 8px; }
	.lobby-form select, .lobby-form input { background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-secondary); padding: 6px; border-radius: var(--radius-sm); font-size: 13px; }
	.lobby-form button { background: var(--purple); }
	.campaign-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 10px; margin-bottom: 6px; }
	.campaign-card.inactive { opacity: 0.5; }
	.campaign-info { display: flex; justify-content: space-between; margin-bottom: 6px; font-size: 13px; }
	.region { font-weight: 600; }
	.policy { color: var(--purple-light); font-size: 12px; }
	.progress-bar { height: 4px; background: var(--bg-hover); border-radius: 2px; margin-bottom: 6px; }
	.fill { height: 100%; background: var(--purple); border-radius: 2px; transition: width 0.3s; }
	.campaign-stats { display: flex; justify-content: space-between; align-items: center; font-size: 12px; color: var(--text-muted); }
	.cancel { background: var(--red-bg); color: var(--red-light); border: none; padding: 2px 8px; border-radius: var(--radius-sm); cursor: pointer; font-size: 11px; }
</style>
