#include "GTPlayerController.h"
#include "GTGlobeInteraction.h"
#include "GTGlobePawn.h"
#include "GTHexGridRenderer.h"
#include "GTSaveLoadSubsystem.h"
#include "GTGameInstance.h"
#include "Net/UnrealNetwork.h"
#include "Engine/World.h"
#include "Kismet/GameplayStatics.h"

AGTPlayerController::AGTPlayerController()
{
	GlobeInteraction = CreateDefaultSubobject<UGTGlobeInteraction>(TEXT("GlobeInteraction"));
}

void AGTPlayerController::BeginPlay()
{
	Super::BeginPlay();

	// Auto-find the hex grid renderer in the world and assign to interaction component.
	if (GlobeInteraction)
	{
		UWorld* World = GetWorld();
		if (World)
		{
			AGTHexGridRenderer* Renderer = Cast<AGTHexGridRenderer>(
				UGameplayStatics::GetActorOfClass(World, AGTHexGridRenderer::StaticClass()));
			if (Renderer)
			{
				GlobeInteraction->HexGridRenderer = Renderer;
			}
		}
	}

	// Bind to the GlobePawn's OnGlobeClicked delegate so clicks flow to interaction.
	if (APawn* CurrentPawn = GetPawn())
	{
		if (AGTGlobePawn* GlobePawn = Cast<AGTGlobePawn>(CurrentPawn))
		{
			GlobePawn->OnGlobeClicked.AddDynamic(this, &AGTPlayerController::HandleGlobeClicked);
		}
	}
}

void AGTPlayerController::HandleGlobeClicked(FVector HitLocation)
{
	if (GlobeInteraction)
	{
		GlobeInteraction->HandleGlobeClick(HitLocation);
	}
}

void AGTPlayerController::HandleQuickSave()
{
	UGTSaveLoadSubsystem* SaveLoad = GetGameInstance()->GetSubsystem<UGTSaveLoadSubsystem>();
	UGTGameInstance* GTGI = Cast<UGTGameInstance>(GetGameInstance());
	if (SaveLoad && GTGI)
	{
		SaveLoad->SaveGame(
			TEXT("QuickSave"),
			TEXT("Quick Save"),
			GTGI->ActivePlayerCorporationId,
			GTGI->ActivePlayerCorpName
		);
	}
}

void AGTPlayerController::HandleQuickLoad()
{
	UGTSaveLoadSubsystem* SaveLoad = GetGameInstance()->GetSubsystem<UGTSaveLoadSubsystem>();
	UGTGameInstance* GTGI = Cast<UGTGameInstance>(GetGameInstance());
	if (SaveLoad && GTGI && SaveLoad->DoesSlotExist(TEXT("QuickSave")))
	{
		GTGI->PrepareLoadGame(TEXT("QuickSave"));
		UGameplayStatics::OpenLevel(GetWorld(), TEXT("/Game/Maps/GameWorld"),
			false, TEXT("?game=/Script/GlobalTelco.GTSinglePlayerGameMode"));
	}
}

void AGTPlayerController::GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const
{
	Super::GetLifetimeReplicatedProps(OutLifetimeProps);

	DOREPLIFETIME(AGTPlayerController, CorporationId);
}
