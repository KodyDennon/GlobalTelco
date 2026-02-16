#include "GTLandParcelSystem.h"

void UGTLandParcelSystem::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);
	NextParcelId = 0;
}

void UGTLandParcelSystem::Deinitialize()
{
	Parcels.Empty();
	Super::Deinitialize();
}

int32 UGTLandParcelSystem::RegisterParcel(const FGTLandParcel& InParcel)
{
	const int32 Id = NextParcelId++;
	FGTLandParcel Parcel = InParcel;
	Parcel.ParcelId = Id;
	Parcels.Add(Id, Parcel);
	return Id;
}

bool UGTLandParcelSystem::PurchaseParcel(int32 ParcelId, int32 CorporationId, double Price)
{
	FGTLandParcel* Parcel = Parcels.Find(ParcelId);
	if (!Parcel)
	{
		return false;
	}

	if (Parcel->OwnershipType != EGTParcelOwnership::Government &&
		Parcel->OwnershipType != EGTParcelOwnership::Public)
	{
		return false; // Already player-owned; must negotiate with owner.
	}

	Parcel->OwnershipType = EGTParcelOwnership::Player;
	Parcel->OwnerCorporationId = CorporationId;

	// Financial transaction (deduct Price from corporation) handled via economy module event.
	return true;
}

bool UGTLandParcelSystem::LeaseParcel(int32 ParcelId, int32 LesseeCorporationId)
{
	const FGTLandParcel* Parcel = Parcels.Find(ParcelId);
	if (!Parcel)
	{
		return false;
	}

	// Lease processing handled via contract system in GTMultiplayer.
	return true;
}

FGTLandParcel UGTLandParcelSystem::GetParcel(int32 ParcelId) const
{
	const FGTLandParcel* Found = Parcels.Find(ParcelId);
	return Found ? *Found : FGTLandParcel();
}

int32 UGTLandParcelSystem::FindParcelAtCoordinates(FVector2D HexCoords) const
{
	for (const auto& Pair : Parcels)
	{
		if (Pair.Value.HexCoordinates.Equals(HexCoords, 0.01))
		{
			return Pair.Key;
		}
	}
	return -1;
}

bool UGTLandParcelSystem::UpdateParcel(int32 ParcelId, const FGTLandParcel& UpdatedData)
{
	FGTLandParcel* Parcel = Parcels.Find(ParcelId);
	if (!Parcel)
	{
		return false;
	}

	// Preserve the ParcelId — callers cannot change it.
	*Parcel = UpdatedData;
	Parcel->ParcelId = ParcelId;
	return true;
}

int32 UGTLandParcelSystem::FindParcelByCellIndex(int32 CellIndex) const
{
	for (const auto& Pair : Parcels)
	{
		if (Pair.Value.GeodesicCellIndex == CellIndex)
		{
			return Pair.Key;
		}
	}
	return -1;
}

TArray<int32> UGTLandParcelSystem::GetParcelsInRegion(int32 InRegionId) const
{
	TArray<int32> Result;
	for (const auto& Pair : Parcels)
	{
		if (Pair.Value.RegionId == InRegionId)
		{
			Result.Add(Pair.Key);
		}
	}
	return Result;
}

bool UGTLandParcelSystem::IsZoningCompatible(int32 ParcelId, EGTZoningCategory RequiredZoning) const
{
	const FGTLandParcel* Parcel = Parcels.Find(ParcelId);
	if (!Parcel)
	{
		return false;
	}

	// Unrestricted zoning allows everything.
	if (Parcel->Zoning == EGTZoningCategory::Unrestricted)
	{
		return true;
	}

	// Protected and Military zones do not allow player infrastructure.
	if (Parcel->Zoning == EGTZoningCategory::Protected ||
		Parcel->Zoning == EGTZoningCategory::Military)
	{
		return false;
	}

	return Parcel->Zoning == RequiredZoning;
}
