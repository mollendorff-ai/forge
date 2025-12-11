//! Financial function tests module
//!
//! This module contains tests for all financial functions organized by category:
//! - financial_basic: PMT, FV, PV, NPV, NPER, RATE and their edge cases
//! - financial_irr: IRR, XIRR, XNPV and their edge cases
//! - financial_advanced: MIRR, depreciation (DB, DDB, SLN), PPMT, IPMT, EFFECT, NOMINAL, PRICEDISC, YIELDDISC, ACCRINT

mod financial_advanced;
mod financial_basic;
mod financial_irr;
