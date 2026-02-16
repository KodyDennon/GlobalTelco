#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameModeBase.h"
#include "GTGameMode.generated.h"

/**
 * AGTGameMode
 *
 * Base game mode for the Global Telecom Infrastructure MMO.
 * Manages simulation lifecycle: spawns the authoritative simulation subsystem,
 * handles player login/logout, and coordinates the economic tick cycle.
 */
UCLASS()
class GLOBALTELCO_API AGTGameMode : public AGameModeBase
{
	GENERATED_BODY()

public:
	AGTGameMode();

	virtual void InitGame(const FString& MapName, const FString& Options, FString& ErrorMessage) override;
	virtual void StartPlay() override;
	virtual APlayerController* Login(UPlayer* NewPlayer, ENetRole InRemoteRole, const FString& Portal, const FString& Options, const FUniqueNetIdRepl& UniqueId, FString& ErrorMessage) override;
	virtual void Logout(AController* Exiting) override;

protected:
	/** Maximum concurrent players per world instance. */
	UPROPERTY(EditDefaultsOnly, Category = "Simulation")
	int32 MaxConcurrentPlayers = 250;
};
