import * as THREE from 'three';
import * as bridge from '$lib/wasm/bridge';
import type { GridCell, City, VisibleNode } from '$lib/wasm/types';

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

const CORP_COLORS = [0x10b981, 0x3b82f6, 0xf59e0b, 0xef4444, 0x8b5cf6, 0xec4899, 0x14b8a6, 0xf97316];

export class MapRenderer {
	private scene: THREE.Scene;
	private camera: THREE.OrthographicCamera;
	private renderer: THREE.WebGLRenderer;
	private container: HTMLElement;

	private landGroup: THREE.Group;
	private cityGroup: THREE.Group;
	private infraGroup: THREE.Group;
	private edgeGroup: THREE.Group;
	private labelGroup: THREE.Group;

	private isDragging = false;
	private lastMouse = { x: 0, y: 0 };
	private zoom = 1;
	private panX = 0;
	private panY = 0;

	private raycaster: THREE.Raycaster;
	private pointer: THREE.Vector2;
	private entityMeshMap: Map<number, THREE.Object3D> = new Map();

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

		this.landGroup = new THREE.Group();
		this.cityGroup = new THREE.Group();
		this.infraGroup = new THREE.Group();
		this.edgeGroup = new THREE.Group();
		this.labelGroup = new THREE.Group();

		this.scene.add(this.landGroup);
		this.scene.add(this.edgeGroup);
		this.scene.add(this.cityGroup);
		this.scene.add(this.infraGroup);
		this.scene.add(this.labelGroup);

		this.raycaster = new THREE.Raycaster();
		this.pointer = new THREE.Vector2();

		this.setupControls();
		this.setupResize();
	}

	buildMap() {
		if (!bridge.isInitialized()) return;

		// Build land cells
		const cells = bridge.getGridCells();
		this.buildLand(cells);

		// Build cities
		const citiesData = bridge.getCities();
		this.buildCities(citiesData);
	}

	private buildLand(cells: GridCell[]) {
		this.landGroup.clear();
		const cellSize = 1.5;

		for (const cell of cells) {
			const color = TERRAIN_COLORS[cell.terrain] || TERRAIN_COLORS.Ocean;
			const geo = new THREE.CircleGeometry(cellSize, 6);
			const mat = new THREE.MeshBasicMaterial({ color });
			const mesh = new THREE.Mesh(geo, mat);
			mesh.position.set(cell.lon, cell.lat, 0);
			this.landGroup.add(mesh);
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

	updateInfrastructure() {
		if (!bridge.isInitialized()) return;

		this.infraGroup.clear();
		this.edgeGroup.clear();
		this.entityMeshMap.clear();

		// Rebuild city refs
		for (const child of this.cityGroup.children) {
			if (child.userData?.id) {
				this.entityMeshMap.set(child.userData.id, child);
			}
		}

		const corps = bridge.getAllCorporations();

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
			}
		}
	}

	private setupControls() {
		const el = this.renderer.domElement;

		el.addEventListener('mousedown', (e) => {
			this.isDragging = true;
			this.lastMouse = { x: e.clientX, y: e.clientY };
		});

		el.addEventListener('mousemove', (e) => {
			if (this.isDragging) {
				const dx = (e.clientX - this.lastMouse.x) / this.zoom;
				const dy = (e.clientY - this.lastMouse.y) / this.zoom;
				this.panX -= dx * 0.5;
				this.panY += dy * 0.5;
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
		});

		el.addEventListener('click', (e) => {
			if (this.isDragging) return;
			const rect = el.getBoundingClientRect();
			this.pointer.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
			this.pointer.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
			this.raycaster.setFromCamera(this.pointer, this.camera);
			const intersects = this.raycaster.intersectObjects(
				[...this.infraGroup.children, ...this.cityGroup.children],
				false
			);
			if (intersects.length > 0) {
				const obj = intersects[0].object;
				if (obj.userData?.id) {
					window.dispatchEvent(
						new CustomEvent('entity-selected', {
							detail: { id: obj.userData.id, type: obj.userData.type }
						})
					);
				}
			}
		});
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
