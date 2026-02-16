#pragma once

#include "CoreMinimal.h"
#include "GTGeoCoordinates.generated.h"

/**
 * UGTGeoCoordinates
 *
 * Pure-math geographic coordinate utility library. Converts between
 * WGS84 lon/lat/height and Unreal Engine world positions without
 * requiring Cesium or any external plugin. This is the offline fallback
 * used when Cesium streaming is unavailable (singleplayer, no internet).
 *
 * Uses a simplified spherical Earth model with radius 6,371,000 meters.
 * All world positions are in UE centimeters.
 */
UCLASS()
class GTCORE_API UGTGeoCoordinates : public UBlueprintFunctionLibrary
{
	GENERATED_BODY()

public:
	/** Earth radius in meters. */
	static constexpr double EarthRadiusMeters = 6371000.0;

	/** Earth radius in UE centimeters. */
	static constexpr double EarthRadiusCm = EarthRadiusMeters * 100.0;

	/** Scale factor: how many UE cm per real-world meter for the globe. */
	static constexpr double GlobeScale = 0.01;

	/** Effective globe radius in UE cm (scaled down for rendering). */
	static constexpr double GlobeRadiusCm = EarthRadiusMeters * GlobeScale;

	/**
	 * Convert lon/lat/height to Unreal world position (offline mode).
	 * Globe is centered at world origin. Height is meters above surface.
	 */
	UFUNCTION(BlueprintPure, Category = "Geo Coordinates")
	static FVector LonLatHeightToWorld(double Longitude, double Latitude, double HeightMeters);

	/**
	 * Convert Unreal world position back to lon/lat/height (offline mode).
	 * Returns FVector(Longitude, Latitude, HeightMeters).
	 */
	UFUNCTION(BlueprintPure, Category = "Geo Coordinates")
	static FVector WorldToLonLatHeight(const FVector& WorldPosition);

	/**
	 * Convert lon/lat to a unit sphere direction vector.
	 * Useful for dot-product-based nearest-cell lookups.
	 */
	UFUNCTION(BlueprintPure, Category = "Geo Coordinates")
	static FVector LonLatToUnitSphere(double Longitude, double Latitude);

	/**
	 * Convert a unit sphere direction vector to lon/lat.
	 * Returns FVector2D(Longitude, Latitude) in degrees.
	 */
	UFUNCTION(BlueprintPure, Category = "Geo Coordinates")
	static FVector2D UnitSphereToLonLat(const FVector& UnitDirection);

	/**
	 * Get the surface normal (up vector) at a given lon/lat.
	 * This is the same as the unit sphere direction.
	 */
	UFUNCTION(BlueprintPure, Category = "Geo Coordinates")
	static FVector GetSurfaceNormal(double Longitude, double Latitude);

	/**
	 * Compute the rotation that orients an object on the globe surface
	 * with Z pointing away from the center (surface normal as up).
	 */
	UFUNCTION(BlueprintPure, Category = "Geo Coordinates")
	static FRotator GetSurfaceRotation(double Longitude, double Latitude);
};
