#include "GTGlobeActor.h"
#include "CesiumGeoreference.h"
#include "Cesium3DTileset.h"
#include "CesiumSunSky.h"
#include "CesiumIonRasterOverlay.h"
#include "Engine/World.h"

AGTGlobeActor::AGTGlobeActor()
{
	PrimaryActorTick.bCanEverTick = false;
	RootComponent = CreateDefaultSubobject<USceneComponent>(TEXT("Root"));
}

void AGTGlobeActor::BeginPlay()
{
	Super::BeginPlay();

	if (!bUseRealEarth)
	{
		return;
	}

	InitializeGeoreference();
	InitializeTerrainTileset();
	InitializeSunSky();
}

void AGTGlobeActor::InitializeGeoreference()
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	// Find existing georeference or spawn one.
	Georeference = ACesiumGeoreference::GetDefaultGeoreference(World);
	if (!Georeference)
	{
		FActorSpawnParameters SpawnParams;
		SpawnParams.Name = TEXT("GTCesiumGeoreference");
		Georeference = World->SpawnActor<ACesiumGeoreference>(SpawnParams);
	}

	if (Georeference)
	{
		Georeference->SetOriginLongitudeLatitudeHeight(FVector(StartLongitude, StartLatitude, 0.0));
	}
}

void AGTGlobeActor::InitializeTerrainTileset()
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	FActorSpawnParameters SpawnParams;
	SpawnParams.Name = TEXT("GTTerrainTileset");
	TerrainTileset = World->SpawnActor<ACesium3DTileset>(SpawnParams);

	if (!TerrainTileset)
	{
		return;
	}

	// Configure terrain from Cesium ion.
	TerrainTileset->SetIonAssetID(TerrainAssetId);
	if (!CesiumIonAccessToken.IsEmpty())
	{
		TerrainTileset->SetIonAccessToken(CesiumIonAccessToken);
	}

	// Attach imagery overlay for satellite imagery.
	UCesiumIonRasterOverlay* ImageryOverlay = NewObject<UCesiumIonRasterOverlay>(
		TerrainTileset, TEXT("GTImageryOverlay"));
	if (ImageryOverlay)
	{
		ImageryOverlay->IonAssetID = ImageryAssetId;
		if (!CesiumIonAccessToken.IsEmpty())
		{
			ImageryOverlay->IonAccessToken = CesiumIonAccessToken;
		}
		ImageryOverlay->RegisterComponent();
		ImageryOverlay->AttachToComponent(
			TerrainTileset->GetRootComponent(),
			FAttachmentTransformRules::KeepRelativeTransform);
	}
}

void AGTGlobeActor::InitializeSunSky()
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	FActorSpawnParameters SpawnParams;
	SpawnParams.Name = TEXT("GTSunSky");
	SunSky = World->SpawnActor<ACesiumSunSky>(SpawnParams);
}
