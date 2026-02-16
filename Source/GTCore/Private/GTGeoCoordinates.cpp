#include "GTGeoCoordinates.h"

FVector UGTGeoCoordinates::LonLatHeightToWorld(double Longitude, double Latitude, double HeightMeters)
{
	const double LonRad = FMath::DegreesToRadians(Longitude);
	const double LatRad = FMath::DegreesToRadians(Latitude);
	const double CosLat = FMath::Cos(LatRad);

	const double RadiusCm = (EarthRadiusMeters + HeightMeters) * GlobeScale;

	// UE coordinate system: X = forward, Y = right, Z = up.
	// We map: X = towards lon=0 lat=0, Y = towards lon=90, Z = towards north pole.
	return FVector(
		RadiusCm * CosLat * FMath::Cos(LonRad),
		RadiusCm * CosLat * FMath::Sin(LonRad),
		RadiusCm * FMath::Sin(LatRad)
	);
}

FVector UGTGeoCoordinates::WorldToLonLatHeight(const FVector& WorldPosition)
{
	const double DistCm = WorldPosition.Size();
	if (DistCm < KINDA_SMALL_NUMBER)
	{
		return FVector(0.0, 0.0, 0.0);
	}

	const FVector Dir = WorldPosition / DistCm;

	const double Latitude = FMath::RadiansToDegrees(FMath::Asin(FMath::Clamp(Dir.Z, -1.0, 1.0)));
	const double Longitude = FMath::RadiansToDegrees(FMath::Atan2(Dir.Y, Dir.X));
	const double HeightMeters = (DistCm / GlobeScale) - EarthRadiusMeters;

	return FVector(Longitude, Latitude, HeightMeters);
}

FVector UGTGeoCoordinates::LonLatToUnitSphere(double Longitude, double Latitude)
{
	const double LonRad = FMath::DegreesToRadians(Longitude);
	const double LatRad = FMath::DegreesToRadians(Latitude);
	const double CosLat = FMath::Cos(LatRad);

	return FVector(
		CosLat * FMath::Cos(LonRad),
		CosLat * FMath::Sin(LonRad),
		FMath::Sin(LatRad)
	);
}

FVector2D UGTGeoCoordinates::UnitSphereToLonLat(const FVector& UnitDirection)
{
	const double Latitude = FMath::RadiansToDegrees(FMath::Asin(FMath::Clamp(UnitDirection.Z, -1.0, 1.0)));
	const double Longitude = FMath::RadiansToDegrees(FMath::Atan2(UnitDirection.Y, UnitDirection.X));
	return FVector2D(Longitude, Latitude);
}

FVector UGTGeoCoordinates::GetSurfaceNormal(double Longitude, double Latitude)
{
	return LonLatToUnitSphere(Longitude, Latitude);
}

FRotator UGTGeoCoordinates::GetSurfaceRotation(double Longitude, double Latitude)
{
	const FVector UpDir = GetSurfaceNormal(Longitude, Latitude);

	// Compute a forward vector tangent to the surface (pointing north).
	const FVector NorthPole(0.0, 0.0, 1.0);
	FVector ForwardDir = FVector::CrossProduct(NorthPole, UpDir);

	if (ForwardDir.IsNearlyZero())
	{
		// At the poles, pick an arbitrary forward.
		ForwardDir = FVector(1.0, 0.0, 0.0);
	}
	else
	{
		ForwardDir.Normalize();
	}

	// Recompute right to ensure orthogonality.
	const FVector RightDir = FVector::CrossProduct(UpDir, ForwardDir).GetSafeNormal();
	ForwardDir = FVector::CrossProduct(RightDir, UpDir).GetSafeNormal();

	return FRotationMatrix::MakeFromXZ(ForwardDir, UpDir).Rotator();
}
