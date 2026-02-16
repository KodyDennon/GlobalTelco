#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "GTGlobeActor.generated.h"

class ACesiumGeoreference;
class ACesium3DTileset;
class ACesiumSunSky;

/**
 * AGTGlobeActor
 *
 * Top-level actor that configures the Cesium globe for the GlobalTelco world.
 * Spawns and configures the CesiumGeoreference, Cesium World Terrain tileset,
 * Bing Maps imagery, and CesiumSunSky for correct lighting.
 *
 * Place one of these in the level — it handles all globe setup on BeginPlay.
 * Supports switching between real-Earth mode (Cesium streaming) and
 * procedural-generation mode (for offline/testing).
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

	UFUNCTION(BlueprintPure, Category = "Globe")
	ACesiumGeoreference* GetGeoreference() const { return Georeference; }

	UFUNCTION(BlueprintPure, Category = "Globe")
	ACesium3DTileset* GetTerrainTileset() const { return TerrainTileset; }

protected:
	/** Set up the Cesium georeference for the world. */
	void InitializeGeoreference();

	/** Spawn and configure the terrain 3D tileset. */
	void InitializeTerrainTileset();

	/** Spawn and configure the sun/sky actor for correct globe lighting. */
	void InitializeSunSky();

private:
	UPROPERTY()
	TObjectPtr<ACesiumGeoreference> Georeference;

	UPROPERTY()
	TObjectPtr<ACesium3DTileset> TerrainTileset;

	UPROPERTY()
	TObjectPtr<ACesiumSunSky> SunSky;
};
