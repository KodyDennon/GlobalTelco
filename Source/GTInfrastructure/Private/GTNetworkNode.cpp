#include "GTNetworkNode.h"
#include "Net/UnrealNetwork.h"

AGTNetworkNode::AGTNetworkNode()
{
	bReplicates = true;
	bAlwaysRelevant = true;
}

void AGTNetworkNode::ApplyDamage(float CapacityReduction, float LatencyIncrease)
{
	Attributes.Capacity = FMath::Max(0.0f, Attributes.Capacity - CapacityReduction);
	Attributes.LatencyMs += LatencyIncrease;

	if (Attributes.Capacity <= 0.0f)
	{
		Status = EGTInfrastructureStatus::Destroyed;
	}
	else if (Attributes.Reliability < 0.5f)
	{
		Status = EGTInfrastructureStatus::Degraded;
	}

	MarkDirty();
}

void AGTNetworkNode::MarkDirty()
{
	// The network graph subsystem listens for dirty notifications
	// and recalculates affected cluster routing paths.
}

void AGTNetworkNode::GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const
{
	Super::GetLifetimeReplicatedProps(OutLifetimeProps);

	DOREPLIFETIME(AGTNetworkNode, NodeId);
	DOREPLIFETIME(AGTNetworkNode, Status);
	DOREPLIFETIME(AGTNetworkNode, Attributes);
	DOREPLIFETIME(AGTNetworkNode, Terrain);
	DOREPLIFETIME(AGTNetworkNode, OwnerCorporationIds);
}
