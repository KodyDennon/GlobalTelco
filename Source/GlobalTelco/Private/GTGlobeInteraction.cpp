#include "GTGlobeInteraction.h"
#include "GTGeodesicGrid.h"
#include "GTLandParcelSystem.h"
#include "GTHexGridRenderer.h"
#include "CesiumGeoreference.h"
#include "Engine/World.h"

UGTGlobeInteraction::UGTGlobeInteraction()
{
	PrimaryComponentTick.bCanEverTick = false;
}

void UGTGlobeInteraction::BeginPlay()
{
	Super::BeginPlay();
}

void UGTGlobeInteraction::HandleGlobeClick(FVector HitWorldLocation)
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	// Convert world position to longitude/latitude.
	double Longitude, Latitude;
	if (!WorldPositionToLonLat(HitWorldLocation, Longitude, Latitude))
	{
		return;
	}

	// Find the nearest geodesic grid cell.
	UGTGeodesicGrid* Grid = World->GetSubsystem<UGTGeodesicGrid>();
	if (!Grid || !Grid->IsGridGenerated())
	{
		return;
	}

	const int32 CellIndex = Grid->FindNearestCell(Longitude, Latitude);
	if (CellIndex < 0)
	{
		return;
	}

	// Look up the parcel.
	UGTLandParcelSystem* ParcelSystem = World->GetSubsystem<UGTLandParcelSystem>();
	if (!ParcelSystem)
	{
		return;
	}

	const int32 ParcelId = ParcelSystem->FindParcelByCellIndex(CellIndex);

	// Update selection state.
	SelectedCellIndex = CellIndex;
	SelectedParcelId = ParcelId;

	// Highlight the hex in the renderer.
	if (HexGridRenderer)
	{
		HexGridRenderer->HighlightCell(CellIndex);
	}

	// Broadcast selection event.
	if (ParcelId >= 0)
	{
		OnHexSelected.Broadcast(ParcelId);
	}
}

void UGTGlobeInteraction::ClearSelection()
{
	SelectedParcelId = -1;
	SelectedCellIndex = -1;

	if (HexGridRenderer)
	{
		HexGridRenderer->HighlightCell(-1);
	}

	OnSelectionCleared.Broadcast();
}

bool UGTGlobeInteraction::WorldPositionToLonLat(const FVector& WorldPos, double& OutLongitude, double& OutLatitude) const
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return false;
	}

	ACesiumGeoreference* Georef = ACesiumGeoreference::GetDefaultGeoreference(World);
	if (!Georef)
	{
		return false;
	}

	const FVector LonLatHeight = Georef->TransformUnrealPositionToLongitudeLatitudeHeight(WorldPos);
	OutLongitude = LonLatHeight.X;
	OutLatitude = LonLatHeight.Y;

	return true;
}
