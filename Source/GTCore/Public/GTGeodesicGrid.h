#pragma once

#include "CoreMinimal.h"
#include "Subsystems/WorldSubsystem.h"
#include "GTHexGrid.h"
#include "GTGeodesicGrid.generated.h"

/**
 * FGTGeodesicCell
 *
 * A single cell in the geodesic grid, linking a hex coordinate to its
 * position on the sphere (longitude, latitude) and world-space 3D position.
 */
USTRUCT(BlueprintType)
struct GTCORE_API FGTGeodesicCell
{
	GENERATED_BODY()

	/** Index into the geodesic grid's cell array. */
	UPROPERTY(BlueprintReadOnly, Category = "Geodesic")
	int32 CellIndex = -1;

	/** Hex coordinate for grid math (neighbors, distance, etc). */
	UPROPERTY(BlueprintReadOnly, Category = "Geodesic")
	FGTHexCoord HexCoord;

	/** Longitude in degrees (-180 to 180). */
	UPROPERTY(BlueprintReadOnly, Category = "Geodesic")
	double Longitude = 0.0;

	/** Latitude in degrees (-90 to 90). */
	UPROPERTY(BlueprintReadOnly, Category = "Geodesic")
	double Latitude = 0.0;

	/** Unit sphere position (X, Y, Z). */
	UPROPERTY(BlueprintReadOnly, Category = "Geodesic")
	FVector UnitSpherePosition = FVector::ZeroVector;

	/** Normalized elevation (-1.0 deep ocean to 1.0 mountain peak). Set during world generation. */
	UPROPERTY(BlueprintReadOnly, Category = "Geodesic")
	float Elevation = 0.0f;

	/** Whether this cell is land (true) or water (false). Set during world generation. */
	UPROPERTY(BlueprintReadOnly, Category = "Geodesic")
	bool bIsLand = true;
};

/**
 * UGTGeodesicGrid
 *
 * World subsystem that generates and manages an icosahedral geodesic grid
 * mapping hex cells onto the surface of a sphere. Uses recursive subdivision
 * of an icosahedron to produce approximately uniform hex cells.
 *
 * Total cell count is approximately 10 * Frequency^2 + 2 (12 pentagons, rest hexagons).
 * At Frequency=100, this gives ~100,002 cells.
 *
 * Provides:
 * - Cell generation from icosahedral subdivision
 * - Longitude/latitude lookup by cell index
 * - Nearest-cell lookup from longitude/latitude
 * - Neighbor relationships (6 per hex, 5 per pentagon)
 */
UCLASS()
class GTCORE_API UGTGeodesicGrid : public UWorldSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;

	/**
	 * Generate the geodesic grid with the given frequency (subdivision level).
	 * Must be called before any lookups. Typically called by the world generator.
	 */
	UFUNCTION(BlueprintCallable, Category = "Geodesic Grid")
	void GenerateGrid(int32 Frequency);

	/** Total number of cells in the grid. */
	UFUNCTION(BlueprintPure, Category = "Geodesic Grid")
	int32 GetCellCount() const { return Cells.Num(); }

	/** Get a cell by index. */
	UFUNCTION(BlueprintPure, Category = "Geodesic Grid")
	const FGTGeodesicCell& GetCell(int32 CellIndex) const;

	/** Find the nearest cell to a given longitude/latitude. */
	UFUNCTION(BlueprintPure, Category = "Geodesic Grid")
	int32 FindNearestCell(double Longitude, double Latitude) const;

	/** Find the nearest cell to a unit sphere position. */
	int32 FindNearestCellFromUnitPosition(const FVector& UnitPos) const;

	/** Get neighbor cell indices for a given cell. */
	UFUNCTION(BlueprintPure, Category = "Geodesic Grid")
	TArray<int32> GetCellNeighbors(int32 CellIndex) const;

	/** Set cell elevation and land flag. Called by world generator after terrain assignment. */
	void SetCellTerrain(int32 CellIndex, float InElevation, bool bInIsLand);

	/** Whether the grid has been generated. */
	UFUNCTION(BlueprintPure, Category = "Geodesic Grid")
	bool IsGridGenerated() const { return Cells.Num() > 0; }

	/** Get all cells (read-only reference for batch operations). */
	const TArray<FGTGeodesicCell>& GetAllCells() const { return Cells; }

private:
	/** Generate vertex positions on the unit sphere via icosahedral subdivision. */
	void SubdivideIcosahedron(int32 Frequency, TArray<FVector>& OutVertices);

	/** Remove duplicate vertices within a tolerance using spatial hashing. */
	void DeduplicateVertices(TArray<FVector>& Vertices, double Tolerance);

	/** Convert unit sphere position to longitude/latitude in degrees. */
	static void CartesianToLonLat(const FVector& Pos, double& OutLon, double& OutLat);

	/** Convert longitude/latitude in degrees to unit sphere position. */
	static FVector LonLatToCartesian(double Longitude, double Latitude);

	/** Build neighbor adjacency using spatial hash for O(n) performance. */
	void BuildNeighborAdjacency();

	/** Build a spatial hash grid for fast nearest-cell queries. */
	void BuildSpatialHash();

	/** Convert unit sphere position to spatial hash bucket key. */
	int64 SpatialHashKey(const FVector& Pos) const;

	UPROPERTY()
	TArray<FGTGeodesicCell> Cells;

	/** Adjacency: CellIndex -> array of neighbor CellIndices. */
	TMap<int32, TArray<int32>> Adjacency;

	/** Spatial hash: bucket key -> array of cell indices in that bucket. */
	TMap<int64, TArray<int32>> SpatialHash;

	/** Spatial hash bucket size (in unit sphere coordinate space). */
	float SpatialHashBucketSize = 0.05f;

	/** Sentinel cell returned for invalid lookups. */
	static const FGTGeodesicCell InvalidCell;
};
