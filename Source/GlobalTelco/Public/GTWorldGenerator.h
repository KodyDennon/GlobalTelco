#pragma once

#include "CoreMinimal.h"
#include "Subsystems/WorldSubsystem.h"
#include "GTSimulationTypes.h"
#include "GTWorldGenerator.generated.h"

class UGTWorldSettings;
class UGTGeodesicGrid;
class UGTLandParcelSystem;
class UGTRegionalEconomy;

/**
 * UGTWorldGenerator
 *
 * World subsystem that generates the initial game world from a
 * UGTWorldSettings data asset and the geodesic grid. Creates:
 * - All hex parcels with terrain, zoning, and regulatory data
 * - Geographic regions by grouping adjacent parcels
 * - Initial economic data per region (population, GDP, stability)
 * - Government-owned seed infrastructure
 *
 * Called once at world start. After generation, the land parcel system
 * and regional economy subsystem own the data.
 */
UCLASS()
class GLOBALTELCO_API UGTWorldGenerator : public UWorldSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;

	/**
	 * Generate the full world from the given settings.
	 * This is the main entry point — call once during world setup.
	 * Populates the geodesic grid, land parcel system, and regional economy.
	 */
	UFUNCTION(BlueprintCallable, Category = "World Generation")
	void GenerateWorld(UGTWorldSettings* Settings);

	/** Whether the world has been generated. */
	UFUNCTION(BlueprintPure, Category = "World Generation")
	bool IsWorldGenerated() const { return bWorldGenerated; }

protected:
	/** Generate all hex parcels from the geodesic grid. */
	void GenerateParcels(UGTWorldSettings* Settings, UGTGeodesicGrid* Grid, UGTLandParcelSystem* ParcelSystem);

	/** Assign terrain types to parcels based on latitude, elevation heuristics, and noise. */
	void AssignTerrain(UGTWorldSettings* Settings, UGTLandParcelSystem* ParcelSystem, UGTGeodesicGrid* Grid);

	/** Create regions by grouping adjacent parcels (K-means on sphere). */
	void GenerateRegions(UGTWorldSettings* Settings, UGTGeodesicGrid* Grid, UGTLandParcelSystem* ParcelSystem, UGTRegionalEconomy* Economy);

	/** Assign initial economic data to each region. */
	void SeedEconomicData(UGTWorldSettings* Settings, UGTRegionalEconomy* Economy, UGTLandParcelSystem* ParcelSystem);

	/** Assign zoning categories based on terrain and region properties. */
	void AssignZoning(UGTWorldSettings* Settings, UGTLandParcelSystem* ParcelSystem);

private:
	/** Deterministic Perlin-like noise for terrain generation. */
	static double Noise2D(double X, double Y, int32 Seed);

	bool bWorldGenerated = false;
};
