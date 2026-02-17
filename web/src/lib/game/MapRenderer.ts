import * as THREE from 'three';
import * as bridge from '$lib/wasm/bridge';
import type { GridCell, City, Region, CorpSummary } from '$lib/wasm/types';

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

export class MapRenderer {
	private scene: THREE.Scene;
	private camera: THREE.OrthographicCamera;
	private renderer: THREE.WebGLRenderer;
	private container: HTMLElement;

	// Render layers (ordered by z-depth)
	private oceanGroup: THREE.Group; // Layer 1: Ocean base
	private landGroup: THREE.Group; // Layer 2: Land masses
	private borderGroup: THREE.Group; // Layer 3: Region borders
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

	constructor(container: HTMLElement) {
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

		this.renderer = new THREE.WebGLRenderer({ antialias: true });
		this.renderer.setSize(w, h);
		this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
		container.appendChild(this.renderer.domElement);

		// Create groups in render order (z-depth managed by group add order + mesh z position)
		this.oceanGroup = new THREE.Group();
		this.landGroup = new THREE.Group();
		this.borderGroup = new THREE.Group();
		this.ownerGroup = new THREE.Group();
		this.cityGroup = new THREE.Group();
		this.edgeGroup = new THREE.Group();
		this.infraGroup = new THREE.Group();
		this.selectionGroup = new THREE.Group();
		this.labelGroup = new THREE.Group();

		this.scene.add(this.oceanGroup);
		this.scene.add(this.landGroup);
		this.scene.add(this.borderGroup);
		this.scene.add(this.ownerGroup);
		this.scene.add(this.edgeGroup);
		this.scene.add(this.cityGroup);
		this.scene.add(this.infraGroup);
		this.scene.add(this.selectionGroup);
		this.scene.add(this.labelGroup);

		this.raycaster = new THREE.Raycaster();
		this.pointer = new THREE.Vector2();

		this.setupControls();
		this.setupResize();
	}

	buildMap() {
		if (!bridge.isInitialized()) return;

		const cells = bridge.getGridCells();
		const citiesData = bridge.getCities();
		const regions = bridge.getRegions();

		this.buildOcean();
		this.buildLand(cells);
		this.buildRegionBorders(regions, cells);
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
		const cellSize = 1.5;

		// Get parcels to map cell_index → parcel_id
		const parcels = bridge.getParcelsInView(-180, -90, 180, 90);
		const cellToParcel = new Map<number, number>();
		for (const p of parcels) {
			cellToParcel.set(p.cell_index, p.id);
		}

		for (const cell of cells) {
			const color = TERRAIN_COLORS[cell.terrain] || TERRAIN_COLORS.Ocean;
			const geo = new THREE.CircleGeometry(cellSize, 6);
			const mat = new THREE.MeshBasicMaterial({ color });
			const mesh = new THREE.Mesh(geo, mat);
			mesh.position.set(cell.lon, cell.lat, 0);
			const parcelId = cellToParcel.get(cell.index);
			if (parcelId !== undefined) {
				mesh.userData = { parcelId, type: 'parcel' };
			}
			this.landGroup.add(mesh);
		}
	}

	private buildRegionBorders(regions: Region[], cells: GridCell[]) {
		this.borderGroup.clear();

		// Build a cell-to-region lookup
		const cellRegion = new Map<number, number>();
		for (const region of regions) {
			// We need to figure out which cells belong to which region
			// Use the city_ids to find cells in each region
		}

		// For each region, find its boundary cells by checking neighbor regions
		// Simplified approach: draw lines between region centers and their city positions
		const lineMat = new THREE.LineBasicMaterial({
			color: 0x374151,
			opacity: 0.5,
			transparent: true
		});

		for (const region of regions) {
			// Draw a border circle around the region center to indicate territory
			const segments = 32;
			const radius = Math.sqrt(region.cell_count) * 1.2;
			const points: THREE.Vector3[] = [];
			for (let i = 0; i <= segments; i++) {
				const theta = (i / segments) * Math.PI * 2;
				points.push(
					new THREE.Vector3(
						region.center_lon + Math.cos(theta) * radius,
						region.center_lat + Math.sin(theta) * radius,
						0.5
					)
				);
			}
			const geo = new THREE.BufferGeometry().setFromPoints(points);
			const line = new THREE.Line(geo, lineMat);
			this.borderGroup.add(line);
		}
	}

