#pragma once

#include "CoreMinimal.h"
#include "Subsystems/WorldSubsystem.h"
#include "GTEconomyTypes.h"
#include "GTRegionalEconomy.generated.h"

/**
 * UGTRegionalEconomy
 *
 * World subsystem that manages regional economic data across the globe.
 * Each region's population, GDP, technology adoption, and data demand
 * evolve over time based on connectivity quality, political events,
 * and infrastructure investment.
 *
 * Connectivity improves GDP growth, business formation, and political
 * stability — creating positive feedback loops that drive demand.
 */
UCLASS()
class GTECONOMY_API UGTRegionalEconomy : public UWorldSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;

	/** Register a new region. Returns the assigned RegionId. */
	UFUNCTION(BlueprintCallable, Category = "Regional Economy")
	int32 RegisterRegion(const FGTRegionalEconomyData& Data);

	/** Get economic data for a region. */
	UFUNCTION(BlueprintPure, Category = "Regional Economy")
	FGTRegionalEconomyData GetRegionData(int32 RegionId) const;

	/** Update all regions for one economic tick. */
	UFUNCTION(BlueprintCallable, Category = "Regional Economy")
	void ProcessEconomicTick(float TickDeltaSeconds);

	/** Calculate total data demand across all regions. */
	UFUNCTION(BlueprintPure, Category = "Regional Economy")
	double GetGlobalDemand() const;

	/** Update a region's connectivity score (0-1) based on infrastructure coverage. */
	UFUNCTION(BlueprintCallable, Category = "Regional Economy")
	void SetRegionConnectivity(int32 RegionId, float ConnectivityScore);

private:
	UPROPERTY()
	TMap<int32, FGTRegionalEconomyData> Regions;

	/** Per-region connectivity scores from infrastructure. */
	TMap<int32, float> ConnectivityScores;

	int32 NextRegionId = 0;
};
