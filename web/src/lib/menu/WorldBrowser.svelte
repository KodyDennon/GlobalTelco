<script lang="ts">
	import {
		connectionState,
		playerUsername,
		authError,
		isAuthenticated,
		serverInfo,
		type MultiplayerWorldInfo,
	} from "$lib/stores/multiplayerState";
	import * as wsClient from "$lib/multiplayer/WebSocketClient";

	let {
		onBack,
		onJoin,
	}: {
		onBack: () => void;
		onJoin: (worldId: string) => void;
	} = $props();

	let worlds = $state<MultiplayerWorldInfo[]>([]);
	let loadingWorlds = $state(false);
	let loginUsername = $state("");
	let loginPassword = $state("");
	let loginEmail = $state("");
	let authMode = $state<"login" | "register" | "guest">("guest");
	let joiningWorldId = $state<string | null>(null);
	let serverOnline = $state<boolean | null>(null);

	// Check server status on mount
	$effect(() => {
		checkServer();
	});

	// When authenticated, fetch worlds automatically
	$effect(() => {
		if ($isAuthenticated) {
			loadWorlds();
		}
	});

	// Watch for WorldJoined messages to navigate
	let worldIdStore: typeof import("$lib/stores/multiplayerState").worldId;
	import("$lib/stores/multiplayerState").then((mod) => {
		worldIdStore = mod.worldId;
		worldIdStore.subscribe((id) => {
			if (id && joiningWorldId) {
				onJoin(id);
				joiningWorldId = null;
			}
		});
	});

	async function checkServer() {
		const info = await wsClient.fetchServerInfo();
		serverOnline = info !== null;

		if (serverOnline) {
			tryAutoConnect();
		}
	}

	function tryAutoConnect() {
		const storedToken =
			typeof localStorage !== "undefined"
				? localStorage.getItem("gt_access_token")
				: null;

		if (storedToken) {
			wsClient.connect();
			const unsub = connectionState.subscribe((state) => {
				if (state === "connected") {
					wsClient.loginWithToken(storedToken);
					unsub();
				}
			});
		}
	}

	async function loadWorlds() {
		loadingWorlds = true;
		worlds = await wsClient.fetchWorlds();
		loadingWorlds = false;
	}

	function connectAndAuth() {
		authError.set("");
		wsClient.connect();

		const unsub = connectionState.subscribe((state) => {
			if (state === "connected") {
				if (authMode === "guest") {
					wsClient.loginAsGuest();
				} else if (authMode === "login") {
					wsClient.login(loginUsername, loginPassword);
				} else {
					wsClient.register(
						loginUsername,
						loginPassword,
						loginEmail,
					);
				}
				unsub();
			}
		});
	}

	function handleJoin(id: string) {
		joiningWorldId = id;
		wsClient.joinWorld(id);
	}

	function handleDisconnect() {
		wsClient.disconnect();
		isAuthenticated.set(false);
		authError.set("");
		worlds = [];
		joiningWorldId = null;
	}
</script>

