<script lang="ts">
	import { tr, t, locale, loadLocale } from '$lib/i18n/index';
	import { musicVolume, sfxVolume, mapQuality, autoSaveInterval, showNotifications, colorblindMode, uiScale } from '$lib/stores/settings';
	import { resetTutorial } from '$lib/stores/tutorialState';

	let { onBack }: { onBack: () => void } = $props();

	const shortcuts = [
		{ key: 'Space', action: t('settings.shortcut_pause') },
		{ key: 'F5', action: t('settings.shortcut_save') },
		{ key: 'F9', action: t('settings.shortcut_load') },
		{ key: 'B', action: t('settings.shortcut_build_node') },
		{ key: 'E', action: t('settings.shortcut_build_edge') },
		{ key: '1-4', action: t('settings.shortcut_speed') },
		{ key: 'Esc', action: t('settings.shortcut_close') },
		{ key: 'F3', action: t('settings.shortcut_perf') }
	];
</script>

<div class="settings">
	<div class="settings-container">
		<h2>{$tr('settings.title')}</h2>

		<h3>{$tr('settings.audio')}</h3>
		<div class="setting">
			<label for="music">{$tr('settings.music_volume')}</label>
			<input id="music" type="range" min="0" max="1" step="0.1" bind:value={$musicVolume} />
			<span class="val">{Math.round($musicVolume * 100)}%</span>
		</div>

		<div class="setting">
			<label for="sfx">{$tr('settings.sfx_volume')}</label>
			<input id="sfx" type="range" min="0" max="1" step="0.1" bind:value={$sfxVolume} />
			<span class="val">{Math.round($sfxVolume * 100)}%</span>
		</div>

		<h3>{$tr('settings.graphics')}</h3>
		<div class="setting">
			<label for="quality">{$tr('settings.map_quality')}</label>
			<select id="quality" bind:value={$mapQuality}>
				<option value="low">{$tr('settings.quality_low')}</option>
				<option value="medium">{$tr('settings.quality_medium')}</option>
				<option value="high">{$tr('settings.quality_high')}</option>
			</select>
		</div>

		<h3>{$tr('settings.game')}</h3>
		<div class="setting">
			<label for="autosave">{$tr('settings.autosave')}</label>
			<select id="autosave" bind:value={$autoSaveInterval}>
				<option value={25}>{$tr('settings.autosave_every', { n: 25 })}</option>
				<option value={50}>{$tr('settings.autosave_every', { n: 50 })}</option>
				<option value={100}>{$tr('settings.autosave_every', { n: 100 })}</option>
				<option value={200}>{$tr('settings.autosave_every', { n: 200 })}</option>
				<option value={0}>{$tr('settings.autosave_disabled')}</option>
			</select>
		</div>

		<div class="setting">
			<label for="notifs">{$tr('settings.notifications')}</label>
			<label class="toggle">
				<input id="notifs" type="checkbox" bind:checked={$showNotifications} />
				<span class="toggle-label">{$showNotifications ? $tr('settings.on') : $tr('settings.off')}</span>
			</label>
		</div>

		<h3>{$tr('settings.tutorial_section')}</h3>
		<div class="setting">
			<label>{$tr('settings.tutorial')}</label>
			<button class="btn-small" onclick={resetTutorial}>{$tr('settings.reset_tutorial')}</button>
		</div>

		<h3>{$tr('settings.accessibility')}</h3>
		<div class="setting">
			<label for="language">{$tr('settings.language')}</label>
			<select id="language" bind:value={$locale} onchange={(e) => loadLocale(e.currentTarget.value)}>
				<option value="en">{$tr('settings.english')}</option>
			</select>
		</div>

		<div class="setting">
			<label for="colorblind">{$tr('settings.colorblind')}</label>
			<select id="colorblind" bind:value={$colorblindMode}>
				<option value="none">{$tr('settings.colorblind_none')}</option>
				<option value="protanopia">{$tr('settings.colorblind_protanopia')}</option>
				<option value="deuteranopia">{$tr('settings.colorblind_deuteranopia')}</option>
				<option value="tritanopia">{$tr('settings.colorblind_tritanopia')}</option>
			</select>
		</div>

		<div class="setting">
			<label for="uiscale">{$tr('settings.ui_scale')}</label>
			<input id="uiscale" type="range" min="0.8" max="1.5" step="0.1" bind:value={$uiScale} />
			<span class="val">{$uiScale.toFixed(1)}x</span>
		</div>

		<h3>{$tr('settings.keyboard_shortcuts')}</h3>
		<div class="shortcuts">
			{#each shortcuts as s}
				<div class="shortcut-row">
					<kbd>{s.key}</kbd>
					<span>{s.action}</span>
				</div>
			{/each}
		</div>

		<button class="btn" onclick={onBack}>{$tr('menu.back')}</button>
	</div>
</div>

<style>
	.settings {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #0a0e17;
		font-family: system-ui, sans-serif;
		color: #f3f4f6;
	}

	.settings-container {
		width: 440px;
		max-height: 90vh;
		overflow-y: auto;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 12px;
		padding: 32px;
	}

	h2 {
		margin: 0 0 24px;
	}

	h3 {
		font-size: 11px;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin: 20px 0 10px;
		padding-bottom: 6px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	h3:first-of-type {
		margin-top: 0;
	}

	.setting {
		margin-bottom: 14px;
		display: flex;
		align-items: center;
		gap: 12px;
	}

	label {
		width: 120px;
		font-size: 13px;
		color: #9ca3af;
		flex-shrink: 0;
	}

	input[type='range'] {
		flex: 1;
		accent-color: #10b981;
	}

	select {
		flex: 1;
		padding: 8px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #f3f4f6;
		font-size: 14px;
	}

	.val {
		width: 40px;
		text-align: right;
		font-size: 12px;
		color: #6b7280;
		font-family: monospace;
	}

	.toggle {
		display: flex;
		align-items: center;
		gap: 8px;
		width: auto;
		cursor: pointer;
	}

	.toggle input[type='checkbox'] {
		accent-color: #10b981;
		width: 16px;
		height: 16px;
	}

	.toggle-label {
		font-size: 13px;
		color: #d1d5db;
	}

	.shortcuts {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin-bottom: 16px;
	}

	.shortcut-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	kbd {
		display: inline-block;
		min-width: 48px;
		padding: 3px 8px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(75, 85, 99, 0.5);
		border-radius: 4px;
		font-family: monospace;
		font-size: 11px;
		color: #d1d5db;
		text-align: center;
	}

	.shortcut-row span {
		font-size: 12px;
		color: #9ca3af;
	}

	.btn {
		margin-top: 16px;
		width: 100%;
		padding: 12px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #d1d5db;
		font-size: 14px;
		cursor: pointer;
	}

	.btn:hover {
		background: rgba(55, 65, 81, 0.6);
	}

	.btn-small {
		padding: 6px 14px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 4px;
		color: #9ca3af;
		font-size: 12px;
		cursor: pointer;
	}

	.btn-small:hover {
		background: rgba(55, 65, 81, 0.6);
		color: #d1d5db;
	}
</style>
