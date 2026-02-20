import * as THREE from 'three';
import * as bridge from '$lib/wasm/bridge';
import type { GridCell, City, Region, CorpSummary, CellCoverage, TrafficFlows } from '$lib/wasm/types';
import type { IconName } from '$lib/assets/icons';
import { preloadInfrastructureIcons, preloadCityIcons } from './SpriteFactory';
import { GridPathfinder, needsTerrainRouting } from './GridPathfinder';

const TERRAIN_COLORS: Record<string, number> = {
	Urban: 0x4a4a5a,
	Suburban: 0x6b7b6b,
	Rural: 0x5a8a4a,
	Mountainous: 0x8a7a6a,
	Desert: 0xc4a86a,
	Coastal: 0x6a9aaa,
	OceanShallow: 0x2a5a8a,
	OceanDeep: 0x1a3a6a,
	Tundra: 0x9aabb8,
	Frozen: 0xc8d8e8,
	Ocean: 0x06101f // Deep ocean color
};

const POLITICAL_COLORS = [
	0xe6194b, 0x3cb44b, 0xffe119, 0x4363d8, 0xf58231,
	0x911eb4, 0x46f0f0, 0xf032e6, 0xbcf60c, 0xfabebe,
	0x008080, 0xe6beff, 0x9a6324
];

const TERRAIN_TINT: Record<string, { color: number; opacity: number }> = {
	Urban: { color: 0x4a4a5a, opacity: 0.15 },
	Suburban: { color: 0x6b7b6b, opacity: 0.15 },
	Rural: { color: 0x5a8a4a, opacity: 0.15 },
	Mountainous: { color: 0x8a7a6a, opacity: 0.25 },
	Desert: { color: 0xc4a86a, opacity: 0.2 },
	Coastal: { color: 0x6a9aaa, opacity: 0.15 },
	OceanShallow: { color: 0x2a5a8a, opacity: 0.15 },
	OceanDeep: { color: 0x1a3a6a, opacity: 0.15 },
	Tundra: { color: 0x9aabb8, opacity: 0.2 },
	Frozen: { color: 0xc8d8e8, opacity: 0.3 },
};

const CORP_COLORS = [
	0x10b981, 0x3b82f6, 0xf59e0b, 0xef4444, 0x8b5cf6, 0xec4899, 0x14b8a6, 0xf97316
];

// Edge styling per type — radiusFactor is multiplied by baseCellSize at render time
interface EdgeStyle {
	color: number;
	opacity: number;
	radiusFactor: number; // Multiplied by baseCellSize for tube radius
	dashed?: boolean;     // Wireless edges use dashed lines
	dashSize?: number;
	gapSize?: number;
	segments: number;     // Tube radial segments (fewer = cheaper)
}
const EDGE_STYLES: Record<string, EdgeStyle> = {
	FiberLocal: { color: 0x22d3a0, opacity: 0.85, radiusFactor: 0.008, segments: 4 },
	FiberRegional: { color: 0x60a5fa, opacity: 0.9, radiusFactor: 0.012, segments: 4 },
	FiberNational: { color: 0x818cf8, opacity: 0.95, radiusFactor: 0.016, segments: 5 },
	Copper: { color: 0xd97706, opacity: 0.75, radiusFactor: 0.006, segments: 3 },
	Microwave: { color: 0x22d3ee, opacity: 0.6, radiusFactor: 0.008, segments: 3, dashed: true, dashSize: 1.2, gapSize: 0.6 },
	Satellite: { color: 0xfbbf24, opacity: 0.5, radiusFactor: 0.008, segments: 3, dashed: true, dashSize: 2.0, gapSize: 1.0 },
	Submarine: { color: 0x3b82f6, opacity: 0.9, radiusFactor: 0.014, segments: 4 }
};

// Map Rust NodeType variants to SVG icon names
const NODE_TYPE_TO_ICON: Record<string, IconName> = {
	CentralOffice: 'central-office',
	CellTower: 'cell-tower',
	DataCenter: 'data-center',
	ExchangePoint: 'exchange-point',
	SatelliteGround: 'satellite-ground',
	SubmarineLanding: 'submarine-landing',
	WirelessRelay: 'wireless-relay',
	BackboneRouter: 'exchange-point'
};

export class MapRenderer {
	private scene: THREE.Scene;
	private camera: THREE.OrthographicCamera;
	private renderer: THREE.WebGLRenderer;
	private container: HTMLElement;

	// Render layers (ordered by z-depth)
	private oceanGroup: THREE.Group; // Layer 1: Ocean base
	private landGroup: THREE.Group; // Layer 2: Land masses
	private borderGroup: THREE.Group; // Layer 3: Region borders
	private cityGlowGroup: THREE.Group; // Layer 3.5: City glow lights
	private cityGroup: THREE.Group; // Layer 4: Cities
	private edgeGroup: THREE.Group; // Layer 5a: Infrastructure edges
	private infraGroup: THREE.Group; // Layer 5b: Infrastructure nodes
	private ownerGroup: THREE.Group; // Layer 6: Ownership overlay
	private selectionGroup: THREE.Group; // Layer 7: Selection highlight
	private labelGroup: THREE.Group; // Layer 8: Labels

	private isDragging = false;
	private dragMoved = false;
	private lastMouse = { x: 0, y: 0 };
	private zoom = 1;
	private panX = 0;
	private panY = 0;

	private raycaster: THREE.Raycaster;
	private pointer: THREE.Vector2;
	private entityMeshMap: Map<number, THREE.Object3D> = new Map();
	private selectedId: number | null = null;
	private selectionRing: THREE.Mesh | null = null;

	// Cached data for ownership overlay
	private corpColorMap: Map<number, number> = new Map();

	// Overlay state
	private activeOverlay: string = 'none';
	private overlayGroup: THREE.Group; // Dedicated overlay layer
	private cachedCells: GridCell[] = [];
	private cachedRegions: Region[] = [];

	// Computed cell size for hex grid
	private baseCellSize = 1.5;

	// SVG icon textures for infrastructure nodes and cities
	private iconTextures: Map<IconName, THREE.CanvasTexture> = new Map();
	private cityTextures: Map<IconName, THREE.CanvasTexture> = new Map();

	// Edge source highlight group
	private edgePreviewGroup: THREE.Group;
	private currentEdgeSourceId: number | null = null;
	private previewLine: THREE.Line | null = null;


	// Grid pathfinder for terrain-aware edge routing
	private pathfinder = new GridPathfinder();


	// Set of cell indices that belong to cities (for urban overlay rendering)
	private cityCellSet = new Set<number>();

	// Quality setting: controls antialias, pixel ratio, label density
	public quality: 'low' | 'medium' | 'high' = 'medium';

