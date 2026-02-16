#include "GTGameState.h"
#include "Net/UnrealNetwork.h"

AGTGameState::AGTGameState()
{
}

void AGTGameState::GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const
{
	Super::GetLifetimeReplicatedProps(OutLifetimeProps);

	DOREPLIFETIME(AGTGameState, SimulationTick);
	DOREPLIFETIME(AGTGameState, SimulationTimeSeconds);
}
