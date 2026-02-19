import * as THREE from 'three';
import { icons, type IconName } from '$lib/assets/icons';

interface SpriteOptions {
	size: number;
	color: string;
	padding: number;
}

const textureCache = new Map<string, THREE.CanvasTexture>();

function cacheKey(name: IconName, opts: SpriteOptions): string {
	return `${name}:${opts.size}:${opts.color}:${opts.padding ?? 0}`;
}

/**
 * Rasterize an SVG icon string to a canvas at the given pixel size.
 * Returns a promise that resolves once the image has been drawn.
 */
function rasterizeSvg(
	svgString: string,
	size: number,
	color: string,
	padding: number
): Promise<HTMLCanvasElement> {
	return new Promise((resolve, reject) => {
		const canvas = document.createElement('canvas');
		const totalSize = size + padding * 2;
		canvas.width = totalSize;
		canvas.height = totalSize;

		const ctx = canvas.getContext('2d');
		if (!ctx) {
			reject(new Error('Failed to get 2d context'));
			return;
		}

		// Inject color into SVG
		const colored = svgString.replace(/currentColor/g, color);

		// Create blob URL from SVG string
		const blob = new Blob([colored], { type: 'image/svg+xml;charset=utf-8' });
		const url = URL.createObjectURL(blob);

		const img = new Image();
		img.onload = () => {
			ctx.drawImage(img, padding, padding, size, size);
			URL.revokeObjectURL(url);
			resolve(canvas);
		};
		img.onerror = () => {
			URL.revokeObjectURL(url);
			reject(new Error(`Failed to load SVG: ${url}`));
		};
		img.src = url;
	});
}

/**
 * Create a Three.js CanvasTexture from an SVG icon.
 * Results are cached — same icon/size/color combo returns the same texture.
 */
export async function createIconTexture(
	name: IconName,
	options: Partial<SpriteOptions> = {}
): Promise<THREE.CanvasTexture> {
	const opts: SpriteOptions = {
		size: options.size ?? 64,
		color: options.color ?? '#ffffff',
		padding: options.padding ?? 4,
	};

	const key = cacheKey(name, opts);
	const cached = textureCache.get(key);
	if (cached) return cached;

	const svgString = icons[name];
	if (!svgString) {
		throw new Error(`Unknown icon: ${name}`);
	}

	const canvas = await rasterizeSvg(svgString, opts.size, opts.color, opts.padding);

	const texture = new THREE.CanvasTexture(canvas);
	texture.minFilter = THREE.LinearFilter;
	texture.magFilter = THREE.LinearFilter;
	texture.needsUpdate = true;

	textureCache.set(key, texture);
	return texture;
}

/**
 * Create a Three.js Sprite with an SVG icon as its texture.
 * The sprite is centered and scaled to world units.
 */
export async function createIconSprite(
	name: IconName,
	options: Partial<SpriteOptions> & { worldSize?: number } = {}
): Promise<THREE.Sprite> {
	const texture = await createIconTexture(name, options);
	const material = new THREE.SpriteMaterial({
		map: texture,
		transparent: true,
		depthTest: false,
	});

	const sprite = new THREE.Sprite(material);
	const worldSize = options.worldSize ?? 1;
	sprite.scale.set(worldSize, worldSize, 1);

	return sprite;
}

/**
 * Batch-create textures for all infrastructure icons.
 * Useful for preloading at game start to avoid pop-in.
 */
export async function preloadInfrastructureIcons(
	color: string = '#ffffff',
	size: number = 64
): Promise<Map<IconName, THREE.CanvasTexture>> {
	const infraNames: IconName[] = [
		'central-office',
		'exchange-point',
		'cell-tower',
		'data-center',
		'satellite-ground',
		'submarine-landing',
		'wireless-relay',
	];

	const results = new Map<IconName, THREE.CanvasTexture>();
	const promises = infraNames.map(async (name) => {
		const texture = await createIconTexture(name, { color, size });
		results.set(name, texture);
	});

	await Promise.all(promises);
	return results;
}

/**
 * Batch-create textures for all city tier icons.
 */
export async function preloadCityIcons(
	color: string = '#ffffff',
	size: number = 64
): Promise<Map<IconName, THREE.CanvasTexture>> {
	const cityNames: IconName[] = [
		'hamlet',
		'town',
		'city',
		'metropolis',
		'megalopolis',
	];

	const results = new Map<IconName, THREE.CanvasTexture>();
	const promises = cityNames.map(async (name) => {
		const texture = await createIconTexture(name, { color, size });
		results.set(name, texture);
	});

	await Promise.all(promises);
	return results;
}

/**
 * Clear the texture cache. Call when changing themes or color schemes.
 */
export function clearTextureCache(): void {
	for (const texture of textureCache.values()) {
		texture.dispose();
	}
	textureCache.clear();
}
