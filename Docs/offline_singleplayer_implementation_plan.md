# Offline Single-Player Mode — Implementation Plan

## Context

The GlobalTelco project is designed as an infrastructure civilization simulator with single-player explicitly specified as a launch feature (per `game_design_decisions.md`). However, the current codebase only has the multiplayer-oriented foundation:
- GameMode is server-authoritative with Login/Logout/250-cap
- No AI corporation controller exists despite `UGTWorldSettings` having `AICorpCount`/`AIAggressiveness`
- No corporation lifecycle management (nothing spawns or tracks corps)
- No save/load for local persistence
- No simulation speed controls
- `UGTLandParcelSystem` has no `UpdateParcel()` method, so world generator can't actually write terrain/zoning

This plan adds a fully functional offline single-player mode with AI corporations, save/load, speed controls, and minimal menu UI.

---

## Phase 1: Foundation Fixes

### 1a. Add `UpdateParcel()` to `UGTLandParcelSystem`
**File:** `Source/GTMultiplayer/Public/GTLandParcelSystem.h` + `.cpp`
- Add `bool UpdateParcel(int32 ParcelId, const FGTLandParcel& UpdatedData)`
- Fix `AssignTerrain()` and `AssignZoning()` in world generator to actually call this

### 1b. Corporation Manager Subsystem
**New file:** `Source/GTEconomy/Public/GTCorporationManager.h` + `.cpp`
- `UGTCorporationManager` — `UWorldSubsystem`
- `int32 CreateCorporation(const FString& Name, double StartingCapital, bool bIsAI)` — Creates a `UGTCorporation`, assigns ID, stores it
- `UGTCorporation* GetCorporation(int32 CorporationId) const`
- `TArray<UGTCorporation*> GetAllCorporations() const`
- `TArray<UGTCorporation*> GetAICorporations() const`
- `void ProcessAllCorporationTicks(float DeltaSeconds)` — calls `ProcessEconomicTick` on every corp
- `bool bIsAI` flag added to `UGTCorporation` (new `UPROPERTY`)
- `int32 ArchetypeIndex` field on `UGTCorporation` for AI personality reference

### 1c. Add `bIsAI` to `UGTCorporation`
**File:** `Source/GTEconomy/Public/GTCorporation.h`
- Add `UPROPERTY(BlueprintReadOnly) bool bIsAI = false`
- Add `UPROPERTY(BlueprintReadOnly) int32 ArchetypeIndex = -1`

---

## Phase 2: AI Archetype System

### 2a. AI Archetype Data Asset
**New file:** `Source/GTCore/Public/GTAIArchetype.h` + `.cpp`
- `UGTAIArchetype` — `UDataAsset`
- Fields:
  - `FString ArchetypeName` (e.g., "Aggressive Expander")
  - `FString Description`
  - `TArray<FString> CompanyNamePool` (names the AI can use)
  - `float ExpansionWeight` (0-1, how much to prioritize building new infra)
  - `float ConsolidationWeight` (0-1, improve existing network)
  - `float TechInvestmentWeight` (0-1, R&D priority)
  - `float AggressionWeight` (0-1, competitive behavior vs cooperative)
  - `float RiskTolerance` (0-1, willingness to take on debt/dangerous regions)
  - `float FinancialPrudence` (0-1, cash reserve preferences)
- Provide 4 built-in archetypes constructed in code (no editor assets needed):
  1. **Aggressive Expander** — high expansion, high aggression, low prudence
  2. **Defensive Consolidator** — high consolidation, low aggression, high prudence
  3. **Tech Innovator** — high tech investment, moderate expansion, moderate risk
  4. **Budget Operator** — high financial prudence, low risk, moderate consolidation

### 2b. AI Archetype Registry
**New file:** `Source/GTCore/Public/GTAIArchetypeRegistry.h` + `.cpp`
- `UGTAIArchetypeRegistry` — Singleton-like UObject created by GTCore module startup
- `static const TArray<FGTAIArchetypeData>& GetArchetypes()` — returns the 4 built-in archetype structs
- Use a struct `FGTAIArchetypeData` instead of UDataAsset so we avoid needing editor assets. Keep it pure code.

---

## Phase 3: AI Corporation Controller + Behavior Tree

### 3a. Build Module Dependencies
**File:** `Source/GlobalTelco/GlobalTelco.Build.cs`
- Add `"AIModule"`, `"GameplayTasks"`, `"GameplayAbilities"` (if needed) to dependencies
- These provide `AAIController`, `UBehaviorTreeComponent`, `UBlackboardComponent`, BT task base classes

