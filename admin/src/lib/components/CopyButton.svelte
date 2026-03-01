<script lang="ts">
	interface Props {
		text: string;
		label?: string;
	}
	let { text, label = 'Copy' }: Props = $props();
	let copied = $state(false);

	async function handleCopy() {
		await navigator.clipboard.writeText(text);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}
</script>

<button class="copy-btn" onclick={handleCopy} title="Copy to clipboard">
	{copied ? 'Copied!' : label}
</button>

<style>
	.copy-btn {
		padding: 3px 10px;
		font-size: 11px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		color: var(--text-muted);
		cursor: pointer;
		font-family: inherit;
		transition: all 0.15s;
	}
	.copy-btn:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
</style>
