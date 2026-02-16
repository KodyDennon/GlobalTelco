#include "GTPlayerController.h"
#include "Net/UnrealNetwork.h"

AGTPlayerController::AGTPlayerController()
{
}

void AGTPlayerController::BeginPlay()
{
	Super::BeginPlay();
}

void AGTPlayerController::GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const
{
	Super::GetLifetimeReplicatedProps(OutLifetimeProps);

	DOREPLIFETIME(AGTPlayerController, CorporationId);
}
