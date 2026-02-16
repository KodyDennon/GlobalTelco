#pragma once

#include "CoreMinimal.h"
#include "Subsystems/WorldSubsystem.h"
#include "GTSimulationTypes.h"
#include "GTSimulationSubsystem.generated.h"

class UGTEventQueue;

/**
 * UGTSimulationSubsystem
 *
 * World subsystem that drives the deterministic simulation.
 * Runs the economic tick cycle (every 3-5 real-time seconds),
 * drains the centralized event queue, and dispatches events
 * to registered module handlers.
 *
 * This is the "single simulation engine" mandated by the dev charter —
 * all simulation logic flows through this subsystem to guarantee
 * determinism and cross-module consistency.
 */
UCLASS()
class GTCORE_API UGTSimulationSubsystem : public UTickableWorldSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;
	virtual void Tick(float DeltaTime) override;
	virtual TStatId GetStatId() const override;

	/** Access the centralized event queue. */
	UFUNCTION(BlueprintPure, Category = "Simulation")
	UGTEventQueue* GetEventQueue() const { return EventQueue; }

	/** Current simulation tick counter. */
	UFUNCTION(BlueprintPure, Category = "Simulation")
	int64 GetCurrentTick() const { return CurrentTick; }

	/** Economic tick interval in seconds (default 4.0, range 3-5 per design doc). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Simulation", meta = (ClampMin = "3.0", ClampMax = "5.0"))
	float EconomicTickInterval = 4.0f;

protected:
	/** Process a single economic tick: drain events, update all systems. */
	virtual void ProcessEconomicTick();

private:
	UPROPERTY()
	TObjectPtr<UGTEventQueue> EventQueue;

	int64 CurrentTick = 0;
	float TimeSinceLastTick = 0.0f;
};
