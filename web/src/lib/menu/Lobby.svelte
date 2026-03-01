<script lang="ts">
	import {
		connectionState,
		playerUsername,
		authError,
		isAuthenticated,
		serverInfo,
		worldId,
		type MultiplayerWorldInfo
	} from '$lib/stores/multiplayerState';
	import * as wsClient from '$lib/multiplayer/WebSocketClient';
	import { getGitHubAuthUrl } from '$lib/multiplayer/accountApi';
	import { fetchCatalog, type WorldTemplate, type WorldListEntry } from '$lib/multiplayer/lobbyApi';
	import WorldList from './lobby/WorldList.svelte';
	import WorldCreator from './lobby/WorldCreator.svelte';
	import InviteJoin from './lobby/InviteJoin.svelte';
	import RecentWorlds from './lobby/RecentWorlds.svelte';

	let {
		onBack,
		onJoin,
		onForgotPassword
	}: {
		onBack: () => void;
		onJoin: (worldId: string) => void;
		onForgotPassword: () => void;
	} = $props();

	let worlds = $state<MultiplayerWorldInfo[]>([]);
	let templates = $state<WorldTemplate[]>([]);
	let loadingWorlds = $state(false);
	let loginUsername = $state('');
	let loginPassword = $state('');
	let loginEmail = $state('');
	let authMode = $state<'login' | 'register' | 'guest'>('guest');
	let joiningWorldId = $state<string | null>(null);
	let serverOnline = $state<boolean | null>(null);
	let githubLoading = $state(false);
	let activeTab = $state<'browse' | 'create' | 'recent'>('browse');

	// Check server status on mount
	$effect(() => {
		checkServer();
	});

	// When authenticated, fetch data
	$effect(() => {
		if ($isAuthenticated) {
			loadWorlds();
			loadCatalog();
		}
	});

	// Watch for WorldJoined messages to navigate
	$effect(() => {
		const id = $worldId;
		if (id && joiningWorldId) {
			joiningWorldId = null;
			onJoin(id);
		}
	});

	async function checkServer() {
		const info = await wsClient.fetchServerInfo();
		serverOnline = info !== null;
		if (serverOnline) tryAutoConnect();
	}

	function tryAutoConnect() {
		const storedToken =
			typeof localStorage !== 'undefined' ? localStorage.getItem('gt_access_token') : null;
		if (storedToken) {
			wsClient.connect();
			const unsub = connectionState.subscribe((state) => {
				if (state === 'connected') {
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

	async function loadCatalog() {
		try {
			templates = await fetchCatalog();
		} catch {
			// Catalog may not be available
		}
	}

	function connectAndAuth() {
		authError.set('');
		wsClient.connect();
		const unsub = connectionState.subscribe((state) => {
			if (state === 'connected') {
				if (authMode === 'guest') {
					wsClient.loginAsGuest();
				} else if (authMode === 'login') {
					wsClient.login(loginUsername, loginPassword);
				} else {
					wsClient.register(loginUsername, loginPassword, loginEmail);
				}
				unsub();
			}
		});
	}

	function handleJoin(id: string) {
		joiningWorldId = id;
		wsClient.joinWorld(id);
	}

	function handleCreated(newWorldId: string) {
		handleJoin(newWorldId);
	}

	function handleDisconnect() {
		wsClient.disconnect();
		isAuthenticated.set(false);
		authError.set('');
		worlds = [];
		templates = [];
		joiningWorldId = null;
	}

	async function handleGitHubLogin() {
		githubLoading = true;
		try {
			const url = await getGitHubAuthUrl();
			window.location.href = url;
		} catch {
			authError.set('GitHub OAuth not available');
			githubLoading = false;
		}
	}
</script>

<div class="lobby">
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
			{:else if $connectionState === 'connected' && $isAuthenticated}
				<span class="status-dot online"></span>
				<span class="status-text">Connected as {$playerUsername}</span>
			{:else if $connectionState === 'connecting' || $connectionState === 'reconnecting'}
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
			&middot; {$serverInfo.active_worlds} world{$serverInfo.active_worlds !== 1 ? 's' : ''}
			&middot; {$serverInfo.connected_players} player{$serverInfo.connected_players !== 1 ? 's' : ''} online
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
				<button class:active={authMode === 'guest'} onclick={() => (authMode = 'guest')}>Guest</button>
				<button class:active={authMode === 'login'} onclick={() => (authMode = 'login')}>Login</button>
				<button class:active={authMode === 'register'} onclick={() => (authMode = 'register')}>Register</button>
			</div>

			<form onsubmit={(e) => { e.preventDefault(); connectAndAuth(); }}>
				{#if authMode !== 'guest'}
					<div class="form-group">
						<label for="login-username">Username</label>
						<input id="login-username" type="text" autocomplete="username" bind:value={loginUsername} placeholder="Username" />
					</div>
					<div class="form-group">
						<label for="login-password">Password</label>
						<input id="login-password" type="password" autocomplete={authMode === 'register' ? 'new-password' : 'current-password'} bind:value={loginPassword} placeholder="Password" />
					</div>
				{/if}

				{#if authMode === 'register'}
					<div class="form-group">
						<label for="login-email">Email</label>
						<input id="login-email" type="email" autocomplete="email" bind:value={loginEmail} placeholder="you@example.com" />
					</div>
				{/if}

				{#if $authError}
					<div class="error">{$authError}</div>
				{/if}

				<button class="btn-connect" type="submit" disabled={$connectionState === 'connecting'}>
					{$connectionState === 'connecting' ? 'Connecting...' : 'Connect'}
				</button>

				{#if authMode === 'login'}
					<button class="btn-forgot" type="button" onclick={onForgotPassword}>
						Forgot password?
					</button>
				{/if}
			</form>

			<div class="oauth-divider"><span>or</span></div>
			<button class="btn-github" onclick={handleGitHubLogin} disabled={githubLoading}>
				{githubLoading ? 'Redirecting...' : 'Login with GitHub'}
			</button>
		</div>
	{:else}
		<div class="lobby-tabs">
			<button class:active={activeTab === 'browse'} onclick={() => activeTab = 'browse'}>Browse</button>
			<button class:active={activeTab === 'create'} onclick={() => activeTab = 'create'}>Create</button>
			<button class:active={activeTab === 'recent'} onclick={() => activeTab = 'recent'}>Recent</button>
		</div>

		<div class="lobby-content">
			{#if activeTab === 'browse'}
				{#if loadingWorlds}
					<div class="loading">Loading worlds...</div>
				{:else}
					<WorldList worlds={worlds as unknown as WorldListEntry[]} onJoin={handleJoin} {joiningWorldId} />
				{/if}
				<button class="btn-refresh" onclick={loadWorlds} disabled={loadingWorlds}>
					{loadingWorlds ? 'Refreshing...' : 'Refresh'}
				</button>
			{:else if activeTab === 'create'}
				<WorldCreator {templates} onCreated={handleCreated} />
			{:else if activeTab === 'recent'}
				<RecentWorlds onJoin={handleJoin} />
			{/if}
		</div>

		<div class="footer">
			<span class="logged-in">Logged in as: {$playerUsername}</span>
			<button class="btn-disconnect" onclick={handleDisconnect}>Disconnect</button>
		</div>
	{/if}
</div>

<style>
	.lobby {
		width: 100vw;
		height: 100vh;
		display: flex;
		flex-direction: column;
		background: linear-gradient(135deg, #0a0e17 0%, #111827 50%, #0a0e17 100%);
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

	/* Auth Section */
	.auth-section {
		max-width: 400px;
		margin: 0 auto;
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

	.error {
		color: #ef4444;
		font-size: 13px;
		margin-bottom: 12px;
		padding: 8px 12px;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.2);
		border-radius: 6px;
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

	.btn-forgot {
		background: none;
		border: none;
		color: #6b7280;
		font-size: 13px;
		cursor: pointer;
		padding: 8px 0 0;
		font-family: system-ui, sans-serif;
	}

	.btn-forgot:hover {
		color: #9ca3af;
	}

	.oauth-divider {
		display: flex;
		align-items: center;
		gap: 12px;
		margin: 20px 0;
		color: #4b5563;
		font-size: 13px;
	}

	.oauth-divider::before,
	.oauth-divider::after {
		content: '';
		flex: 1;
		height: 1px;
		background: rgba(55, 65, 81, 0.5);
	}

	.btn-github {
		width: 100%;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		color: #d1d5db;
		padding: 12px;
		border-radius: 6px;
		font-size: 14px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
	}

	.btn-github:hover:not(:disabled) {
		background: rgba(55, 65, 81, 0.8);
		color: #f3f4f6;
	}

	.btn-github:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Lobby Tabs & Content */
	.lobby-tabs {
		display: flex;
		gap: 4px;
		margin-bottom: 20px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
		padding-bottom: 0;
	}

	.lobby-tabs button {
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		color: #9ca3af;
		padding: 10px 20px;
		cursor: pointer;
		font-size: 14px;
		font-family: system-ui, sans-serif;
		margin-bottom: -1px;
	}

	.lobby-tabs button:hover {
		color: #d1d5db;
	}

	.lobby-tabs button.active {
		color: #60a5fa;
		border-bottom-color: #60a5fa;
	}

	.lobby-content {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.loading {
		text-align: center;
		color: #6b7280;
		padding: 40px;
	}

	.btn-refresh {
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		color: #9ca3af;
		padding: 10px;
		border-radius: 6px;
		cursor: pointer;
		margin-top: 8px;
		font-family: system-ui, sans-serif;
		align-self: stretch;
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
		padding-top: 16px;
		border-top: 1px solid rgba(55, 65, 81, 0.3);
		margin-top: 16px;
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
</style>