	private buildCities(citiesData: City[]) {
		this.cityGroup.clear();

		for (const city of citiesData) {
			const size = Math.max(0.5, Math.log10(city.population) * 0.5);
			const geo = new THREE.CircleGeometry(size, 16);
			const mat = new THREE.MeshBasicMaterial({ color: 0xfbbf24 });
			const mesh = new THREE.Mesh(geo, mat);
			mesh.position.set(city.x, city.y, 1);
			mesh.userData = { type: 'city', id: city.id, name: city.name };
			this.cityGroup.add(mesh);
			this.entityMeshMap.set(city.id, mesh);
		}
	}

	private buildLabels(cities: City[], regions: Region[]) {
		this.labelGroup.clear();

		// City labels — only visible at closer zoom
		for (const city of cities) {
			const sprite = this.createTextSprite(city.name, 0xd1d5db, 0.8);
			sprite.position.set(city.x, city.y - 1.5, 5);
			sprite.userData = { labelType: 'city', minZoom: 2.0 };
			this.labelGroup.add(sprite);
		}

		// Region labels — visible at wider zoom
		for (const region of regions) {
			const sprite = this.createTextSprite(region.name, 0x6b7280, 1.5);
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

		ctx.font = 'bold 24px system-ui, sans-serif';
		ctx.textAlign = 'center';
		ctx.textBaseline = 'middle';

		// Text shadow for readability
		ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
		ctx.fillText(text, canvas.width / 2 + 1, canvas.height / 2 + 1);

		const hexColor = '#' + color.toString(16).padStart(6, '0');
		ctx.fillStyle = hexColor;
		ctx.fillText(text, canvas.width / 2, canvas.height / 2);

		const texture = new THREE.CanvasTexture(canvas);
		texture.minFilter = THREE.LinearFilter;
		const spriteMat = new THREE.SpriteMaterial({ map: texture, transparent: true });
		const sprite = new THREE.Sprite(spriteMat);
		sprite.scale.set(scale * 8, scale * 2, 1);
		return sprite;
	}

	updateInfrastructure() {
		if (!bridge.isInitialized()) return;

		this.infraGroup.clear();
		this.edgeGroup.clear();
		this.ownerGroup.clear();
		this.entityMeshMap.clear();

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

			// Draw edges
			for (const edge of infra.edges) {
				const points = [
					new THREE.Vector3(edge.src_x, edge.src_y, 1.5),
					new THREE.Vector3(edge.dst_x, edge.dst_y, 1.5)
				];
				const geo = new THREE.BufferGeometry().setFromPoints(points);
				const mat = new THREE.LineBasicMaterial({
					color,
					opacity: 0.6,
					transparent: true
				});
				const line = new THREE.Line(geo, mat);
				this.edgeGroup.add(line);
			}

			// Draw nodes
			const positions: THREE.Vector3[] = [];
			for (const node of infra.nodes) {
				const size = node.under_construction ? 0.4 : 0.6;
				const geo = new THREE.CircleGeometry(size, 8);
				const nodeColor = node.under_construction ? 0x6b7280 : color;
				const mat = new THREE.MeshBasicMaterial({ color: nodeColor });
				const mesh = new THREE.Mesh(geo, mat);
				mesh.position.set(node.x, node.y, 2);
				mesh.userData = { type: 'node', id: node.id, node_type: node.node_type };
				this.infraGroup.add(mesh);
				this.entityMeshMap.set(node.id, mesh);
				positions.push(new THREE.Vector3(node.x, node.y, 0));
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

		for (const [corpId, positions] of corpPositions) {
			const color = this.corpColorMap.get(corpId) ?? 0x888888;

			// Draw a semi-transparent circle around each node to show territory
			for (const pos of positions) {
				const geo = new THREE.CircleGeometry(3.0, 16);
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

		// Glow ring around selected entity
		const ringGeo = new THREE.RingGeometry(1.0, 1.4, 24);
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

		// Outer glow
		const outerGeo = new THREE.RingGeometry(1.4, 2.0, 24);
		const outerMat = new THREE.MeshBasicMaterial({
			color: 0x10b981,
			opacity: 0.3,
			transparent: true,
			side: THREE.DoubleSide
		});
		const outer = new THREE.Mesh(outerGeo, outerMat);
		outer.position.copy(obj.position);
		outer.position.z = 3;
		this.selectionGroup.add(outer);
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
		for (const child of this.labelGroup.children) {
			const minZoom = child.userData?.minZoom ?? 0;
			const maxZoom = child.userData?.maxZoom ?? Infinity;
			child.visible = this.zoom >= minZoom && this.zoom <= maxZoom;
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

	render() {
		this.renderer.render(this.scene, this.camera);
	}

	dispose() {
		this.renderer.dispose();
		this.renderer.domElement.remove();
	}
}