### 3b. AI Corporation Controller
**New file:** `Source/GlobalTelco/Public/GTAICorporationController.h` + `.cpp`
- `AGTAICorporationController` — extends `AAIController`
- No pawn possessed (operates as a pure decision-making agent)
- Properties:
  - `int32 CorporationId` — which corp this controls
  - `FGTAIArchetypeData Archetype` — personality configuration
  - `float AggressivenessMultiplier` — from world settings
- On `BeginPlay()`:
  - Gets references to all subsystems (SimulationSubsystem, CorporationManager, NetworkGraph, LandParcelSystem, AllianceManager, RegionalEconomy)
  - Sets up blackboard with initial values
  - Constructs and starts the behavior tree programmatically
- Listens to `UGTEventQueue::OnEventDispatched` for `EconomicTick` events to trigger decision-making

### 3c. Blackboard Keys
Blackboard data (updated each tick by service node):
- `CashOnHand` (float)
- `TotalDebt` (float)
- `OwnedNodeCount` (int)
- `OwnedEdgeCount` (int)
- `OwnedParcelCount` (int)
- `CreditRating` (enum as int)
- `CanAffordNode` (bool)
- `CanAffordParcel` (bool)
- `HasExpandableRegions` (bool)
- `CurrentStrategy` (enum: Expand, Consolidate, Compete, Survive)

### 3d. Behavior Tree Tasks (all in GlobalTelco module)

**BT Service: `UBTService_UpdateWorldState`**
- Updates blackboard with current corporation state from subsystems
- Evaluates strategy based on archetype weights + current financial state
- Sets `CurrentStrategy` blackboard key

**BT Task: `UBTTask_AcquireLand`**
- Scans parcels for government-owned land in valuable regions
- Scores parcels by: terrain suitability, regional demand, proximity to existing network, cost
- Applies archetype weights to scoring
- Purchases the top-scoring parcel if affordable

**BT Task: `UBTTask_BuildNode`**
- Finds owned parcels without infrastructure
- Chooses node type based on regional demand and network gaps
- Places a node and registers it in the network graph
- Fires `InfrastructureBuilt` event

**BT Task: `UBTTask_BuildEdge`**
- Finds pairs of owned nodes that could benefit from an edge
- Chooses edge type based on distance, terrain, node types
- Creates the edge in the network graph

**BT Task: `UBTTask_ManageFinances`**
- If cash low + credit good: take a loan
- If cash high + debt outstanding: pay down debt
- Adjust spending based on financial health

**BT Task: `UBTTask_ProposeContract`**
- Identifies nearby AI or player corps
- Proposes peering/transit contracts based on mutual benefit
- Archetype affects terms (aggressive = cheaper to undercut, prudent = fair pricing)

**BT Decorator: `UBTDecorator_CheckFinances`**
- Gates branches based on financial health (can afford X, debt ratio below Y)

**BT Decorator: `UBTDecorator_CheckStrategy`**
- Gates branches based on current strategy state

### 3e. Programmatic Behavior Tree Construction
In `AGTAICorporationController`, build the tree in code:
```
Root (Selector)
├── [Decorator: Strategy==Survive] → Sequence: Cut costs, sell assets, seek contracts
├── [Decorator: Strategy==Expand] → Sequence: Buy land → Build nodes → Build edges
├── [Decorator: Strategy==Consolidate] → Sequence: Upgrade existing → Build edges → Contracts
├── [Decorator: Strategy==Compete] → Sequence: Undercut competitors → Aggressive expansion
└── Fallback: ManageFinances
Service on Root: UpdateWorldState (runs every tick)
```

---

## Phase 4: Single-Player Game Mode

### 4a. Single-Player Game Mode
**New file:** `Source/GlobalTelco/Public/GTSinglePlayerGameMode.h` + `.cpp`
- `AGTSinglePlayerGameMode` — extends `AGameModeBase` (NOT `AGTGameMode`, to avoid networking assumptions)
- `InitGame()`:
  - Creates `UGTWorldSettings` from configuration
  - Calls `UGTWorldGenerator::GenerateWorld()`
  - Creates player corporation via `UGTCorporationManager`
  - Spawns AI corporation controllers (one per `AICorpCount`)
  - Assigns archetypes to AI controllers (round-robin from registry, varied by seed)
  - Sets `EconomicTickInterval` from world settings
- `StartPlay()`:
  - Starts all AI controllers
  - Begins simulation
- Uses existing `AGTGameState` and `AGTPlayerController`
- `MaxConcurrentPlayers = 1` (single player)
- Sets `DefaultPawnClass` to `AGTGlobePawn`

