#include "GTNewGameWidget.h"
#include "Components/EditableTextBox.h"
#include "Components/ComboBoxString.h"
#include "Components/Slider.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Kismet/GameplayStatics.h"

void UGTNewGameWidget::NativeConstruct()
{
	Super::NativeConstruct();

	// Populate difficulty dropdown.
	if (DifficultyDropdown)
	{
		DifficultyDropdown->ClearOptions();
		DifficultyDropdown->AddOption(TEXT("Easy"));
		DifficultyDropdown->AddOption(TEXT("Normal"));
		DifficultyDropdown->AddOption(TEXT("Hard"));
		DifficultyDropdown->AddOption(TEXT("Custom"));
		DifficultyDropdown->SetSelectedOption(TEXT("Normal"));
		DifficultyDropdown->OnSelectionChanged.AddDynamic(this, &UGTNewGameWidget::HandleDifficultyChanged);
	}

	// Populate disaster severity dropdown.
	if (DisasterSeverityDropdown)
	{
		DisasterSeverityDropdown->ClearOptions();
		DisasterSeverityDropdown->AddOption(TEXT("Calm"));
		DisasterSeverityDropdown->AddOption(TEXT("Moderate"));
		DisasterSeverityDropdown->AddOption(TEXT("Brutal"));
		DisasterSeverityDropdown->SetSelectedOption(TEXT("Moderate"));
	}

	// Configure AI corporation count slider (0-10).
	if (AICorpCountSlider)
	{
		AICorpCountSlider->SetMinValue(0.0f);
		AICorpCountSlider->SetMaxValue(10.0f);
		AICorpCountSlider->SetValue(5.0f);
		AICorpCountSlider->SetStepSize(1.0f);
		AICorpCountSlider->OnValueChanged.AddDynamic(this, &UGTNewGameWidget::HandleAISliderChanged);
	}

	// Default labels.
	if (AICorpCountLabel)
	{
		AICorpCountLabel->SetText(FText::FromString(TEXT("5")));
	}

	// Default corporation name.
	if (CorporationNameInput)
	{
		CorporationNameInput->SetText(FText::FromString(TEXT("Player Corp")));
	}

	// World seed default.
	if (WorldSeedInput)
	{
		WorldSeedInput->SetText(FText::FromString(TEXT("0")));
	}

	// Button bindings.
	if (StartGameButton)
	{
		StartGameButton->OnClicked.AddDynamic(this, &UGTNewGameWidget::HandleStartGameClicked);
	}

	if (BackButton)
	{
		BackButton->OnClicked.AddDynamic(this, &UGTNewGameWidget::HandleBackClicked);
	}
}

void UGTNewGameWidget::HandleDifficultyChanged(FString SelectedItem, ESelectInfo::Type SelectionType)
{
	// When difficulty changes, update the AI slider to defaults for that difficulty.
	if (SelectedItem == TEXT("Easy"))
	{
		if (AICorpCountSlider) AICorpCountSlider->SetValue(3.0f);
		if (AICorpCountLabel) AICorpCountLabel->SetText(FText::FromString(TEXT("3")));
		if (DisasterSeverityDropdown) DisasterSeverityDropdown->SetSelectedOption(TEXT("Calm"));
	}
	else if (SelectedItem == TEXT("Normal"))
	{
		if (AICorpCountSlider) AICorpCountSlider->SetValue(5.0f);
		if (AICorpCountLabel) AICorpCountLabel->SetText(FText::FromString(TEXT("5")));
		if (DisasterSeverityDropdown) DisasterSeverityDropdown->SetSelectedOption(TEXT("Moderate"));
	}
	else if (SelectedItem == TEXT("Hard"))
	{
		if (AICorpCountSlider) AICorpCountSlider->SetValue(8.0f);
		if (AICorpCountLabel) AICorpCountLabel->SetText(FText::FromString(TEXT("8")));
		if (DisasterSeverityDropdown) DisasterSeverityDropdown->SetSelectedOption(TEXT("Brutal"));
	}
	// Custom: don't change anything.
}

void UGTNewGameWidget::HandleAISliderChanged(float Value)
{
	const int32 Count = FMath::RoundToInt(Value);
	if (AICorpCountLabel)
	{
		AICorpCountLabel->SetText(FText::FromString(FString::FromInt(Count)));
	}
}

void UGTNewGameWidget::HandleStartGameClicked()
{
	// Build world settings from current UI state.
	UGTWorldSettings* Settings = BuildWorldSettings();
	if (!Settings)
	{
		UE_LOG(LogTemp, Error, TEXT("GTNewGame: Failed to build world settings."));
		return;
	}

	const FString CorpName = GetCorporationName();

	// Broadcast to listeners (GameInstance code in GlobalTelco module binds to this).
	OnStartGameRequested.Broadcast(Settings, CorpName);
}

void UGTNewGameWidget::HandleBackClicked()
{
	OnBackRequested.Broadcast();
}

UGTWorldSettings* UGTNewGameWidget::BuildWorldSettings() const
{
	UGTWorldSettings* Settings = NewObject<UGTWorldSettings>();

	// Difficulty.
	if (DifficultyDropdown)
	{
		const FString Selected = DifficultyDropdown->GetSelectedOption();
		if (Selected == TEXT("Easy"))
		{
			Settings->Difficulty = EGTDifficulty::Easy;
		}
		else if (Selected == TEXT("Normal"))
		{
			Settings->Difficulty = EGTDifficulty::Normal;
		}
		else if (Selected == TEXT("Hard"))
		{
			Settings->Difficulty = EGTDifficulty::Hard;
		}
		else
		{
			Settings->Difficulty = EGTDifficulty::Custom;
		}
	}

	// Apply difficulty defaults first, then override with UI values.
	Settings->ApplyDifficultyDefaults();

	// AI corporation count.
	if (AICorpCountSlider)
	{
		Settings->AICorpCount = FMath::RoundToInt(AICorpCountSlider->GetValue());
	}

	// Disaster severity.
	if (DisasterSeverityDropdown)
	{
		const FString Selected = DisasterSeverityDropdown->GetSelectedOption();
		if (Selected == TEXT("Calm"))
		{
			Settings->DisasterSeverity = EGTDisasterSeverity::Calm;
			Settings->DisasterFrequencyMultiplier = 0.25f;
			Settings->DisasterDamageMultiplier = 0.5f;
		}
		else if (Selected == TEXT("Moderate"))
		{
			Settings->DisasterSeverity = EGTDisasterSeverity::Moderate;
			Settings->DisasterFrequencyMultiplier = 1.0f;
			Settings->DisasterDamageMultiplier = 1.0f;
		}
		else if (Selected == TEXT("Brutal"))
		{
			Settings->DisasterSeverity = EGTDisasterSeverity::Brutal;
			Settings->DisasterFrequencyMultiplier = 2.5f;
			Settings->DisasterDamageMultiplier = 2.0f;
		}
	}

	// World seed.
	if (WorldSeedInput)
	{
		const FString SeedStr = WorldSeedInput->GetText().ToString();
		Settings->WorldSeed = FCString::Atoi(*SeedStr);
	}

	return Settings;
}

FString UGTNewGameWidget::GetCorporationName() const
{
	if (CorporationNameInput)
	{
		FString Name = CorporationNameInput->GetText().ToString().TrimStartAndEnd();
		if (!Name.IsEmpty())
		{
			return Name;
		}
	}
	return TEXT("Player Corp");
}
