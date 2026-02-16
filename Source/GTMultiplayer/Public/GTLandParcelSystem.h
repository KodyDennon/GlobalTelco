#pragma once

#include "CoreMinimal.h"
#include "Subsystems/WorldSubsystem.h"
#include "GTSimulationTypes.h"
#include "GTLandParcelSystem.generated.h"

/** Ownership category for a land parcel. */
UENUM(BlueprintType)
enum class EGTParcelOwnership : uint8
{
	Government,
	Public,
	Player,
	Contested
};

/** Zoning category controlling what can be built on a parcel. */
UENUM(BlueprintType)
enum class EGTZoningCategory : uint8
{
	Unrestricted,
	Commercial,
	Industrial,
	Residential,
	Protected,
	Military
};

/**
 * FGTLandParcel
 *
 * A hex-based land parcel in the world. Each parcel has terrain, zoning,
 * ownership, tax rates, regulatory strictness, and disaster risk.
 * This is the fundamental unit of geographic control in the game.
 */
USTRUCT(BlueprintType)
struct GTMULTIPLAYER_API FGTLandParcel
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "Parcel")
	int32 ParcelId = -1;

	UPROPERTY(BlueprintReadOnly, Category = "Parcel")
	FVector2D HexCoordinates = FVector2D::ZeroVector;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Parcel")
	EGTTerrainType Terrain = EGTTerrainType::Urban;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Parcel")
	EGTZoningCategory Zoning = EGTZoningCategory::Unrestricted;

	UPROPERTY(BlueprintReadOnly, Category = "Parcel")
	EGTParcelOwnership OwnershipType = EGTParcelOwnership::Government;

	/** Corporation ID of the owning player. -1 if government/public. */
	UPROPERTY(BlueprintReadOnly, Category = "Parcel")
	int32 OwnerCorporationId = -1;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Parcel")
	float LeaseRatePerTick = 0.0f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Parcel")
	float TaxRate = 0.1f;

	/** 0.0 (no regulation) to 1.0 (maximum strictness). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Parcel", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float RegulatoryStrictness = 0.3f;

	/** 0.0 (unstable) to 1.0 (fully stable). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Parcel", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float PoliticalStability = 0.7f;

	/** 0.0 (no risk) to 1.0 (extreme risk). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Parcel", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float DisasterRisk = 0.1f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Parcel")
	float LaborCostMultiplier = 1.0f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Parcel")
	float PowerGridReliability = 0.95f;
};

/**
 * UGTLandParcelSystem
 *
 * World subsystem that manages the hex-based land parcel grid.
 * Handles ownership, leasing, auctions, zoning compliance, and
 * infrastructure placement permits.
 */
UCLASS()
class GTMULTIPLAYER_API UGTLandParcelSystem : public UWorldSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;

	/** Register a parcel. Returns the assigned ParcelId. */
	UFUNCTION(BlueprintCallable, Category = "Land")
	int32 RegisterParcel(const FGTLandParcel& Parcel);

	/** Purchase a government-owned parcel. */
	UFUNCTION(BlueprintCallable, Category = "Land")
	bool PurchaseParcel(int32 ParcelId, int32 CorporationId, double Price);

	/** Lease a parcel from its owner. */
	UFUNCTION(BlueprintCallable, Category = "Land")
	bool LeaseParcel(int32 ParcelId, int32 LesseeCorporationId);

	/** Get parcel data by ID. */
	UFUNCTION(BlueprintPure, Category = "Land")
	FGTLandParcel GetParcel(int32 ParcelId) const;

	/** Get the parcel at given hex coordinates. Returns -1 if none found. */
	UFUNCTION(BlueprintPure, Category = "Land")
	int32 FindParcelAtCoordinates(FVector2D HexCoords) const;

	/** Check if a given infrastructure type can be built on this parcel's zoning. */
	UFUNCTION(BlueprintPure, Category = "Land")
	bool IsZoningCompatible(int32 ParcelId, EGTZoningCategory RequiredZoning) const;

private:
	TMap<int32, FGTLandParcel> Parcels;
	int32 NextParcelId = 0;
};
