#include "GTWorldSettings.h"

void UGTWorldSettings::ApplyDifficultyDefaults()
{
	switch (Difficulty)
	{
	case EGTDifficulty::Easy:
		StartingCapital = 50000000.0;
		DemandGrowthMultiplier = 1.5f;
		ConstructionCostMultiplier = 0.7f;
		MaintenanceCostMultiplier = 0.7f;
		TickIntervalSeconds = 4.0f;
		ResearchSpeed = EGTResearchSpeed::Fast;
		ResearchSpeedMultiplier = 2.0f;
		DisasterSeverity = EGTDisasterSeverity::Calm;
		DisasterFrequencyMultiplier = 0.3f;
		DisasterDamageMultiplier = 0.5f;
		AICorpCount = 3;
		AIAggressiveness = 0.5f;
		break;

	case EGTDifficulty::Normal:
		StartingCapital = 10000000.0;
		DemandGrowthMultiplier = 1.0f;
		ConstructionCostMultiplier = 1.0f;
		MaintenanceCostMultiplier = 1.0f;
		TickIntervalSeconds = 4.0f;
		ResearchSpeed = EGTResearchSpeed::Normal;
		ResearchSpeedMultiplier = 1.0f;
		DisasterSeverity = EGTDisasterSeverity::Moderate;
		DisasterFrequencyMultiplier = 1.0f;
		DisasterDamageMultiplier = 1.0f;
		AICorpCount = 5;
		AIAggressiveness = 1.0f;
		break;

	case EGTDifficulty::Hard:
		StartingCapital = 1000000.0;
		DemandGrowthMultiplier = 0.7f;
		ConstructionCostMultiplier = 1.5f;
		MaintenanceCostMultiplier = 1.5f;
		TickIntervalSeconds = 3.0f;
		ResearchSpeed = EGTResearchSpeed::Slow;
		ResearchSpeedMultiplier = 0.5f;
		DisasterSeverity = EGTDisasterSeverity::Brutal;
		DisasterFrequencyMultiplier = 2.0f;
		DisasterDamageMultiplier = 2.0f;
		AICorpCount = 8;
		AIAggressiveness = 2.0f;
		break;

	case EGTDifficulty::Custom:
		// Custom: leave all values as-is for manual tuning.
		break;
	}
}
