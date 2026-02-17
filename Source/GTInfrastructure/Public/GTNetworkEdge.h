#pragma once

#include "CoreMinimal.h"
#include "UObject/NoExportTypes.h"
#include "GTInfrastructureTypes.h"
#include "GTNetworkEdge.generated.h"

/**
 * UGTNetworkEdge
 *
 * A directed edge in the network graph connecting two infrastructure nodes.
 * Edges represent physical links: fiber runs, microwave links, subsea cables,
 * or satellite connections.
 *
 * Edges are UObjects (not actors) — they exist in the graph data structure,
 * not as placed world objects. Their visual representation is handled by
 * the frontend module.
 */
UCLASS(BlueprintType)
class GTINFRASTRUCTURE_API UGTNetworkEdge : public UObject
{
	GENERATED_BODY()

public:
	/** Unique edge ID within the network graph. */
	UPROPERTY(BlueprintReadOnly, Category = "Network")
	int32 EdgeId = -1;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Network")
	EGTEdgeType EdgeType = EGTEdgeType::None;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Network")
	EGTInfrastructureStatus Status = EGTInfrastructureStatus::UnderConstruction;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Network")
	FGTEdgeAttributes Attributes;

	/** Graph ID of the source node. */
	UPROPERTY(BlueprintReadOnly, Category = "Network")
	int32 SourceNodeId = -1;

	/** Graph ID of the destination node. */
	UPROPERTY(BlueprintReadOnly, Category = "Network")
	int32 TargetNodeId = -1;

	/** Corporation IDs that co-own this edge. */
	UPROPERTY(BlueprintReadOnly, Category = "Ownership")
	TArray<int32> OwnerCorporationIds;

	/** Calculate the current effective latency weight (base + congestion + degradation). */
	UFUNCTION(BlueprintPure, Category = "Network")
	float GetEffectiveLatency() const;

	/** Calculate current usable capacity after degradation. */
	UFUNCTION(BlueprintPure, Category = "Network")
	float GetEffectiveCapacity() const;

	UFUNCTION(BlueprintPure, Category = "Network")
	bool IsOperational() const { return Status == EGTInfrastructureStatus::Operational && !Attributes.bUnderConstruction; }
};
