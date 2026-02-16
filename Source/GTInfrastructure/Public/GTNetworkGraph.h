#pragma once

#include "CoreMinimal.h"
#include "Subsystems/WorldSubsystem.h"
#include "GTInfrastructureTypes.h"
#include "GTNetworkGraph.generated.h"

class AGTNetworkNode;
class UGTNetworkEdge;

/**
 * UGTNetworkGraph
 *
 * World subsystem that maintains the hierarchical global network graph.
 * Manages the 5-level topology (Local -> Regional -> National -> Continental -> Global),
 * performs event-driven routing recalculation with dirty-node invalidation,
 * and caches shortest-path trees per cluster.
 *
 * This operates on aggregate bandwidth — no packet-level simulation.
 */
UCLASS()
class GTINFRASTRUCTURE_API UGTNetworkGraph : public UWorldSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;

	/** Register a newly constructed node into the graph. Returns its assigned NodeId. */
	UFUNCTION(BlueprintCallable, Category = "Network Graph")
	int32 RegisterNode(AGTNetworkNode* Node);

	/** Remove a node from the graph (decommission or destruction). */
	UFUNCTION(BlueprintCallable, Category = "Network Graph")
	void UnregisterNode(int32 NodeId);

	/** Create an edge connecting two nodes. Returns the assigned EdgeId. */
	UFUNCTION(BlueprintCallable, Category = "Network Graph")
	int32 CreateEdge(int32 SourceNodeId, int32 TargetNodeId, EGTEdgeType EdgeType, const FGTEdgeAttributes& Attributes);

	/** Remove an edge from the graph. */
	UFUNCTION(BlueprintCallable, Category = "Network Graph")
	void RemoveEdge(int32 EdgeId);

	/** Mark a node as dirty, triggering cluster-based route recalculation. */
	UFUNCTION(BlueprintCallable, Category = "Network Graph")
	void MarkNodeDirty(int32 NodeId);

	/** Recalculate routes for all dirty clusters. Called by the simulation subsystem. */
	UFUNCTION(BlueprintCallable, Category = "Network Graph")
	void RecalculateDirtyRoutes();

	/** Get shortest-path latency between two nodes. Returns -1 if no path exists. */
	UFUNCTION(BlueprintPure, Category = "Network Graph")
	float GetShortestPathLatency(int32 FromNodeId, int32 ToNodeId) const;

	/** Get all edges connected to a node. */
	UFUNCTION(BlueprintPure, Category = "Network Graph")
	TArray<int32> GetConnectedEdges(int32 NodeId) const;

	/** Look up a node actor by its graph ID. */
	UFUNCTION(BlueprintPure, Category = "Network Graph")
	AGTNetworkNode* GetNode(int32 NodeId) const;

	/** Look up an edge object by its graph ID. */
	UFUNCTION(BlueprintPure, Category = "Network Graph")
	UGTNetworkEdge* GetEdge(int32 EdgeId) const;

private:
	UPROPERTY()
	TMap<int32, TObjectPtr<AGTNetworkNode>> Nodes;

	UPROPERTY()
	TMap<int32, TObjectPtr<UGTNetworkEdge>> Edges;

	/** Adjacency list: NodeId -> array of EdgeIds. */
	TMap<int32, TArray<int32>> AdjacencyList;

	/** Set of node IDs that have been marked dirty since last recalculation. */
	TSet<int32> DirtyNodes;

	int32 NextNodeId = 0;
	int32 NextEdgeId = 0;
};
