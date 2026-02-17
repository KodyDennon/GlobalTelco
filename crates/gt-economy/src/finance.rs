use gt_common::types::{CreditRating, Money};

/// Balance sheet snapshot for a corporation
#[derive(Debug, Clone)]
pub struct BalanceSheet {
    pub cash: Money,
    pub infrastructure_value: Money,
    pub total_assets: Money,
    pub total_debt: Money,
    pub equity: Money,
    pub debt_to_equity_ratio: f64,
}

impl BalanceSheet {
    pub fn compute(cash: Money, infra_value: Money, debt: Money) -> Self {
        let total_assets = cash + infra_value;
        let equity = total_assets - debt;
        let debt_to_equity_ratio = if equity > 0 {
            debt as f64 / equity as f64
        } else if debt > 0 {
            f64::MAX
        } else {
            0.0
        };
        Self {
            cash,
            infrastructure_value: infra_value,
            total_assets,
            total_debt: debt,
            equity,
            debt_to_equity_ratio,
        }
    }
}

/// Income statement for a tick period
#[derive(Debug, Clone)]
pub struct IncomeStatement {
    pub revenue: Money,
    pub infrastructure_revenue: Money,
    pub contract_revenue: Money,
    pub operating_cost: Money,
    pub maintenance_cost: Money,
    pub debt_service: Money,
    pub net_income: Money,
    pub profit_margin: f64,
}

impl IncomeStatement {
    pub fn compute(
        infrastructure_revenue: Money,
        contract_revenue: Money,
        operating_cost: Money,
        maintenance_cost: Money,
        debt_service: Money,
    ) -> Self {
        let revenue = infrastructure_revenue + contract_revenue;
        let total_cost = operating_cost + maintenance_cost + debt_service;
        let net_income = revenue - total_cost;
        let profit_margin = if revenue > 0 {
            net_income as f64 / revenue as f64
        } else {
            -1.0
        };
        Self {
            revenue,
            infrastructure_revenue,
            contract_revenue,
            operating_cost,
            maintenance_cost,
            debt_service,
            net_income,
            profit_margin,
        }
    }
}

/// Credit analysis result
#[derive(Debug, Clone)]
pub struct CreditAnalysis {
    pub debt_ratio: f64,
    pub profit_margin: f64,
    pub cash_ratio: f64,
    pub rating: CreditRating,
    pub is_bankrupt: bool,
}

impl CreditAnalysis {
    /// Compute credit rating from financial metrics
    pub fn analyze(cash: Money, debt: Money, revenue: Money, cost: Money) -> Self {
        let debt_ratio = if revenue > 0 {
            debt as f64 / (revenue as f64 * 365.0)
        } else if debt > 0 {
            10.0
        } else {
            0.0
        };

        let profit_margin = if revenue > 0 {
            (revenue - cost) as f64 / revenue as f64
        } else {
            -1.0
        };

        let cash_ratio = cash as f64 / (cost as f64 * 30.0).max(1.0);

        let rating = if debt_ratio < 0.5 && profit_margin > 0.2 && cash_ratio > 5.0 {
            CreditRating::AAA
        } else if debt_ratio < 1.0 && profit_margin > 0.1 && cash_ratio > 3.0 {
            CreditRating::AA
        } else if debt_ratio < 2.0 && profit_margin > 0.05 && cash_ratio > 1.0 {
            CreditRating::A
        } else if debt_ratio < 3.0 && profit_margin > 0.0 {
            CreditRating::BBB
        } else if debt_ratio < 5.0 && cash > 0 {
            CreditRating::BB
        } else if cash > 0 {
            CreditRating::B
        } else if cash > -cost * 30 {
            CreditRating::CCC
        } else {
            CreditRating::D
        };

        let is_bankrupt = rating == CreditRating::D && cash < -cost * 90;

        Self {
            debt_ratio,
            profit_margin,
            cash_ratio,
            rating,
            is_bankrupt,
        }
    }
}

/// Revenue calculation for a node based on its properties
pub struct RevenueCalculator;

impl RevenueCalculator {
    /// Calculate revenue rate for a node type
    pub fn node_revenue_rate(node_type: gt_common::types::NodeType) -> f64 {
        match node_type {
            gt_common::types::NodeType::DataCenter => 0.05,
            gt_common::types::NodeType::ExchangePoint => 0.03,
            gt_common::types::NodeType::SubmarineLanding => 0.08,
            gt_common::types::NodeType::SatelliteGround => 0.04,
            _ => 0.02,
        }
    }

    /// Calculate revenue for a single node
    pub fn compute_node_revenue(
        throughput: f64,
        node_type: gt_common::types::NodeType,
        utilization: f64,
        health_factor: f64,
    ) -> Money {
        let rate = Self::node_revenue_rate(node_type);
        let base = (throughput * rate) as Money;
        (base as f64 * utilization * health_factor) as Money
    }
}
