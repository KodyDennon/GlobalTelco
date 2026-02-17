#include "GTCorporation.h"

void UGTCorporation::ProcessEconomicTick(float TickDeltaSeconds)
{
	// Revenue and maintenance are set externally by UGTRevenueCalculator
	// before this method is called. We handle debt interest and net income here.

	// Accrue interest on each debt instrument.
	double TotalInterest = 0.0;
	for (FGTDebtInstrument& Debt : DebtInstruments)
	{
		// Per-tick interest = Principal * (AnnualRate / 100) * (TickDelta / SecondsPerYear).
		// Simplified: treat each tick as a discrete period. Rate is annual percentage.
		// With ~4s ticks, ~8640 ticks/day, ~3.15M ticks/year.
		// Use a simpler model: per-tick rate = AnnualRate / (365 * 24 * 3600 / TickDelta).
		// For gameplay feel, use rate / 1000 per tick (roughly quarterly at 4s ticks).
		const double TickInterest = Debt.Principal * (Debt.InterestRate / 100.0) / 1000.0;
		TotalInterest += TickInterest;
	}
	LastTickIncome.InterestExpense += TotalInterest;

	// Update balance sheet cash position from net income.
	const double NetIncome = LastTickIncome.NetIncome();
	BalanceSheet.CashOnHand += NetIncome;

	// Recompute TotalDebt from individual instruments.
	TotalDebt = 0.0;
	for (const FGTDebtInstrument& Debt : DebtInstruments)
	{
		TotalDebt += Debt.Principal;
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

	// Create individual debt instrument.
	FGTDebtInstrument NewDebt;
	NewDebt.InstrumentType = Instrument;
	NewDebt.Principal = Amount;
	NewDebt.InterestRate = InterestRate;
	NewDebt.MaturityTick = -1; // Perpetual by default; callers can set maturity.
	NewDebt.IssuedTick = 0;    // Caller should set this from simulation tick.
	DebtInstruments.Add(NewDebt);

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
