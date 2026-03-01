<script lang="ts">
	interface Props {
		color?: 'green' | 'red' | 'blue' | 'amber' | 'purple' | 'gray';
		children: import('svelte').Snippet;
	}
	let { color = 'gray', children }: Props = $props();

	const colorMap: Record<string, string> = {
		green: 'var(--green)',
		red: 'var(--red)',
		blue: 'var(--blue)',
		amber: 'var(--amber)',
		purple: 'var(--purple)',
		gray: 'var(--text-dim)'
	};
</script>

<span class="badge" style="--badge-color: {colorMap[color]}">
	{@render children()}
</span>

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		padding: 1px 8px;
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		border-radius: 10px;
		color: var(--badge-color);
		background: color-mix(in srgb, var(--badge-color) 15%, transparent);
		border: 1px solid color-mix(in srgb, var(--badge-color) 30%, transparent);
		white-space: nowrap;
	}
</style>
