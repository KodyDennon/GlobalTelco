#pragma once

#include "CoreMinimal.h"
#include "GameFramework/PlayerController.h"
#include "GTPlayerController.generated.h"

/**
 * AGTPlayerController
 *
 * Player controller for the GlobalTelco MMO. Each connected player
 * has one controller which owns their corporation and dispatches
 * player actions to the authoritative simulation server.
 */
UCLASS()
class GLOBALTELCO_API AGTPlayerController : public APlayerController
{
	GENERATED_BODY()

public:
	AGTPlayerController();

	virtual void BeginPlay() override;

	/** Unique corporation ID assigned to this player on login. */
	UPROPERTY(Replicated, BlueprintReadOnly, Category = "Corporation")
	int32 CorporationId = -1;

	virtual void GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const override;
};
