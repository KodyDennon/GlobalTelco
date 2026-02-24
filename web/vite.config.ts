import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	build: {
		// deck.gl (~1MB) and maplibre (~800KB) are large WebGL rendering libs
		// that cannot be meaningfully split further
		chunkSizeWarningLimit: 1100,
		rollupOptions: {
			output: {
				manualChunks(id) {
					if (id.includes('@deck.gl')) return 'deckgl';
					if (id.includes('maplibre-gl')) return 'maplibre';
					if (id.includes('d3-') || id.includes('d3/')) return 'd3';
				}
			}
		}
	}
});
