#include "GTGlobeActor.h"
#include "GTGeoCoordinates.h"
#include "CesiumGeoreference.h"
#include "Cesium3DTileset.h"
#include "CesiumSunSky.h"
#include "CesiumIonRasterOverlay.h"
#include "Components/StaticMeshComponent.h"
#include "Engine/StaticMesh.h"
#include "Engine/World.h"
#include "UObject/ConstructorHelpers.h"

AGTGlobeActor::AGTGlobeActor()
{
	PrimaryActorTick.bCanEverTick = false;
	RootComponent = CreateDefaultSubobject<USceneComponent>(TEXT("Root"));

	// Create the offline globe mesh component (hidden by default, enabled in offline mode).
	OfflineGlobeMesh = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("OfflineGlobeMesh"));
	OfflineGlobeMesh->SetupAttachment(RootComponent);
	OfflineGlobeMesh->SetVisibility(false);
	OfflineGlobeMesh->SetCollisionEnabled(ECollisionEnabled::QueryOnly);
	OfflineGlobeMesh->SetCollisionResponseToAllChannels(ECR_Ignore);
	OfflineGlobeMesh->SetCollisionResponseToChannel(ECC_Visibility, ECR_Block);

	// Use UE5's built-in sphere mesh.
	static ConstructorHelpers::FObjectFinder<UStaticMesh> SphereMesh(
		TEXT("/Engine/BasicShapes/Sphere.Sphere"));
	if (SphereMesh.Succeeded())
	{
		OfflineGlobeMesh->SetStaticMesh(SphereMesh.Object);
	}
}

void AGTGlobeActor::BeginPlay()
{
	Super::BeginPlay();

	if (bUseRealEarth)
	{
		InitializeGeoreference();
		InitializeTerrainTileset();
		InitializeSunSky();
	}
	else
	{
		InitializeOfflineGlobe();
	}
}

void AGTGlobeActor::InitializeGeoreference()
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

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

	TerrainTileset->SetIonAssetID(TerrainAssetId);
	if (!CesiumIonAccessToken.IsEmpty())
	{
		TerrainTileset->SetIonAccessToken(CesiumIonAccessToken);
	}

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

void AGTGlobeActor::InitializeOfflineGlobe()
{
	if (!OfflineGlobeMesh)
	{
		return;
	}

	OfflineGlobeMesh->SetVisibility(true);

	// Scale the sphere to represent the globe.
	// The default UE sphere has radius 50cm. We want GlobeRadiusCm.
	const double MeshScale = UGTGeoCoordinates::GlobeRadiusCm / 50.0;
	OfflineGlobeMesh->SetWorldScale3D(FVector(MeshScale));
	OfflineGlobeMesh->SetWorldLocation(FVector::ZeroVector);

	UE_LOG(LogTemp, Log, TEXT("GTGlobeActor: Offline mode — procedural globe (radius=%.0f cm, scale=%.2f)"),
		UGTGeoCoordinates::GlobeRadiusCm, MeshScale);
}

FVector AGTGlobeActor::LonLatHeightToWorld(double Longitude, double Latitude, double HeightMeters) const
{
	if (Georeference)
	{
		return Georeference->TransformLongitudeLatitudeHeightPositionToUnreal(
			FVector(Longitude, Latitude, HeightMeters));
	}

	// Offline fallback: pure math.
	return UGTGeoCoordinates::LonLatHeightToWorld(Longitude, Latitude, HeightMeters);
}

FVector AGTGlobeActor::WorldToLonLatHeight(const FVector& WorldPosition) const
{
	if (Georeference)
	{
		return Georeference->TransformUnrealPositionToLongitudeLatitudeHeight(WorldPosition);
	}

	// Offline fallback: pure math.
	return UGTGeoCoordinates::WorldToLonLatHeight(WorldPosition);
}
