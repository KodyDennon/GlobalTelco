#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "GTGlobeActor.generated.h"

class ACesiumGeoreference;
class ACesium3DTileset;
class ACesiumSunSky;
class UStaticMeshComponent;
class UMaterialInstanceDynamic;

/**
 * AGTGlobeActor
 *
 * Top-level actor that configures the globe for the GlobalTelco world.
 * Supports two modes:
 *
 * Online mode (bUseRealEarth = true):
 *   Spawns Cesium 3D Tileset with streaming terrain/imagery from Cesium ion.
 *   Requires internet connection.
 *
 * Offline mode (bUseRealEarth = false):
 *   Renders a procedural sphere mesh with a solid color or bundled texture.
 *   Works fully offline for singleplayer. Uses UGTGeoCoordinates for
 *   all coordinate conversions instead of Cesium georeference.
 *
 * Place one of these in the level — it handles all globe setup on BeginPlay.
 */
UCLASS()
class GLOBALTELCO_API AGTGlobeActor : public AActor
{
	GENERATED_BODY()

public:
	AGTGlobeActor();

	virtual void BeginPlay() override;

	/** Whether to use real Cesium streaming terrain or a simple procedural sphere. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Globe")
	bool bUseRealEarth = true;

	/** Cesium ion access token for streaming terrain and imagery. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Globe")
	FString CesiumIonAccessToken;

	/** Cesium ion asset ID for world terrain (default: Cesium World Terrain). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Globe")
	int64 TerrainAssetId = 1;

	/** Cesium ion asset ID for imagery overlay (default: Bing Maps Aerial). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Globe")
	int64 ImageryAssetId = 2;

	/** Starting longitude for the initial camera view. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Globe|Initial View")
	double StartLongitude = 0.0;

	/** Starting latitude for the initial camera view. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Globe|Initial View")
	double StartLatitude = 20.0;

	/** Starting altitude in meters above the globe surface. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Globe|Initial View")
	double StartAltitude = 20000000.0;

	/** Whether we're running in offline mode (no Cesium). */
	UFUNCTION(BlueprintPure, Category = "Globe")
	bool IsOfflineMode() const { return !bUseRealEarth; }

	UFUNCTION(BlueprintPure, Category = "Globe")
	ACesiumGeoreference* GetGeoreference() const { return Georeference; }

	UFUNCTION(BlueprintPure, Category = "Globe")
	ACesium3DTileset* GetTerrainTileset() const { return TerrainTileset; }

	/** Get the offline sphere mesh (only valid when bUseRealEarth = false). */
	UFUNCTION(BlueprintPure, Category = "Globe")
	UStaticMeshComponent* GetOfflineGlobeMesh() const { return OfflineGlobeMesh; }

	/**
	 * Convert lon/lat/height to world position. Automatically uses Cesium
	 * georeference if available, otherwise falls back to pure math.
	 */
	UFUNCTION(BlueprintPure, Category = "Globe")
	FVector LonLatHeightToWorld(double Longitude, double Latitude, double HeightMeters) const;

	/**
	 * Convert world position to lon/lat/height. Automatically uses Cesium
	 * georeference if available, otherwise falls back to pure math.
	 */
	UFUNCTION(BlueprintPure, Category = "Globe")
	FVector WorldToLonLatHeight(const FVector& WorldPosition) const;

protected:
	/** Set up the Cesium georeference for the world (online mode). */
	void InitializeGeoreference();

	/** Spawn and configure the terrain 3D tileset (online mode). */
	void InitializeTerrainTileset();

	/** Spawn and configure the sun/sky actor (online mode). */
	void InitializeSunSky();

	/** Create a procedural sphere mesh for the offline globe. */
	void InitializeOfflineGlobe();

private:
	UPROPERTY()
	TObjectPtr<ACesiumGeoreference> Georeference;

	UPROPERTY()
	TObjectPtr<ACesium3DTileset> TerrainTileset;

	UPROPERTY()
	TObjectPtr<ACesiumSunSky> SunSky;

	UPROPERTY(VisibleAnywhere)
	TObjectPtr<UStaticMeshComponent> OfflineGlobeMesh;
};
