#include "GTAIArchetype.h"

TArray<FGTAIArchetypeData> UGTAIArchetypeRegistry::BuiltInArchetypes;
bool UGTAIArchetypeRegistry::bInitialized = false;

void UGTAIArchetypeRegistry::InitializeArchetypes()
{
	if (bInitialized)
	{
		return;
	}

	BuiltInArchetypes.SetNum(4);

	// --- Archetype 0: Aggressive Expander ---
	{
		FGTAIArchetypeData& A = BuiltInArchetypes[0];
		A.ArchetypeName = TEXT("Aggressive Expander");
		A.Description = TEXT("Rapidly acquires territory and builds infrastructure at high speed. "
			"Takes on heavy debt to finance expansion. Undercuts competitors on pricing. "
			"High risk, high reward — dominates early but vulnerable to downturns.");
		A.CompanyNamePool = {
			TEXT("RapidLink Global"), TEXT("Velocity Networks"), TEXT("BlitzComm"),
			TEXT("TerraConnect"), TEXT("ExpanTel"), TEXT("SwiftFiber Corp"),
			TEXT("Dominion Communications"), TEXT("FrontRunner Telecom")
		};
		A.ExpansionWeight = 0.9f;
		A.ConsolidationWeight = 0.2f;
		A.TechInvestmentWeight = 0.3f;
		A.AggressionWeight = 0.85f;
		A.RiskTolerance = 0.8f;
		A.FinancialPrudence = 0.15f;
		A.MinCashReserveRatio = 0.05f;
		A.MaxDebtToEquityRatio = 3.0f;
	}

	// --- Archetype 1: Defensive Consolidator ---
	{
		FGTAIArchetypeData& A = BuiltInArchetypes[1];
		A.ArchetypeName = TEXT("Defensive Consolidator");
		A.Description = TEXT("Builds a dense, high-quality network in select regions rather than "
			"spreading thin. Prioritizes reliability and customer retention. Conservative "
			"finances and low risk tolerance. Steady, predictable growth.");
		A.CompanyNamePool = {
			TEXT("SteadyLine Networks"), TEXT("Fortress Telecom"), TEXT("ReliaCom"),
			TEXT("Bastion Communications"), TEXT("Guardian Networks"), TEXT("AnchorTel"),
			TEXT("Ironclad Fiber"), TEXT("StableNet Corp")
		};
		A.ExpansionWeight = 0.3f;
		A.ConsolidationWeight = 0.9f;
		A.TechInvestmentWeight = 0.5f;
		A.AggressionWeight = 0.15f;
		A.RiskTolerance = 0.2f;
		A.FinancialPrudence = 0.85f;
		A.MinCashReserveRatio = 0.3f;
		A.MaxDebtToEquityRatio = 0.8f;
	}

	// --- Archetype 2: Tech Innovator ---
	{
		FGTAIArchetypeData& A = BuiltInArchetypes[2];
		A.ArchetypeName = TEXT("Tech Innovator");
		A.Description = TEXT("Invests heavily in R&D and technology advancement. Builds premium "
			"infrastructure with cutting-edge capacity. Moderate expansion focused on "
			"high-value urban markets. Charges premium pricing for superior service.");
		A.CompanyNamePool = {
			TEXT("NovaTech Communications"), TEXT("QuantumLink"), TEXT("Prism Networks"),
			TEXT("Apex Digital"), TEXT("NeuralNet Telecom"), TEXT("Synapse Corp"),
			TEXT("Photon Systems"), TEXT("Vanguard Fiber")
		};
		A.ExpansionWeight = 0.5f;
		A.ConsolidationWeight = 0.5f;
		A.TechInvestmentWeight = 0.95f;
		A.AggressionWeight = 0.4f;
		A.RiskTolerance = 0.5f;
		A.FinancialPrudence = 0.5f;
		A.MinCashReserveRatio = 0.15f;
		A.MaxDebtToEquityRatio = 1.5f;
	}

	// --- Archetype 3: Budget Operator ---
	{
		FGTAIArchetypeData& A = BuiltInArchetypes[3];
		A.ArchetypeName = TEXT("Budget Operator");
		A.Description = TEXT("Maximizes cost efficiency above all else. Targets underserved rural and "
			"suburban markets where land is cheap. Minimal debt, high cash reserves. "
			"Slow but virtually immune to economic downturns and disasters.");
		A.CompanyNamePool = {
			TEXT("ValueConnect"), TEXT("EconoTel"), TEXT("ThriftLine Networks"),
			TEXT("Penny Fiber"), TEXT("LeanComm"), TEXT("FrugalNet"),
			TEXT("BudgetBand Corp"), TEXT("SimpliConnect")
		};
		A.ExpansionWeight = 0.4f;
		A.ConsolidationWeight = 0.6f;
		A.TechInvestmentWeight = 0.2f;
		A.AggressionWeight = 0.2f;
		A.RiskTolerance = 0.15f;
		A.FinancialPrudence = 0.95f;
		A.MinCashReserveRatio = 0.4f;
		A.MaxDebtToEquityRatio = 0.5f;
	}

	bInitialized = true;
}

const TArray<FGTAIArchetypeData>& UGTAIArchetypeRegistry::GetArchetypes()
{
	InitializeArchetypes();
	return BuiltInArchetypes;
}

const FGTAIArchetypeData& UGTAIArchetypeRegistry::GetArchetype(int32 Index)
{
	InitializeArchetypes();
	if (Index >= 0 && Index < BuiltInArchetypes.Num())
	{
		return BuiltInArchetypes[Index];
	}
	// Return first archetype as fallback.
	return BuiltInArchetypes[0];
}

int32 UGTAIArchetypeRegistry::GetArchetypeCount()
{
	InitializeArchetypes();
	return BuiltInArchetypes.Num();
}

FString UGTAIArchetypeRegistry::GetRandomCompanyName(int32 ArchetypeIndex, int32 Seed)
{
	InitializeArchetypes();

	const FGTAIArchetypeData& Archetype = GetArchetype(ArchetypeIndex);
	if (Archetype.CompanyNamePool.Num() == 0)
	{
		return FString::Printf(TEXT("AI Corp %d"), Seed);
	}

	// Deterministic selection based on seed.
	const int32 NameIndex = FMath::Abs(Seed) % Archetype.CompanyNamePool.Num();
	return Archetype.CompanyNamePool[NameIndex];
}