### 4b. Speed Controls on Simulation Subsystem
**File:** `Source/GTCore/Public/GTSimulationSubsystem.h` + `.cpp`
- Add `bool bIsPaused = false`
- Add `float SimulationSpeedMultiplier = 1.0f` (1x, 2x, 4x, 8x)
- Add `void SetPaused(bool bPaused)`
- Add `void SetSpeedMultiplier(float Multiplier)`
- Add `bool IsPaused() const`
- Add `float GetSpeedMultiplier() const`
- Modify `Tick()`: if paused, skip accumulation. Apply speed multiplier to DeltaTime before accumulation.

---

## Phase 5: Save/Load System

### 5a. Save Game Object
**New file:** `Source/GlobalTelco/Public/GTSaveGame.h` + `.cpp`
- `UGTSaveGame` — extends `USaveGame`
- Serialized data:
  - `FString SaveName`
  - `FDateTime SaveTimestamp`
  - `int32 SaveVersion` (for migration)
  - `EGTDifficulty Difficulty`
  - `int64 SimulationTick`
  - `double SimulationTimeSeconds`
  - World settings snapshot (starting capital, multipliers, AI settings)
  - `TArray<FGTLandParcel> AllParcels`
  - `TArray<FGTSavedCorporation> Corporations` (new struct with all corp data + bIsAI + archetype)
  - `TArray<FGTSavedNode> InfrastructureNodes` (node type, attributes, owners, parcel)
  - `TArray<FGTSavedEdge> InfrastructureEdges` (edge type, attributes, endpoints)
  - `TArray<FGTRegionalEconomyData> Regions`
  - `TArray<FGTContract> ActiveContracts`
  - `TArray<FGTAlliance> ActiveAlliances`
  - `int32 WorldSeed`
  - Player corporation ID

### 5b. Save/Load Manager
**New file:** `Source/GlobalTelco/Public/GTSaveLoadSubsystem.h` + `.cpp`
- `UGTSaveLoadSubsystem` — `UGameInstanceSubsystem`
- `bool SaveGame(const FString& SlotName)` — snapshots all subsystem state into `UGTSaveGame`, calls `UGameplayStatics::SaveGameToSlot()`
- `bool LoadGame(const FString& SlotName)` — loads, then restores all subsystem state
- `bool DeleteSave(const FString& SlotName)`
- `TArray<FGTSaveSlotInfo> GetAllSaveSlots()` — lists available saves with metadata
- `bool DoesSlotExist(const FString& SlotName) const`
- Auto-save: `void AutoSave()` — saves to "AutoSave" slot, called every N ticks from simulation

### 5c. Serialization Helper Structs
**In GTSaveGame.h:**
- `FGTSavedCorporation` — serializable version of corporation state
- `FGTSavedNode` — serializable version of network node
- `FGTSavedEdge` — serializable version of network edge

---

## Phase 6: Minimal UI

### 6a. Main Menu Widget
**New file:** `Source/GTFrontend/Public/GTMainMenuWidget.h` + `.cpp`
- `UGTMainMenuWidget` — `UUserWidget`
- Buttons: New Game, Load Game, Quit
- New Game opens the settings panel
- Load Game shows save slot list

### 6b. New Game Settings Widget
**New file:** `Source/GTFrontend/Public/GTNewGameWidget.h` + `.cpp`
- `UGTNewGameWidget` — `UUserWidget`
- Fields:
  - Corporation name (editable text box)
  - Difficulty dropdown (Easy/Normal/Hard/Custom)
  - AI Corporation count (slider 0-10)
  - Disaster severity dropdown
  - World seed (text box, 0=random)
  - Start Game button
- On Start: creates `UGTWorldSettings`, applies difficulty defaults, opens game level with settings passed via `GameInstance`

### 6c. Speed Control Widget
**New file:** `Source/GTFrontend/Public/GTSpeedControlWidget.h` + `.cpp`
- `UGTSpeedControlWidget` — `UUserWidget`
- Buttons: Pause (||), Play (>), 2x (>>), 4x (>>>)
- Current speed display text
- Save button, Load button (quick access)

### 6d. GTFrontend Build.cs Update
**File:** `Source/GTFrontend/GTFrontend.Build.cs`
- Add `"GTInfrastructure"` to dependencies (for save/load serialization types)

---

## Phase 7: Integration Wiring

### 7a. Game Instance for Session State
**New file:** `Source/GlobalTelco/Public/GTGameInstance.h` + `.cpp`
- `UGTGameInstance` — `UGameInstance`
- Holds `UGTWorldSettings*` for the current session
- Holds player corporation name
- Holds save slot name (for auto-save)
- Bridges main menu -> gameplay level transition

### 7b. Wire Simulation Subsystem to Corporation Manager
**File:** `Source/GTCore/Private/GTSimulationSubsystem.cpp`
- In `ProcessEconomicTick()`, after draining events, call `UGTCorporationManager::ProcessAllCorporationTicks()`
- Wire up the AllianceManager contract processing too

