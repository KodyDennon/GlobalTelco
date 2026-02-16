#include "GTEventQueue.h"

void UGTEventQueue::Enqueue(const FGTSimulationEvent& Event)
{
	FScopeLock Lock(&QueueLock);
	PendingEvents.Add(Event);
}

int32 UGTEventQueue::DrainEvents(TArray<FGTSimulationEvent>& OutEvents)
{
	FScopeLock Lock(&QueueLock);

	OutEvents.Append(PendingEvents);
	const int32 Count = PendingEvents.Num();
	PendingEvents.Reset();

	// Stable sort by tick to preserve insertion order within the same tick.
	OutEvents.Sort([](const FGTSimulationEvent& A, const FGTSimulationEvent& B)
	{
		return A.Tick < B.Tick;
	});

	return Count;
}

int32 UGTEventQueue::GetPendingCount() const
{
	return PendingEvents.Num();
}
