#pragma once

#include "CoreMinimal.h"
#include "AIController.h"
#include "GTAIArchetype.h"
#include "GTAITasks.h"
#include "GTAICorporationController.generated.h"

class UGTCorporation;
class UGTCorporationManager;
class UGTNetworkGraph;
class UGTLandParcelSystem;
class UGTRegionalEconomy;
class UGTAllianceManager;
class UGTSimulationSubsystem;
class UBehaviorTree;
class UBehaviorTreeComponent;

/**
 * AGTAICorporationController
 *
 * AI controller that drives a single AI corporation in single-player mode.
 * Uses UE5's behavior tree framework to make strategic decisions each
 * simulation tick. Does not possess a pawn — operates as a pure
 * decision-making agent on the simulation data.
 *
 * The controller:
 * - Listens for EconomicTick events from the simulation subsystem
 * - Maintains cached references to all game subsystems
 * - Runs a programmatically constructed behavior tree
 * - Uses archetype weights to produce distinct AI personalities
 */
UCLASS()
class GLOBALTELCO_API AGTAICorporationController : public AAIController
{
	GENERATED_BODY()

public:
	AGTAICorporationController();

	virtual void BeginPlay() override;
	virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;

	// --- Configuration ---

	/** Set up this controller to manage a specific corporation. */
	void InitializeForCorporation(int32 InCorporationId, const FGTAIArchetypeData& InArchetype, float InAggressiveness);

	// --- Accessors for BT tasks ---

	UGTCorporation* GetCorporation() const;
	UGTCorporationManager* GetCorporationManager() const { return CachedCorpManager; }
	UGTNetworkGraph* GetNetworkGraph() const { return CachedNetworkGraph; }
	UGTLandParcelSystem* GetParcelSystem() const { return CachedParcelSystem; }
	UGTRegionalEconomy* GetRegionalEconomy() const { return CachedRegionalEconomy; }
	UGTAllianceManager* GetAllianceManager() const { return CachedAllianceManager; }
	const FGTAIArchetypeData& GetArchetype() const { return Archetype; }
	float GetAggressivenessMultiplier() const { return AggressivenessMultiplier; }

	// --- Cached state (written by BT service, read by BT tasks) ---

	double CachedCash = 0.0;
	double CachedDebtToEquity = 0.0;
	int32 CachedOwnedNodeCount = 0;
	int32 CachedOwnedEdgeCount = 0;
	EGTAIStrategy CachedStrategy = EGTAIStrategy::Expand;

protected:
	/** Corporation ID this controller manages. */
	UPROPERTY()
	int32 CorporationId = -1;

	/** AI personality configuration. */
	UPROPERTY()
	FGTAIArchetypeData Archetype;

	/** World-settings aggressiveness multiplier. */
	UPROPERTY()
	float AggressivenessMultiplier = 1.0f;

	/** Behavior tree component for AI decision-making. */
	UPROPERTY()
	TObjectPtr<UBehaviorTreeComponent> BTComponent;

private:
	/** Build and start the behavior tree programmatically. */
	void ConstructAndRunBehaviorTree();

	/** Cache subsystem references for fast access during BT ticks. */
	void CacheSubsystems();

	/** Handle an economic tick event from the simulation. */
	void OnEconomicTick(const struct FGTSimulationEvent& Event);

	// Cached subsystem pointers (non-UPROPERTY since subsystems are managed by UWorld).
	UGTCorporationManager* CachedCorpManager = nullptr;
	UGTNetworkGraph* CachedNetworkGraph = nullptr;
	UGTLandParcelSystem* CachedParcelSystem = nullptr;
	UGTRegionalEconomy* CachedRegionalEconomy = nullptr;
	UGTAllianceManager* CachedAllianceManager = nullptr;
	UGTSimulationSubsystem* CachedSimSubsystem = nullptr;

	/** Delegate handle for event subscription. */
	FDelegateHandle EventDelegateHandle;
};
