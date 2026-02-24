<script lang="ts">
	import { onMount } from "svelte";
	import { adminKey, adminAuthed, clearAdmin } from "$lib/admin/store";
	import {
		validateAdminKey,
		fetchHealth,
		fetchPlayers,
		fetchWorlds,
		kickPlayer,
		pauseWorld,
		setWorldSpeed,
		createWorld,
		deleteWorld,
		broadcastMessage,
		fetchAuditLog,
		type ServerHealth,
		type PlayerInfo,
		type WorldInfo,
		type AuditEntry,
	} from "$lib/admin/api";

	// ── Auth state ────────────────────────────────────────────────────────
	let keyInput = $state("");
	let authError = $state("");
	let authLoading = $state(false);

	// ── Dashboard state ───────────────────────────────────────────────────
	let health = $state<ServerHealth | null>(null);
	let healthError = $state("");

	let players = $state<PlayerInfo[]>([]);
	let playersError = $state("");
	let playersLoading = $state(false);

	let worlds = $state<WorldInfo[]>([]);
	let worldsError = $state("");
	let worldsLoading = $state(false);

	let auditLog = $state<AuditEntry[]>([]);
	let auditError = $state("");
	let auditLoading = $state(false);

	// ── Create World form ─────────────────────────────────────────────────
	let showCreateWorld = $state(false);
	let newWorldName = $state("");
	let newWorldMaxPlayers = $state(8);
	let newWorldType = $state<"procgen" | "real_earth">("procgen");
	let newWorldEra = $state("Internet");
	let newWorldDifficulty = $state("Normal");
	let newWorldMapSize = $state("Medium");
	let newWorldAiCorps = $state(4);
	let newWorldSeed = $state(Math.floor(Math.random() * 1_000_000));
	let createWorldLoading = $state(false);

	const ERA_OPTIONS = ["Telegraph", "Telephone", "EarlyDigital", "Internet", "Modern", "NearFuture"];
	const DIFFICULTY_OPTIONS = ["Easy", "Normal", "Hard", "Expert"];
	const MAP_SIZE_OPTIONS = ["Small", "Medium", "Large", "Huge"];

	// ── Broadcast form ────────────────────────────────────────────────────
	let showBroadcast = $state(false);
	let broadcastText = $state("");
	let broadcastWorldId = $state("");
	let broadcastLoading = $state(false);
	let broadcastResult = $state("");

	// ── Auth ──────────────────────────────────────────────────────────────

	async function handleLogin() {
		authError = "";
		authLoading = true;
		try {
			const valid = await validateAdminKey(keyInput);
			if (valid) {
				$adminKey = keyInput;
				keyInput = "";
				await loadAll();
			} else {
				authError = "Invalid admin key";
			}
		} catch {
			authError = "Server unreachable";
		} finally {
			authLoading = false;
		}
	}

	function handleLogout() {
		clearAdmin();
		health = null;
		players = [];
		worlds = [];
		auditLog = [];
	}

	// ── Data loading ──────────────────────────────────────────────────────

	async function loadAll() {
		await Promise.allSettled([
			loadHealth(),
			loadPlayers(),
			loadWorlds(),
			loadAuditLog(),
		]);
	}

	async function loadHealth() {
		healthError = "";
		try {
			health = await fetchHealth($adminKey);
		} catch {
			healthError = "Failed to fetch server health";
			health = null;
		}
	}

	async function loadPlayers() {
		playersError = "";
		playersLoading = true;
		try {
			players = await fetchPlayers($adminKey);
		} catch {
			playersError = "Failed to fetch players";
			players = [];
		} finally {
			playersLoading = false;
		}
	}

	async function loadWorlds() {
		worldsError = "";
		worldsLoading = true;
		try {
			worlds = await fetchWorlds();
		} catch {
			worldsError = "Failed to fetch worlds";
			worlds = [];
		} finally {
			worldsLoading = false;
		}
	}

	async function loadAuditLog() {
		auditError = "";
		auditLoading = true;
		try {
			auditLog = await fetchAuditLog($adminKey);
		} catch {
			auditError = "Failed to fetch audit log";
			auditLog = [];
		} finally {
			auditLoading = false;
		}
	}

	// ── Actions ───────────────────────────────────────────────────────────

	async function handleKick(player: PlayerInfo) {
		if (!confirm(`Kick player "${player.username}"?`)) return;
		try {
			await kickPlayer($adminKey, player.id);
			await Promise.allSettled([loadPlayers(), loadHealth()]);
		} catch {
			playersError = "Failed to kick player";
		}
	}

	async function handleTogglePause(world: WorldInfo) {
		try {
			await pauseWorld($adminKey, world.id);
			await loadWorlds();
		} catch {
			worldsError = "Failed to toggle pause";
		}
	}

	async function handleSetSpeed(world: WorldInfo, speed: string) {
		try {
			await setWorldSpeed($adminKey, world.id, speed);
			await loadWorlds();
		} catch {
			worldsError = "Failed to set speed";
		}
	}

	async function handleCreateWorld() {
		if (!newWorldName.trim()) return;
		createWorldLoading = true;
		try {
			await createWorld($adminKey, newWorldName.trim(), newWorldMaxPlayers, {
				seed: newWorldSeed,
				starting_era: newWorldEra,
				difficulty: newWorldDifficulty,
				map_size: newWorldMapSize,
				ai_corporations: newWorldAiCorps,
				use_real_earth: newWorldType === "real_earth",
			});
			newWorldName = "";
			newWorldMaxPlayers = 8;
			newWorldType = "procgen";
			newWorldSeed = Math.floor(Math.random() * 1_000_000);
			showCreateWorld = false;
			await Promise.allSettled([loadWorlds(), loadHealth()]);
		} catch {
			worldsError = "Failed to create world";
		} finally {
			createWorldLoading = false;
		}
	}

	async function handleDeleteWorld(world: WorldInfo) {
		if (!confirm(`Delete world "${world.name}"? All players will be kicked.`)) return;
		try {
			await deleteWorld($adminKey, world.id);
			await Promise.allSettled([loadWorlds(), loadPlayers(), loadHealth()]);
		} catch {
			worldsError = "Failed to delete world";
		}
	}

	async function handleBroadcast() {
		if (!broadcastText.trim()) return;
		broadcastLoading = true;
		broadcastResult = "";
		try {
			const result = await broadcastMessage(
				$adminKey,
				broadcastText.trim(),
				broadcastWorldId || undefined,
			);
			broadcastResult = `Sent to ${result.scope === "all" ? "all worlds" : "1 world"}`;
			broadcastText = "";
			setTimeout(() => (broadcastResult = ""), 3000);
		} catch {
			broadcastResult = "Failed to send";
		} finally {
			broadcastLoading = false;
		}
	}

	// ── Helpers ───────────────────────────────────────────────────────────

	function timeAgo(unixSeconds: number): string {
		const now = Math.floor(Date.now() / 1000);
		const diff = now - unixSeconds;
		if (diff < 60) return `${diff}s ago`;
		if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
		if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
		return `${Math.floor(diff / 86400)}d ago`;
	}

	function formatUptime(secs: number): string {
		const d = Math.floor(secs / 86400);
		const h = Math.floor((secs % 86400) / 3600);
		const m = Math.floor((secs % 3600) / 60);
		if (d > 0) return `${d}d ${h}h ${m}m`;
		if (h > 0) return `${h}h ${m}m`;
		return `${m}m`;
	}

	const SPEED_OPTIONS = ["Paused", "Normal", "Fast", "VeryFast", "Ultra"];

	// ── Lifecycle ─────────────────────────────────────────────────────────

	onMount(() => {
		if ($adminAuthed) {
			loadAll();
		}
	});
