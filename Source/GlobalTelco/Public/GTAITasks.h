#pragma once

#include "CoreMinimal.h"
#include "BehaviorTree/BTTaskNode.h"
#include "BehaviorTree/BTService.h"
#include "BehaviorTree/BTDecorator.h"
#include "GTAIArchetype.h"
#include "GTAITasks.generated.h"

class UGTCorporationManager;
class UGTNetworkGraph;
class UGTLandParcelSystem;
class UGTRegionalEconomy;
class UGTAllianceManager;
class UGTCorporation;

/** AI strategic state — drives which branch of the behavior tree executes. */
UENUM(BlueprintType)
enum class EGTAIStrategy : uint8
{
	Expand,
	Consolidate,
	Compete,
	Survive
};

// ---------------------------------------------------------------------------
// BT Service: Evaluates world state each tick and updates blackboard
// ---------------------------------------------------------------------------

/**
 * UBTService_GTUpdateWorldState
 *
 * Runs on every BT tick. Reads the corporation's current financial state,
 * owned assets, and market conditions. Updates the blackboard and determines
 * the current strategic state based on archetype weights and conditions.
 */
UCLASS()
class GLOBALTELCO_API UBTService_GTUpdateWorldState : public UBTService
{
	GENERATED_BODY()

public:
	UBTService_GTUpdateWorldState();
	virtual void TickNode(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory, float DeltaSeconds) override;

protected:
	virtual FString GetStaticDescription() const override { return TEXT("Update AI world state"); }
};

// ---------------------------------------------------------------------------
// BT Task: Acquire Land
// ---------------------------------------------------------------------------

/**
 * UBTTask_GTAcquireLand
 *
 * Scans government-owned parcels and purchases the most strategically
 * valuable one. Scoring considers: regional demand, terrain suitability,
 * proximity to existing owned infrastructure, cost efficiency, and
 * archetype-driven preferences.
 */
UCLASS()
class GLOBALTELCO_API UBTTask_GTAcquireLand : public UBTTaskNode
{
	GENERATED_BODY()

public:
	UBTTask_GTAcquireLand();
	virtual EBTNodeResult::Type ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) override;

protected:
	virtual FString GetStaticDescription() const override { return TEXT("Acquire land parcel"); }

private:
	/** Score a parcel for acquisition. Higher = more desirable. */
	double ScoreParcel(const struct FGTLandParcel& Parcel, UGTCorporation* Corp,
		UGTRegionalEconomy* Economy, UGTLandParcelSystem* ParcelSystem,
		const FGTAIArchetypeData& Archetype, float AggressivenessMultiplier) const;
};

// ---------------------------------------------------------------------------
// BT Task: Build Infrastructure Node
// ---------------------------------------------------------------------------

/**
 * UBTTask_GTBuildNode
 *
 * Finds owned parcels lacking infrastructure and constructs the most
 * appropriate node type based on regional demand, existing network
 * topology, and archetype preferences. Fires an InfrastructureBuilt event.
 */
UCLASS()
class GLOBALTELCO_API UBTTask_GTBuildNode : public UBTTaskNode
{
	GENERATED_BODY()

public:
	UBTTask_GTBuildNode();
	virtual EBTNodeResult::Type ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) override;

protected:
	virtual FString GetStaticDescription() const override { return TEXT("Build infrastructure node"); }
};

// ---------------------------------------------------------------------------
// BT Task: Build Network Edge
// ---------------------------------------------------------------------------

/**
 * UBTTask_GTBuildEdge
 *
 * Connects pairs of owned nodes with network edges. Chooses edge type
 * based on distance, terrain, and node hierarchy levels. Prioritizes
 * closing network gaps and improving connectivity.
 */
UCLASS()
class GLOBALTELCO_API UBTTask_GTBuildEdge : public UBTTaskNode
{
	GENERATED_BODY()

public:
	UBTTask_GTBuildEdge();
	virtual EBTNodeResult::Type ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) override;

protected:
	virtual FString GetStaticDescription() const override { return TEXT("Build network edge"); }
};

// ---------------------------------------------------------------------------
// BT Task: Manage Finances
// ---------------------------------------------------------------------------

/**
 * UBTTask_GTManageFinances
 *
 * Reviews the corporation's financial health and takes corrective actions:
 * - Takes loans when cash is low but credit is good
 * - Pays down debt when cash reserves are high
 * - Adjusts spending behavior based on financial stress
 */
UCLASS()
class GLOBALTELCO_API UBTTask_GTManageFinances : public UBTTaskNode
{
	GENERATED_BODY()

public:
	UBTTask_GTManageFinances();
	virtual EBTNodeResult::Type ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) override;

protected:
	virtual FString GetStaticDescription() const override { return TEXT("Manage finances"); }
};

// ---------------------------------------------------------------------------
// BT Task: Propose Contract
// ---------------------------------------------------------------------------

/**
 * UBTTask_GTProposeContract
 *
 * Identifies nearby corporations and proposes mutually beneficial contracts.
 * Contract terms are influenced by archetype (aggressive = undercut pricing,
 * prudent = fair market rates). Handles peering, transit, and capacity contracts.
 */
UCLASS()
class GLOBALTELCO_API UBTTask_GTProposeContract : public UBTTaskNode
{
	GENERATED_BODY()

public:
	UBTTask_GTProposeContract();
	virtual EBTNodeResult::Type ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) override;

protected:
	virtual FString GetStaticDescription() const override { return TEXT("Propose contract"); }
};

// ---------------------------------------------------------------------------
// BT Decorator: Check Strategy
// ---------------------------------------------------------------------------

/**
 * UBTDecorator_GTCheckStrategy
 *
 * Gates a behavior tree branch based on the current AI strategic state.
 * The strategy is set by UBTService_GTUpdateWorldState each tick.
 */
UCLASS()
class GLOBALTELCO_API UBTDecorator_GTCheckStrategy : public UBTDecorator
{
	GENERATED_BODY()

public:
	UBTDecorator_GTCheckStrategy();

	UPROPERTY(EditAnywhere, Category = "AI")
	EGTAIStrategy RequiredStrategy = EGTAIStrategy::Expand;

	virtual bool CalculateRawConditionValue(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) const override;
	virtual FString GetStaticDescription() const override;
};

// ---------------------------------------------------------------------------
// BT Decorator: Check Can Afford
// ---------------------------------------------------------------------------

/**
 * UBTDecorator_GTCheckCanAfford
 *
 * Gates a branch based on whether the corporation has enough cash
 * to perform the next action (adjusted by archetype's minimum reserve ratio).
 */
UCLASS()
class GLOBALTELCO_API UBTDecorator_GTCheckCanAfford : public UBTDecorator
{
	GENERATED_BODY()

public:
	UBTDecorator_GTCheckCanAfford();

	/** Minimum cash threshold to pass this check. */
	UPROPERTY(EditAnywhere, Category = "AI")
	double MinimumCash = 100000.0;

	virtual bool CalculateRawConditionValue(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) const override;
	virtual FString GetStaticDescription() const override;
};
