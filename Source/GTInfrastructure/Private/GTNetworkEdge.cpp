#include "GTNetworkEdge.h"

float UGTNetworkEdge::GetEffectiveLatency() const
{
	if (Status == EGTInfrastructureStatus::Destroyed)
	{
		return TNumericLimits<float>::Max();
	}

	float EffectiveLatency = Attributes.LatencyWeightMs;

	if (Status == EGTInfrastructureStatus::Degraded)
	{
		EffectiveLatency *= 2.0f;
	}

	return EffectiveLatency;
}

float UGTNetworkEdge::GetEffectiveCapacity() const
{
	if (Status == EGTInfrastructureStatus::Destroyed ||
		Status == EGTInfrastructureStatus::Decommissioned)
	{
		return 0.0f;
	}

	float EffectiveCapacity = Attributes.Capacity;

	if (Status == EGTInfrastructureStatus::Degraded)
	{
		EffectiveCapacity *= 0.5f;
	}

	return FMath::Max(0.0f, EffectiveCapacity);
}