</script>

<svelte:head>
	<title>GlobalTelco Admin</title>
</svelte:head>

<div class="admin-page">
	{#if !$adminAuthed}
		<!-- Login Gate -->
		<div class="login-container">
			<div class="login-card">
				<h1 class="login-title">GlobalTelco Admin</h1>
				<form onsubmit={(e) => { e.preventDefault(); handleLogin(); }}>
					<label class="input-label" for="admin-key">Admin Key</label>
					<input
						id="admin-key"
						type="password"
						class="input-field"
						placeholder="Enter admin key..."
						bind:value={keyInput}
						disabled={authLoading}
					/>
					{#if authError}
						<p class="error-text">{authError}</p>
					{/if}
					<button class="btn btn-primary" type="submit" disabled={authLoading || !keyInput}>
						{authLoading ? "Validating..." : "Login"}
					</button>
				</form>
			</div>
		</div>
	{:else}
		<!-- Dashboard -->
		<div class="dashboard">
			<header class="dashboard-header">
				<h1 class="dashboard-title">GlobalTelco Admin</h1>
				<div class="header-actions">
					<button class="btn btn-outline" onclick={loadAll}>Refresh All</button>
					<button class="btn btn-outline" onclick={handleLogout}>Logout</button>
				</div>
			</header>

			<!-- Server Health -->
			<section class="section">
				<div class="section-header">
					<h2 class="section-title">SERVER HEALTH</h2>
					<button class="btn btn-small" onclick={loadHealth}>Refresh</button>
				</div>
				{#if healthError}
					<p class="error-text">{healthError}</p>
				{:else if health}
					<div class="stat-row">
						<div class="stat-card">
							<span class="stat-label">Version</span>
							<span class="stat-value mono">{health.version}</span>
						</div>
						<div class="stat-card">
							<span class="stat-label">Uptime</span>
							<span class="stat-value mono">{formatUptime(health.uptime_secs)}</span>
						</div>
						<div class="stat-card">
							<span class="stat-label">Worlds</span>
							<span class="stat-value mono">{health.active_worlds}</span>
						</div>
						<div class="stat-card">
							<span class="stat-label">Players</span>
							<span class="stat-value mono">{health.connected_players}</span>
						</div>
						<div class="stat-card">
							<span class="stat-label">Accounts</span>
							<span class="stat-value mono">{health.registered_accounts}</span>
						</div>
						<div class="stat-card">
							<span class="stat-label">Database</span>
							<span class="stat-value" class:text-green={health.has_database} class:text-red={!health.has_database}>
								{health.has_database ? "Connected" : "None"}
							</span>
						</div>
					</div>
				{:else}
					<p class="muted-text">Loading...</p>
				{/if}
			</section>

			<!-- Worlds Management -->
			<section class="section">
				<div class="section-header">
					<h2 class="section-title">WORLDS</h2>
					<div class="header-actions">
						<button class="btn btn-small btn-success" onclick={() => (showCreateWorld = !showCreateWorld)}>
							{showCreateWorld ? "Cancel" : "+ New World"}
						</button>
						<button class="btn btn-small" onclick={loadWorlds} disabled={worldsLoading}>
							{worldsLoading ? "..." : "Refresh"}
						</button>
					</div>
				</div>

				{#if showCreateWorld}
					<div class="create-world-form">
						<div class="form-row">
							<label class="form-label">World Name
								<input type="text" class="input-field" placeholder="My World..." bind:value={newWorldName} />
							</label>
						</div>
						<div class="form-row">
							<span class="form-label">World Type</span>
							<div class="type-toggle">
								<button class="type-btn" class:type-btn-active={newWorldType === "procgen"} onclick={() => (newWorldType = "procgen")}>Procedural</button>
								<button class="type-btn" class:type-btn-active={newWorldType === "real_earth"} onclick={() => (newWorldType = "real_earth")}>Real Earth</button>
							</div>
						</div>
						<div class="form-grid">
							<label class="form-label">Era
								<select class="input-field" bind:value={newWorldEra}>
									{#each ERA_OPTIONS as era}<option value={era}>{era}</option>{/each}
								</select>
							</label>
							<label class="form-label">Difficulty
								<select class="input-field" bind:value={newWorldDifficulty}>
									{#each DIFFICULTY_OPTIONS as diff}<option value={diff}>{diff}</option>{/each}
								</select>
							</label>
							<label class="form-label">Map Size
								<select class="input-field" bind:value={newWorldMapSize}>
									{#each MAP_SIZE_OPTIONS as size}<option value={size}>{size}</option>{/each}
								</select>
							</label>
							<label class="form-label">Max Players
								<input type="number" class="input-field" min="1" max="64" bind:value={newWorldMaxPlayers} />
							</label>
							<label class="form-label">AI Corps
								<input type="number" class="input-field" min="0" max="20" bind:value={newWorldAiCorps} />
							</label>
							<label class="form-label">Seed
								<div class="seed-row">
									<input type="number" class="input-field seed-input" bind:value={newWorldSeed} />
									<button class="btn btn-small" onclick={() => (newWorldSeed = Math.floor(Math.random() * 1_000_000))}>Rand</button>
								</div>
							</label>
						</div>
						<div class="form-actions">
							<button class="btn btn-small btn-success" onclick={handleCreateWorld} disabled={createWorldLoading || !newWorldName.trim()}>
								{createWorldLoading ? "Creating..." : "Create World"}
							</button>
							<button class="btn btn-small" onclick={() => (showCreateWorld = false)}>Cancel</button>
						</div>
					</div>
				{/if}

				{#if worldsError}
					<p class="error-text">{worldsError}</p>
				{:else if worlds.length === 0}
					<p class="muted-text">No worlds active</p>
				{:else}
					<div class="table-container">
						{#each worlds as world}
							<div class="table-row">
								<div class="row-info">
									<span class="row-name">{world.name}</span>
									<span class="row-detail">
										{world.player_count}/{world.max_players} players
										<span class="separator">|</span>
										Tick <span class="mono">{world.tick}</span>
										<span class="separator">|</span>
										{world.era}
										<span class="separator">|</span>
										{world.map_size}
									</span>
								</div>
								<div class="row-actions">
									<select
										class="speed-select"
										value={world.speed}
										onchange={(e) => handleSetSpeed(world, (e.target as HTMLSelectElement).value)}
									>
										{#each SPEED_OPTIONS as opt}
											<option value={opt}>{opt}</option>
										{/each}
									</select>
									<button
										class="btn btn-small"
										class:btn-warning={world.speed !== "Paused"}
										class:btn-success={world.speed === "Paused"}
										onclick={() => handleTogglePause(world)}
									>
										{world.speed === "Paused" ? "Resume" : "Pause"}
									</button>
									<button
										class="btn btn-small btn-danger"
										onclick={() => handleDeleteWorld(world)}
									>
										Delete
									</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>

			<!-- Players -->
			<section class="section">
				<div class="section-header">
					<h2 class="section-title">PLAYERS ({players.length})</h2>
					<button class="btn btn-small" onclick={loadPlayers} disabled={playersLoading}>
						{playersLoading ? "..." : "Refresh"}
					</button>
				</div>
				{#if playersError}
					<p class="error-text">{playersError}</p>
				{:else if players.length === 0}
					<p class="muted-text">No players connected</p>
				{:else}
					<div class="table-container">
						{#each players as player}
							<div class="table-row">
								<div class="row-info">
									<span class="row-name">
										{player.username}
										{#if player.is_admin}
											<span class="badge badge-blue">admin</span>
										{/if}
										{#if player.is_guest}
											<span class="badge badge-gray">guest</span>
										{:else}
											<span class="badge badge-green">registered</span>
										{/if}
									</span>
									<span class="row-detail">
										ID: <span class="mono">{player.id.slice(0, 8)}...</span>
										<span class="separator">|</span>
										{player.world_id ? `World: ${player.world_id.slice(0, 8)}...` : "No world"}
										{#if player.corp_id}
											<span class="separator">|</span>
											Corp: <span class="mono">{player.corp_id}</span>
										{/if}
									</span>
								</div>
								<div class="row-actions">
									<button class="btn btn-small btn-danger" onclick={() => handleKick(player)}>
										Kick
									</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>

			<!-- Broadcast -->
			<section class="section">
				<div class="section-header">
					<h2 class="section-title">BROADCAST</h2>
					<button class="btn btn-small" onclick={() => (showBroadcast = !showBroadcast)}>
						{showBroadcast ? "Hide" : "Show"}
					</button>
				</div>
				{#if showBroadcast}
					<div class="inline-form">
						<input
							type="text"
							class="input-field input-inline"
							placeholder="Message to all players..."
							bind:value={broadcastText}
						/>
						<select class="speed-select" bind:value={broadcastWorldId}>
							<option value="">All worlds</option>
							{#each worlds as world}
								<option value={world.id}>{world.name}</option>
							{/each}
						</select>
						<button
							class="btn btn-small btn-primary-sm"
							onclick={handleBroadcast}
							disabled={broadcastLoading || !broadcastText.trim()}
						>
							{broadcastLoading ? "..." : "Send"}
						</button>
						{#if broadcastResult}
							<span class="broadcast-result">{broadcastResult}</span>
						{/if}
					</div>
				{/if}
			</section>

			<!-- Audit Log -->
			<section class="section">
				<div class="section-header">
					<h2 class="section-title">AUDIT LOG ({auditLog.length})</h2>
					<button class="btn btn-small" onclick={loadAuditLog} disabled={auditLoading}>
						{auditLoading ? "..." : "Refresh"}
					</button>
				</div>
				{#if auditError}
					<p class="error-text">{auditError}</p>
				{:else if auditLog.length === 0}
					<p class="muted-text">No audit entries</p>
				{:else}
					<div class="table-container audit-table">
						{#each auditLog.toReversed().slice(0, 100) as entry}
							<div class="table-row audit-row">
								<span class="mono audit-tick">Tick {entry.tick}</span>
								<span class="audit-player mono">{entry.player_id.slice(0, 8)}...</span>
								<span class="badge badge-blue">{entry.command_type}</span>
								<span class="muted-text">{timeAgo(entry.timestamp)}</span>
							</div>
						{/each}
					</div>
				{/if}
			</section>
		</div>
	{/if}
</div>

<style>
	/* ── Layout ──────────────────────────────────────────────────────── */

	.admin-page {
		min-height: 100vh;
		background: linear-gradient(135deg, #0a0e17, #111827, #0a0e17);
		color: #f3f4f6;
		font-family: system-ui, -apple-system, sans-serif;
	}

	/* ── Login ───────────────────────────────────────────────────────── */

	.login-container {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
	}

	.login-card {
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 12px;
		padding: 40px;
		width: 360px;
	}

	.login-title {
		font-size: 1.5rem;
		font-weight: 700;
		margin: 0 0 24px 0;
		text-align: center;
		color: #f3f4f6;
	}

	.input-label {
		display: block;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #9ca3af;
		margin-bottom: 6px;
	}

	.input-field {
		width: 100%;
		padding: 10px 12px;
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #f3f4f6;
		font-size: 0.9rem;
		font-family: monospace;
		outline: none;
		box-sizing: border-box;
	}

	.input-field:focus {
		border-color: #60a5fa;
	}

	.input-field::placeholder {
		color: #6b7280;
	}

	.input-inline {
		flex: 1;
		min-width: 0;
	}

	.input-number {
		width: 70px;
		flex: none;
	}

	/* ── Dashboard ───────────────────────────────────────────────────── */

	.dashboard {
		max-width: 960px;
		margin: 0 auto;
		padding: 32px 24px 64px;
	}

	.dashboard-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 32px;
		padding-bottom: 16px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.5);
	}

	.dashboard-title {
		font-size: 1.5rem;
		font-weight: 700;
		margin: 0;
	}

	.header-actions {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	/* ── Sections ────────────────────────────────────────────────────── */

	.section {
		margin-bottom: 32px;
	}

	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 12px;
	}

	.section-title {
		font-size: 0.75rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: #9ca3af;
		margin: 0 0 12px 0;
	}

	.section-header .section-title {
		margin-bottom: 0;
	}

	/* ── Stats ───────────────────────────────────────────────────────── */

	.stat-row {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
		gap: 10px;
	}

	.stat-card {
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 8px;
		padding: 14px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.stat-label {
		font-size: 0.65rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #6b7280;
	}

	.stat-value {
		font-size: 1.1rem;
		font-weight: 700;
		color: #f3f4f6;
	}

	/* ── Table ───────────────────────────────────────────────────────── */

	.table-container {
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 8px;
		overflow: hidden;
	}

	.audit-table {
		max-height: 400px;
		overflow-y: auto;
	}

	.table-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 16px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.table-row:last-child {
		border-bottom: none;
	}

	.audit-row {
		gap: 16px;
		justify-content: flex-start;
	}

	.audit-tick {
		min-width: 80px;
	}

	.audit-player {
		min-width: 90px;
		color: #d1d5db;
	}

	.row-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.row-name {
		font-weight: 600;
		font-size: 0.9rem;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.row-detail {
		font-size: 0.8rem;
		color: #9ca3af;
	}

	.row-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-shrink: 0;
	}

	.separator {
		color: #4b5563;
		margin: 0 2px;
	}

	/* ── Create World Form ───────────────────────────────────────────── */

	.create-world-form {
		margin-bottom: 12px;
		padding: 16px;
		background: rgba(31, 41, 55, 0.4);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 8px;
	}

	.form-row {
		margin-bottom: 12px;
	}

	.form-label {
		display: block;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #9ca3af;
		margin-bottom: 4px;
	}

	.form-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 12px;
		margin-bottom: 14px;
	}

	.type-toggle {
		display: flex;
		gap: 0;
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		overflow: hidden;
		width: fit-content;
		margin-top: 4px;
	}

	.type-btn {
		padding: 7px 18px;
		font-size: 0.8rem;
		font-weight: 600;
		background: rgba(17, 24, 39, 0.6);
		color: #9ca3af;
		border: none;
		cursor: pointer;
		font-family: inherit;
		transition: background 0.15s, color 0.15s;
	}

	.type-btn:hover {
		background: rgba(55, 65, 81, 0.5);
	}

	.type-btn-active {
		background: rgba(16, 185, 129, 0.2);
		color: #10b981;
	}

	.seed-row {
		display: flex;
		gap: 6px;
		align-items: center;
	}

	.seed-input {
		flex: 1;
		min-width: 0;
	}

	.form-actions {
		display: flex;
		gap: 8px;
		padding-top: 4px;
	}

	/* ── Inline Form ─────────────────────────────────────────────────── */

	.inline-form {
		display: flex;
		gap: 10px;
		align-items: center;
		margin-bottom: 12px;
		padding: 12px 16px;
		background: rgba(31, 41, 55, 0.4);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 8px;
		flex-wrap: wrap;
	}

	.inline-label {
		font-size: 0.8rem;
		color: #9ca3af;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.broadcast-result {
		font-size: 0.8rem;
		color: #10b981;
	}

	/* ── Speed Select ────────────────────────────────────────────────── */

	.speed-select {
		padding: 5px 8px;
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 4px;
		color: #d1d5db;
		font-size: 0.75rem;
		font-family: inherit;
		cursor: pointer;
	}

	/* ── Badges ──────────────────────────────────────────────────────── */

	.badge {
		font-size: 0.65rem;
		font-weight: 600;
		padding: 2px 6px;
		border-radius: 3px;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.badge-green {
		background: rgba(16, 185, 129, 0.15);
		color: #10b981;
	}

	.badge-blue {
		background: rgba(96, 165, 250, 0.15);
		color: #60a5fa;
	}

	.badge-gray {
		background: rgba(107, 114, 128, 0.15);
		color: #9ca3af;
	}

	/* ── Buttons ─────────────────────────────────────────────────────── */

	.btn {
		border: none;
		border-radius: 6px;
		font-weight: 600;
		cursor: pointer;
		font-family: inherit;
		transition: background 0.15s;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-primary {
		width: 100%;
		padding: 10px;
		margin-top: 16px;
		font-size: 0.9rem;
		background: #3b82f6;
		color: #fff;
	}

	.btn-primary:hover:not(:disabled) {
		background: #2563eb;
	}

	.btn-primary-sm {
		padding: 5px 14px;
		font-size: 0.75rem;
		background: #3b82f6;
		color: #fff;
	}

	.btn-primary-sm:hover:not(:disabled) {
		background: #2563eb;
	}

	.btn-outline {
		padding: 6px 14px;
		font-size: 0.8rem;
		background: transparent;
		border: 1px solid rgba(55, 65, 81, 0.5);
		color: #9ca3af;
	}

	.btn-outline:hover:not(:disabled) {
		background: rgba(55, 65, 81, 0.3);
		color: #f3f4f6;
	}

	.btn-small {
		padding: 5px 12px;
		font-size: 0.75rem;
		background: rgba(55, 65, 81, 0.4);
		color: #d1d5db;
	}

	.btn-small:hover:not(:disabled) {
		background: rgba(55, 65, 81, 0.7);
	}

	.btn-danger {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}

	.btn-danger:hover:not(:disabled) {
		background: rgba(239, 68, 68, 0.3);
	}

	.btn-warning {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}

	.btn-warning:hover:not(:disabled) {
		background: rgba(245, 158, 11, 0.3);
	}

	.btn-success {
		background: rgba(16, 185, 129, 0.15);
		color: #10b981;
	}

	.btn-success:hover:not(:disabled) {
		background: rgba(16, 185, 129, 0.3);
	}

	/* ── Text ────────────────────────────────────────────────────────── */

	.mono {
		font-family: "SF Mono", "Cascadia Code", "Fira Code", monospace;
	}

	.error-text {
		color: #ef4444;
		font-size: 0.8rem;
		margin: 8px 0 0 0;
	}

	.muted-text {
		color: #6b7280;
		font-size: 0.85rem;
		margin: 0;
	}

	.text-green {
		color: #10b981;
	}

	.text-red {
		color: #ef4444;
	}
</style>
