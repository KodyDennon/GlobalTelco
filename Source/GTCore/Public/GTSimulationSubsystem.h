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
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Simulation", meta = (ClampMin = "1.0", ClampMax = "10.0"))
	float EconomicTickInterval = 4.0f;

	// --- Speed Controls ---

	/** Pause or resume the simulation. */
	UFUNCTION(BlueprintCallable, Category = "Simulation")
	void SetPaused(bool bPaused);

	/** Check if the simulation is currently paused. */
	UFUNCTION(BlueprintPure, Category = "Simulation")
	bool IsPaused() const { return bSimulationPaused; }

	/** Set the simulation speed multiplier (1.0 = normal, 2.0 = 2x, etc.). */
	UFUNCTION(BlueprintCallable, Category = "Simulation")
	void SetSpeedMultiplier(float Multiplier);

	/** Get the current simulation speed multiplier. */
	UFUNCTION(BlueprintPure, Category = "Simulation")
	float GetSpeedMultiplier() const { return SimulationSpeedMultiplier; }

	/** Number of auto-save ticks between auto-saves (0 = disabled). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Simulation")
	int32 AutoSaveInterval = 50;

protected:
	/** Process a single economic tick: drain events, update all systems. */
	virtual void ProcessEconomicTick();

private:
	UPROPERTY()
	TObjectPtr<UGTEventQueue> EventQueue;

	int64 CurrentTick = 0;
	float TimeSinceLastTick = 0.0f;
	bool bSimulationPaused = false;
	float SimulationSpeedMultiplier = 1.0f;
};
