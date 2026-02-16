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
	Edge->Status = EGTInfrastructureStatus::Operational;

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
