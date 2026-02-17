#include "GTNetworkGraph.h"
#include "GTNetworkNode.h"
#include "GTNetworkEdge.h"

void UGTNetworkGraph::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);

	NextNodeId = 0;
	NextEdgeId = 0;
}

void UGTNetworkGraph::Deinitialize()
{
	Nodes.Empty();
	Edges.Empty();
	AdjacencyList.Empty();
	DirtyNodes.Empty();
	Super::Deinitialize();
}

int32 UGTNetworkGraph::RegisterNode(AGTNetworkNode* Node)
{
	if (!Node)
	{
		return -1;
	}

	const int32 Id = NextNodeId++;
	Node->NodeId = Id;
	Nodes.Add(Id, Node);
	AdjacencyList.Add(Id, TArray<int32>());
	MarkNodeDirty(Id);
	return Id;
}

void UGTNetworkGraph::UnregisterNode(int32 NodeId)
{
	// Remove all connected edges first.
	if (const TArray<int32>* ConnectedEdges = AdjacencyList.Find(NodeId))
	{
		TArray<int32> EdgesToRemove = *ConnectedEdges;
		for (int32 EdgeId : EdgesToRemove)
		{
			RemoveEdge(EdgeId);
		}
	}

	Nodes.Remove(NodeId);
	AdjacencyList.Remove(NodeId);
	DirtyNodes.Remove(NodeId);
}

int32 UGTNetworkGraph::CreateEdge(int32 SourceNodeId, int32 TargetNodeId, EGTEdgeType EdgeType, const FGTEdgeAttributes& InAttributes)
{
	if (!Nodes.Contains(SourceNodeId) || !Nodes.Contains(TargetNodeId))
	{
		return -1;
	}

	const int32 Id = NextEdgeId++;

	UGTNetworkEdge* Edge = NewObject<UGTNetworkEdge>(this);
	Edge->EdgeId = Id;
	Edge->SourceNodeId = SourceNodeId;
	Edge->TargetNodeId = TargetNodeId;
	Edge->EdgeType = EdgeType;
	Edge->Attributes = InAttributes;
	// If the edge has construction time remaining, start it under construction.
	if (InAttributes.bUnderConstruction && InAttributes.RemainingConstructionTicks > 0)
	{
		Edge->Status = EGTInfrastructureStatus::UnderConstruction;
	}
	else
	{
		Edge->Status = EGTInfrastructureStatus::Operational;
	}

	Edges.Add(Id, Edge);
	AdjacencyList.FindOrAdd(SourceNodeId).Add(Id);
	AdjacencyList.FindOrAdd(TargetNodeId).Add(Id);

	MarkNodeDirty(SourceNodeId);
	MarkNodeDirty(TargetNodeId);

	return Id;
}

void UGTNetworkGraph::RemoveEdge(int32 EdgeId)
{
	UGTNetworkEdge* Edge = GetEdge(EdgeId);
	if (!Edge)
	{
		return;
	}

	// Remove from adjacency lists of both endpoints.
	if (TArray<int32>* SourceAdj = AdjacencyList.Find(Edge->SourceNodeId))
	{
		SourceAdj->Remove(EdgeId);
	}
	if (TArray<int32>* TargetAdj = AdjacencyList.Find(Edge->TargetNodeId))
	{
		TargetAdj->Remove(EdgeId);
	}

	MarkNodeDirty(Edge->SourceNodeId);
	MarkNodeDirty(Edge->TargetNodeId);

	Edges.Remove(EdgeId);
}

void UGTNetworkGraph::MarkNodeDirty(int32 NodeId)
{
	DirtyNodes.Add(NodeId);
}

void UGTNetworkGraph::RecalculateDirtyRoutes()
{
	if (DirtyNodes.Num() == 0)
	{
		return;
	}

	// Cluster-based recalculation: group dirty nodes by network level,
	// then recompute shortest-path trees for affected clusters.
	// Full implementation will use Dijkstra per cluster with cached results.

	DirtyNodes.Empty();
}

