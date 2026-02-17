<script lang="ts">
	import { tr } from '$lib/i18n/index';

	let { children } = $props();
	let error: string | null = $state(null);
	let stack: string | null = $state(null);
	let showStack = $state(false);
	let copied = $state(false);

	function handleError(e: Event) {
		if (e instanceof ErrorEvent) {
			error = e.message;
			stack = e.error?.stack ?? null;
		} else {
			error = 'Unknown error';
		}
		console.error('ErrorBoundary caught:', error, stack);
	}

	function handleRejection(e: PromiseRejectionEvent) {
		const reason = e.reason;
		error = reason?.message ?? String(reason);
		stack = reason?.stack ?? null;
	}

	function categorize(err: string): string {
		if (err.includes('WASM') || err.includes('wasm') || err.includes('RuntimeError')) return 'wasm';
		if (err.includes('network') || err.includes('fetch') || err.includes('WebSocket')) return 'network';
		if (err.includes('memory') || err.includes('heap') || err.includes('allocation')) return 'memory';
		return 'runtime';
	}

	async function copyReport() {
		const report = [
			'GlobalTelco Error Report',
			`Time: ${new Date().toISOString()}`,
			`UserAgent: ${navigator.userAgent}`,
			`Category: ${error ? categorize(error) : 'unknown'}`,
			`Error: ${error}`,
			stack ? `Stack:\n${stack}` : ''
		].filter(Boolean).join('\n');

		await navigator.clipboard.writeText(report);
		copied = true;
		setTimeout(() => { copied = false; }, 2000);
	}

	function returnToMenu() {
		window.location.hash = '';
		window.location.reload();
	}
</script>

<svelte:window onerror={handleError} onunhandledrejection={handleRejection} />

{#if error}
	<div class="error-boundary" role="alert">
		<div class="error-content">
			<h2>{$tr('common.error')}</h2>
			{#if error}
				{@const cat = categorize(error)}
				<span class="error-category">{cat.toUpperCase()}</span>
			{/if}
			<p class="error-msg">{error}</p>
			{#if stack}
				<button class="stack-toggle" onclick={() => (showStack = !showStack)}
					aria-expanded={showStack}>
					{showStack ? $tr('error.hide_details') : $tr('error.show_details')}
				</button>
				{#if showStack}
					<pre class="stack-trace">{stack}</pre>
				{/if}
			{/if}
			<div class="error-actions">
				<button class="btn btn-copy" onclick={copyReport} aria-label={$tr('error.copy_report')}>
					{copied ? $tr('error.copied') : $tr('error.copy_report')}
				</button>
				<button class="btn btn-menu" onclick={returnToMenu}>{$tr('error.return_to_menu')}</button>
				<button class="btn btn-reload" onclick={() => window.location.reload()}>{$tr('common.reload')}</button>
			</div>
		</div>
	</div>
{:else}
	{@render children()}
{/if}

<style>
	.error-boundary {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #0a0e17;
		color: #f3f4f6;
		font-family: system-ui, sans-serif;
	}

	.error-content {
		text-align: center;
		max-width: 560px;
		padding: 48px;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 12px;
	}

	h2 {
		color: #ef4444;
		margin-bottom: 12px;
	}

	.error-category {
		display: inline-block;
		font-size: 10px;
		font-weight: 700;
		padding: 2px 8px;
		border-radius: 4px;
		background: rgba(239, 68, 68, 0.15);
		color: #f87171;
		letter-spacing: 0.5px;
		margin-bottom: 12px;
	}

	.error-msg {
		color: #9ca3af;
		font-size: 14px;
		margin-bottom: 16px;
		word-break: break-word;
	}

	.stack-toggle {
		background: none;
		border: none;
		color: #6b7280;
		font-size: 12px;
		cursor: pointer;
		margin-bottom: 8px;
		text-decoration: underline;
	}

	.stack-toggle:hover {
		color: #9ca3af;
	}

	.stack-trace {
		text-align: left;
		font-family: monospace;
		font-size: 11px;
		color: #6b7280;
		background: rgba(0, 0, 0, 0.3);
		padding: 12px;
		border-radius: 6px;
		max-height: 200px;
		overflow-y: auto;
		margin-bottom: 16px;
		white-space: pre-wrap;
		word-break: break-all;
	}

	.error-actions {
		display: flex;
		gap: 8px;
		justify-content: center;
		flex-wrap: wrap;
	}

	.btn {
		padding: 10px 24px;
		border: 1px solid;
		font-size: 13px;
		border-radius: 6px;
		cursor: pointer;
	}

	.btn-copy {
		background: rgba(107, 114, 128, 0.2);
		border-color: rgba(107, 114, 128, 0.3);
		color: #9ca3af;
	}

	.btn-copy:hover {
		background: rgba(107, 114, 128, 0.3);
	}

	.btn-menu {
		background: rgba(245, 158, 11, 0.2);
		border-color: rgba(245, 158, 11, 0.3);
		color: #fbbf24;
	}

	.btn-menu:hover {
		background: rgba(245, 158, 11, 0.3);
	}

	.btn-reload {
		background: rgba(59, 130, 246, 0.2);
		border-color: rgba(59, 130, 246, 0.3);
		color: #60a5fa;
	}

	.btn-reload:hover {
		background: rgba(59, 130, 246, 0.3);
	}
</style>
