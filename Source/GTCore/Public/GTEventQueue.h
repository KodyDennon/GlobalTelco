#pragma once

#include "CoreMinimal.h"
#include "GTSimulationTypes.h"
#include "GTEventQueue.generated.h"

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnSimulationEvent, const FGTSimulationEvent&, Event);

/**
 * UGTEventQueue
 *
 * Centralized event queue for the deterministic simulation.
 * All modules push events here; the simulation subsystem drains and dispatches
 * them each tick. This ensures a single ordered timeline of state mutations,
 * which is critical for deterministic replay and server-authoritative consistency.
 */
UCLASS(BlueprintType)
class GTCORE_API UGTEventQueue : public UObject
{
	GENERATED_BODY()

public:
	/** Enqueue a simulation event. Thread-safe. */
	UFUNCTION(BlueprintCallable, Category = "Simulation")
	void Enqueue(const FGTSimulationEvent& Event);

	/** Drain all pending events into OutEvents, sorted by tick then insertion order. Returns count drained. */
	int32 DrainEvents(TArray<FGTSimulationEvent>& OutEvents);

	/** Number of pending events. */
	UFUNCTION(BlueprintPure, Category = "Simulation")
	int32 GetPendingCount() const;

	/** Fired for every event as it is dispatched during tick processing. */
	UPROPERTY(BlueprintAssignable, Category = "Simulation")
	FOnSimulationEvent OnEventDispatched;

private:
	FCriticalSection QueueLock;
	TArray<FGTSimulationEvent> PendingEvents;
};
