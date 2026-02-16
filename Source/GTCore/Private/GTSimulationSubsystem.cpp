#include "GTSimulationSubsystem.h"
#include "GTEventQueue.h"

void UGTSimulationSubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);

	EventQueue = NewObject<UGTEventQueue>(this);
	CurrentTick = 0;
	TimeSinceLastTick = 0.0f;
	bSimulationPaused = false;
	SimulationSpeedMultiplier = 1.0f;
}

void UGTSimulationSubsystem::Deinitialize()
{
	EventQueue = nullptr;
	Super::Deinitialize();
}

void UGTSimulationSubsystem::Tick(float DeltaTime)
{
	Super::Tick(DeltaTime);

	if (bSimulationPaused)
	{
		return;
	}

	// Apply speed multiplier to the effective delta time.
	const float EffectiveDelta = DeltaTime * SimulationSpeedMultiplier;
	TimeSinceLastTick += EffectiveDelta;

	if (TimeSinceLastTick >= EconomicTickInterval)
	{
		TimeSinceLastTick -= EconomicTickInterval;
		ProcessEconomicTick();
	}
}

TStatId UGTSimulationSubsystem::GetStatId() const
{
	RETURN_QUICK_DECLARE_CYCLE_STAT(UGTSimulationSubsystem, STATGROUP_Tickables);
}

void UGTSimulationSubsystem::SetPaused(bool bPaused)
{
	bSimulationPaused = bPaused;
	UE_LOG(LogTemp, Log, TEXT("GTSimulation: %s"), bPaused ? TEXT("PAUSED") : TEXT("RESUMED"));
}

void UGTSimulationSubsystem::SetSpeedMultiplier(float Multiplier)
{
	SimulationSpeedMultiplier = FMath::Clamp(Multiplier, 0.25f, 8.0f);
	UE_LOG(LogTemp, Log, TEXT("GTSimulation: Speed set to %.1fx"), SimulationSpeedMultiplier);
}

void UGTSimulationSubsystem::ProcessEconomicTick()
{
	CurrentTick++;

	// Drain all pending events from the centralized queue.
	TArray<FGTSimulationEvent> Events;
	EventQueue->DrainEvents(Events);

	// Dispatch each event to listeners.
	for (const FGTSimulationEvent& Event : Events)
	{
		EventQueue->OnEventDispatched.Broadcast(Event);
	}

	// Enqueue an EconomicTick event so downstream systems know a tick occurred.
	FGTSimulationEvent TickEvent;
	TickEvent.EventType = EGTSimulationEventType::EconomicTick;
	TickEvent.Tick = CurrentTick;
	TickEvent.Timestamp = GetWorld()->GetTimeSeconds();
	EventQueue->OnEventDispatched.Broadcast(TickEvent);
}
