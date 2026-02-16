#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameStateBase.h"
#include "GTGameState.generated.h"

/**
 * AGTGameState
 *
 * Replicated game state for a GlobalTelco world instance.
 * Holds the current simulation tick count and global world time
 * that all clients observe.
 */
UCLASS()
class GLOBALTELCO_API AGTGameState : public AGameStateBase
{
	GENERATED_BODY()

public:
	AGTGameState();

	/** Current simulation tick index, incremented every economic tick cycle. */
	UPROPERTY(Replicated, BlueprintReadOnly, Category = "Simulation")
	int64 SimulationTick = 0;

	/** Elapsed simulation time in seconds (accumulated from ticks). */
	UPROPERTY(Replicated, BlueprintReadOnly, Category = "Simulation")
	double SimulationTimeSeconds = 0.0;

	virtual void GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const override;
};
