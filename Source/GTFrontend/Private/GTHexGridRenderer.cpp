#include "GTHexGridRenderer.h"
#include "Components/InstancedStaticMeshComponent.h"
#include "GTGeodesicGrid.h"
#include "GTGeoCoordinates.h"
#include "GTLandParcelSystem.h"
#include "CesiumGeoreference.h"
#include "Engine/World.h"
#include "GameFramework/PlayerController.h"
#include "Camera/CameraComponent.h"

AGTHexGridRenderer::AGTHexGridRenderer()
{
	PrimaryActorTick.bCanEverTick = true;
	PrimaryActorTick.TickInterval = 0.1f; // Refresh at 10 Hz, not every frame.

	RootComponent = CreateDefaultSubobject<USceneComponent>(TEXT("Root"));

	HexISM = CreateDefaultSubobject<UInstancedStaticMeshComponent>(TEXT("HexISM"));
	HexISM->SetupAttachment(RootComponent);
	HexISM->SetCollisionEnabled(ECollisionEnabled::NoCollision);
	HexISM->SetCastShadow(false);
	HexISM->NumCustomDataFloats = 4; // RGBA per-instance color.

	HighlightISM = CreateDefaultSubobject<UInstancedStaticMeshComponent>(TEXT("HighlightISM"));
	HighlightISM->SetupAttachment(RootComponent);
	HighlightISM->SetCollisionEnabled(ECollisionEnabled::NoCollision);
	HighlightISM->SetCastShadow(false);
}

void AGTHexGridRenderer::BeginPlay()
{
	Super::BeginPlay();

	if (HexTileMesh)
	{
		HexISM->SetStaticMesh(HexTileMesh);
		HighlightISM->SetStaticMesh(HexTileMesh);
	}

	if (HexTileMaterial)
	{
		HexISM->SetMaterial(0, HexTileMaterial);
	}

	if (HexHighlightMaterial)
	{
		HighlightISM->SetMaterial(0, HexHighlightMaterial);
	}
}

void AGTHexGridRenderer::Tick(float DeltaTime)
{
	Super::Tick(DeltaTime);

	// Get current camera position to determine LOD.
	APlayerController* PC = GetWorld()->GetFirstPlayerController();
	if (!PC)
	{
		return;
	}

	FVector CameraLocation;
	FRotator CameraRotation;
	PC->GetPlayerViewPoint(CameraLocation, CameraRotation);

	// Only refresh if camera has moved significantly.
	const double MoveDist = FVector::Dist(CameraLocation, LastCameraPosition);
	if (MoveDist < CameraMovementThreshold && HexISM->GetInstanceCount() > 0)
	{
		return;
	}

	LastCameraPosition = CameraLocation;
	RefreshVisibleHexes();
}

void AGTHexGridRenderer::RefreshVisibleHexes()
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	UGTGeodesicGrid* Grid = World->GetSubsystem<UGTGeodesicGrid>();
	UGTLandParcelSystem* ParcelSystem = World->GetSubsystem<UGTLandParcelSystem>();

	if (!Grid || !Grid->IsGridGenerated() || !ParcelSystem)
	{
		return;
	}

	ACesiumGeoreference* Georef = ACesiumGeoreference::GetDefaultGeoreference(World);

	// Clear existing instances.
	HexISM->ClearInstances();

	// Get camera position for LOD calculation.
	APlayerController* PC = World->GetFirstPlayerController();
	if (!PC)
	{
		return;
	}

	FVector CameraLocation;
	FRotator CameraRotation;
	PC->GetPlayerViewPoint(CameraLocation, CameraRotation);

	// Estimate altitude from camera distance to globe center.
	const double CameraDistCm = CameraLocation.Size();
	const double CameraAltitude = (CameraDistCm / UGTGeoCoordinates::GlobeScale) - UGTGeoCoordinates::EarthRadiusMeters;
	LastCameraAltitude = CameraAltitude;

	// If camera is too far, don't render individual hexes.
	if (CameraAltitude > MaxHexRenderAltitude)
	{
		return;
	}

	const int32 CellCount = Grid->GetCellCount();
	int32 RenderedCount = 0;

	// Determine which cells are visible (frustum + distance culling).
	for (int32 i = 0; i < CellCount && RenderedCount < MaxVisibleInstances; ++i)
	{
		const FTransform CellTransform = GetCellWorldTransform(i, Grid, Georef);
		const FVector CellWorldPos = CellTransform.GetLocation();

		// Distance culling: skip cells too far from camera.
		const double DistSq = FVector::DistSquared(CameraLocation, CellWorldPos);
		const double MaxRenderDist = MaxHexRenderAltitude * UGTGeoCoordinates::GlobeScale;
		const double MaxRenderDistSq = MaxRenderDist * MaxRenderDist;
		if (DistSq > MaxRenderDistSq)
		{
			continue;
		}

		// Add instance.
		const int32 InstanceIdx = HexISM->AddInstance(CellTransform, /*bWorldSpace=*/true);

		// Set per-instance color based on terrain.
		const int32 ParcelId = ParcelSystem->FindParcelByCellIndex(i);
		FLinearColor Color = FLinearColor::Gray;

		if (ParcelId >= 0)
		{
			const FGTLandParcel Parcel = ParcelSystem->GetParcel(ParcelId);
			Color = GetTerrainColor(Parcel.Terrain);
		}

		HexISM->SetCustomDataValue(InstanceIdx, 0, Color.R);
		HexISM->SetCustomDataValue(InstanceIdx, 1, Color.G);
		HexISM->SetCustomDataValue(InstanceIdx, 2, Color.B);
		HexISM->SetCustomDataValue(InstanceIdx, 3, Color.A);

		++RenderedCount;
	}
}

