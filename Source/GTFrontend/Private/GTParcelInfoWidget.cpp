#include "GTParcelInfoWidget.h"

void UGTParcelInfoWidget::ShowParcelInfo_Implementation(const FGTLandParcel& Parcel)
{
	CurrentParcel = Parcel;
	bIsShowingInfo = true;
}

void UGTParcelInfoWidget::HideParcelInfo_Implementation()
{
	bIsShowingInfo = false;
}

FString UGTParcelInfoWidget::GetTerrainDisplayName(EGTTerrainType Terrain)
{
	switch (Terrain)
	{
	case EGTTerrainType::Urban:        return TEXT("Urban");
	case EGTTerrainType::Suburban:     return TEXT("Suburban");
	case EGTTerrainType::Rural:        return TEXT("Rural");
	case EGTTerrainType::Mountainous:  return TEXT("Mountainous");
	case EGTTerrainType::Desert:       return TEXT("Desert");
	case EGTTerrainType::Coastal:      return TEXT("Coastal");
	case EGTTerrainType::OceanShallow: return TEXT("Shallow Ocean");
	case EGTTerrainType::OceanDeep:    return TEXT("Deep Ocean");
	case EGTTerrainType::Tundra:       return TEXT("Tundra");
	case EGTTerrainType::Frozen:       return TEXT("Frozen");
	default:                           return TEXT("Unknown");
	}
}

FString UGTParcelInfoWidget::GetZoningDisplayName(EGTZoningCategory Zoning)
{
	switch (Zoning)
	{
	case EGTZoningCategory::Unrestricted: return TEXT("Unrestricted");
	case EGTZoningCategory::Commercial:   return TEXT("Commercial");
	case EGTZoningCategory::Industrial:   return TEXT("Industrial");
	case EGTZoningCategory::Residential:  return TEXT("Residential");
	case EGTZoningCategory::Protected:    return TEXT("Protected");
	case EGTZoningCategory::Military:     return TEXT("Military");
	default:                              return TEXT("Unknown");
	}
}

FString UGTParcelInfoWidget::GetOwnershipDisplayName(EGTParcelOwnership Ownership)
{
	switch (Ownership)
	{
	case EGTParcelOwnership::Government: return TEXT("Government");
	case EGTParcelOwnership::Public:     return TEXT("Public");
	case EGTParcelOwnership::Player:     return TEXT("Player-Owned");
	case EGTParcelOwnership::Contested:  return TEXT("Contested");
	default:                             return TEXT("Unknown");
	}
}