float UGTNetworkGraph::GetShortestPathLatency(int32 FromNodeId, int32 ToNodeId) const
{
	if (!Nodes.Contains(FromNodeId) || !Nodes.Contains(ToNodeId))
	{
		return -1.0f;
	}

	if (FromNodeId == ToNodeId)
	{
		return 0.0f;
	}

	// Dijkstra shortest path using edge latency weights.
	// Uses a priority queue to find minimum-latency path.
	struct FPathEntry
	{
		int32 NodeId;
		float TotalLatency;
		bool operator<(const FPathEntry& Other) const { return TotalLatency > Other.TotalLatency; }
	};

	TMap<int32, float> BestLatency;
	TArray<FPathEntry> PriorityQueue;

	PriorityQueue.HeapPush(FPathEntry{FromNodeId, 0.0f});
	BestLatency.Add(FromNodeId, 0.0f);

	while (PriorityQueue.Num() > 0)
	{
		FPathEntry Current;
		PriorityQueue.HeapPop(Current);

		if (Current.NodeId == ToNodeId)
		{
			return Current.TotalLatency;
		}

		if (const float* Known = BestLatency.Find(Current.NodeId))
		{
			if (Current.TotalLatency > *Known)
			{
				continue;
			}
		}

		const TArray<int32>* ConnectedEdges = AdjacencyList.Find(Current.NodeId);
		if (!ConnectedEdges)
		{
			continue;
		}

		for (int32 EdgeId : *ConnectedEdges)
		{
			const UGTNetworkEdge* Edge = GetEdge(EdgeId);
			if (!Edge || !Edge->IsOperational())
			{
				continue;
			}

			// Determine the neighbor node on the other end of this edge.
			int32 NeighborId = (Edge->SourceNodeId == Current.NodeId) ? Edge->TargetNodeId : Edge->SourceNodeId;
			float NewLatency = Current.TotalLatency + Edge->GetEffectiveLatency();

			const float* ExistingLatency = BestLatency.Find(NeighborId);
			if (!ExistingLatency || NewLatency < *ExistingLatency)
			{
				BestLatency.Add(NeighborId, NewLatency);
				PriorityQueue.HeapPush(FPathEntry{NeighborId, NewLatency});
			}
		}
	}

	return -1.0f; // No path found.
}

TArray<int32> UGTNetworkGraph::GetConnectedEdges(int32 NodeId) const
{
	const TArray<int32>* Found = AdjacencyList.Find(NodeId);
	return Found ? *Found : TArray<int32>();
}

AGTNetworkNode* UGTNetworkGraph::GetNode(int32 NodeId) const
{
	const TObjectPtr<AGTNetworkNode>* Found = Nodes.Find(NodeId);
	return Found ? Found->Get() : nullptr;
}

UGTNetworkEdge* UGTNetworkGraph::GetEdge(int32 EdgeId) const
{
	const TObjectPtr<UGTNetworkEdge>* Found = Edges.Find(EdgeId);
	return Found ? Found->Get() : nullptr;
}

void UGTNetworkGraph::ProcessConstructionTick()
{
	TArray<int32> CompletedNodes;
	TArray<int32> CompletedEdges;
	ProcessConstructionTick(CompletedNodes, CompletedEdges);
}

