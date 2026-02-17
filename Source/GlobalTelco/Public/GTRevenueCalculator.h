#pragma once

#include "CoreMinimal.h"
#include "Subsystems/WorldSubsystem.h"
#include "GTRevenueCalculator.generated.h"

class UGTCorporation;
class UGTCorporationManager;
class UGTNetworkGraph;
class UGTRegionalEconomy;
class UGTLandParcelSystem;

/**
 * FGTCorpRevenueBreakdown
 *
 * Per-corporation revenue and cost breakdown for one economic tick.
 */
USTRUCT(BlueprintType)
struct FGTCorpRevenueBreakdown
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly)
	int32 CorporationId = -1;

	/** Revenue from bandwidth delivery (serving regional demand). */
	UPROPERTY(BlueprintReadOnly)
	double BandwidthRevenue = 0.0;

	/** Revenue from transit agreements (other corps routing through your edges). */
	UPROPERTY(BlueprintReadOnly)
	double TransitRevenue = 0.0;

	/** Revenue from peering contracts. */
	UPROPERTY(BlueprintReadOnly)
	double PeeringRevenue = 0.0;

	/** Total maintenance cost for all owned nodes. */
	UPROPERTY(BlueprintReadOnly)
	double NodeMaintenanceCost = 0.0;

	/** Total maintenance cost for all owned edges. */
	UPROPERTY(BlueprintReadOnly)
	double EdgeMaintenanceCost = 0.0;

	/** Total interest expense across all debt instruments. */
	UPROPERTY(BlueprintReadOnly)
	double InterestExpense = 0.0;

	double TotalRevenue() const { return BandwidthRevenue + TransitRevenue + PeeringRevenue; }
	double TotalCosts() const { return NodeMaintenanceCost + EdgeMaintenanceCost + InterestExpense; }
	double NetIncome() const { return TotalRevenue() - TotalCosts(); }
};

/**
 * UGTRevenueCalculator
 *
 * World subsystem that computes per-corporation revenue and costs each
 * economic tick. Connects infrastructure performance to financial outcomes:
 *
 * Revenue sources:
 * - Bandwidth delivery: owned nodes serve regional demand proportional
 *   to capacity, SLA quality, and demand satisfaction ratio.
 * - Transit: owned edges carry traffic for other corps (future).
 * - Peering contracts: fixed revenue from active peering agreements.
 *
 * Cost sources:
 * - Node maintenance: per-node cost from FGTNodeAttributes::MaintenanceCostPerTick.
 * - Edge maintenance: per-edge cost from FGTEdgeAttributes::MaintenanceCostPerTick.
 * - Terrain multipliers applied to maintenance.
 * - World settings multipliers applied to all costs.
 */
UCLASS()
class GLOBALTELCO_API UGTRevenueCalculator : public UWorldSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;

	/**
	 * Calculate revenue and costs for all corporations and apply to their financials.
	 * Called once per economic tick by the simulation subsystem or game mode.
	 */
	UFUNCTION(BlueprintCallable, Category = "Revenue")
	void ProcessRevenueTick(float MaintenanceCostMultiplier, float DemandGrowthMultiplier);

	/** Get the last computed breakdown for a corporation. */
	UFUNCTION(BlueprintPure, Category = "Revenue")
	FGTCorpRevenueBreakdown GetLastBreakdown(int32 CorporationId) const;

	/** Get all last computed breakdowns. */
	UFUNCTION(BlueprintPure, Category = "Revenue")
	TArray<FGTCorpRevenueBreakdown> GetAllBreakdowns() const;

	/**
	 * Update regional connectivity scores based on current infrastructure.
	 * Scans all operational nodes per region and computes a 0-1 connectivity score.
	 */
	UFUNCTION(BlueprintCallable, Category = "Revenue")
	void UpdateRegionalConnectivity();

private:
	/** Calculate bandwidth revenue for a single corporation. */
	double CalculateBandwidthRevenue(UGTCorporation* Corp, UGTNetworkGraph* Graph,
		UGTRegionalEconomy* Economy, UGTLandParcelSystem* ParcelSystem,
		float DemandGrowthMultiplier) const;

	/** Calculate total node maintenance cost for a corporation. */
	double CalculateNodeMaintenance(UGTCorporation* Corp, UGTNetworkGraph* Graph,
		float CostMultiplier) const;

	/** Calculate total edge maintenance cost for a corporation. */
	double CalculateEdgeMaintenance(UGTCorporation* Corp, UGTNetworkGraph* Graph,
		float CostMultiplier) const;

	/** Cached breakdowns from last tick. */
	UPROPERTY()
	TMap<int32, FGTCorpRevenueBreakdown> LastBreakdowns;
};
