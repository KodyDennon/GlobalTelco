#pragma once

#include "CoreMinimal.h"
#include "GameFramework/PlayerController.h"
#include "GTPlayerController.generated.h"

class UGTGlobeInteraction;
class AGTGlobePawn;
class UGTSpeedControlWidget;

/**
 * AGTPlayerController
 *
 * Player controller for the GlobalTelco MMO. Each connected player
 * has one controller which owns their corporation and dispatches
 * player actions to the authoritative simulation server.
 *
 * In single-player, also wires globe interaction (click → hex select)
 * and speed control widget (quick-save/load).
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

	/** Globe interaction component (click to select hex parcels). */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Interaction")
	TObjectPtr<UGTGlobeInteraction> GlobeInteraction;

	virtual void GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const override;

protected:
	/** Callback from GlobePawn's OnGlobeClicked delegate. */
	UFUNCTION()
	void HandleGlobeClicked(FVector HitLocation);

	/** Callback for quick-save from speed control widget. */
	UFUNCTION()
	void HandleQuickSave();

	/** Callback for quick-load from speed control widget. */
	UFUNCTION()
	void HandleQuickLoad();
};