void AGTHexGridRenderer::HighlightCell(int32 CellIndex)
{
	HighlightISM->ClearInstances();
	HighlightedCellIndex = CellIndex;

	if (CellIndex < 0)
	{
		return;
	}

	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	UGTGeodesicGrid* Grid = World->GetSubsystem<UGTGeodesicGrid>();
	if (!Grid || !Grid->IsGridGenerated())
	{
		return;
	}

	ACesiumGeoreference* Georef = ACesiumGeoreference::GetDefaultGeoreference(World);

	// Add a slightly scaled-up instance for the highlight.
	FTransform HighlightTransform = GetCellWorldTransform(CellIndex, Grid, Georef);
	HighlightTransform.SetScale3D(HighlightTransform.GetScale3D() * 1.05);
	HighlightISM->AddInstance(HighlightTransform, /*bWorldSpace=*/true);
}

void AGTHexGridRenderer::SetColorMode(EGTTerrainType DebugTerrainFilter)
{
	// Force a refresh with the new color mode.
	RefreshVisibleHexes();
}

FTransform AGTHexGridRenderer::GetCellWorldTransform(int32 CellIndex, const UGTGeodesicGrid* Grid, const ACesiumGeoreference* Georef) const
{
	const FGTGeodesicCell& Cell = Grid->GetCell(CellIndex);

	FVector WorldPos;

	if (Georef)
	{
		WorldPos = Georef->TransformLongitudeLatitudeHeightPositionToUnreal(
			FVector(Cell.Longitude, Cell.Latitude, 10.0));
	}
	else
	{
		// Offline mode: use pure-math coordinate conversion.
		WorldPos = UGTGeoCoordinates::LonLatHeightToWorld(Cell.Longitude, Cell.Latitude, 10.0);
	}

	// Compute orientation: Z-up aligned with sphere normal.
	const FVector UpDir = Cell.UnitSpherePosition.GetSafeNormal();
	const FQuat Rotation = FQuat::FindBetweenNormals(FVector::UpVector, UpDir);

	// Scale based on hex size on the globe surface.
	// At grid resolution 100, each hex covers ~50km radius on Earth.
	const double HexRadiusMeters = 25000.0;
	const double ScaleCm = HexRadiusMeters * UGTGeoCoordinates::GlobeScale * HexScale;

	return FTransform(Rotation, WorldPos, FVector(ScaleCm, ScaleCm, 1.0));
}

FLinearColor AGTHexGridRenderer::GetTerrainColor(EGTTerrainType Terrain)
{
	switch (Terrain)
	{
	case EGTTerrainType::Urban:       return FLinearColor(0.7f, 0.7f, 0.7f, 0.6f);
	case EGTTerrainType::Suburban:    return FLinearColor(0.6f, 0.8f, 0.5f, 0.6f);
	case EGTTerrainType::Rural:       return FLinearColor(0.3f, 0.7f, 0.3f, 0.6f);
	case EGTTerrainType::Mountainous: return FLinearColor(0.6f, 0.4f, 0.2f, 0.6f);
	case EGTTerrainType::Desert:      return FLinearColor(0.9f, 0.8f, 0.4f, 0.6f);
	case EGTTerrainType::Coastal:     return FLinearColor(0.4f, 0.7f, 0.9f, 0.6f);
	case EGTTerrainType::OceanShallow:return FLinearColor(0.2f, 0.4f, 0.8f, 0.4f);
	case EGTTerrainType::OceanDeep:   return FLinearColor(0.1f, 0.2f, 0.6f, 0.3f);
	case EGTTerrainType::Tundra:      return FLinearColor(0.7f, 0.8f, 0.75f, 0.6f);
	case EGTTerrainType::Frozen:      return FLinearColor(0.9f, 0.95f, 1.0f, 0.6f);
	default:                          return FLinearColor(0.5f, 0.5f, 0.5f, 0.5f);
	}
}

FLinearColor AGTHexGridRenderer::GetOwnershipColor(EGTParcelOwnership Ownership)
{
	switch (Ownership)
	{
	case EGTParcelOwnership::Government: return FLinearColor(0.5f, 0.5f, 0.5f, 0.4f);
	case EGTParcelOwnership::Public:     return FLinearColor(0.3f, 0.6f, 0.3f, 0.4f);
	case EGTParcelOwnership::Player:     return FLinearColor(0.2f, 0.4f, 0.9f, 0.6f);
	case EGTParcelOwnership::Contested:  return FLinearColor(0.9f, 0.2f, 0.2f, 0.6f);
	default:                             return FLinearColor(0.5f, 0.5f, 0.5f, 0.5f);
	}
}
