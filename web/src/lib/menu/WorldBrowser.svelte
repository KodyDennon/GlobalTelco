<script lang="ts">
	import {
		connectionState,
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
	let loading = $state(true);
	let serverAddress = $state("ws://localhost:3001/ws");
	let loginUsername = $state("");
	let loginPassword = $state("");
	let authMode = $state<"login" | "register" | "guest">("guest");
	let authError = $state("");
	let isAuthenticated = $state(false);

	async function fetchWorlds() {
		loading = true;
		try {
			const apiUrl = serverAddress
				.replace("ws://", "http://")
				.replace("wss://", "https://")
				.replace("/ws", "/api/worlds");
			const token =
				typeof localStorage !== "undefined"
					? localStorage.getItem("gt_access_token")
					: null;

			const headers: Record<string, string> = {};
			if (token) {
				headers["Authorization"] = `Bearer ${token}`;
			}

			const res = await fetch(apiUrl, { headers });
			if (res.ok) {
				worlds = await res.json();
			}
		} catch {
			worlds = [];
		}
		loading = false;
	}

	function connectAndAuth() {
		authError = "";
		wsClient.connect(serverAddress);

		// Wait for connection then authenticate
		const unsub = connectionState.subscribe((state) => {
			if (state === "connected") {
				const storedToken =
					typeof localStorage !== "undefined"
						? localStorage.getItem("gt_access_token")
						: null;

				if (storedToken && authMode === "guest") {
					wsClient.loginWithToken(storedToken);
				} else if (authMode === "guest") {
					wsClient.loginAsGuest();
				} else if (authMode === "login") {
					wsClient.login(loginUsername, loginPassword);
				} else {
					wsClient.register(
						loginUsername,
						loginPassword,
						loginUsername + "@example.com",
					);
				}
				isAuthenticated = true;
				fetchWorlds();
				unsub();
			}
		});
	}

	function handleJoin(id: string) {
		wsClient.joinWorld(id);
		onJoin(id);
	}
</script>

<div class="world-browser">
	<div class="header">
		<button class="btn-back" onclick={onBack}>Back</button>
		<h2>Multiplayer</h2>
	</div>

	{#if !isAuthenticated}
		<div class="auth-section">
			<div class="form-group">
				<label for="server-address">Server Address</label>
				<input
					id="server-address"
					type="text"
					bind:value={serverAddress}
					placeholder="ws://localhost:3001/ws"
				/>
			</div>

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

			{#if authError}
				<div class="error">{authError}</div>
			{/if}

			<button class="btn-connect" onclick={connectAndAuth}>Connect</button
			>
		</div>
	{:else}
		<div class="world-list">
			{#if loading}
				<div class="loading">Loading worlds...</div>
			{:else if worlds.length === 0}
				<div class="empty">No active worlds found</div>
			{:else}
				{#each worlds as world}
					<div class="world-card">
						<div class="world-info">
							<h3>{world.name}</h3>
							<div class="world-meta">
								<span
									>Players: {world.player_count}/{world.max_players}</span
								>
								<span>Tick: {world.tick}</span>
								<span>Era: {world.era}</span>
							</div>
						</div>
						<button
							class="btn-join"
							onclick={() => handleJoin(world.id)}
							disabled={world.player_count >= world.max_players}
						>
							{world.player_count >= world.max_players
								? "Full"
								: "Join"}
						</button>
					</div>
				{/each}
			{/if}
			<button class="btn-refresh" onclick={fetchWorlds}>Refresh</button>
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
		margin-bottom: 32px;
	}

	.header h2 {
		font-size: 28px;
		margin: 0;
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

	.btn-connect:hover {
		background: rgba(16, 185, 129, 0.3);
	}

	.error {
		color: #ef4444;
		font-size: 13px;
		margin-bottom: 12px;
	}

	.world-list {
		max-width: 600px;
		margin: 0 auto;
		width: 100%;
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

	.world-info h3 {
		margin: 0 0 4px;
		font-size: 16px;
	}

	.world-meta {
		display: flex;
		gap: 16px;
		font-size: 12px;
		color: #9ca3af;
	}

	.btn-join {
		background: rgba(59, 130, 246, 0.2);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: #60a5fa;
		padding: 8px 20px;
		border-radius: 6px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
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

	.btn-refresh:hover {
		background: rgba(55, 65, 81, 0.6);
	}

	.loading,
	.empty {
		text-align: center;
		color: #6b7280;
		padding: 40px;
	}
</style>