void UGTNetworkGraph::ProcessConstructionTick(TArray<int32>& OutCompletedNodeIds, TArray<int32>& OutCompletedEdgeIds)
{
	// Advance construction timers on nodes.
	for (auto& Pair : Nodes)
	{
		AGTNetworkNode* Node = Pair.Value.Get();
		if (!Node)
		{
			continue;
		}

		if (Node->Attributes.bUnderConstruction && Node->Attributes.RemainingConstructionTicks > 0)
		{
			Node->Attributes.RemainingConstructionTicks--;

			if (Node->Attributes.RemainingConstructionTicks <= 0)
			{
				Node->Attributes.bUnderConstruction = false;
				Node->Status = EGTInfrastructureStatus::Operational;
				MarkNodeDirty(Pair.Key);
				OutCompletedNodeIds.Add(Pair.Key);

				UE_LOG(LogTemp, Log, TEXT("GTNetworkGraph: Node %d construction complete."), Pair.Key);
			}
		}
	}

	// Advance construction timers on edges.
	for (auto& Pair : Edges)
	{
		UGTNetworkEdge* Edge = Pair.Value.Get();
		if (!Edge)
		{
			continue;
		}

		if (Edge->Attributes.bUnderConstruction && Edge->Attributes.RemainingConstructionTicks > 0)
		{
			Edge->Attributes.RemainingConstructionTicks--;

			if (Edge->Attributes.RemainingConstructionTicks <= 0)
			{
				Edge->Attributes.bUnderConstruction = false;
				Edge->Status = EGTInfrastructureStatus::Operational;
				MarkNodeDirty(Edge->SourceNodeId);
				MarkNodeDirty(Edge->TargetNodeId);
				OutCompletedEdgeIds.Add(Pair.Key);

				UE_LOG(LogTemp, Log, TEXT("GTNetworkGraph: Edge %d construction complete."), Pair.Key);
			}
		}
	}
}

void UGTNetworkGraph::CalculateUtilization()
{
	// Reset all utilization values.
	for (auto& Pair : Nodes)
	{
		if (AGTNetworkNode* Node = Pair.Value.Get())
		{
			Node->Attributes.CurrentUtilization = 0.0f;
		}
	}
	for (auto& Pair : Edges)
	{
		if (UGTNetworkEdge* Edge = Pair.Value.Get())
		{
			Edge->Attributes.CurrentUtilization = 0.0f;
		}
	}

	// For each operational edge, calculate utilization based on the demand
	// routed through it relative to its capacity.
	// Simplified model: each edge's utilization is proportional to the
	// demand at its endpoint nodes divided by the edge's bandwidth capacity.
	for (auto& Pair : Edges)
	{
		UGTNetworkEdge* Edge = Pair.Value.Get();
		if (!Edge || !Edge->IsOperational())
		{
			continue;
		}

		const float EdgeCapacity = Edge->Attributes.Capacity;
		if (EdgeCapacity <= 0.0f)
		{
			continue;
		}

		// Sum the capacity demands of the nodes at both endpoints.
		float DemandOnEdge = 0.0f;
		AGTNetworkNode* SourceNode = GetNode(Edge->SourceNodeId);
		AGTNetworkNode* TargetNode = GetNode(Edge->TargetNodeId);

		if (SourceNode && SourceNode->IsOperational())
		{
			DemandOnEdge += SourceNode->Attributes.Capacity * 0.5f;
		}
		if (TargetNode && TargetNode->IsOperational())
		{
			DemandOnEdge += TargetNode->Attributes.Capacity * 0.5f;
		}

		Edge->Attributes.CurrentUtilization = FMath::Clamp(DemandOnEdge / EdgeCapacity, 0.0f, 2.0f);
	}

	// Node utilization: based on connected edge traffic.
	for (auto& Pair : Nodes)
	{
		AGTNetworkNode* Node = Pair.Value.Get();
		if (!Node || !Node->IsOperational())
		{
			continue;
		}

		const float NodeCapacity = Node->Attributes.Capacity;
		if (NodeCapacity <= 0.0f)
		{
			continue;
		}

		float TotalEdgeTraffic = 0.0f;
		const TArray<int32> ConnectedEdgeIds = GetConnectedEdges(Pair.Key);
		for (int32 EdgeId : ConnectedEdgeIds)
		{
			const UGTNetworkEdge* Edge = GetEdge(EdgeId);
			if (Edge && Edge->IsOperational())
			{
				TotalEdgeTraffic += Edge->Attributes.Capacity * Edge->Attributes.CurrentUtilization;
			}
		}

		Node->Attributes.CurrentUtilization = FMath::Clamp(TotalEdgeTraffic / NodeCapacity, 0.0f, 2.0f);
	}
}
