<script lang="ts">
	import { icons, type IconName } from '$lib/assets/icons';

	interface Props {
		name: IconName;
		size?: number;
		color?: string;
		class?: string;
		title?: string;
	}

	let { name, size = 24, color = 'currentColor', class: className = '', title }: Props = $props();

	let svgContent = $derived(icons[name] ?? '');

	function colorize(svg: string, fillColor: string): string {
		return svg.replace(/currentColor/g, fillColor);
	}
</script>

<span
	class="gt-icon {className}"
	style="width: {size}px; height: {size}px; color: {color};"
	role={title ? 'img' : 'presentation'}
	aria-label={title}
	aria-hidden={!title}
>
	{@html colorize(svgContent, color)}
</span>

<style>
	.gt-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		line-height: 0;
	}

	.gt-icon :global(svg) {
		width: 100%;
		height: 100%;
	}
</style>