<div class="world-browser">
	<div class="header">
		<button class="btn-back" onclick={onBack}>Back</button>
		<h2>Multiplayer</h2>
		<div class="connection-status">
			{#if serverOnline === null}
				<span class="status-dot checking"></span>
				<span class="status-text">Checking...</span>
			{:else if !serverOnline}
				<span class="status-dot offline"></span>
				<span class="status-text">Server Offline</span>
			{:else if $connectionState === "connected" && $isAuthenticated}
				<span class="status-dot online"></span>
				<span class="status-text">Connected</span>
			{:else if $connectionState === "connecting" || $connectionState === "reconnecting"}
				<span class="status-dot checking"></span>
				<span class="status-text">Connecting...</span>
			{:else}
				<span class="status-dot online"></span>
				<span class="status-text">Online</span>
			{/if}
		</div>
	</div>

	{#if $serverInfo}
		<div class="server-bar">
			Server: v{$serverInfo.version}
			&middot; {$serverInfo.active_worlds} world{$serverInfo.active_worlds !== 1 ? "s" : ""}
			&middot; {$serverInfo.connected_players} player{$serverInfo.connected_players !== 1 ? "s" : ""} online
		</div>
	{/if}

	{#if serverOnline === false}
		<div class="offline-notice">
			<p>The multiplayer server is not responding.</p>
			<button class="btn-retry" onclick={checkServer}>Retry</button>
		</div>
	{:else if !$isAuthenticated}
		<div class="auth-section">
			<div class="auth-tabs">
				<button
					class:active={authMode === "guest"}
					onclick={() => (authMode = "guest")}>Guest</button
				>
				<button
					class:active={authMode === "login"}
					onclick={() => (authMode = "login")}>Login</button
				>
				<button
					class:active={authMode === "register"}
					onclick={() => (authMode = "register")}>Register</button
				>
			</div>

			{#if authMode !== "guest"}
				<div class="form-group">
					<label for="login-username">Username</label>
					<input
						id="login-username"
						type="text"
						bind:value={loginUsername}
						placeholder="Username"
					/>
				</div>
				<div class="form-group">
					<label for="login-password">Password</label>
					<input
						id="login-password"
						type="password"
						bind:value={loginPassword}
						placeholder="Password"
					/>
				</div>
			{/if}

			{#if authMode === "register"}
				<div class="form-group">
					<label for="login-email">Email</label>
					<input
						id="login-email"
						type="email"
						bind:value={loginEmail}
						placeholder="you@example.com"
					/>
				</div>
			{/if}

			{#if $authError}
				<div class="error">{$authError}</div>
			{/if}

			<button
				class="btn-connect"
				onclick={connectAndAuth}
				disabled={$connectionState === "connecting"}
			>
				{#if $connectionState === "connecting"}
					Connecting...
				{:else}
					Connect
				{/if}
			</button>
		</div>
	{:else}
		<div class="world-list">
			{#if loadingWorlds}
				<div class="loading">Loading worlds...</div>
			{:else if worlds.length === 0}
				<div class="empty">No active worlds found</div>
			{:else}
				{#each worlds as world}
					<div class="world-card">
						<div class="world-info">
							<h3>{world.name}</h3>
							<div class="world-details">
								<div class="player-bar">
									<div
										class="player-fill"
										style="width: {(world.player_count / world.max_players) * 100}%"
									></div>
								</div>
								<span>{world.player_count}/{world.max_players} players</span>
								<span>&middot; Tick {world.tick}</span>
								<span>&middot; {world.era}</span>
							</div>
							<div class="world-meta">
								<span>Speed: {world.speed}</span>
								<span>&middot; Map: {world.map_size}</span>
							</div>
						</div>
						<button
							class="btn-join"
							onclick={() => handleJoin(world.id)}
							disabled={world.player_count >= world.max_players || joiningWorldId === world.id}
						>
							{#if joiningWorldId === world.id}
								Joining...
							{:else if world.player_count >= world.max_players}
								Full
							{:else}
								Join
							{/if}
						</button>
					</div>
				{/each}
			{/if}
			<button class="btn-refresh" onclick={loadWorlds} disabled={loadingWorlds}>
				{loadingWorlds ? "Refreshing..." : "Refresh"}
			</button>
		</div>

		<div class="footer">
			<span class="logged-in">Logged in as: {$playerUsername}</span>
			<button class="btn-disconnect" onclick={handleDisconnect}>Disconnect</button>
		</div>
	{/if}
</div>

<style>
	.world-browser {
		width: 100vw;
		height: 100vh;
		display: flex;
		flex-direction: column;
		background: linear-gradient(
			135deg,
			#0a0e17 0%,
			#111827 50%,
			#0a0e17 100%
		);
		color: #f3f4f6;
		font-family: system-ui, sans-serif;
		padding: 40px;
	}

	.header {
		display: flex;
		align-items: center;
		gap: 16px;
		margin-bottom: 16px;
	}

	.header h2 {
		font-size: 28px;
		margin: 0;
		flex: 1;
	}

	.connection-status {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
	}

	.status-dot.online {
		background: #10b981;
		box-shadow: 0 0 6px #10b981;
	}

	.status-dot.offline {
		background: #ef4444;
		box-shadow: 0 0 6px #ef4444;
	}

	.status-dot.checking {
		background: #f59e0b;
		box-shadow: 0 0 6px #f59e0b;
		animation: pulse 1.5s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.4; }
	}

	.status-text {
		font-size: 13px;
		color: #9ca3af;
	}

	.server-bar {
		font-size: 13px;
		color: #6b7280;
		padding: 8px 12px;
		background: rgba(31, 41, 55, 0.4);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 6px;
		margin-bottom: 24px;
	}

	.btn-back {
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		color: #d1d5db;
		padding: 8px 16px;
		border-radius: 6px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
	}

	.btn-back:hover {
		background: rgba(55, 65, 81, 0.8);
	}

	.offline-notice {
		max-width: 400px;
		margin: 60px auto;
		text-align: center;
		color: #9ca3af;
	}

	.offline-notice p {
		margin-bottom: 16px;
	}

	.btn-retry {
		background: rgba(59, 130, 246, 0.2);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: #60a5fa;
		padding: 10px 24px;
		border-radius: 6px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
	}

	.btn-retry:hover {
		background: rgba(59, 130, 246, 0.3);
	}

	.auth-section {
		max-width: 400px;
		margin: 0 auto;
	}

	.form-group {
		margin-bottom: 16px;
	}

	.form-group label {
		display: block;
		font-size: 13px;
		color: #9ca3af;
		margin-bottom: 4px;
	}

	.form-group input {
		width: 100%;
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		color: #f3f4f6;
		padding: 10px 12px;
		border-radius: 6px;
		font-size: 14px;
		font-family: system-ui, sans-serif;
		box-sizing: border-box;
	}

	.auth-tabs {
		display: flex;
		gap: 8px;
		margin-bottom: 16px;
	}

	.auth-tabs button {
		flex: 1;
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		color: #9ca3af;
		padding: 8px;
		border-radius: 6px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
	}

	.auth-tabs button.active {
		background: rgba(59, 130, 246, 0.2);
		border-color: rgba(59, 130, 246, 0.4);
		color: #60a5fa;
	}

	.btn-connect {
		width: 100%;
		background: rgba(16, 185, 129, 0.2);
		border: 1px solid rgba(16, 185, 129, 0.3);
		color: #10b981;
		padding: 12px;
		border-radius: 6px;
		font-size: 16px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
	}

	.btn-connect:hover:not(:disabled) {
		background: rgba(16, 185, 129, 0.3);
	}

	.btn-connect:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.error {
		color: #ef4444;
		font-size: 13px;
		margin-bottom: 12px;
		padding: 8px 12px;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.2);
		border-radius: 6px;
	}

	.world-list {
		max-width: 600px;
		margin: 0 auto;
		width: 100%;
		flex: 1;
		overflow-y: auto;
	}

	.world-card {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 8px;
		padding: 16px;
		margin-bottom: 12px;
	}

	.world-info {
		flex: 1;
		min-width: 0;
	}

	.world-info h3 {
		margin: 0 0 6px;
		font-size: 16px;
	}

	.world-details {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		color: #9ca3af;
		margin-bottom: 4px;
	}

	.player-bar {
		width: 40px;
		height: 6px;
		background: rgba(55, 65, 81, 0.6);
		border-radius: 3px;
		overflow: hidden;
	}

	.player-fill {
		height: 100%;
		background: #10b981;
		border-radius: 3px;
		transition: width 0.3s;
	}

	.world-meta {
		font-size: 11px;
		color: #6b7280;
	}

	.btn-join {
		background: rgba(59, 130, 246, 0.2);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: #60a5fa;
		padding: 8px 20px;
		border-radius: 6px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
		white-space: nowrap;
		margin-left: 16px;
	}

	.btn-join:hover:not(:disabled) {
		background: rgba(59, 130, 246, 0.3);
	}

	.btn-join:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.btn-refresh {
		width: 100%;
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		color: #9ca3af;
		padding: 10px;
		border-radius: 6px;
		cursor: pointer;
		margin-top: 8px;
		font-family: system-ui, sans-serif;
	}

	.btn-refresh:hover:not(:disabled) {
		background: rgba(55, 65, 81, 0.6);
	}

	.btn-refresh:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		max-width: 600px;
		margin: 16px auto 0;
		width: 100%;
		padding-top: 16px;
		border-top: 1px solid rgba(55, 65, 81, 0.3);
	}

	.logged-in {
		font-size: 13px;
		color: #9ca3af;
	}

	.btn-disconnect {
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #ef4444;
		padding: 6px 16px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 13px;
		font-family: system-ui, sans-serif;
	}

	.btn-disconnect:hover {
		background: rgba(239, 68, 68, 0.25);
	}

	.loading,
	.empty {
		text-align: center;
		color: #6b7280;
		padding: 40px;
	}
</style>