	constructor(container: HTMLElement, quality: 'low' | 'medium' | 'high' = 'medium') {
		this.quality = quality;
		this.container = container;
		const w = container.clientWidth;
		const h = container.clientHeight;
		const aspect = w / h;

		this.scene = new THREE.Scene();
		this.scene.background = new THREE.Color(0x0a0e17);

		const viewSize = 200;
		this.camera = new THREE.OrthographicCamera(
			-viewSize * aspect,
			viewSize * aspect,
			viewSize,
			-viewSize,
			-1000,
			1000
		);
		this.camera.position.z = 100;

		this.renderer = new THREE.WebGLRenderer({ antialias: quality !== 'low' });
		this.renderer.setSize(w, h);
		const maxDpr = quality === 'high' ? 2 : quality === 'medium' ? 1.5 : 1;
		this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, maxDpr));
		container.appendChild(this.renderer.domElement);

		// Create groups in render order (z-depth managed by group add order + mesh z position)
		this.oceanGroup = new THREE.Group();
		this.landGroup = new THREE.Group();
		this.borderGroup = new THREE.Group();
		this.cityGlowGroup = new THREE.Group();
		this.ownerGroup = new THREE.Group();
		this.cityGroup = new THREE.Group();
		this.edgeGroup = new THREE.Group();
		this.infraGroup = new THREE.Group();
		this.selectionGroup = new THREE.Group();
		this.labelGroup = new THREE.Group();
		this.overlayGroup = new THREE.Group();
		this.edgePreviewGroup = new THREE.Group();

		this.scene.add(this.oceanGroup);
		this.scene.add(this.landGroup);
		this.scene.add(this.borderGroup);
		this.scene.add(this.cityGlowGroup);
		this.scene.add(this.ownerGroup);
		this.scene.add(this.overlayGroup);
		this.scene.add(this.edgeGroup);
		this.scene.add(this.cityGroup);
		this.scene.add(this.infraGroup);
		this.scene.add(this.edgePreviewGroup);
		this.scene.add(this.selectionGroup);
		this.scene.add(this.labelGroup);

		this.raycaster = new THREE.Raycaster();
		this.pointer = new THREE.Vector2();

		this.setupControls();
		this.setupResize();
	}

	private latLonToMercator(lat: number, lon: number): { x: number; y: number } {
		const CLAMP = 82; // Tighten clamp to avoid extreme polar stretching
		const clampedLat = Math.max(-CLAMP, Math.min(CLAMP, lat));
		const latRad = (clampedLat * Math.PI) / 180;
		return {
			x: lon,
			y: (180 / Math.PI) * Math.log(Math.tan(Math.PI / 4 + latRad / 2))
		};
	}

	async buildMap() {
		if (!bridge.isInitialized()) return;

		// Preload SVG icon textures (infra + city)
		try {
			const [infraTex, cityTex] = await Promise.all([
				preloadInfrastructureIcons('#ffffff', 64),
				preloadCityIcons('#e8d5b0', 128)  // Warm tone for city silhouettes
			]);
			this.iconTextures = infraTex;
			this.cityTextures = cityTex;
		} catch {
			// Fallback: proceed without icons
		}

		const cells = bridge.getGridCells();
		const citiesData = bridge.getCities();
		const regions = bridge.getRegions();

		this.cachedCells = cells;
		this.cachedRegions = regions;

		// Initialize pathfinder with grid data for terrain-aware edge routing
		this.pathfinder.init(cells);

		// Compute base cell size from cell count.
		// Use circumradius that makes hexagons overlap slightly to avoid gaps.
		// Geodesic grid spacing ≈ 360/sqrt(N) degrees; hexagon circumradius needs
		// to be ≈ spacing / (2 * cos(30°)) = spacing / 1.732 for edge-to-edge contact.
		// We use a higher multiplier (0.75) to ensure full coverage with slight overlap.
		if (cells.length > 0) {
			this.baseCellSize = (360 / Math.sqrt(cells.length)) * 0.75;
		}

		this.buildOcean();
		this.buildLand(cells);
		this.buildRegionBorders(regions, citiesData);
		this.buildCities(citiesData);
		this.buildLabels(citiesData, regions);
	}

	private buildOcean() {
		this.oceanGroup.clear();
		// Large plane covering the entire coordinate space (clamped at 85 lat)
		// Mercator range: x in [-180, 180], y in [-203, 203] (at 85 deg)
		const geo = new THREE.PlaneGeometry(1000, 600);
		const mat = new THREE.MeshBasicMaterial({ color: TERRAIN_COLORS.Ocean });
		const plane = new THREE.Mesh(geo, mat);
		plane.position.set(0, 0, -5); // Deepest layer
		this.oceanGroup.add(plane);
	}

	private async buildLand(cells: GridCell[]) {
		this.landGroup.clear();
		const isRealEarth = bridge.isRealEarth();

		if (isRealEarth) {
			await this.buildRealEarthPolygons();
		} else {
			this.buildProcgenPolygons();
		}

		// Always build invisible parcel hit targets for clicking
		this.buildParcelHitTargets(cells);

		// Always build terrain tint overlay
		this.buildTerrainTintOverlay(cells);
	}

	private async buildRealEarthPolygons() {
		try {
			const res = await fetch('/countries-110m.json');
			const data = await res.json();

			for (const feature of data.features) {
				const mapColor = feature.properties.MAPCOLOR13 ?? 1;
				const color = POLITICAL_COLORS[(mapColor - 1) % POLITICAL_COLORS.length] ?? 0x888888;
				this.renderGeoJsonFeature(feature, color);
			}
		} catch (e) {
			console.error('Failed to load countries-110m.json', e);
		}
	}

	private normalizePolygon(pts: [number, number][]): [number, number][] {
		if (pts.length < 2) return pts;
		const result: [number, number][] = [pts[0]];
		for (let i = 1; i < pts.length; i++) {
			let lon = pts[i][0];
			const prevLon = result[i - 1][0];
			// Normalize lon to be within 180 of prevLon
			while (lon - prevLon > 180) lon -= 360;
			while (lon - prevLon < -180) lon += 360;
			result.push([lon, pts[i][1]]);
		}
		return result;
	}

	private buildProcgenPolygons() {
		const regions = bridge.getRegions();
		for (let i = 0; i < regions.length; i++) {
			const r = regions[i];
			if (!r.boundary_polygon || r.boundary_polygon.length < 3) continue;

			const color = POLITICAL_COLORS[i % POLITICAL_COLORS.length];
			const pts = this.normalizePolygon(r.boundary_polygon.map(p => [p[1], p[0]]));

			const shape = new THREE.Shape();
			const first = this.latLonToMercator(pts[0][1], pts[0][0]);
			shape.moveTo(first.x, first.y);
			for (let j = 1; j < pts.length; j++) {
				const pt = this.latLonToMercator(pts[j][1], pts[j][0]);
				shape.lineTo(pt.x, pt.y);
			}

			const geo = new THREE.ShapeGeometry(shape);
			const mat = new THREE.MeshBasicMaterial({ color, side: THREE.DoubleSide });

			// Triple render for wrap-around
			for (const offset of [-360, 0, 360]) {
				const mesh = new THREE.Mesh(geo, mat);
				mesh.position.set(offset, 0, -2);
				this.landGroup.add(mesh);
			}
		}
	}

	private renderGeoJsonFeature(feature: any, color: number) {
		const geometries = feature.geometry.type === 'Polygon'
			? [feature.geometry.coordinates]
			: feature.geometry.coordinates;

		const mat = new THREE.MeshBasicMaterial({ color, side: THREE.DoubleSide });
		const borderMat = new THREE.LineBasicMaterial({ color: 0x000000, opacity: 0.4, transparent: true });

		for (const poly of geometries) {
			const outerRing = this.normalizePolygon(poly[0] as [number, number][]);
			const shape = new THREE.Shape();
			const first = this.latLonToMercator(outerRing[0][1], outerRing[0][0]);
			shape.moveTo(first.x, first.y);

			for (let i = 1; i < outerRing.length; i++) {
				const pt = this.latLonToMercator(outerRing[i][1], outerRing[i][0]);
				shape.lineTo(pt.x, pt.y);
			}

			// Holes
			for (let h = 1; h < poly.length; h++) {
				const holeRing = this.normalizePolygon(poly[h] as [number, number][]);
				const holePath = new THREE.Path();
				const hFirst = this.latLonToMercator(holeRing[0][1], holeRing[0][0]);
				holePath.moveTo(hFirst.x, hFirst.y);
				for (let i = 1; i < holeRing.length; i++) {
					const hPt = this.latLonToMercator(holeRing[i][1], holeRing[i][0]);
					holePath.lineTo(hPt.x, hPt.y);
				}
				shape.holes.push(holePath);
			}

			const geo = new THREE.ShapeGeometry(shape);
			const borderGeo = new THREE.BufferGeometry().setFromPoints(
				outerRing.map(p => {
					const pos = this.latLonToMercator(p[1], p[0]);
					return new THREE.Vector3(pos.x, pos.y, 0.1);
				})
			);

			for (const offset of [-360, 0, 360]) {
				const mesh = new THREE.Mesh(geo, mat);
				mesh.position.set(offset, 0, -2);
				this.landGroup.add(mesh);

				const border = new THREE.Line(borderGeo, borderMat);
				border.position.set(offset, 0, 0);
				this.borderGroup.add(border);
			}
		}
	}

	private buildParcelHitTargets(cells: GridCell[]) {
		const parcels = bridge.getParcelsInView(-180, -90, 180, 90);
		const cellToParcel = new Map<number, number>();
		for (const p of parcels) cellToParcel.set(p.cell_index, p.id);

		// Transparent discs for raycasting hit detection
		const unitDisc = new THREE.CircleGeometry(1, 8);
		const invisibleMat = new THREE.MeshBasicMaterial({ visible: false });

		for (const cell of cells) {
			if (Math.abs(cell.lat) > 85) continue;
			if (cell.terrain === 'OceanDeep' || cell.terrain === 'OceanShallow') continue;

			const mesh = new THREE.Mesh(unitDisc, invisibleMat);
			const pos = this.latLonToMercator(cell.lat, cell.lon);
			mesh.position.set(pos.x, pos.y, 1); // Top for raycasting
			mesh.scale.set(this.baseCellSize, this.baseCellSize, 1);

			const parcelId = cellToParcel.get(cell.index);
			if (parcelId !== undefined) {
				mesh.userData = { parcelId, type: 'parcel' };
			}
			this.landGroup.add(mesh);
		}
	}

	private buildTerrainTintOverlay(cells: GridCell[]) {
		const unitDisc = new THREE.CircleGeometry(1, 12);
		for (const cell of cells) {
			if (cell.terrain === 'Ocean' || cell.terrain === 'OceanDeep') continue;
			if (Math.abs(cell.lat) > 85) continue;
			const tint = TERRAIN_TINT[cell.terrain];
			if (!tint) continue;

			const mat = new THREE.MeshBasicMaterial({
				color: tint.color,
				transparent: true,
				opacity: tint.opacity
			});
			const mesh = new THREE.Mesh(unitDisc, mat);
			const pos = this.latLonToMercator(cell.lat, cell.lon);
			mesh.position.set(pos.x, pos.y, -1.9); // Just above land polygons
			mesh.rotation.x = 0; // Ensure it's flat
			mesh.scale.set(this.baseCellSize * 1.05, this.baseCellSize * 1.05, 1);
			this.landGroup.add(mesh);
		}
	}

	private buildRegionBorders(regions: Region[], cities: City[]) {
		this.borderGroup.clear();
		const lineMat = new THREE.LineBasicMaterial({
			color: 0x000000,
			opacity: 0.4,
			transparent: true
		});

		if (!bridge.isRealEarth()) {
			for (const region of regions) {
				if (!region.boundary_polygon || region.boundary_polygon.length < 3) continue;
				const pts = this.normalizePolygon(region.boundary_polygon.map(p => [p[1], p[0]]));
				const threePts = pts.map(p => {
					const pos = this.latLonToMercator(p[1], p[0]);
					return new THREE.Vector3(pos.x, pos.y, 0.1);
				});
				// Close loop
				const first = this.latLonToMercator(pts[0][1], pts[0][0]);
				threePts.push(new THREE.Vector3(first.x, first.y, 0.1));

				const geo = new THREE.BufferGeometry().setFromPoints(threePts);
				for (const offset of [-360, 0, 360]) {
					const line = new THREE.Line(geo, lineMat);
					line.position.set(offset, 0, 0);
					this.borderGroup.add(line);
				}
			}
		}
	}

	/** Get city tier icon name based on population */
	private getCityTier(pop: number): IconName {
		if (pop >= 5_000_000) return 'megalopolis';
		if (pop >= 1_000_000) return 'metropolis';
		if (pop >= 200_000) return 'city';
		if (pop >= 50_000) return 'town';
		return 'hamlet';
	}

	/** Seeded PRNG for deterministic dot scatter (xorshift32) */
	private dotRng(seed: number): () => number {
		let s = seed | 1;
		return () => {
			s ^= s << 13;
			s ^= s >> 17;
			s ^= s << 5;
			return (s >>> 0) / 4294967296;
		};
	}

	private buildCities(citiesData: City[]) {
		this.cityGroup.clear();
		this.cityGlowGroup.clear();
		this.cityCellSet.clear();

		const cs = this.baseCellSize;

		for (const city of citiesData) {
			if (Math.abs(city.y) > 85) continue;
			const pos = this.latLonToMercator(city.y, city.x);
			const pop = Math.max(city.population, 1);
			const sat = city.infrastructure_satisfaction ?? 0;
			const cellPositions = city.cell_positions ?? [];

			for (const cp of cellPositions) {
				this.cityCellSet.add(cp.index);
			}

			const popScale = Math.log10(Math.max(pop, 100)) / 7;

			// Scattered subtle lights
			const dotCount = Math.min(100, Math.max(4, Math.floor(Math.sqrt(pop) * 0.1)));
			const rand = this.dotRng(city.id * 7919 + 31);
			const dotRadius = cs * 0.012;
			const dotGeo = new THREE.CircleGeometry(dotRadius, 6);

			const cellsToUse = cellPositions.length > 0 ? cellPositions : [{ lat: city.y, lon: city.x }];

			for (let d = 0; d < dotCount; d++) {
				const cp = cellsToUse[Math.floor(rand() * cellsToUse.length)];
				const angle = rand() * Math.PI * 2;
				const dist = rand() * cs * 0.4;
				const mPos = this.latLonToMercator(cp.lat, cp.lon);

				const dx = Math.cos(angle) * dist;
				const dy = Math.sin(angle) * dist;

				const mat = new THREE.MeshBasicMaterial({
					color: 0xfff0d0,
					opacity: 0.4,
					transparent: true
				});
				const dot = new THREE.Mesh(dotGeo, mat);
				dot.position.set(mPos.x + dx, mPos.y + dy, 0.4);
				this.cityGlowGroup.add(dot);
			}

			// Soft diffuse glow
			const glowSize = cs * (0.2 + popScale * 0.4);
			const glowGeo = new THREE.CircleGeometry(glowSize, 16);
			const glowMat = new THREE.MeshBasicMaterial({
				color: 0xffd080,
				opacity: 0.05,
				transparent: true
			});
			const glow = new THREE.Mesh(glowGeo, glowMat);
			glow.position.set(pos.x, pos.y, 0.3);
			this.cityGlowGroup.add(glow);

			// Clickable center marker
			const hitSize = cs * 0.15;
			const hitGeo = new THREE.CircleGeometry(hitSize, 8);
			const hitMat = new THREE.MeshBasicMaterial({ visible: false });
			const hitMesh = new THREE.Mesh(hitGeo, hitMat);
			hitMesh.position.set(pos.x, pos.y, 1);
			hitMesh.userData = { type: 'city', id: city.id, name: city.name };
			this.cityGroup.add(hitMesh);
			this.entityMeshMap.set(city.id, hitMesh);

			// Satisfaction indicator
			const satColor = sat >= 0.7 ? 0x10b981 : sat >= 0.4 ? 0xf59e0b : 0xef4444;
			const satDotGeo = new THREE.CircleGeometry(cs * 0.02, 8);
			const satDotMat = new THREE.MeshBasicMaterial({
				color: satColor,
				opacity: 0.8,
				transparent: true
			});
			const satDot = new THREE.Mesh(satDotGeo, satDotMat);
			satDot.position.set(pos.x, pos.y, 1.5);
			satDot.userData = { labelType: 'cityIndicator', cityId: city.id };
			this.cityGlowGroup.add(satDot);

			// City name label
			const labelScale = cs * 0.035;
			const label = this.createTextSprite(city.name, 0xe8e8e8, labelScale);
			label.position.set(pos.x, pos.y - cs * 0.08, 5);
			label.userData = { labelType: 'cityName', minZoom: 1.5 };
			this.cityGroup.add(label);

			// Population label
			const popStr = pop >= 1_000_000 ? `${(pop / 1_000_000).toFixed(1)}M`
				: pop >= 1_000 ? `${(pop / 1_000).toFixed(0)}K`
					: `${pop}`;
			const popLabel = this.createTextSprite(popStr, 0x888888, labelScale * 0.7);
			popLabel.position.set(pos.x, pos.y - cs * 0.14, 5);
			popLabel.userData = { labelType: 'cityPop', minZoom: 2.5 };
			this.cityGroup.add(popLabel);
		}
	}

	updateCities() {
		if (!bridge.isInitialized()) return;
		const citiesData = bridge.getCities();

		const satMap = new Map<number, number>();
		for (const city of citiesData) {
			satMap.set(city.id, city.infrastructure_satisfaction ?? 0);
		}

		for (const group of [this.cityGroup, this.cityGlowGroup]) {
			for (const child of group.children) {
				if (child.userData?.labelType !== 'cityIndicator') continue;
				const cityId = child.userData.cityId as number;
				const sat = satMap.get(cityId) ?? 0;
				const satColor = sat >= 0.7 ? 0x10b981 : sat >= 0.4 ? 0xf59e0b : 0xef4444;
				const mesh = child as THREE.Mesh;
				(mesh.material as THREE.MeshBasicMaterial).color.setHex(satColor);
			}
		}
	}

	highlightEdgeSource(nodeId: number | null) {
		this.edgePreviewGroup.clear();
		this.currentEdgeSourceId = nodeId;
		if (nodeId === null) {
			this.previewLine = null;
			return;
		}

		const obj = this.entityMeshMap.get(nodeId);
		if (!obj) return;

		const cs = this.baseCellSize;
		const rs = cs * 0.08;
		const ringGeo = new THREE.RingGeometry(rs, rs * 1.25, 16);
		const ringMat = new THREE.MeshBasicMaterial({
			color: 0x3b82f6,
			opacity: 0.85,
			transparent: true,
			side: THREE.DoubleSide
		});
		const ring = new THREE.Mesh(ringGeo, ringMat);
		ring.position.copy(obj.position);
		ring.position.z = 3.5;
		this.edgePreviewGroup.add(ring);

		// Small "SOURCE" label above
		const label = this.createTextSprite('SOURCE', 0x3b82f6, cs * 0.04);
		label.position.set(obj.position.x, obj.position.y + cs * 0.2, 5);
		this.edgePreviewGroup.add(label);
	}

	private buildLabels(_cities: City[], regions: Region[]) {
		this.labelGroup.clear();

		// Region labels — visible at wider zoom
		const regionLabelSize = this.baseCellSize * 0.08;
		for (const region of regions) {
			const pos = this.latLonToMercator(region.center_lat, region.center_lon);
			const sprite = this.createTextSprite(region.name, 0x6b7280, regionLabelSize);
			sprite.position.set(pos.x, pos.y, 5);
			sprite.userData = { labelType: 'region', minZoom: 0.5, maxZoom: 4.0 };
			this.labelGroup.add(sprite);
		}
	}

	private createTextSprite(text: string, color: number, scale: number): THREE.Sprite {
		const canvas = document.createElement('canvas');
		const ctx = canvas.getContext('2d')!;
		canvas.width = 256;
		canvas.height = 64;

		ctx.fillStyle = 'transparent';
		ctx.fillRect(0, 0, canvas.width, canvas.height);

		ctx.font = 'bold 28px system-ui, sans-serif';
		ctx.textAlign = 'center';
		ctx.textBaseline = 'middle';

		// Text shadow for readability
		ctx.fillStyle = 'rgba(0, 0, 0, 0.8)';
		ctx.fillText(text, canvas.width / 2 + 1, canvas.height / 2 + 1);

		const hexColor = '#' + color.toString(16).padStart(6, '0');
		ctx.fillStyle = hexColor;
		ctx.fillText(text, canvas.width / 2, canvas.height / 2);

		const texture = new THREE.CanvasTexture(canvas);
		texture.minFilter = THREE.LinearFilter;
		const spriteMat = new THREE.SpriteMaterial({ map: texture, transparent: true });
		const sprite = new THREE.Sprite(spriteMat);
		// Scale: width=4x, height=1x of the base scale value
		sprite.scale.set(scale * 4, scale * 1, 1);
		return sprite;
	}

	updateInfrastructure() {
		if (!bridge.isInitialized()) return;

		this.infraGroup.clear();
		this.edgeGroup.clear();
		this.ownerGroup.clear();
		this.entityMeshMap.clear();
		this.pathfinder.clearCache();

		// Rebuild city refs
		for (const child of this.cityGroup.children) {
			if (child.userData?.id) {
				this.entityMeshMap.set(child.userData.id, child);
			}
		}

		const corps = bridge.getAllCorporations();
		this.buildCorpColorMap(corps);

		// Ownership node positions per corp for overlay
		const corpPositions = new Map<number, THREE.Vector3[]>();

		for (let i = 0; i < corps.length; i++) {
			const corp = corps[i];
			const color = CORP_COLORS[i % CORP_COLORS.length];
			const infra = bridge.getInfrastructureList(corp.id);

			const cellSize = this.baseCellSize;
			for (const edge of infra.edges) {
				const style: EdgeStyle = EDGE_STYLES[edge.edge_type] ?? {
					color, opacity: 0.7, radiusFactor: 0.008, segments: 4
				};

				let rawWaypoints: [number, number][];
				if (needsTerrainRouting(edge.edge_type) && edge.src_cell !== undefined && edge.dst_cell !== undefined) {
					rawWaypoints = this.pathfinder.findPath(edge.src_cell, edge.dst_cell, edge.edge_type);
					if (rawWaypoints.length < 2) {
						rawWaypoints = [[edge.src_x, edge.src_y], [edge.dst_x, edge.dst_y]];
					}
				} else {
					rawWaypoints = [[edge.src_x, edge.src_y], [edge.dst_x, edge.dst_y]];
				}

				// Normalize waypoints to be contiguous in coordinate space
				const pts = this.normalizePolygon(rawWaypoints);
				const waypoints = pts.map(p => {
					const pos = this.latLonToMercator(p[1], p[0]);
					return new THREE.Vector3(pos.x, pos.y, 1.5);
				});
				// Snap ends
				const srcP = this.latLonToMercator(edge.src_y, edge.src_x);
				const dstP = this.latLonToMercator(edge.dst_y, edge.dst_x);
				waypoints[0].set(srcP.x, srcP.y, 1.5);
				waypoints[waypoints.length - 1].set(dstP.x, dstP.y, 1.5);

				for (const offset of [-360, 0, 360]) {
					if (style.dashed) {
						const geo = new THREE.BufferGeometry().setFromPoints(waypoints);
						const mat = new THREE.LineDashedMaterial({
							color: style.color,
							opacity: style.opacity,
							transparent: true,
							dashSize: style.dashSize ?? 1.0,
							gapSize: style.gapSize ?? 0.5
						});
						const line = new THREE.Line(geo, mat);
						line.position.set(offset, 0, 0);
						line.computeLineDistances();
						line.userData = { type: 'edge', id: edge.id, edge_type: edge.edge_type, corpId: corp.id };
						this.edgeGroup.add(line);
					} else {
						const curve = waypoints.length >= 3
							? new THREE.CatmullRomCurve3(waypoints, false, 'centripetal', 0.5)
							: new THREE.LineCurve3(waypoints[0], waypoints[waypoints.length - 1]);
						const tubeSeg = Math.max(8, waypoints.length * 4);
						const tubeRadius = style.radiusFactor * cellSize;
						const tubeGeo = new THREE.TubeGeometry(curve, tubeSeg, tubeRadius, style.segments, false);
						const mat = new THREE.MeshBasicMaterial({
							color: style.color,
							opacity: style.opacity,
							transparent: true,
							depthTest: false
						});
						const mesh = new THREE.Mesh(tubeGeo, mat);
						mesh.position.set(offset, 0, 0);
						mesh.userData = { type: 'edge', id: edge.id, edge_type: edge.edge_type, corpId: corp.id };
						this.edgeGroup.add(mesh);
					}
				}
			}

			const cs = this.baseCellSize;
			const iconSize = cs * 0.15;
			const positions: THREE.Vector3[] = [];
			const positionCounts = new Map<string, number>();

			for (const node of infra.nodes) {
				const iconName = NODE_TYPE_TO_ICON[node.node_type];
				const texture = iconName ? this.iconTextures.get(iconName) : undefined;
				const pos = this.latLonToMercator(node.y, node.x);

				const posKey = `${pos.x.toFixed(2)},${pos.y.toFixed(2)}`;
				const stackIdx = positionCounts.get(posKey) ?? 0;
				positionCounts.set(posKey, stackIdx + 1);

				const offsetDist = cs * 0.12;
				const angle = (stackIdx * Math.PI * 2) / 3 + Math.PI / 6;
				const baseOx = pos.x + Math.cos(angle) * offsetDist * (stackIdx > 0 ? 1 : 0.3);
				const baseOy = pos.y + Math.sin(angle) * offsetDist * (stackIdx > 0 ? 1 : 0.3);

				for (const offset of [-360, 0, 360]) {
					const ox = baseOx + offset;
					const oy = baseOy;

					let obj: THREE.Object3D;
					if (texture && !node.under_construction) {
						const mat = new THREE.SpriteMaterial({
							map: texture,
							transparent: true,
							depthTest: false,
							color
						});
						const sprite = new THREE.Sprite(mat);
						sprite.scale.set(iconSize, iconSize, 1);
						sprite.position.set(ox, oy, 2);
						sprite.userData = { type: 'node', id: node.id, node_type: node.node_type };
						this.infraGroup.add(sprite);
						obj = sprite;
					} else {
						const size = node.under_construction ? cs * 0.04 : cs * 0.06;
						const nodeColor = node.under_construction ? 0x6b7280 : color;
						const geo = this.getNodeGeometry(node.node_type, size);
						const mat = new THREE.MeshBasicMaterial({ color: nodeColor });
						const mesh = new THREE.Mesh(geo, mat);
						mesh.position.set(ox, oy, 2);
						mesh.userData = { type: 'node', id: node.id, node_type: node.node_type };
						this.infraGroup.add(mesh);
						obj = mesh;
					}
					if (offset === 0) {
						this.entityMeshMap.set(node.id, obj);
					}
					positions.push(new THREE.Vector3(ox, oy, 0));

					if (!node.under_construction && corp.name) {
						const badgeSize = cs * 0.025;
						const badge = this.createTextSprite(corp.name[0], color, badgeSize);
						badge.position.set(ox + iconSize * 0.4, oy + iconSize * 0.4, 4);
						badge.userData = { labelType: 'badge', minZoom: 4.0 };
						this.infraGroup.add(badge);
					}
				}
			}
			if (positions.length > 0) {
				corpPositions.set(corp.id, positions);
			}
		}

		// Build ownership overlay (semi-transparent circles around corp infrastructure clusters)
		this.buildOwnershipOverlay(corpPositions);

		// Re-apply selection highlight if something was selected
		if (this.selectedId !== null) {
			this.applySelectionHighlight(this.selectedId);
		}
	}

	private buildCorpColorMap(corps: CorpSummary[]) {
		this.corpColorMap.clear();
		for (let i = 0; i < corps.length; i++) {
			this.corpColorMap.set(corps[i].id, CORP_COLORS[i % CORP_COLORS.length]);
		}
	}

	private buildOwnershipOverlay(corpPositions: Map<number, THREE.Vector3[]>) {
		this.ownerGroup.clear();
		const radius = this.baseCellSize * 0.18;

		for (const [corpId, positions] of corpPositions) {
			const color = this.corpColorMap.get(corpId) ?? 0x888888;

			for (const pos of positions) {
				const geo = new THREE.CircleGeometry(radius, 10);
				const mat = new THREE.MeshBasicMaterial({
					color,
					opacity: 0.08,
					transparent: true
				});
				const mesh = new THREE.Mesh(geo, mat);
				mesh.position.set(pos.x, pos.y, 0.3);
				this.ownerGroup.add(mesh);
			}
		}
	}

	private getNodeGeometry(nodeType: string, size: number): THREE.BufferGeometry {
		switch (nodeType) {
			case 'CentralOffice':
				// Square
				return new THREE.PlaneGeometry(size * 1.6, size * 1.6);
			case 'CellTower':
				// Triangle
				return new THREE.CircleGeometry(size, 3);
			case 'DataCenter':
				// Pentagon
				return new THREE.CircleGeometry(size * 1.1, 5);
			case 'ExchangePoint':
				// Hexagon
				return new THREE.CircleGeometry(size, 6);
			case 'SatelliteGround': {
				// Star (5-pointed via custom shape)
				const shape = new THREE.Shape();
				for (let j = 0; j < 5; j++) {
					const outerAngle = (j * 2 * Math.PI) / 5 - Math.PI / 2;
					const innerAngle = outerAngle + Math.PI / 5;
					const ox = Math.cos(outerAngle) * size;
					const oy = Math.sin(outerAngle) * size;
					const ix = Math.cos(innerAngle) * size * 0.45;
					const iy = Math.sin(innerAngle) * size * 0.45;
					if (j === 0) shape.moveTo(ox, oy);
					else shape.lineTo(ox, oy);
					shape.lineTo(ix, iy);
				}
				shape.closePath();
				return new THREE.ShapeGeometry(shape);
			}
			case 'SubmarineLanding': {
				// Diamond (rotated square)
				const s = size * 1.2;
				const dShape = new THREE.Shape();
				dShape.moveTo(0, s);
				dShape.lineTo(s * 0.7, 0);
				dShape.lineTo(0, -s);
				dShape.lineTo(-s * 0.7, 0);
				dShape.closePath();
				return new THREE.ShapeGeometry(dShape);
			}
			case 'WirelessRelay':
				// Small circle with more segments
				return new THREE.CircleGeometry(size * 0.8, 12);
			default:
				return new THREE.CircleGeometry(size, 8);
		}
	}

	setSelected(id: number | null) {
		this.selectedId = id;
		this.selectionGroup.clear();
		if (id !== null) {
			this.applySelectionHighlight(id);
		}
	}

	private applySelectionHighlight(id: number) {
		this.selectionGroup.clear();
		const obj = this.entityMeshMap.get(id);
		if (!obj) return;

		const cs = this.baseCellSize;
		const inner = cs * 0.08;
		const mid = cs * 0.1;
		const outer = cs * 0.14;

		const ringGeo = new THREE.RingGeometry(inner, mid, 16);
		const ringMat = new THREE.MeshBasicMaterial({
			color: 0x10b981,
			opacity: 0.8,
			transparent: true,
			side: THREE.DoubleSide
		});
		const ring = new THREE.Mesh(ringGeo, ringMat);
		ring.position.copy(obj.position);
		ring.position.z = 3;
		this.selectionGroup.add(ring);

		const outerGeo = new THREE.RingGeometry(mid, outer, 16);
		const outerMat = new THREE.MeshBasicMaterial({
			color: 0x10b981,
			opacity: 0.3,
			transparent: true,
			side: THREE.DoubleSide
		});
		const outerMesh = new THREE.Mesh(outerGeo, outerMat);
		outerMesh.position.copy(obj.position);
		outerMesh.position.z = 3;
		this.selectionGroup.add(outerMesh);
	}

	private setupControls() {
		const el = this.renderer.domElement;

		el.addEventListener('mousedown', (e) => {
			this.isDragging = true;
			this.dragMoved = false;
			this.lastMouse = { x: e.clientX, y: e.clientY };
		});

		el.addEventListener('mousemove', (e) => {
			if (this.isDragging) {
				const dx = e.clientX - this.lastMouse.x;
				const dy = e.clientY - this.lastMouse.y;
				if (Math.abs(dx) > 2 || Math.abs(dy) > 2) {
					this.dragMoved = true;
				}
				this.panX -= (dx / this.zoom) * 0.5;
				this.panY += (dy / this.zoom) * 0.5;

				// Clamp pan (Mercator world is approx 360x360)
				this.panX = Math.max(-150, Math.min(150, this.panX));
				this.panY = Math.max(-130, Math.min(130, this.panY));

				this.lastMouse = { x: e.clientX, y: e.clientY };
				this.updateCamera();
			}
		});

		el.addEventListener('mouseup', () => {
			this.isDragging = false;
		});

		el.addEventListener('wheel', (e) => {
			e.preventDefault();
			const factor = e.deltaY > 0 ? 0.9 : 1.1;
			// Tighten zoom (0.8 prevents excessive world duplication)
			this.zoom = Math.max(0.8, Math.min(50, this.zoom * factor));
			this.updateCamera();
			this.updateLabelVisibility();
		});

		el.addEventListener('click', (e) => {
			if (this.dragMoved) return;
			const rect = el.getBoundingClientRect();
			this.pointer.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
			this.pointer.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
			this.raycaster.setFromCamera(this.pointer, this.camera);

			// Check infrastructure/city clicks first
			const entityIntersects = this.raycaster.intersectObjects(
				[...this.infraGroup.children, ...this.cityGroup.children],
				false
			);
			if (entityIntersects.length > 0) {
				const obj = entityIntersects[0].object;
				if (obj.userData?.id) {
					this.setSelected(obj.userData.id);
					window.dispatchEvent(
						new CustomEvent('entity-selected', {
							detail: { id: obj.userData.id, type: obj.userData.type }
						})
					);
					return;
				}
			}

			// Check land parcel clicks (for build mode)
			const landIntersects = this.raycaster.intersectObjects(
				this.landGroup.children,
				false
			);
			if (landIntersects.length > 0) {
				const obj = landIntersects[0].object;
				if (obj.userData?.parcelId) {
					window.dispatchEvent(
						new CustomEvent('parcel-clicked', {
							detail: { id: obj.userData.parcelId, x: obj.position.x, y: obj.position.y }
						})
					);
					return;
				}
			}

			// Click on empty space — deselect
			this.setSelected(null);
			window.dispatchEvent(
				new CustomEvent('entity-selected', {
					detail: { id: null, type: null }
				})
			);
		});
	}

	private updateLabelVisibility() {
		// Update visibility for all groups that contain labels
		for (const group of [this.labelGroup, this.cityGroup, this.infraGroup]) {
			for (const child of group.children) {
				const minZoom = child.userData?.minZoom;
				const maxZoom = child.userData?.maxZoom;
				if (minZoom !== undefined || maxZoom !== undefined) {
					const min = minZoom ?? 0;
					const max = maxZoom ?? Infinity;
					child.visible = this.zoom >= min && this.zoom <= max;
				}
			}
		}
	}

	private updateCamera() {
		const w = this.container.clientWidth;
		const h = this.container.clientHeight;
		const aspect = w / h;
		const viewSize = 200 / this.zoom;

		this.camera.left = -viewSize * aspect + this.panX;
		this.camera.right = viewSize * aspect + this.panX;
		this.camera.top = viewSize + this.panY;
		this.camera.bottom = -viewSize + this.panY;
		this.camera.updateProjectionMatrix();
	}

	private setupResize() {
		const observer = new ResizeObserver(() => {
			const w = this.container.clientWidth;
			const h = this.container.clientHeight;
			this.renderer.setSize(w, h);
			this.updateCamera();
		});
		observer.observe(this.container);
	}

	setOverlay(overlay: string) {
		this.activeOverlay = overlay;
		this.overlayGroup.clear();

		// Toggle ownership overlay visibility based on overlay type
		this.ownerGroup.visible = overlay === 'none' || overlay === 'ownership';

		if (overlay === 'none') return;

		switch (overlay) {
			case 'terrain':
				this.renderTerrainOverlay();
				break;
			case 'ownership':
				this.renderOwnershipOverlay();
				break;
			case 'demand':
				this.renderDemandOverlay();
				break;
			case 'coverage':
				this.renderCoverageOverlay();
				break;
			case 'disaster':
				this.renderDisasterRiskOverlay();
				break;
			case 'congestion':
				this.renderCongestionOverlay();
				break;
			case 'traffic':
				this.renderTrafficOverlay();
				break;
		}
	}

	private renderTerrainOverlay() {
		// Use a tiled grid look instead of individual discs
		const terrainOverlayColors: Record<string, number> = {
			Urban: 0xfbbf24,
			Suburban: 0x8bc34a,
			Rural: 0x4caf50,
			Mountainous: 0x9e9e9e,
			Desert: 0xff9800,
			Coastal: 0x00bcd4,
			OceanShallow: 0x0288d1,
			OceanDeep: 0x01579b,
			Tundra: 0xb0bec5,
			Frozen: 0xe3f2fd
		};

		// Use PlaneGeometry for a crisp square/tile look
		// Scaling slightly larger (1.05) to ensure gapless grid
		const r = this.baseCellSize * 1.05;
		const tileGeo = new THREE.PlaneGeometry(r, r);

		for (const cell of this.cachedCells) {
			const color = terrainOverlayColors[cell.terrain] ?? 0xcccccc;
			if (Math.abs(cell.lat) > 80) continue; // Clip polar extremes

			const pos = this.latLonToMercator(cell.lat, cell.lon);
			const mat = new THREE.MeshBasicMaterial({
				color,
				opacity: 0.25, // Subtle but clearly visible background
				transparent: true
			});

			for (const offset of [-360, 0, 360]) {
				const mesh = new THREE.Mesh(tileGeo, mat);
				mesh.position.set(pos.x + offset, pos.y, 0.2);
				this.overlayGroup.add(mesh);
			}
		}
	}

	private renderOwnershipOverlay() {
		// Professional soft aura for ownership clusters
		if (!bridge.isInitialized()) return;

		const corps = bridge.getAllCorporations();
		this.buildCorpColorMap(corps);

		for (let i = 0; i < corps.length; i++) {
			const corp = corps[i];
			const color = CORP_COLORS[i % CORP_COLORS.length];
			const infra = bridge.getInfrastructureList(corp.id);

			for (const node of infra.nodes) {
				const pos = this.latLonToMercator(node.y, node.x);
				// Use a soft aura that feels integrated into the map
				const geo = new THREE.CircleGeometry(this.baseCellSize * 0.6, 16);
				const mat = new THREE.MeshBasicMaterial({
					color,
					opacity: 0.1, // Very subtle aura
					transparent: true
				});

				for (const offset of [-360, 0, 360]) {
					const mesh = new THREE.Mesh(geo, mat);
					mesh.position.set(pos.x + offset, pos.y, 0.15);
					this.overlayGroup.add(mesh);
				}
			}
		}
	}

	private renderDemandOverlay() {
		// Color regions by population density with a professional soft glow
		for (const region of this.cachedRegions) {
			const pop = region.population ?? 0;
			// Normalize: low pop = blue, high pop = red
			const intensity = Math.min(1.0, pop / 500000);
			const r = Math.floor(intensity * 255);
			const b = Math.floor((1 - intensity) * 255);
			const color = (r << 16) | (50 << 8) | b;

			const pos = this.latLonToMercator(region.center_lat, region.center_lon);
			// Use a very soft, large blurred disc for a 'heat' feel rather than a hard circle
			const radius = Math.sqrt(region.cell_count) * this.baseCellSize * 0.4;
			const geo = new THREE.CircleGeometry(radius, 32);
			const mat = new THREE.MeshBasicMaterial({
				color,
				opacity: 0.12,
				transparent: true
			});

			for (const offset of [-360, 0, 360]) {
				const mesh = new THREE.Mesh(geo, mat);
				mesh.position.set(pos.x + offset, pos.y, 0.18);
				this.overlayGroup.add(mesh);
			}
		}
	}

	private renderCoverageOverlay() {
		// Show real per-cell coverage data as a tiled heatmap
		if (!bridge.isInitialized()) return;

		const coverageData = bridge.getCellCoverage();
		if (coverageData.length === 0) return;

		// Find max signal for normalization
		let maxSignal = 0;
		for (const cov of coverageData) {
			if (cov.signal_strength > maxSignal) maxSignal = cov.signal_strength;
		}
		if (maxSignal <= 0) return;

		// Build corp color map for dominant owner coloring
		const corps = bridge.getAllCorporations();
		this.buildCorpColorMap(corps);

		// Use gapless tiled planes
		const r = this.baseCellSize * 1.05;
		const tileGeo = new THREE.PlaneGeometry(r, r);

		for (const cov of coverageData) {
			const intensity = Math.min(1.0, cov.signal_strength / maxSignal);
			if (intensity < 0.01) continue;
			if (Math.abs(cov.lat) > 80) continue;

			// Color by dominant owner, fall back to green
			let color: number;
			if (cov.dominant_owner !== null) {
				color = this.corpColorMap.get(cov.dominant_owner) ?? 0x10b981;
			} else {
				color = 0x10b981;
			}

			const pos = this.latLonToMercator(cov.lat, cov.lon);
			const mat = new THREE.MeshBasicMaterial({
				color,
				opacity: 0.15 + intensity * 0.35,
				transparent: true
			});

			for (const offset of [-360, 0, 360]) {
				const mesh = new THREE.Mesh(tileGeo, mat);
				mesh.position.set(pos.x + offset, pos.y, 0.22); // Slightly above terrain
				this.overlayGroup.add(mesh);
			}
		}
	}

	private renderDisasterRiskOverlay() {
		// Risk heatmap with region-aware sizing
		for (const region of this.cachedRegions) {
			const risk = region.disaster_risk ?? 0;
			const intensity = Math.min(1.0, risk * 5); // Scale up for visibility
			const r = Math.floor(intensity * 255);
			const g = Math.floor((1 - intensity) * 180);
			const color = (r << 16) | (g << 8) | 0;

			const pos = this.latLonToMercator(region.center_lat, region.center_lon);
			const radius = Math.sqrt(region.cell_count) * this.baseCellSize * 0.4;
			const geo = new THREE.CircleGeometry(radius, 32);
			const mat = new THREE.MeshBasicMaterial({
				color,
				opacity: 0.15,
				transparent: true
			});

			for (const offset of [-360, 0, 360]) {
				const mesh = new THREE.Mesh(geo, mat);
				mesh.position.set(pos.x + offset, pos.y, 0.18);
				this.overlayGroup.add(mesh);
			}
		}
	}

	private renderCongestionOverlay() {
		// Professional node status indicators (congestion)
		if (!bridge.isInitialized()) return;

		const corps = bridge.getAllCorporations();
		for (const corp of corps) {
			const infra = bridge.getInfrastructureList(corp.id);

			for (const node of infra.nodes) {
				if (node.under_construction) continue;
				const util = node.utilization;
				const r = Math.floor(Math.min(1, util * 2) * 255);
				const g = Math.floor(Math.max(0, 1 - util) * 200);
				const color = (r << 16) | (g << 8) | 0;

				const pos = this.latLonToMercator(node.y, node.x);
				// Smaller, crisp indicator rings for congestion
				const innerR = this.baseCellSize * 0.12;
				const outerR = this.baseCellSize * (0.12 + util * 0.18);
				const geo = new THREE.RingGeometry(innerR, outerR, 24);
				const mat = new THREE.MeshBasicMaterial({
					color,
					opacity: 0.3,
					transparent: true,
					side: THREE.DoubleSide
				});

				for (const offset of [-360, 0, 360]) {
					const mesh = new THREE.Mesh(geo, mat);
					mesh.position.set(pos.x + offset, pos.y, 0.22);
					this.overlayGroup.add(mesh);
				}
			}
		}
	}

	private renderTrafficOverlay() {
		if (!bridge.isInitialized()) return;

		const flows: TrafficFlows = bridge.getTrafficFlows();

		// Draw edges colored by utilization with subtle, thin tubes
		for (const ef of flows.edge_flows) {
			if (ef.traffic <= 0 && ef.utilization <= 0) continue;

			const util = ef.utilization;
			let color: number;
			let opacity: number;

			if (util > 1.0) {
				color = 0xff2222;
				opacity = 0.6;
			} else if (util > 0.8) {
				const t = (util - 0.8) / 0.2;
				color = (255 << 16) | (Math.floor((1 - t) * 80) << 8) | 0;
				opacity = 0.5;
			} else if (util > 0.5) {
				const t = (util - 0.5) / 0.3;
				color = (Math.floor(200 + t * 55) << 16) | (Math.floor(200 - t * 120) << 8) | 0;
				opacity = 0.35;
			} else {
				const t = util / 0.5;
				color = (Math.floor(t * 100) << 16) | (Math.floor(150 + t * 50) << 8) | 0x20;
				opacity = 0.25;
			}

			// Subtler thickness
			const thickness = this.baseCellSize * (0.005 + Math.min(util, 1.5) * 0.012);

			const srcPos = this.latLonToMercator(ef.src_y, ef.src_x);
			const dstPos = this.latLonToMercator(ef.dst_y, ef.dst_x);

			const mat = new THREE.MeshBasicMaterial({
				color,
				opacity,
				transparent: true,
				depthTest: false
			});

			for (const offset of [-360, 0, 360]) {
				const pts = [
					new THREE.Vector3(srcPos.x + offset, srcPos.y, 2.0),
					new THREE.Vector3(dstPos.x + offset, dstPos.y, 2.0)
				];
				const curve = new THREE.LineCurve3(pts[0], pts[1]);
				const tubeGeo = new THREE.TubeGeometry(curve, 4, thickness, 3, false); // Fewer segments for performance/look
				const mesh = new THREE.Mesh(tubeGeo, mat);
				this.overlayGroup.add(mesh);
			}
		}

		// Draw nodes with subtle status glows
		for (const nf of flows.node_flows) {
			if (nf.traffic <= 0) continue;

			const util = nf.utilization;
			const r = Math.floor(Math.min(1, util * 2) * 255);
			const g = Math.floor(Math.max(0, 1 - util) * 200);
			const color = (r << 16) | (g << 8) | 0;

			const pos = this.latLonToMercator(nf.y, nf.x);
			const radius = this.baseCellSize * (0.1 + Math.min(util, 1.0) * 0.15);
			const geo = new THREE.CircleGeometry(radius, 12);
			const mat = new THREE.MeshBasicMaterial({
				color,
				opacity: 0.25,
				transparent: true,
				depthTest: false
			});

			for (const offset of [-360, 0, 360]) {
				const mesh = new THREE.Mesh(geo, mat);
				mesh.position.set(pos.x + offset, pos.y, 2.0);
				this.overlayGroup.add(mesh);
			}
		}
	}

	getRendererInfo(): { calls: number; triangles: number } {
		const info = this.renderer.info.render;
		return { calls: info.calls, triangles: info.triangles };
	}

	render() {
		this.renderer.render(this.scene, this.camera);
	}

	dispose() {
		this.renderer.dispose();
		this.renderer.domElement.remove();
	}

	handleMouseMove(e: MouseEvent) {
		if (this.currentEdgeSourceId === null) return;

		const rect = this.renderer.domElement.getBoundingClientRect();
		this.pointer.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
		this.pointer.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
		this.raycaster.setFromCamera(this.pointer, this.camera);

		// Project ray onto Z=0 plane (map surface)
		const plane = new THREE.Plane(new THREE.Vector3(0, 0, 1), 0);
		const target = new THREE.Vector3();
		this.raycaster.ray.intersectPlane(plane, target);

		const sourceObj = this.entityMeshMap.get(this.currentEdgeSourceId);
		if (!sourceObj) return;

		// Create or update preview line
		if (!this.previewLine) {
			const geo = new THREE.BufferGeometry().setFromPoints([
				sourceObj.position,
				target
			]);
			const mat = new THREE.LineDashedMaterial({
				color: 0x3b82f6,
				dashSize: 2,
				gapSize: 1,
				transparent: true,
				opacity: 0.6
			});
			this.previewLine = new THREE.Line(geo, mat);
			this.previewLine.renderOrder = 10;
			this.edgePreviewGroup.add(this.previewLine);
		} else {
			const pts = [sourceObj.position, target];
			this.previewLine.geometry.setFromPoints(pts);
			this.previewLine.computeLineDistances();
		}
	}
}
