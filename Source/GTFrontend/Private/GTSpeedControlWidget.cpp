#include "GTSpeedControlWidget.h"
#include "GTSimulationSubsystem.h"
#include "Components/Button.h"
#include "Components/TextBlock.h"
#include "Engine/World.h"

void UGTSpeedControlWidget::NativeConstruct()
{
	Super::NativeConstruct();

	if (PauseButton)
	{
		PauseButton->OnClicked.AddDynamic(this, &UGTSpeedControlWidget::HandlePauseClicked);
	}

	if (PlayButton)
	{
		PlayButton->OnClicked.AddDynamic(this, &UGTSpeedControlWidget::HandlePlayClicked);
	}

	if (FastButton)
	{
		FastButton->OnClicked.AddDynamic(this, &UGTSpeedControlWidget::HandleFastClicked);
	}

	if (FasterButton)
	{
		FasterButton->OnClicked.AddDynamic(this, &UGTSpeedControlWidget::HandleFasterClicked);
	}

	if (QuickSaveButton)
	{
		QuickSaveButton->OnClicked.AddDynamic(this, &UGTSpeedControlWidget::HandleQuickSaveClicked);
	}

	if (QuickLoadButton)
	{
		QuickLoadButton->OnClicked.AddDynamic(this, &UGTSpeedControlWidget::HandleQuickLoadClicked);
	}

	RefreshDisplay();
}

void UGTSpeedControlWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
	Super::NativeTick(MyGeometry, InDeltaTime);

	TimeSinceLastRefresh += InDeltaTime;
	if (TimeSinceLastRefresh >= DisplayRefreshInterval)
	{
		TimeSinceLastRefresh = 0.0f;
		RefreshDisplay();
	}
}

void UGTSpeedControlWidget::SetSpeed(float Multiplier)
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	UGTSimulationSubsystem* SimSub = World->GetSubsystem<UGTSimulationSubsystem>();
	if (SimSub)
	{
		SimSub->SetPaused(false);
		SimSub->SetSpeedMultiplier(Multiplier);
	}

	RefreshDisplay();
}

void UGTSpeedControlWidget::TogglePause()
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	UGTSimulationSubsystem* SimSub = World->GetSubsystem<UGTSimulationSubsystem>();
	if (SimSub)
	{
		SimSub->SetPaused(!SimSub->IsPaused());
	}

	RefreshDisplay();
}

void UGTSpeedControlWidget::RefreshDisplay()
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	UGTSimulationSubsystem* SimSub = World->GetSubsystem<UGTSimulationSubsystem>();
	if (!SimSub)
	{
		return;
	}

	// Update speed label.
	if (SpeedLabel)
	{
		if (SimSub->IsPaused())
		{
			SpeedLabel->SetText(FText::FromString(TEXT("PAUSED")));
		}
		else
		{
			const float Speed = SimSub->GetSpeedMultiplier();
			FString SpeedText;
			if (FMath::IsNearlyEqual(Speed, 1.0f))
			{
				SpeedText = TEXT("1x");
			}
			else if (FMath::IsNearlyEqual(Speed, 2.0f))
			{
				SpeedText = TEXT("2x");
			}
			else if (FMath::IsNearlyEqual(Speed, 4.0f))
			{
				SpeedText = TEXT("4x");
			}
			else
			{
				SpeedText = FString::Printf(TEXT("%.1fx"), Speed);
			}
			SpeedLabel->SetText(FText::FromString(SpeedText));
		}
	}

	// Update tick count.
	if (TickCountLabel)
	{
		const int64 CurrentTick = SimSub->GetCurrentTick();
		TickCountLabel->SetText(FText::FromString(
			FString::Printf(TEXT("Tick: %lld"), CurrentTick)
		));
	}
}

void UGTSpeedControlWidget::HandlePauseClicked()
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	UGTSimulationSubsystem* SimSub = World->GetSubsystem<UGTSimulationSubsystem>();
	if (SimSub)
	{
		SimSub->SetPaused(true);
	}

	RefreshDisplay();
}

void UGTSpeedControlWidget::HandlePlayClicked()
{
	SetSpeed(1.0f);
}

void UGTSpeedControlWidget::HandleFastClicked()
{
	SetSpeed(2.0f);
}

void UGTSpeedControlWidget::HandleFasterClicked()
{
	SetSpeed(4.0f);
}

void UGTSpeedControlWidget::HandleQuickSaveClicked()
{
	OnQuickSaveRequested.Broadcast();
}

void UGTSpeedControlWidget::HandleQuickLoadClicked()
{
	OnQuickLoadRequested.Broadcast();
}
