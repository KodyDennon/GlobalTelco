#include "GTSimulationSubsystem.h"
#include "GTEventQueue.h"

void UGTSimulationSubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);

	EventQueue = NewObject<UGTEventQueue>(this);
	CurrentTick = 0;
	TimeSinceLastTick = 0.0f;
}

void UGTSimulationSubsystem::Deinitialize()
{
	EventQueue = nullptr;
	Super::Deinitialize();
}

void UGTSimulationSubsystem::Tick(float DeltaTime)
{
	Super::Tick(DeltaTime);

	TimeSinceLastTick += DeltaTime;

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
