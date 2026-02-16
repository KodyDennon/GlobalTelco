#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "GTGlobeInteraction.generated.h"

class UGTGeodesicGrid;
class UGTLandParcelSystem;
class AGTHexGridRenderer;
struct FGTLandParcel;

/**
 * UGTGlobeInteraction
 *
 * Actor component that handles click-to-select interaction on the globe.
 * Attached to the player controller or globe pawn. When the user clicks:
 * 1. Line trace from mouse to globe surface
 * 2. Convert hit world position to lon/lat via Cesium georeference
 * 3. Find nearest geodesic grid cell
 * 4. Look up parcel data from the land parcel system
 * 5. Highlight the selected hex in the renderer
 * 6. Broadcast the selection for UI panels to display
 */
UCLASS(ClassGroup = "Globe", meta = (BlueprintSpawnableComponent))
class GLOBALTELCO_API UGTGlobeInteraction : public UActorComponent
{
	GENERATED_BODY()

public:
	UGTGlobeInteraction();

	virtual void BeginPlay() override;

	/** Process a click at a world location (called from the globe pawn's OnGlobeClicked). */
	UFUNCTION(BlueprintCallable, Category = "Globe Interaction")
	void HandleGlobeClick(FVector HitWorldLocation);

	/** Clear the current selection. */
	UFUNCTION(BlueprintCallable, Category = "Globe Interaction")
	void ClearSelection();

	/** Currently selected parcel ID. -1 if nothing selected. */
	UFUNCTION(BlueprintPure, Category = "Globe Interaction")
	int32 GetSelectedParcelId() const { return SelectedParcelId; }

	/** Currently selected geodesic cell index. -1 if nothing selected. */
	UFUNCTION(BlueprintPure, Category = "Globe Interaction")
	int32 GetSelectedCellIndex() const { return SelectedCellIndex; }

	/** Delegate fired when a hex is selected. Passes the parcel ID. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnHexSelected, int32, ParcelId);

	UPROPERTY(BlueprintAssignable, Category = "Globe Interaction")
	FOnHexSelected OnHexSelected;

	/** Delegate fired when selection is cleared. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnSelectionCleared);

	UPROPERTY(BlueprintAssignable, Category = "Globe Interaction")
	FOnSelectionCleared OnSelectionCleared;

	/** Reference to the hex grid renderer for highlighting. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Globe Interaction")
	TObjectPtr<AGTHexGridRenderer> HexGridRenderer;

private:
	/** Convert an Unreal world position to longitude/latitude using Cesium. */
	bool WorldPositionToLonLat(const FVector& WorldPos, double& OutLongitude, double& OutLatitude) const;

	int32 SelectedParcelId = -1;
	int32 SelectedCellIndex = -1;
};
