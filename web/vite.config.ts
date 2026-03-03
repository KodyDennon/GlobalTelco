import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { readFileSync } from 'fs';
import { resolve } from 'path';

const version = readFileSync(resolve(__dirname, '../VERSION'), 'utf-8').trim();

export default defineConfig({
	define: {
		__APP_VERSION__: JSON.stringify(version),
	},
	worker: {
		format: 'es',
	},
	plugins: [
		// Stub Node.js built-ins that @loaders.gl/worker-utils imports.
		// Must run before Vite's built-in __vite-browser-external resolver.
		{
			name: 'stub-node-builtins',
			enforce: 'pre',
			resolveId(id: string) {
				if (id === 'child_process') return '\0child_process';
			},
			load(id: string) {
				if (id === '\0child_process')
					return 'export const spawn = () => {}; export default {};';
			}
		},
		sveltekit(),
	],
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
					if (id.includes('@msgpack/msgpack')) return 'msgpack';
					if (id.includes('fzstd')) return 'fzstd';
				}
			}
		}
	}
});
