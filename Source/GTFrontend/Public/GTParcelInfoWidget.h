#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "GTSimulationTypes.h"
#include "GTLandParcelSystem.h"
#include "GTParcelInfoWidget.generated.h"

/**
 * UGTParcelInfoWidget
 *
 * UI panel that displays information about a selected hex parcel.
 * Shows terrain type, zoning, ownership, tax rate, disaster risk,
 * regulatory strictness, and other parcel properties.
 *
 * Subclass in Blueprints for visual layout. The C++ base class
 * provides data binding via BlueprintNativeEvent methods.
 */
UCLASS(Abstract, Blueprintable)
class GTFRONTEND_API UGTParcelInfoWidget : public UUserWidget
{
	GENERATED_BODY()

public:
	/**
	 * Display the info panel for a given parcel.
	 * Called when the player selects a hex on the globe.
	 */
	UFUNCTION(BlueprintCallable, BlueprintNativeEvent, Category = "Parcel Info")
	void ShowParcelInfo(const FGTLandParcel& Parcel);

	/** Hide the info panel (when selection is cleared). */
	UFUNCTION(BlueprintCallable, BlueprintNativeEvent, Category = "Parcel Info")
	void HideParcelInfo();

	/** Get a human-readable string for a terrain type. */
	UFUNCTION(BlueprintPure, Category = "Parcel Info")
	static FString GetTerrainDisplayName(EGTTerrainType Terrain);

	/** Get a human-readable string for a zoning category. */
	UFUNCTION(BlueprintPure, Category = "Parcel Info")
	static FString GetZoningDisplayName(EGTZoningCategory Zoning);

	/** Get a human-readable string for ownership type. */
	UFUNCTION(BlueprintPure, Category = "Parcel Info")
	static FString GetOwnershipDisplayName(EGTParcelOwnership Ownership);

protected:
	virtual void ShowParcelInfo_Implementation(const FGTLandParcel& Parcel);
	virtual void HideParcelInfo_Implementation();

	/** Currently displayed parcel data. Accessible from Blueprints. */
	UPROPERTY(BlueprintReadOnly, Category = "Parcel Info")
	FGTLandParcel CurrentParcel;

	/** Whether the panel is currently showing parcel info. */
	UPROPERTY(BlueprintReadOnly, Category = "Parcel Info")
	bool bIsShowingInfo = false;
};
