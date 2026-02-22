<script lang="ts">
  import { worldInfo } from '$lib/stores/gameState';

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }

  let spacing = $derived($worldInfo.cell_spacing_km || 100);

  // Must match Rust EdgeType::distance_multiplier() and allowed_tier_connections()
  const EDGES = [
    { key: 'Copper', name: 'Copper', mult: 2, color: '#b45309', tiers: ['T1↔T1', 'T1↔T2'], terrain: 'Land only' },
    { key: 'FiberLocal', name: 'Fiber Local', mult: 5, color: '#22c55e', tiers: ['T1↔T1', 'T1↔T2', 'T2↔T2'], terrain: 'Land only' },
    { key: 'Microwave', name: 'Microwave', mult: 8, color: '#f59e0b', tiers: ['T1↔T1', 'T1↔T2', 'T2↔T2', 'T2↔T3'], terrain: 'Any' },
    { key: 'FiberRegional', name: 'Fiber Regional', mult: 15, color: '#3b82f6', tiers: ['T2↔T2', 'T2↔T3', 'T3↔T3'], terrain: 'Land only' },
    { key: 'FiberNational', name: 'Fiber National', mult: 40, color: '#8b5cf6', tiers: ['T3↔T3', 'T3↔T4', 'T4↔T4'], terrain: 'Land only' },
    { key: 'Satellite', name: 'Satellite', mult: Infinity, color: '#06b6d4', tiers: ['T3↔T5', 'T4↔T5', 'T5↔T5'], terrain: 'Any' },
    { key: 'Submarine', name: 'Submarine Cable', mult: 60, color: '#0ea5e9', tiers: ['T5↔T5'], terrain: 'Needs water' },
  ];

  const NODES = [
    { tier: 1, label: 'Access', types: 'Cell Tower, Wireless Relay', color: '#22c55e', icon: '📡' },
    { tier: 2, label: 'Aggregation', types: 'Central Office, Exchange Point', color: '#3b82f6', icon: '🏢' },
    { tier: 3, label: 'Core', types: 'Data Center', color: '#8b5cf6', icon: '🖥' },
    { tier: 4, label: 'Backbone', types: 'Backbone Router', color: '#f59e0b', icon: '⚡' },
    { tier: 5, label: 'Global', types: 'Satellite Ground, Submarine Landing', color: '#ef4444', icon: '🌐' },
  ];

  function fmtDist(mult: number): string {
    if (!isFinite(mult)) return '∞';
    const km = Math.round(spacing * mult);
    if (km >= 1000) return `${(km / 1000).toFixed(1)}k km`;
    return `${km} km`;
  }

  // Which edge types link adjacent tiers (for the diagram connectors)
  const TIER_LINKS: Record<string, string[]> = {
    '1-2': ['Copper', 'Fiber Local', 'Microwave'],
    '2-3': ['Microwave', 'Fiber Regional'],
    '3-4': ['Fiber National'],
    '4-5': ['Satellite'],
  };

  // Connection matrix: for each tier pair, how many edge types can connect them
  function matrixEdges(r: number, c: number): string[] {
    return EDGES.filter(e => e.tiers.some(t => {
      const [a, b] = t.replace('↔', '-').split('-').map(s => parseInt(s.replace('T', '')));
      return (a === r && b === c) || (a === c && b === r);
    })).map(e => e.name);
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose} onkeydown={handleKeyDown} role="presentation">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="guide" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-label="Network Tier Guide" tabindex="-1">
    <div class="header">
      <span class="title">Network Tier &amp; Connection Guide</span>
      <button class="close-btn" onclick={onclose}>&#x2715;</button>
    </div>
    <div class="body">

      <!-- VISUAL HIERARCHY DIAGRAM -->
      <section class="section">
        <h4>Network Hierarchy</h4>
        <div class="diagram">
          {#each NODES as node}
            <div class="diagram-tier">
              <div class="tier-badge" style="background: {node.color}15; border-color: {node.color}60; color: {node.color}">
                <span class="tier-icon">{node.icon}</span>
                <span class="tier-num">T{node.tier}</span>
                <span class="tier-name">{node.label}</span>
              </div>
              <div class="tier-types">{node.types}</div>
            </div>
            {#if node.tier < 5}
              <div class="tier-connector">
                {#each TIER_LINKS[`${node.tier}-${node.tier + 1}`] ?? [] as edgeName}
                  <span class="conn-tag" style="color: {EDGES.find(e => e.name === edgeName)?.color ?? '#888'}">{edgeName}</span>
                {/each}
                <span class="conn-arrow">↕</span>
              </div>
            {/if}
          {/each}
        </div>
      </section>

      <!-- EDGE TYPES TABLE -->
      <section class="section">
        <h4>Edge Types &amp; Range (this map: {Math.round(spacing)}km grid)</h4>
        <div class="edge-table">
          <div class="edge-hdr">
            <span class="eh-name">Type</span>
            <span class="eh-range">Max Range</span>
            <span class="eh-tiers">Connects</span>
            <span class="eh-terrain">Terrain</span>
          </div>
          {#each EDGES as edge}
            <div class="edge-row">
              <span class="er-name">
                <span class="er-dot" style="background: {edge.color}"></span>
                {edge.name}
              </span>
              <span class="er-range" class:unlimited={!isFinite(edge.mult)}>{fmtDist(edge.mult)}</span>
              <span class="er-tiers">{edge.tiers.join(', ')}</span>
              <span class="er-terrain">{edge.terrain}</span>
            </div>
          {/each}
        </div>
      </section>

      <!-- CONNECTION MATRIX -->
      <section class="section">
        <h4>Compatibility Matrix</h4>
        <div class="matrix">
          <div class="mx-hdr">
            <span class="mx-blank"></span>
            {#each NODES as n}
              <span class="mx-col" style="color: {n.color}">T{n.tier}</span>
            {/each}
          </div>
          {#each NODES as rowNode}
            <div class="mx-row">
              <span class="mx-label" style="color: {rowNode.color}">T{rowNode.tier} {rowNode.label}</span>
              {#each NODES as colNode}
                {#if matrixEdges(rowNode.tier, colNode.tier).length > 0}
                  <span class="mx-cell has" title={matrixEdges(rowNode.tier, colNode.tier).join(', ')}>
                    <span class="mx-count">{matrixEdges(rowNode.tier, colNode.tier).length}</span>
                  </span>
                {:else}
                  <span class="mx-cell none"><span class="mx-x">—</span></span>
                {/if}
              {/each}
            </div>
          {/each}
        </div>
        <p class="mx-hint">Numbers = edge types that link those tiers. Hover for names.</p>
      </section>

      <!-- BUILD TIPS -->
      <section class="section">
        <h4>Build Strategy</h4>
        <div class="tips">
          <div class="tip">
            <span class="ti">🏗</span>
            <span class="tt"><strong>Build bottom-up:</strong> Cell Towers (T1) → Fiber Local to Central Offices (T2) → Fiber Regional to Data Centers (T3) → Fiber National to Backbone (T4) → Satellite to Global (T5).</span>
          </div>
          <div class="tip">
            <span class="ti">📏</span>
            <span class="tt"><strong>Too far?</strong> Use Microwave instead of Fiber Local — same tier support, {fmtDist(8)} range vs {fmtDist(5)}. Or drop Wireless Relays between cities as stepping stones.</span>
          </div>
          <div class="tip">
            <span class="ti">🔗</span>
            <span class="tt"><strong>Multi-connect:</strong> A Central Office can link to dozens of Cell Towers. Nodes aren't limited to one edge — hub-and-spoke patterns work great.</span>
          </div>
          <div class="tip">
            <span class="ti">🌊</span>
            <span class="tt"><strong>Water:</strong> Fiber/Copper can't cross deep ocean. Use Submarine Cable (T5↔T5, needs water) or Satellite (unlimited range, any terrain).</span>
          </div>
          <div class="tip">
            <span class="ti">💡</span>
            <span class="tt"><strong>Same-tier links:</strong> Most edge types can connect nodes of the same tier (e.g. T2↔T2 with Fiber Local). Great for metro rings and mesh networks.</span>
          </div>
        </div>
      </section>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .guide {
    background: rgba(12, 18, 30, 0.98);
    border: 1px solid rgba(55, 65, 81, 0.6);
    border-radius: 12px;
    width: 620px;
    max-height: 85vh;
    overflow-y: auto;
    box-shadow: 0 12px 48px rgba(0, 0, 0, 0.6);
  }
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid rgba(55, 65, 81, 0.4);
    position: sticky;
    top: 0;
    background: rgba(12, 18, 30, 0.98);
    z-index: 1;
  }
  .title {
    font-family: var(--font-sans, system-ui, sans-serif);
    font-size: 15px;
    font-weight: 700;
    color: #e5e7eb;
  }
  .close-btn {
    background: transparent;
    border: none;
    color: #6b7280;
    font-size: 16px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
  }
  .close-btn:hover {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }
  .body { padding: 18px; }
  .section { margin-bottom: 20px; }
  .section:last-child { margin-bottom: 0; }
  .section h4 {
    font-family: var(--font-sans, system-ui, sans-serif);
    font-size: 11px;
    font-weight: 700;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    margin: 0 0 10px;
  }

  /* DIAGRAM */
  .diagram { display: flex; flex-direction: column; align-items: stretch; gap: 0; }
  .diagram-tier { display: flex; align-items: center; gap: 12px; padding: 5px 0; }
  .tier-badge {
    display: flex; align-items: center; gap: 6px;
    padding: 6px 12px; border: 1px solid; border-radius: 8px;
    font-size: 12px; font-weight: 600; min-width: 170px;
    font-family: var(--font-sans, system-ui, sans-serif);
  }
  .tier-icon { font-size: 16px; }
  .tier-num { font-family: var(--font-mono, monospace); font-weight: 800; font-size: 13px; }
  .tier-name { font-weight: 600; }
  .tier-types { color: #9ca3af; font-size: 11px; font-family: var(--font-sans, system-ui, sans-serif); }
  .tier-connector {
    display: flex; align-items: center; gap: 6px;
    padding: 1px 0 1px 24px;
  }
  .conn-tag {
    font-size: 10px; font-weight: 600;
    font-family: var(--font-mono, monospace);
    padding: 1px 6px; background: rgba(255,255,255,0.04); border-radius: 3px;
  }
  .conn-arrow { color: #4b5563; font-size: 12px; }

  /* EDGE TABLE */
  .edge-table { border: 1px solid rgba(55, 65, 81, 0.3); border-radius: 8px; overflow: hidden; }
  .edge-hdr, .edge-row {
    display: grid;
    grid-template-columns: 130px 80px 1fr 80px;
    padding: 7px 12px;
    align-items: center;
  }
  .edge-hdr {
    background: rgba(31, 41, 55, 0.5);
    font-size: 10px; font-weight: 700; color: #6b7280;
    text-transform: uppercase; letter-spacing: 0.5px;
    font-family: var(--font-sans, system-ui, sans-serif);
  }
  .edge-row {
    border-top: 1px solid rgba(55, 65, 81, 0.15);
    font-size: 12px;
  }
  .edge-row:hover { background: rgba(55, 65, 81, 0.15); }
  .er-name {
    display: flex; align-items: center; gap: 6px;
    font-weight: 600; color: #d1d5db;
    font-family: var(--font-sans, system-ui, sans-serif);
  }
  .er-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .er-range {
    font-family: var(--font-mono, monospace); font-weight: 700;
    color: #f59e0b; font-size: 11px;
  }
  .er-range.unlimited { color: #06b6d4; }
  .er-tiers { color: #9ca3af; font-size: 11px; }
  .er-terrain { color: #6b7280; font-size: 10px; text-align: right; }

  /* MATRIX */
  .matrix { border: 1px solid rgba(55, 65, 81, 0.3); border-radius: 8px; overflow: hidden; }
  .mx-hdr, .mx-row {
    display: grid;
    grid-template-columns: 120px repeat(5, 1fr);
    padding: 5px 8px;
  }
  .mx-hdr { background: rgba(31, 41, 55, 0.5); }
  .mx-blank { display: block; }
  .mx-col {
    text-align: center; font-size: 11px; font-weight: 700;
    font-family: var(--font-mono, monospace);
  }
  .mx-row { border-top: 1px solid rgba(55, 65, 81, 0.15); }
  .mx-label {
    font-size: 11px; font-weight: 600;
    font-family: var(--font-sans, system-ui, sans-serif);
    display: flex; align-items: center;
  }
  .mx-cell {
    display: flex; align-items: center; justify-content: center;
    padding: 2px 0;
  }
  .mx-count {
    font-weight: 700; color: #10b981;
    background: rgba(16, 185, 129, 0.1);
    width: 24px; height: 24px;
    display: flex; align-items: center; justify-content: center;
    border-radius: 4px; font-family: var(--font-mono, monospace);
    font-size: 12px; cursor: help;
  }
  .mx-x { color: #374151; }
  .mx-hint {
    font-size: 10px; color: #6b7280;
    padding: 6px 12px; margin: 0;
    font-family: var(--font-sans, system-ui, sans-serif);
  }

  /* TIPS */
  .tips { display: flex; flex-direction: column; gap: 8px; }
  .tip {
    display: flex; gap: 10px;
    padding: 8px 12px; background: rgba(31, 41, 55, 0.4);
    border-radius: 6px; border-left: 3px solid rgba(59, 130, 246, 0.4);
  }
  .ti { font-size: 16px; flex-shrink: 0; line-height: 1.4; }
  .tt {
    font-size: 12px; color: #9ca3af; line-height: 1.6;
    font-family: var(--font-sans, system-ui, sans-serif);
  }
  .tt strong { color: #d1d5db; }
</style>
