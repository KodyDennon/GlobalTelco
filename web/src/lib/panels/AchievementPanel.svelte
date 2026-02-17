<script lang="ts">
	import { getAchievements, getVictoryState, getPlayerCorpId } from '$lib/wasm/bridge';
	import type { AchievementsInfo, VictoryInfo } from '$lib/wasm/types';
	import { tr } from '$lib/i18n/index';

	let achievements: AchievementsInfo = $state({ unlocked: [], progress: {} });
	let victory: VictoryInfo = $state({ domination_score: 0, tech_score: 0, wealth_score: 0, infrastructure_score: 0, total_score: 0, victory_type: null });

	const achievementKeys = [
		'first_node',
		'first_profit',
		'growing_network',
		'network_empire',
		'millionaire',
		'billionaire',
		'global_reach',
		'corporate_raider',
		'regional_monopoly',
		'aaa_rating',
		'phoenix',
		'debt_free',
		'tech_pioneer',
		'backbone_builder',
		'ocean_cable',
		'deal_maker',
		'spy_master',
		'political_player',
		'partnership',
		'storm_weatherer'
	];

	const achievementIdMap: Record<string, string> = {
		'first_node': 'first_node',
		'first_profit': 'first_profit',
		'ten_nodes': 'growing_network',
		'hundred_nodes': 'network_empire',
		'million_revenue': 'millionaire',
		'billion_revenue': 'billionaire',
		'all_regions': 'global_reach',
		'first_merger': 'corporate_raider',
		'monopoly_region': 'regional_monopoly',
		'aaa_rating': 'aaa_rating',
		'survive_bankruptcy': 'phoenix',
		'debt_free': 'debt_free',
		'research_complete': 'tech_pioneer',
		'global_backbone': 'backbone_builder',
		'ocean_cable': 'ocean_cable',
		'first_contract': 'deal_maker',
		'espionage_success': 'spy_master',
		'lobbyist': 'political_player',
		'co_owner': 'partnership',
		'disaster_survivor': 'storm_weatherer'
	};

	const allAchievements = [
		'first_node',
		'first_profit',
		'ten_nodes',
		'hundred_nodes',
		'million_revenue',
		'billion_revenue',
		'all_regions',
		'first_merger',
		'monopoly_region',
		'aaa_rating',
		'survive_bankruptcy',
		'debt_free',
		'research_complete',
		'global_backbone',
		'ocean_cable',
		'first_contract',
		'espionage_success',
		'lobbyist',
		'co_owner',
		'disaster_survivor'
	];

	function refresh() {
		const corpId = getPlayerCorpId();
		achievements = getAchievements(corpId);
		victory = getVictoryState();
	}

	$effect(() => {
		refresh();
		const interval = setInterval(refresh, 3000);
		return () => clearInterval(interval);
	});

	const unlockedCount = $derived(achievements.unlocked.length);
	const totalCount = $derived(allAchievements.length);

	function scoreValue(score: number): number {
		return Math.min(score * 100, 100);
	}

	function scoreBar(score: number): string {
		return `${scoreValue(score).toFixed(0)}%`;
	}
</script>

