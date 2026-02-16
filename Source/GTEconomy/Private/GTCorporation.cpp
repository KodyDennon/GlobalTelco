#include "GTCorporation.h"

void UGTCorporation::ProcessEconomicTick(float TickDeltaSeconds)
{
	// Reset per-tick income statement.
	LastTickIncome = FGTIncomeStatement();

	// Calculate maintenance costs from owned infrastructure.
	// Actual cost calculation will query GTInfrastructure module for node/edge maintenance costs.

	// Apply interest on outstanding debt.
	// Interest expense is computed per debt instrument with individual rates.

	// Update balance sheet cash position.
	double NetIncome = LastTickIncome.NetIncome();
	BalanceSheet.CashOnHand += NetIncome;

	// Check insolvency.
	if (IsInsolvent())
	{
		// Trigger bankruptcy event via GTCore event queue.
	}

	RecalculateCreditRating();
}

void UGTCorporation::AddRevenue(double Amount, EGTRevenueSource Source)
{
	LastTickIncome.Revenue += Amount;
}

bool UGTCorporation::IssueDept(double Amount, EGTFinancialInstrument Instrument, float InterestRate)
{
	if (CreditRating == EGTCreditRating::Default || CreditRating == EGTCreditRating::CCC)
	{
		return false;
	}

	TotalDebt += Amount;
	BalanceSheet.CashOnHand += Amount;
	BalanceSheet.TotalLiabilities += Amount;

	return true;
}

bool UGTCorporation::IsInsolvent() const
{
	return BalanceSheet.TotalLiabilities > BalanceSheet.TotalAssets;
}

void UGTCorporation::RecalculateCreditRating()
{
	if (IsInsolvent())
	{
		CreditRating = EGTCreditRating::Default;
		return;
	}

	double EquityValue = BalanceSheet.TotalAssets - BalanceSheet.TotalLiabilities;
	double DebtToEquity = (EquityValue > 0.0) ? (TotalDebt / EquityValue) : TNumericLimits<double>::Max();

	if (DebtToEquity < 0.3)
	{
		CreditRating = EGTCreditRating::AAA;
	}
	else if (DebtToEquity < 0.5)
	{
		CreditRating = EGTCreditRating::AA;
	}
	else if (DebtToEquity < 0.8)
	{
		CreditRating = EGTCreditRating::A;
	}
	else if (DebtToEquity < 1.2)
	{
		CreditRating = EGTCreditRating::BBB;
	}
	else if (DebtToEquity < 2.0)
	{
		CreditRating = EGTCreditRating::BB;
	}
	else if (DebtToEquity < 3.0)
	{
		CreditRating = EGTCreditRating::B;
	}
	else
	{
		CreditRating = EGTCreditRating::CCC;
	}
}
