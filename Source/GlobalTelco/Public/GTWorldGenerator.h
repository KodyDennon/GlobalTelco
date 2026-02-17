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
 * - Elevation map using multi-octave fractal noise on sphere
 * - All hex parcels with terrain, zoning, and regulatory data
 * - Geographic regions by grouping adjacent parcels (land-aware K-means)
 * - Initial economic data per region (population, GDP, stability)
 *
 * Called once at world start. After generation, the land parcel system
 * and regional economy subsystem own the data.
 *
 * Noise system uses 3D sphere-based sampling to avoid lon/lat seam
 * artifacts. Elevation drives terrain type, which drives economics.
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
	/** Generate elevation map on the geodesic grid using fractal noise. */
	void GenerateElevation(UGTGeodesicGrid* Grid, int32 Seed);

	/** Generate all hex parcels from the geodesic grid. */
	void GenerateParcels(UGTWorldSettings* Settings, UGTGeodesicGrid* Grid, UGTLandParcelSystem* ParcelSystem);

	/** Assign terrain types to parcels based on elevation, latitude, and biome noise. */
	void AssignTerrain(UGTWorldSettings* Settings, UGTLandParcelSystem* ParcelSystem, UGTGeodesicGrid* Grid);

	/** Create regions by grouping adjacent parcels (land-aware K-means on sphere). */
	void GenerateRegions(UGTWorldSettings* Settings, UGTGeodesicGrid* Grid, UGTLandParcelSystem* ParcelSystem, UGTRegionalEconomy* Economy);

	/** Assign initial economic data to each region based on terrain composition. */
	void SeedEconomicData(UGTWorldSettings* Settings, UGTRegionalEconomy* Economy, UGTLandParcelSystem* ParcelSystem);

	/** Assign zoning categories based on terrain and region properties. */
	void AssignZoning(UGTWorldSettings* Settings, UGTLandParcelSystem* ParcelSystem);

private:
	/** 2D value noise with smoothstep interpolation. Used for per-region variation. */
	static double Noise2D(double X, double Y, int32 Seed);

	/** 3D value noise with trilinear interpolation. Core building block for sphere noise. */
	static double Noise3D(double X, double Y, double Z, int32 Seed);

	/** Fractal Brownian Motion using 3D noise. Samples sphere positions directly to avoid
	 *  lon/lat seam artifacts. Multiple octaves produce natural terrain variation. */
	static double FractalNoise3D(const FVector& Pos, int32 Octaves, double Frequency, double Persistence, int32 Seed);

	bool bWorldGenerated = false;
};
