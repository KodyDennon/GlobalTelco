#include "GTGameMode.h"
#include "GTGameState.h"
#include "GTPlayerController.h"

AGTGameMode::AGTGameMode()
{
	GameStateClass = AGTGameState::StaticClass();
	PlayerControllerClass = AGTPlayerController::StaticClass();
}

void AGTGameMode::InitGame(const FString& MapName, const FString& Options, FString& ErrorMessage)
{
	Super::InitGame(MapName, Options, ErrorMessage);
}

void AGTGameMode::StartPlay()
{
	Super::StartPlay();
}

APlayerController* AGTGameMode::Login(UPlayer* NewPlayer, ENetRole InRemoteRole, const FString& Portal, const FString& Options, const FUniqueNetIdRepl& UniqueId, FString& ErrorMessage)
{
	if (GetNumPlayers() >= MaxConcurrentPlayers)
	{
		ErrorMessage = TEXT("Server is full.");
		return nullptr;
	}

	return Super::Login(NewPlayer, InRemoteRole, Portal, Options, UniqueId, ErrorMessage);
}

void AGTGameMode::Logout(AController* Exiting)
{
	Super::Logout(Exiting);
}
