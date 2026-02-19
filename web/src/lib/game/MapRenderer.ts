import * as THREE from 'three';
import * as bridge from '$lib/wasm/bridge';
import type { GridCell, City, Region, CorpSummary, CellCoverage } from '$lib/wasm/types';
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
	Ocean: 0x0a1a3a
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
	WirelessRelay: 'wireless-relay'
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
		// Large plane covering the entire coordinate space
		const geo = new THREE.PlaneGeometry(800, 400);
		const mat = new THREE.MeshBasicMaterial({ color: 0x0a1a3a });
		const plane = new THREE.Mesh(geo, mat);
		plane.position.set(0, 0, -1);
		this.oceanGroup.add(plane);
	}

	private buildLand(cells: GridCell[]) {
		this.landGroup.clear();

		// Get parcels to map cell_index → parcel_id
		const parcels = bridge.getParcelsInView(-180, -90, 180, 90);
		const cellToParcel = new Map<number, number>();
		for (const p of parcels) {
			cellToParcel.set(p.cell_index, p.id);
		}

		for (const cell of cells) {
			const color = TERRAIN_COLORS[cell.terrain] || TERRAIN_COLORS.Ocean;
			const geo = new THREE.CircleGeometry(this.baseCellSize, 6);
			const mat = new THREE.MeshBasicMaterial({ color });
			const mesh = new THREE.Mesh(geo, mat);
			mesh.position.set(cell.lon, cell.lat, 0);

			// Latitude correction: stretch horizontally to compensate for longitude convergence at poles
			const latRad = (cell.lat * Math.PI) / 180;
			const cosLat = Math.max(0.3, Math.cos(latRad));
			mesh.scale.x = 1 / cosLat;

			const parcelId = cellToParcel.get(cell.index);
			if (parcelId !== undefined) {
				mesh.userData = { parcelId, type: 'parcel' };
			}
			this.landGroup.add(mesh);
		}
	}

	private buildRegionBorders(regions: Region[], cities: City[]) {
		this.borderGroup.clear();

		const lineMat = new THREE.LineBasicMaterial({
			color: 0x374151,
			opacity: 0.35,
			transparent: true
		});

		// Build city lookup by ID
		const cityById = new Map<number, City>();
		for (const c of cities) cityById.set(c.id, c);

		for (const region of regions) {
			// Gather city positions for this region
			const pts: { x: number; y: number }[] = [];
			for (const cid of region.city_ids) {
				const city = cityById.get(cid);
				if (city) pts.push({ x: city.x, y: city.y });
			}

			// Also include region center as a point
			pts.push({ x: region.center_lon, y: region.center_lat });

			if (pts.length < 3) {
				// Fallback: small circle for tiny regions
				const radius = Math.max(2, Math.sqrt(region.cell_count) * 0.4);
				const segments = 24;
				const circPts: THREE.Vector3[] = [];
				for (let i = 0; i <= segments; i++) {
					const theta = (i / segments) * Math.PI * 2;
					circPts.push(new THREE.Vector3(
						region.center_lon + Math.cos(theta) * radius,
						region.center_lat + Math.sin(theta) * radius,
						0.5
					));
				}
				const geo = new THREE.BufferGeometry().setFromPoints(circPts);
				this.borderGroup.add(new THREE.Line(geo, lineMat));
				continue;
			}

			// Graham scan convex hull
			const hull = this.convexHull(pts);

			// Expand hull outward by padding
			const padding = 1.5;
			const cx = hull.reduce((s, p) => s + p.x, 0) / hull.length;
			const cy = hull.reduce((s, p) => s + p.y, 0) / hull.length;
			const expanded = hull.map(p => {
				const dx = p.x - cx;
				const dy = p.y - cy;
				const dist = Math.sqrt(dx * dx + dy * dy) || 1;
				return { x: p.x + (dx / dist) * padding, y: p.y + (dy / dist) * padding };
			});

			// Close the polyline
			const hullPts = [...expanded, expanded[0]].map(
				p => new THREE.Vector3(p.x, p.y, 0.5)
			);
			const geo = new THREE.BufferGeometry().setFromPoints(hullPts);
			this.borderGroup.add(new THREE.Line(geo, lineMat));
		}
	}

	/** Graham scan convex hull. Returns points in CCW order. */
	private convexHull(pts: { x: number; y: number }[]): { x: number; y: number }[] {
		const sorted = [...pts].sort((a, b) => a.x - b.x || a.y - b.y);
		if (sorted.length <= 2) return sorted;

		const cross = (o: { x: number; y: number }, a: { x: number; y: number }, b: { x: number; y: number }) =>
			(a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);

		const lower: { x: number; y: number }[] = [];
		for (const p of sorted) {
			while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], p) <= 0)
				lower.pop();
			lower.push(p);
		}

		const upper: { x: number; y: number }[] = [];
		for (let i = sorted.length - 1; i >= 0; i--) {
			const p = sorted[i];
			while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], p) <= 0)
				upper.pop();
			upper.push(p);
		}

		// Remove last point of each half because it's repeated
		lower.pop();
		upper.pop();
		return lower.concat(upper);
	}

	/** Get city tier icon name based on population */
	private getCityTier(pop: number): IconName {
		if (pop >= 5_000_000) return 'megalopolis';
		if (pop >= 1_000_000) return 'metropolis';
		if (pop >= 200_000) return 'city';
		if (pop >= 50_000) return 'town';
		return 'hamlet';
	}

	private buildCities(citiesData: City[]) {
		this.cityGroup.clear();
		this.cityGlowGroup.clear();
		this.cityCellSet.clear();

		const cs = this.baseCellSize;

		for (const city of citiesData) {
			const pop = Math.max(city.population, 1);
			const sat = city.infrastructure_satisfaction ?? 0;

			// Track city cells
			for (const cp of (city.cell_positions ?? [])) {
				this.cityCellSet.add(cp.index);
			}

			// City icon sprite — sized by population
			const tierName = this.getCityTier(pop);
			const tierTexture = this.cityTextures.get(tierName);
			const popScale = Math.log10(Math.max(pop, 100)) / 7; // ~0.28 to ~1.0
			const iconSize = cs * (0.15 + popScale * 0.35);

			if (tierTexture) {
				const spriteMat = new THREE.SpriteMaterial({
					map: tierTexture,
					transparent: true,
					depthTest: false,
					opacity: 0.9
				});
				const sprite = new THREE.Sprite(spriteMat);
				sprite.scale.set(iconSize, iconSize, 1);
				sprite.position.set(city.x, city.y, 1);
				sprite.userData = { type: 'city', id: city.id, name: city.name };
				this.cityGroup.add(sprite);
				this.entityMeshMap.set(city.id, sprite);
			} else {
				// Fallback: warm dot
				const dotSize = cs * 0.03 + popScale * cs * 0.05;
				const dotGeo = new THREE.CircleGeometry(dotSize, 8);
				const dotMat = new THREE.MeshBasicMaterial({ color: 0xffe8c0 });
				const dot = new THREE.Mesh(dotGeo, dotMat);
				dot.position.set(city.x, city.y, 1);
				dot.userData = { type: 'city', id: city.id, name: city.name };
				this.cityGroup.add(dot);
				this.entityMeshMap.set(city.id, dot);
			}

			// Subtle warm glow behind city (night-earth city lights)
			const glowSize = iconSize * 1.0;
			const glowGeo = new THREE.CircleGeometry(glowSize, 10);
			const glowMat = new THREE.MeshBasicMaterial({
				color: 0xffe0a0,
				opacity: 0.06 + popScale * 0.04,
				transparent: true
			});
			const glow = new THREE.Mesh(glowGeo, glowMat);
			glow.position.set(city.x, city.y, 0.5);
			this.cityGlowGroup.add(glow);

			// Satisfaction ring — thin colored ring around city
			const satColor = sat >= 0.7 ? 0x10b981 : sat >= 0.4 ? 0xf59e0b : 0xef4444;
			const ri = iconSize * 0.5;
			const ro = iconSize * 0.58;
			const ringGeo = new THREE.RingGeometry(ri, ro, 16);
			const ringMat = new THREE.MeshBasicMaterial({
				color: satColor,
				opacity: 0.45,
				transparent: true,
				side: THREE.DoubleSide
			});
			const ring = new THREE.Mesh(ringGeo, ringMat);
			ring.position.set(city.x, city.y, 1.5);
			ring.userData = { labelType: 'cityIndicator', cityId: city.id };
			this.cityGlowGroup.add(ring);

			// City name label
			const labelScale = cs * 0.035;
			const label = this.createTextSprite(city.name, 0xe8e8e8, labelScale);
			label.position.set(city.x, city.y - iconSize * 0.55, 5);
			label.userData = { labelType: 'cityName', minZoom: 1.5 };
			this.cityGroup.add(label);

			// Population label
			const popStr = pop >= 1_000_000 ? `${(pop / 1_000_000).toFixed(1)}M`
				: pop >= 1_000 ? `${(pop / 1_000).toFixed(0)}K`
					: `${pop}`;
			const popLabel = this.createTextSprite(popStr, 0x888888, labelScale * 0.7);
			popLabel.position.set(city.x, city.y - iconSize * 0.8, 5);
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

		// City labels are now rendered in buildCities() with multi-cell fills

		// Region labels — visible at wider zoom
		const regionLabelSize = this.baseCellSize * 0.08;
		for (const region of regions) {
			const sprite = this.createTextSprite(region.name, 0x6b7280, regionLabelSize);
			sprite.position.set(region.center_lon, region.center_lat, 5);
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

			// Draw edges as terrain-aware spline tubes
			const cellSize = this.baseCellSize;
			for (const edge of infra.edges) {
				const style: EdgeStyle = EDGE_STYLES[edge.edge_type] ?? {
					color, opacity: 0.7, radiusFactor: 0.008, segments: 4
				};

				// Compute routed waypoints through terrain cells
				let waypoints: THREE.Vector3[];
				if (needsTerrainRouting(edge.edge_type) && edge.src_cell !== undefined && edge.dst_cell !== undefined) {
					const pathPts = this.pathfinder.findPath(edge.src_cell, edge.dst_cell, edge.edge_type);
					if (pathPts.length >= 2) {
						waypoints = pathPts.map(([lon, lat]) => new THREE.Vector3(lon, lat, 1.5));
						waypoints[0].set(edge.src_x, edge.src_y, 1.5);
						waypoints[waypoints.length - 1].set(edge.dst_x, edge.dst_y, 1.5);
					} else {
						waypoints = [
							new THREE.Vector3(edge.src_x, edge.src_y, 1.5),
							new THREE.Vector3(edge.dst_x, edge.dst_y, 1.5)
						];
					}
				} else {
					waypoints = [
						new THREE.Vector3(edge.src_x, edge.src_y, 1.5),
						new THREE.Vector3(edge.dst_x, edge.dst_y, 1.5)
					];
				}

				if (style.dashed) {
					// Wireless edges: dashed line (no tube — these are radio/satellite links)
					const geo = new THREE.BufferGeometry().setFromPoints(waypoints);
					const mat = new THREE.LineDashedMaterial({
						color: style.color,
						opacity: style.opacity,
						transparent: true,
						dashSize: style.dashSize ?? 1.0,
						gapSize: style.gapSize ?? 0.5
					});
					const line = new THREE.Line(geo, mat);
					line.computeLineDistances();
					line.userData = { type: 'edge', id: edge.id, edge_type: edge.edge_type };
					this.edgeGroup.add(line);
				} else {
					// Wired edges: smooth tube following terrain
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
					mesh.userData = { type: 'edge', id: edge.id, edge_type: edge.edge_type };
					this.edgeGroup.add(mesh);
				}
			}

			// Draw nodes with SVG icon sprites (fallback to geometry)
			const cs = this.baseCellSize;
			const iconSize = cs * 0.15;  // Small icons proportional to hex cells
			const positions: THREE.Vector3[] = [];

			// Track how many nodes are stacked at same position for offset
			const positionCounts = new Map<string, number>();

			for (const node of infra.nodes) {
				const iconName = NODE_TYPE_TO_ICON[node.node_type];
				const texture = iconName ? this.iconTextures.get(iconName) : undefined;

				// Offset nodes slightly from exact cell center so they don't cover city dots
				// Multiple nodes at same position fan out in a circle
				const posKey = `${node.x.toFixed(2)},${node.y.toFixed(2)}`;
				const stackIdx = positionCounts.get(posKey) ?? 0;
				positionCounts.set(posKey, stackIdx + 1);

				const offsetDist = cs * 0.12;
				const angle = (stackIdx * Math.PI * 2) / 3 + Math.PI / 6;
				const ox = node.x + Math.cos(angle) * offsetDist * (stackIdx > 0 ? 1 : 0.3);
				const oy = node.y + Math.sin(angle) * offsetDist * (stackIdx > 0 ? 1 : 0.3);

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
				this.entityMeshMap.set(node.id, obj);
				positions.push(new THREE.Vector3(ox, oy, 0));

				// Company badge — tiny letter near node (only visible when zoomed in)
				if (!node.under_construction && corp.name) {
					const badgeSize = cs * 0.025;
					const badge = this.createTextSprite(corp.name[0], color, badgeSize);
					badge.position.set(ox + iconSize * 0.4, oy + iconSize * 0.4, 4);
					badge.userData = { labelType: 'badge', minZoom: 4.0 };
					this.infraGroup.add(badge);
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
			this.zoom = Math.max(0.1, Math.min(50, this.zoom * factor));
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
		}
	}

	private renderTerrainOverlay() {
		// Enhance terrain visibility with stronger colors
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

		for (const cell of this.cachedCells) {
			const color = terrainOverlayColors[cell.terrain];
			if (!color) continue;
			const geo = new THREE.CircleGeometry(this.baseCellSize, 6);
			const mat = new THREE.MeshBasicMaterial({
				color,
				opacity: 0.35,
				transparent: true
			});
			const mesh = new THREE.Mesh(geo, mat);
			mesh.position.set(cell.lon, cell.lat, 0.2);

			// Apply same latitude correction as buildLand
			const latRad = (cell.lat * Math.PI) / 180;
			const cosLat = Math.max(0.3, Math.cos(latRad));
			mesh.scale.x = 1 / cosLat;

			this.overlayGroup.add(mesh);
		}
	}

	private renderOwnershipOverlay() {
		// Make ownership circles larger and more opaque
		if (!bridge.isInitialized()) return;

		const corps = bridge.getAllCorporations();
		this.buildCorpColorMap(corps);

		for (let i = 0; i < corps.length; i++) {
			const corp = corps[i];
			const color = CORP_COLORS[i % CORP_COLORS.length];
			const infra = bridge.getInfrastructureList(corp.id);

			for (const node of infra.nodes) {
				const geo = new THREE.CircleGeometry(this.baseCellSize * 0.3, 16);
				const mat = new THREE.MeshBasicMaterial({
					color,
					opacity: 0.15,
					transparent: true
				});
				const mesh = new THREE.Mesh(geo, mat);
				mesh.position.set(node.x, node.y, 0.25);
				this.overlayGroup.add(mesh);
			}
		}
	}

	private renderDemandOverlay() {
		// Color regions by population density (demand proxy)
		for (const region of this.cachedRegions) {
			const pop = region.population ?? 0;
			// Normalize: low pop = blue, high pop = red
			const intensity = Math.min(1.0, pop / 500000);
			const r = Math.floor(intensity * 255);
			const b = Math.floor((1 - intensity) * 255);
			const color = (r << 16) | (50 << 8) | b;

			const radius = Math.sqrt(region.cell_count) * this.baseCellSize * 0.15;
			const geo = new THREE.CircleGeometry(radius, 24);
			const mat = new THREE.MeshBasicMaterial({
				color,
				opacity: 0.2,
				transparent: true
			});
			const mesh = new THREE.Mesh(geo, mat);
			mesh.position.set(region.center_lon, region.center_lat, 0.2);
			this.overlayGroup.add(mesh);
		}
	}

	private renderCoverageOverlay() {
		// Show real per-cell coverage data as a heatmap
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

		for (const cov of coverageData) {
			const intensity = Math.min(1.0, cov.signal_strength / maxSignal);
			if (intensity < 0.01) continue;

			// Color by dominant owner, fall back to green
			let color: number;
			if (cov.dominant_owner !== null) {
				color = this.corpColorMap.get(cov.dominant_owner) ?? 0x10b981;
			} else {
				color = 0x10b981;
			}

			const geo = new THREE.CircleGeometry(this.baseCellSize * 0.9, 6);
			const mat = new THREE.MeshBasicMaterial({
				color,
				opacity: 0.08 + intensity * 0.22,
				transparent: true
			});
			const mesh = new THREE.Mesh(geo, mat);
			mesh.position.set(cov.lon, cov.lat, 0.2);

			// Apply latitude correction
			const latRad = (cov.lat * Math.PI) / 180;
			const cosLat = Math.max(0.3, Math.cos(latRad));
			mesh.scale.x = 1 / cosLat;

			this.overlayGroup.add(mesh);
		}
	}

	private renderDisasterRiskOverlay() {
		// Color regions by disaster risk: green (low) → yellow → red (high)
		for (const region of this.cachedRegions) {
			const risk = region.disaster_risk ?? 0;
			const intensity = Math.min(1.0, risk * 5); // Scale up for visibility
			const r = Math.floor(intensity * 255);
			const g = Math.floor((1 - intensity) * 180);
			const color = (r << 16) | (g << 8) | 0;

			const radius = Math.sqrt(region.cell_count) * this.baseCellSize * 0.15;
			const geo = new THREE.CircleGeometry(radius, 24);
			const mat = new THREE.MeshBasicMaterial({
				color,
				opacity: 0.25,
				transparent: true
			});
			const mesh = new THREE.Mesh(geo, mat);
			mesh.position.set(region.center_lon, region.center_lat, 0.2);
			this.overlayGroup.add(mesh);
		}
	}

	private renderCongestionOverlay() {
		// Show node congestion: green (low utilization) → red (high utilization)
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

				const radius = this.baseCellSize * (0.15 + util * 0.15);
				const geo = new THREE.CircleGeometry(radius, 16);
				const mat = new THREE.MeshBasicMaterial({
					color,
					opacity: 0.25,
					transparent: true
				});
				const mesh = new THREE.Mesh(geo, mat);
				mesh.position.set(node.x, node.y, 0.2);
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
