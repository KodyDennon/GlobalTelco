#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "GTSimulationTypes.h"
#include "GTLandParcelSystem.h"
#include "GTHexGridRenderer.generated.h"

class UInstancedStaticMeshComponent;
class UStaticMesh;
class UMaterialInterface;
class UMaterialInstanceDynamic;
class UGTGeodesicGrid;
class UGTLandParcelSystem;
class ACesiumGeoreference;

/**
 * AGTHexGridRenderer
 *
 * Actor that renders the hex grid overlay on the Cesium globe using
 * Instanced Static Mesh components. Handles:
 * - LOD: Only renders visible hexes based on camera distance
 * - Color coding: terrain type, ownership, zoning
 * - Selection highlight on hover/click
 * - Far zoom: region outlines only
 * - Close zoom: individual hex parcels with terrain coloring
 *
 * Uses UInstancedStaticMeshComponent for GPU-instanced rendering of 100k+ hexes.
 */
UCLASS()
class GTFRONTEND_API AGTHexGridRenderer : public AActor
{
	GENERATED_BODY()

public:
	AGTHexGridRenderer();

	virtual void BeginPlay() override;
	virtual void Tick(float DeltaTime) override;

	// --- Configuration ---

	/** Static mesh to use for hex tiles (assign a flat hex mesh in editor). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Hex Rendering")
	TObjectPtr<UStaticMesh> HexTileMesh;

	/** Base material for hex tiles (should support per-instance color). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Hex Rendering")
	TObjectPtr<UMaterialInterface> HexTileMaterial;

	/** Material for the selected/highlighted hex. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Hex Rendering")
	TObjectPtr<UMaterialInterface> HexHighlightMaterial;

	/** Hex tile scale relative to actual hex cell size. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Hex Rendering", meta = (ClampMin = "0.1", ClampMax = "2.0"))
	float HexScale = 0.95f;

	/** Maximum camera altitude at which individual hexes are rendered. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Hex Rendering|LOD")
	double MaxHexRenderAltitude = 500000.0;

	/** Camera altitude below which hexes transition from region to individual mode. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Hex Rendering|LOD")
	double HexDetailAltitude = 200000.0;

	/** Maximum number of hex instances to render at once for performance. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Hex Rendering|LOD")
	int32 MaxVisibleInstances = 10000;

	// --- Runtime API ---

	/** Highlight a specific hex cell (by geodesic cell index). Pass -1 to clear. */
	UFUNCTION(BlueprintCallable, Category = "Hex Rendering")
	void HighlightCell(int32 CellIndex);

	/** Rebuild the visible hex instances based on current camera position. */
	UFUNCTION(BlueprintCallable, Category = "Hex Rendering")
	void RefreshVisibleHexes();

	/** Set the color mode for hex rendering. */
	UFUNCTION(BlueprintCallable, Category = "Hex Rendering")
	void SetColorMode(EGTTerrainType DebugTerrainFilter);

protected:
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Components")
	TObjectPtr<UInstancedStaticMeshComponent> HexISM;

	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Components")
	TObjectPtr<UInstancedStaticMeshComponent> HighlightISM;

	/** Get color for a terrain type. */
	static FLinearColor GetTerrainColor(EGTTerrainType Terrain);

	/** Get color for an ownership type. */
	static FLinearColor GetOwnershipColor(EGTParcelOwnership Ownership);

	/** Convert a geodesic cell's lon/lat to Unreal world position via Cesium georeference. */
	FTransform GetCellWorldTransform(int32 CellIndex, const UGTGeodesicGrid* Grid, const ACesiumGeoreference* Georef) const;

private:
	/** Currently highlighted cell index (-1 = none). */
	int32 HighlightedCellIndex = -1;

	/** Cached camera altitude from last refresh. */
	double LastCameraAltitude = 0.0;

	/** Cached camera position from last refresh. */
	FVector LastCameraPosition = FVector::ZeroVector;

	/** Minimum camera movement before re-rendering. */
	static constexpr double CameraMovementThreshold = 1000.0;
};
