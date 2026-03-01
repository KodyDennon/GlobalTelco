<script lang="ts">
	interface Props {
		value?: string;
		placeholder?: string;
		onchange: (value: string) => void;
		debounceMs?: number;
	}
	let { value = '', placeholder = 'Search...', onchange, debounceMs = 300 }: Props = $props();

	let timer: ReturnType<typeof setTimeout>;

	function handleInput(e: Event) {
		const v = (e.target as HTMLInputElement).value;
		value = v;
		clearTimeout(timer);
		timer = setTimeout(() => onchange(v), debounceMs);
	}
</script>

<div class="search-wrap">
	<span class="search-icon">{'\u{1F50D}'}</span>
	<input type="text" {value} {placeholder} oninput={handleInput} class="search-input" />
	{#if value}
		<button class="clear-btn" onclick={() => { value = ''; onchange(''); }}>&times;</button>
	{/if}
</div>

<style>
	.search-wrap {
		position: relative;
		display: flex;
		align-items: center;
	}
	.search-icon {
		position: absolute;
		left: 10px;
		font-size: 13px;
		color: var(--text-dim);
		pointer-events: none;
	}
	.search-input {
		width: 100%;
		padding: 6px 30px 6px 32px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		color: var(--text-primary);
		font-size: 13px;
		font-family: inherit;
	}
	.search-input::placeholder {
		color: var(--text-dim);
	}
	.clear-btn {
		position: absolute;
		right: 6px;
		background: none;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		font-size: 16px;
		padding: 2px 4px;
	}
	.clear-btn:hover {
		color: var(--text-primary);
	}
</style>