<div class="panel" role="region" aria-label={$tr('panels.achievements')}>
	<h2>{$tr('panels.achievements')}</h2>

	<section class="victory-section">
		<h3>{$tr('panels.victory_progress')}</h3>
		{#if victory.victory_type}
			<div class="victory-banner">{victory.victory_type} Victory!</div>
		{/if}
		<div class="score-row">
			<span class="score-label">{$tr('panels.domination')}</span>
			<div class="score-bar" role="progressbar" aria-valuenow={scoreValue(victory.domination_score)} aria-valuemin={0} aria-valuemax={100}><div class="score-fill domination" style:width={scoreBar(victory.domination_score)}></div></div>
			<span class="score-pct">{scoreBar(victory.domination_score)}</span>
		</div>
		<div class="score-row">
			<span class="score-label">{$tr('panels.technology')}</span>
			<div class="score-bar" role="progressbar" aria-valuenow={scoreValue(victory.tech_score)} aria-valuemin={0} aria-valuemax={100}><div class="score-fill tech" style:width={scoreBar(victory.tech_score)}></div></div>
			<span class="score-pct">{scoreBar(victory.tech_score)}</span>
		</div>
		<div class="score-row">
			<span class="score-label">{$tr('panels.wealth')}</span>
			<div class="score-bar" role="progressbar" aria-valuenow={scoreValue(victory.wealth_score)} aria-valuemin={0} aria-valuemax={100}><div class="score-fill wealth" style:width={scoreBar(victory.wealth_score)}></div></div>
			<span class="score-pct">{scoreBar(victory.wealth_score)}</span>
		</div>
		<div class="score-row">
			<span class="score-label">{$tr('panels.infrastructure')}</span>
			<div class="score-bar" role="progressbar" aria-valuenow={scoreValue(victory.infrastructure_score)} aria-valuemin={0} aria-valuemax={100}><div class="score-fill infra" style:width={scoreBar(victory.infrastructure_score)}></div></div>
			<span class="score-pct">{scoreBar(victory.infrastructure_score)}</span>
		</div>
		<div class="total-score">{$tr('panels.total_score')}: <strong>{(victory.total_score * 100).toFixed(0)}%</strong></div>
	</section>

	<section>
		<h3>{$tr('panels.achievements_section')} ({unlockedCount}/{totalCount})</h3>
		<div class="achievement-grid">
			{#each allAchievements as achId}
				{@const i18nKey = achievementIdMap[achId]}
				<div class="achievement" class:unlocked={achievements.unlocked.includes(achId)}>
					<div class="ach-icon">{achievements.unlocked.includes(achId) ? '\u2605' : '\u2606'}</div>
					<div class="ach-info">
						<div class="ach-name">{$tr(`achievements.${i18nKey}`)}</div>
						<div class="ach-desc">{$tr(`achievements.${i18nKey}_desc`)}</div>
					</div>
				</div>
			{/each}
		</div>
	</section>
</div>

<style>
	.panel { padding: 16px; color: #e5e7eb; }
	h2 { font-size: 16px; margin: 0 0 12px; color: #60a5fa; }
	h3 { font-size: 13px; color: #9ca3af; margin: 12px 0 8px; text-transform: uppercase; letter-spacing: 0.05em; }
	section { margin-bottom: 16px; }
	.victory-banner { background: linear-gradient(135deg, #f59e0b, #d97706); color: #000; font-weight: 700; text-align: center; padding: 12px; border-radius: 6px; font-size: 18px; margin-bottom: 12px; }
	.score-row { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
	.score-label { width: 90px; font-size: 12px; color: #9ca3af; }
	.score-bar { flex: 1; height: 8px; background: #374151; border-radius: 4px; overflow: hidden; }
	.score-fill { height: 100%; border-radius: 4px; transition: width 0.5s; }
	.domination { background: #ef4444; }
	.tech { background: #8b5cf6; }
	.wealth { background: #f59e0b; }
	.infra { background: #10b981; }
	.score-pct { width: 40px; font-size: 12px; font-family: monospace; text-align: right; }
	.total-score { text-align: center; font-size: 14px; margin-top: 8px; padding: 8px; background: rgba(31, 41, 55, 0.8); border-radius: 4px; }
	.achievement-grid { display: flex; flex-direction: column; gap: 4px; }
	.achievement { display: flex; gap: 10px; padding: 8px; background: rgba(31, 41, 55, 0.5); border: 1px solid #374151; border-radius: 4px; opacity: 0.5; }
	.achievement.unlocked { opacity: 1; border-color: #f59e0b; background: rgba(245, 158, 11, 0.1); }
	.ach-icon { font-size: 18px; color: #6b7280; }
	.achievement.unlocked .ach-icon { color: #f59e0b; }
	.ach-name { font-size: 13px; font-weight: 600; }
	.ach-desc { font-size: 11px; color: #9ca3af; }
</style>