### 7c. DefaultEngine.ini
**File:** `Config/DefaultEngine.ini`
- Set `GameDefaultMap` to main menu level
- Set `GameInstanceClass` to `UGTGameInstance`

---

## Phase 8: Documentation Updates

### 8a. CLAUDE.md
- Add single-player section describing the offline architecture
- Add AI Corporation Controller class to Key Classes
- Add save/load commands
- Add GTGameInstance, GTSinglePlayerGameMode to Key Classes
- Update module dependency graph (GlobalTelco now depends on AIModule)

### 8b. game_design_decisions.md
- Document AI archetype system decisions
- Document save format (UE5 SaveGame)
- Document speed control options

---

## Files Modified (existing)
| File | Changes |
|------|---------|
| `Source/GTMultiplayer/Public/GTLandParcelSystem.h` | Add `UpdateParcel()` |
| `Source/GTMultiplayer/Private/GTLandParcelSystem.cpp` | Implement `UpdateParcel()` |
| `Source/GlobalTelco/Private/GTWorldGenerator.cpp` | Fix terrain/zoning to call `UpdateParcel()` |
| `Source/GTEconomy/Public/GTCorporation.h` | Add `bIsAI`, `ArchetypeIndex` |
| `Source/GTCore/Public/GTSimulationSubsystem.h` | Add speed controls |
| `Source/GTCore/Private/GTSimulationSubsystem.cpp` | Implement speed controls, wire corp manager |
| `Source/GlobalTelco/GlobalTelco.Build.cs` | Add `AIModule`, `GameplayTasks` |
| `Source/GTFrontend/GTFrontend.Build.cs` | Add `GTInfrastructure` dependency |
| `GlobalTelco.uproject` | (possibly) add plugins if needed |
| `Config/DefaultEngine.ini` | Set default map/game instance |
| `CLAUDE.md` | Document new systems |
| `Docs/game_design_decisions.md` | Document SP decisions |

## Files Created (new)
| File | Class |
|------|-------|
| `Source/GTCore/Public/GTAIArchetype.h` | `FGTAIArchetypeData` struct, `UGTAIArchetypeRegistry` |
| `Source/GTCore/Private/GTAIArchetype.cpp` | Built-in archetype definitions |
| `Source/GTEconomy/Public/GTCorporationManager.h` | `UGTCorporationManager` |
| `Source/GTEconomy/Private/GTCorporationManager.cpp` | Implementation |
| `Source/GlobalTelco/Public/GTAICorporationController.h` | `AGTAICorporationController` |
| `Source/GlobalTelco/Private/GTAICorporationController.cpp` | BT construction + AI logic |
| `Source/GlobalTelco/Public/GTAITasks.h` | All BT task/service/decorator classes |
| `Source/GlobalTelco/Private/GTAITasks.cpp` | BT task implementations |
| `Source/GlobalTelco/Public/GTSinglePlayerGameMode.h` | `AGTSinglePlayerGameMode` |
| `Source/GlobalTelco/Private/GTSinglePlayerGameMode.cpp` | SP session management |
| `Source/GlobalTelco/Public/GTSaveGame.h` | `UGTSaveGame` + serialization structs |
| `Source/GlobalTelco/Private/GTSaveGame.cpp` | Serialization implementation |
| `Source/GlobalTelco/Public/GTSaveLoadSubsystem.h` | `UGTSaveLoadSubsystem` |
| `Source/GlobalTelco/Private/GTSaveLoadSubsystem.cpp` | Save/Load/List/Delete |
| `Source/GlobalTelco/Public/GTGameInstance.h` | `UGTGameInstance` |
| `Source/GlobalTelco/Private/GTGameInstance.cpp` | Session state bridge |
| `Source/GTFrontend/Public/GTMainMenuWidget.h` | Main menu UI |
| `Source/GTFrontend/Private/GTMainMenuWidget.cpp` | Menu implementation |
| `Source/GTFrontend/Public/GTNewGameWidget.h` | New game settings UI |
| `Source/GTFrontend/Private/GTNewGameWidget.cpp` | Settings implementation |
| `Source/GTFrontend/Public/GTSpeedControlWidget.h` | Speed controls UI |
| `Source/GTFrontend/Private/GTSpeedControlWidget.cpp` | Speed controls implementation |

## Verification

1. **Build test:** Compile `GlobalTelcoEditor Mac Development` — must succeed with no errors
2. **World gen:** Launch editor, create a level, verify world generates with parcels + regions
3. **AI corps:** Verify AI controllers spawn, receive economic tick events, and make decisions
4. **Speed controls:** Verify pause/resume/2x/4x work on simulation
5. **Save/Load:** Save a game, quit, load it back, verify state matches
6. **Main menu:** Verify New Game flow creates a playable session with AI opponents
