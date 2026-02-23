import { tooltipData } from '$lib/stores/uiState';

const TOOLTIP_DELAY_MS = 1200; // Show after 1.2s hover (doesn't pop during fast clicks)

/**
 * Svelte action for intelligent delayed tooltips.
 * Usage: <button use:tooltip={'Some helpful text'}>
 * Or with a function: <button use:tooltip={() => `Cash: ${formatMoney(cash)}`}>
 *
 * Tooltips appear after hovering for ~1.2 seconds and disappear on mouseout or click.
 * Context-aware: pass a function to generate tooltip text from current game state.
 */
export function tooltip(node: HTMLElement, content: string | (() => string)) {
	let timer: ReturnType<typeof setTimeout> | null = null;
	let showing = false;

	function getContent(): string {
		return typeof content === 'function' ? content() : content;
	}

	function show(e: MouseEvent) {
		// Don't show if content is empty
		const text = getContent();
		if (!text) return;
		showing = true;
		tooltipData.set({ x: e.clientX, y: e.clientY, content: text });
	}

	function onMouseEnter(e: MouseEvent) {
		if (timer) clearTimeout(timer);
		timer = setTimeout(() => show(e), TOOLTIP_DELAY_MS);
	}

	function onMouseMove(e: MouseEvent) {
		if (showing) {
			const text = getContent();
			if (text) {
				tooltipData.set({ x: e.clientX, y: e.clientY, content: text });
			}
		}
	}

	function hide() {
		if (timer) {
			clearTimeout(timer);
			timer = null;
		}
		if (showing) {
			showing = false;
			tooltipData.set(null);
		}
	}

	// Remove native title to avoid double tooltip
	const originalTitle = node.getAttribute('title');
	if (originalTitle) {
		node.removeAttribute('title');
	}

	node.addEventListener('mouseenter', onMouseEnter);
	node.addEventListener('mousemove', onMouseMove);
	node.addEventListener('mouseleave', hide);
	node.addEventListener('mousedown', hide); // Hide on click for fast interactions
	node.addEventListener('focus', hide); // Hide on keyboard focus too

	return {
		update(newContent: string | (() => string)) {
			content = newContent;
			// If currently showing, update immediately
			if (showing) {
				const text = getContent();
				if (!text) {
					hide();
				}
			}
		},
		destroy() {
			hide();
			node.removeEventListener('mouseenter', onMouseEnter);
			node.removeEventListener('mousemove', onMouseMove);
			node.removeEventListener('mouseleave', hide);
			node.removeEventListener('mousedown', hide);
			node.removeEventListener('focus', hide);
			// Restore original title if it existed
			if (originalTitle) {
				node.setAttribute('title', originalTitle);
			}
		}
	};
}
