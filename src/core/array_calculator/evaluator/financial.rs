//! Financial functions: PMT, FV, PV, NPV, IRR, NPER, RATE, SLN, DB, DDB, MIRR, XIRR, XNPV,
//! PPMT, IPMT, EFFECT, NOMINAL, PRICEDISC, YIELDDISC, ACCRINT

use super::{collect_numeric_values, evaluate, require_args, require_args_range};
use super::{EvalContext, EvalError, Expr, Value};

/// Try to evaluate a financial function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "PMT" => {
            require_args_range(name, args, 3, 5)?;
            let rate = evaluate(&args[0], ctx)?.as_number().unwrap_or(0.0);
            let nper = evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0);
            let pv = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0);
            let fv = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };
            let pmt_type = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };

            if rate == 0.0 {
                Value::Number(-(pv + fv) / nper)
            } else {
                let pmt = if pmt_type == 1 {
                    (-pv * rate * (1.0 + rate).powf(nper) - fv * rate)
                        / ((1.0 + rate).powf(nper) - 1.0)
                        / (1.0 + rate)
                } else {
                    (-pv * rate * (1.0 + rate).powf(nper) - fv * rate)
                        / ((1.0 + rate).powf(nper) - 1.0)
                };
                Value::Number(pmt)
            }
        },

        "FV" => {
            require_args_range(name, args, 3, 5)?;
            let rate = evaluate(&args[0], ctx)?.as_number().unwrap_or(0.0);
            let nper = evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0);
            let pmt = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0);
            let pv = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };

            if rate == 0.0 {
                Value::Number(-pv - pmt * nper)
            } else {
                let fv =
                    -pv * (1.0 + rate).powf(nper) - pmt * ((1.0 + rate).powf(nper) - 1.0) / rate;
                Value::Number(fv)
            }
        },

        "PV" => {
            require_args_range(name, args, 3, 5)?;
            let rate = evaluate(&args[0], ctx)?.as_number().unwrap_or(0.0);
            let nper = evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0);
            let pmt = evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0);
            let fv = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };

            if rate == 0.0 {
                Value::Number(-fv - pmt * nper)
            } else {
                let pv =
                    (-fv - pmt * ((1.0 + rate).powf(nper) - 1.0) / rate) / (1.0 + rate).powf(nper);
                Value::Number(pv)
            }
        },

        "NPV" => {
            require_args_range(name, args, 2, 255)?;
            let rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("NPV rate must be a number"))?;

            let mut npv = 0.0;
            let mut period = 1;

            for arg in &args[1..] {
                let val = evaluate(arg, ctx)?;
                match val {
                    Value::Array(arr) => {
                        for v in arr {
                            if let Some(n) = v.as_number() {
                                npv += n / (1.0 + rate).powi(period);
                                period += 1;
                            }
                        }
                    },
                    Value::Number(n) => {
                        npv += n / (1.0 + rate).powi(period);
                        period += 1;
                    },
                    _ => {},
                }
            }

            Value::Number(npv)
        },

        "IRR" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("IRR requires cash flows"));
            }
            let mut rate: f64 = 0.1;
            for _ in 0..100 {
                let mut npv: f64 = 0.0;
                let mut npv_deriv: f64 = 0.0;
                for (i, cf) in values.iter().enumerate() {
                    let factor = (1.0_f64 + rate).powi(i as i32);
                    npv += cf / factor;
                    if i > 0 {
                        npv_deriv -= (i as f64) * cf / (factor * (1.0 + rate));
                    }
                }
                if npv_deriv.abs() < 1e-10 {
                    break;
                }
                let new_rate = rate - npv / npv_deriv;
                if (new_rate - rate).abs() < 1e-7 {
                    rate = new_rate;
                    break;
                }
                rate = new_rate;
            }
            Value::Number(rate)
        },

        "XIRR" => {
            require_args(name, args, 2)?;
            let values = collect_numeric_values(&args[..1], ctx)?;
            let dates_val = evaluate(&args[1], ctx)?;
            let dates: Vec<f64> = match dates_val {
                Value::Array(arr) => arr.iter().filter_map(super::Value::as_number).collect(),
                Value::Number(n) => vec![n],
                _ => return Err(EvalError::new("XIRR requires dates")),
            };
            if values.len() != dates.len() || values.is_empty() {
                return Err(EvalError::new(
                    "XIRR: values and dates must have same length",
                ));
            }
            let base_date = dates[0];
            let year_fracs: Vec<f64> = dates.iter().map(|d| (d - base_date) / 365.0).collect();
            let mut rate: f64 = 0.1;
            for _ in 0..100 {
                let mut npv: f64 = 0.0;
                let mut npv_deriv: f64 = 0.0;
                for (i, cf) in values.iter().enumerate() {
                    let t = year_fracs[i];
                    let factor = (1.0_f64 + rate).powf(t);
                    npv += cf / factor;
                    if t != 0.0 {
                        npv_deriv -= t * cf / (factor * (1.0 + rate));
                    }
                }
                if npv_deriv.abs() < 1e-10 {
                    break;
                }
                let new_rate = rate - npv / npv_deriv;
                if (new_rate - rate).abs() < 1e-7 {
                    rate = new_rate;
                    break;
                }
                rate = new_rate;
            }
            Value::Number(rate)
        },

        "XNPV" => {
            require_args(name, args, 3)?;
            let rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("XNPV requires rate"))?;
            let values = collect_numeric_values(&args[1..2], ctx)?;
            let dates_val = evaluate(&args[2], ctx)?;
            let dates: Vec<f64> = match dates_val {
                Value::Array(arr) => arr.iter().filter_map(super::Value::as_number).collect(),
                Value::Number(n) => vec![n],
                _ => return Err(EvalError::new("XNPV requires dates")),
            };
            if values.len() != dates.len() || values.is_empty() {
                return Err(EvalError::new(
                    "XNPV: values and dates must have same length",
                ));
            }
            let base_date = dates[0];
            let mut npv = 0.0;
            for (i, cf) in values.iter().enumerate() {
                let t = (dates[i] - base_date) / 365.0;
                npv += cf / (1.0 + rate).powf(t);
            }
            Value::Number(npv)
        },

        "NPER" => {
            require_args_range(name, args, 3, 5)?;
            let rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("NPER requires rate"))?;
            let pmt = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("NPER requires payment"))?;
            let pv = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("NPER requires present value"))?;
            let fv = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };

            if rate == 0.0 {
                Value::Number(-(pv + fv) / pmt)
            } else {
                let n = ((-fv * rate + pmt) / (pv * rate + pmt)).ln() / (1.0 + rate).ln();
                Value::Number(n)
            }
        },

        "RATE" => {
            require_args_range(name, args, 3, 6)?;
            let nper = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("RATE requires nper"))?;
            let pmt = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("RATE requires payment"))?;
            let pv = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("RATE requires present value"))?;
            let fv = if args.len() > 3 {
                evaluate(&args[3], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };
            let guess = if args.len() > 5 {
                evaluate(&args[5], ctx)?.as_number().unwrap_or(0.1)
            } else {
                0.1
            };

            let mut rate = guess;
            for _ in 0..100 {
                let f = pv * (1.0 + rate).powf(nper)
                    + pmt * ((1.0 + rate).powf(nper) - 1.0) / rate
                    + fv;
                let f_deriv = nper * pv * (1.0 + rate).powf(nper - 1.0)
                    + pmt
                        * (nper * rate * (1.0 + rate).powf(nper - 1.0) - (1.0 + rate).powf(nper)
                            + 1.0)
                        / (rate * rate);
                if f_deriv.abs() < 1e-10 {
                    break;
                }
                let new_rate = rate - f / f_deriv;
                if (new_rate - rate).abs() < 1e-7 {
                    rate = new_rate;
                    break;
                }
                rate = new_rate;
            }
            Value::Number(rate)
        },

        "SLN" => {
            require_args(name, args, 3)?;
            let cost = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SLN requires cost"))?;
            let salvage = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SLN requires salvage"))?;
            let life = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SLN requires life"))?;

            if life == 0.0 {
                return Err(EvalError::new("SLN: life cannot be zero"));
            }
            Value::Number((cost - salvage) / life)
        },

        "DB" => {
            require_args_range(name, args, 4, 5)?;
            let cost = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DB requires cost"))?;
            let salvage = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DB requires salvage"))?;
            let life = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DB requires life"))?;
            let period = evaluate(&args[3], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DB requires period"))?;
            let month = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(12.0)
            } else {
                12.0
            };

            if life == 0.0 || cost == 0.0 {
                return Ok(Some(Value::Number(0.0)));
            }

            let rate = 1.0 - (salvage / cost).powf(1.0 / life);
            let rate = (rate * 1000.0).round() / 1000.0;

            let mut depreciation = 0.0;
            let mut remaining = cost;

            for p in 1..=(period as i32) {
                if p == 1 {
                    depreciation = cost * rate * month / 12.0;
                } else if p == (life as i32 + 1) {
                    depreciation = remaining * rate * (12.0 - month) / 12.0;
                } else {
                    depreciation = remaining * rate;
                }
                remaining -= depreciation;
            }

            Value::Number(depreciation)
        },

        "DDB" => {
            require_args_range(name, args, 4, 5)?;
            let cost = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DDB requires cost"))?;
            let salvage = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DDB requires salvage"))?;
            let life = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DDB requires life"))?;
            let period = evaluate(&args[3], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DDB requires period"))?;
            let factor = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(2.0)
            } else {
                2.0
            };

            if life == 0.0 {
                return Err(EvalError::new("DDB: life cannot be zero"));
            }

            let rate = factor / life;
            let mut remaining = cost;
            let mut depreciation = 0.0;

            for _p in 1..=(period as i32) {
                depreciation = remaining * rate;
                if remaining - depreciation < salvage {
                    depreciation = remaining - salvage;
                }
                if depreciation < 0.0 {
                    depreciation = 0.0;
                }
                remaining -= depreciation;
            }

            Value::Number(depreciation)
        },

        "MIRR" => {
            require_args(name, args, 3)?;
            let values = collect_numeric_values(&args[..1], ctx)?;
            let finance_rate = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("MIRR requires finance rate"))?;
            let reinvest_rate = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("MIRR requires reinvest rate"))?;

            if values.is_empty() {
                return Err(EvalError::new("MIRR requires cash flows"));
            }

            let n = values.len() as f64;
            let mut npv_neg = 0.0;
            let mut npv_pos = 0.0;

            for (i, &cf) in values.iter().enumerate() {
                if cf < 0.0 {
                    npv_neg += cf / (1.0 + finance_rate).powi(i as i32);
                } else {
                    npv_pos += cf * (1.0 + reinvest_rate).powi((n - 1.0 - i as f64) as i32);
                }
            }

            if npv_neg == 0.0 || npv_pos == 0.0 {
                return Err(EvalError::new(
                    "MIRR requires both positive and negative cash flows",
                ));
            }

            let mirr = (-npv_pos / npv_neg).powf(1.0 / (n - 1.0)) - 1.0;
            Value::Number(mirr)
        },

        "PPMT" => {
            // Principal payment for a given period
            // PPMT(rate, per, nper, pv, [fv], [type])
            require_args_range(name, args, 4, 6)?;
            let rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("PPMT requires rate"))?;
            let per = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("PPMT requires period"))?;
            let nper = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("PPMT requires nper"))?;
            let pv = evaluate(&args[3], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("PPMT requires present value"))?;
            let fv = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };
            let pmt_type = if args.len() > 5 {
                evaluate(&args[5], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };

            if rate == 0.0 {
                Value::Number(-(pv + fv) / nper)
            } else {
                // Calculate total payment (PMT)
                let payment = if pmt_type == 1 {
                    (-pv * rate * (1.0 + rate).powf(nper) - fv * rate)
                        / ((1.0 + rate).powf(nper) - 1.0)
                        / (1.0 + rate)
                } else {
                    (-pv * rate * (1.0 + rate).powf(nper) - fv * rate)
                        / ((1.0 + rate).powf(nper) - 1.0)
                };

                // Calculate balance at start of period
                // Use FV formula: FV = PV * (1+r)^n + PMT * ((1+r)^n - 1) / r
                // Rearranged to get remaining balance after (per-1) payments
                let periods_elapsed = per - 1.0;
                let balance = if periods_elapsed > 0.0 {
                    pv * (1.0 + rate).powf(periods_elapsed)
                        + payment * ((1.0 + rate).powf(periods_elapsed) - 1.0) / rate
                } else {
                    pv
                };

                // Interest for this period (as negative payment)
                let interest = -(balance * rate);

                // Principal payment = total payment - interest payment
                // Since both are negative (payments out), subtracting gives the principal portion
                Value::Number(payment - interest)
            }
        },

        "IPMT" => {
            // Interest payment for a given period
            // IPMT(rate, per, nper, pv, [fv], [type])
            require_args_range(name, args, 4, 6)?;
            let rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("IPMT requires rate"))?;
            let per = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("IPMT requires period"))?;
            let nper = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("IPMT requires nper"))?;
            let pv = evaluate(&args[3], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("IPMT requires present value"))?;
            let fv = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };
            let pmt_type = if args.len() > 5 {
                evaluate(&args[5], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };

            if rate == 0.0 {
                Value::Number(0.0)
            } else {
                // Calculate total payment (PMT)
                let payment = if pmt_type == 1 {
                    (-pv * rate * (1.0 + rate).powf(nper) - fv * rate)
                        / ((1.0 + rate).powf(nper) - 1.0)
                        / (1.0 + rate)
                } else {
                    (-pv * rate * (1.0 + rate).powf(nper) - fv * rate)
                        / ((1.0 + rate).powf(nper) - 1.0)
                };

                // Calculate balance at start of period
                let periods_elapsed = per - 1.0;
                let balance = if periods_elapsed > 0.0 {
                    pv * (1.0 + rate).powf(periods_elapsed)
                        + payment * ((1.0 + rate).powf(periods_elapsed) - 1.0) / rate
                } else {
                    pv
                };

                // Interest payment = -(balance * rate) - negative because it's a payment
                Value::Number(-(balance * rate))
            }
        },

        "EFFECT" => {
            // Effective annual interest rate
            // EFFECT(nominal_rate, npery)
            require_args(name, args, 2)?;
            let nominal_rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EFFECT requires nominal rate"))?;
            let npery = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EFFECT requires periods per year"))?;

            if npery < 1.0 {
                return Err(EvalError::new("EFFECT: periods per year must be >= 1"));
            }
            if nominal_rate <= 0.0 {
                return Err(EvalError::new("EFFECT: nominal rate must be positive"));
            }

            let effect = (1.0 + nominal_rate / npery).powf(npery) - 1.0;
            Value::Number(effect)
        },

        "NOMINAL" => {
            // Nominal annual interest rate
            // NOMINAL(effect_rate, npery)
            require_args(name, args, 2)?;
            let effect_rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("NOMINAL requires effective rate"))?;
            let npery = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("NOMINAL requires periods per year"))?;

            if npery < 1.0 {
                return Err(EvalError::new("NOMINAL: periods per year must be >= 1"));
            }
            if effect_rate <= 0.0 {
                return Err(EvalError::new("NOMINAL: effective rate must be positive"));
            }

            let nominal = ((1.0 + effect_rate).powf(1.0 / npery) - 1.0) * npery;
            Value::Number(nominal)
        },

        "PRICEDISC" => {
            // Price of a discounted security per $100 face value
            // PRICEDISC(settlement, maturity, discount, redemption, [basis])
            require_args_range(name, args, 4, 5)?;
            let settlement = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("PRICEDISC requires settlement date"))?;
            let maturity = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("PRICEDISC requires maturity date"))?;
            let discount = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("PRICEDISC requires discount rate"))?;
            let redemption = evaluate(&args[3], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("PRICEDISC requires redemption value"))?;
            let _basis = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };

            if settlement >= maturity {
                return Err(EvalError::new(
                    "PRICEDISC: settlement must be before maturity",
                ));
            }

            // Simplified calculation: assumes 360-day year (basis 0)
            let days = maturity - settlement;
            let frac = days / 360.0;
            let price = redemption - (discount * redemption * frac);
            Value::Number(price)
        },

        "YIELDDISC" => {
            // Annual yield of a discounted security
            // YIELDDISC(settlement, maturity, price, redemption, [basis])
            require_args_range(name, args, 4, 5)?;
            let settlement = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("YIELDDISC requires settlement date"))?;
            let maturity = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("YIELDDISC requires maturity date"))?;
            let price = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("YIELDDISC requires price"))?;
            let redemption = evaluate(&args[3], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("YIELDDISC requires redemption value"))?;
            let _basis = if args.len() > 4 {
                evaluate(&args[4], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };

            if settlement >= maturity {
                return Err(EvalError::new(
                    "YIELDDISC: settlement must be before maturity",
                ));
            }
            if price <= 0.0 {
                return Err(EvalError::new("YIELDDISC: price must be positive"));
            }

            // Simplified calculation: assumes 360-day year (basis 0)
            let days = maturity - settlement;
            let frac = days / 360.0;
            let yld = ((redemption - price) / price) * (1.0 / frac);
            Value::Number(yld)
        },

        "ACCRINT" => {
            // Accrued interest for a security that pays periodic interest
            // ACCRINT(issue, first_interest, settlement, rate, par, frequency, [basis])
            require_args_range(name, args, 6, 7)?;
            let issue = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ACCRINT requires issue date"))?;
            let _first_interest = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ACCRINT requires first interest date"))?;
            let settlement = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ACCRINT requires settlement date"))?;
            let rate = evaluate(&args[3], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ACCRINT requires rate"))?;
            let par = evaluate(&args[4], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ACCRINT requires par value"))?;
            let frequency = evaluate(&args[5], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ACCRINT requires frequency"))?;
            let _basis = if args.len() > 6 {
                evaluate(&args[6], ctx)?.as_number().unwrap_or(0.0)
            } else {
                0.0
            };

            if issue >= settlement {
                return Err(EvalError::new(
                    "ACCRINT: issue date must be before settlement",
                ));
            }
            if rate < 0.0 {
                return Err(EvalError::new("ACCRINT: rate must be non-negative"));
            }
            if par <= 0.0 {
                return Err(EvalError::new("ACCRINT: par value must be positive"));
            }
            if frequency != 1.0 && frequency != 2.0 && frequency != 4.0 {
                return Err(EvalError::new("ACCRINT: frequency must be 1, 2, or 4"));
            }

            // Simplified calculation: assumes 360-day year (basis 0)
            let days = settlement - issue;
            let accrued = par * rate * (days / 360.0);
            Value::Number(accrued)
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;

    #[test]
    fn test_pmt() {
        let ctx = EvalContext::new();
        // PMT for $100,000 loan at 5% for 30 years
        let pmt = eval("PMT(0.05/12, 360, 100000)", &ctx).unwrap();
        assert!(matches!(pmt, Value::Number(n) if (n + 536.82).abs() < 0.01));
    }

    #[test]
    fn test_sln() {
        let ctx = EvalContext::new();
        // Straight-line depreciation: cost=10000, salvage=1000, life=5
        assert_eq!(
            eval("SLN(10000, 1000, 5)", &ctx).unwrap(),
            Value::Number(1800.0)
        );
    }

    #[test]
    fn test_irr() {
        let ctx = EvalContext::new();
        // IRR for cash flows: -100, 30, 35, 40, 45
        let irr = eval("IRR(-100, 30, 35, 40, 45)", &ctx).unwrap();
        assert!(matches!(irr, Value::Number(n) if (n - 0.178).abs() < 0.01));
    }

    #[test]
    fn test_npv() {
        let ctx = EvalContext::new();
        // NPV at 10% for cash flows 100, 100, 100
        let npv = eval("NPV(0.10, 100, 100, 100)", &ctx).unwrap();
        assert!(matches!(npv, Value::Number(n) if (n - 248.69).abs() < 0.01));
    }

    // === EDGE CASES FOR 100% COVERAGE ===

    #[test]
    fn test_sln_zero_life() {
        let ctx = EvalContext::new();
        let result = eval("SLN(10000, 1000, 0)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("zero"));
    }

    #[test]
    fn test_db_zero_life() {
        let ctx = EvalContext::new();
        // DB with life = 0 returns 0
        assert_eq!(
            eval("DB(10000, 1000, 0, 1)", &ctx).unwrap(),
            Value::Number(0.0)
        );
    }

    #[test]
    fn test_db_zero_cost() {
        let ctx = EvalContext::new();
        // DB with cost = 0 returns 0
        assert_eq!(eval("DB(0, 0, 5, 1)", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_db_with_month() {
        let ctx = EvalContext::new();
        // DB with custom month parameter
        let result = eval("DB(10000, 1000, 5, 1, 6)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_db_last_period() {
        let ctx = EvalContext::new();
        // DB last period (life + 1)
        let result = eval("DB(10000, 1000, 5, 6, 6)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n >= 0.0));
    }

    #[test]
    fn test_ddb_zero_life() {
        let ctx = EvalContext::new();
        let result = eval("DDB(10000, 1000, 0, 1)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("zero"));
    }

    #[test]
    fn test_ddb_with_factor() {
        let ctx = EvalContext::new();
        // DDB with custom factor
        let result = eval("DDB(10000, 1000, 5, 1, 1.5)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_ddb_below_salvage() {
        let ctx = EvalContext::new();
        // DDB where depreciation would go below salvage
        // High salvage relative to cost
        let result = eval("DDB(10000, 9000, 5, 5)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n >= 0.0));
    }

    #[test]
    fn test_mirr_positive_flows_only() {
        let ctx = EvalContext::new();
        // MIRR requires both positive and negative flows
        let result = eval("MIRR({100, 200, 300}, 0.10, 0.12)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_nper_zero_rate() {
        let ctx = EvalContext::new();
        // NPER with rate = 0
        let result = eval("NPER(0, -100, 1000)", &ctx).unwrap();
        assert_eq!(result, Value::Number(10.0)); // 1000 / 100 = 10
    }

    #[test]
    fn test_nper_with_fv() {
        let ctx = EvalContext::new();
        // NPER with future value
        let result = eval("NPER(0.05, -100, 1000, 500)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_rate_function() {
        let ctx = EvalContext::new();
        // RATE: 12 periods, -100 payment, 1000 PV
        let result = eval("RATE(12, -100, 1000)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n.abs() < 0.1));
    }

    #[test]
    fn test_rate_with_fv() {
        let ctx = EvalContext::new();
        // RATE with future value
        let result = eval("RATE(12, -100, 1000, 100)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(_)));
    }

    #[test]
    fn test_rate_with_guess() {
        let ctx = EvalContext::new();
        // RATE with guess (6th param, skip 5th for pmt_type)
        let result = eval("RATE(12, -100, 1000, 0, 0, 0.05)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(_)));
    }

    #[test]
    fn test_fv_function() {
        let ctx = EvalContext::new();
        // FV: 5% rate, 12 periods, -100 payment
        let result = eval("FV(0.05, 12, -100)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_fv_with_pv() {
        let ctx = EvalContext::new();
        // FV with present value
        let result = eval("FV(0.05, 12, -100, -1000)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(_)));
    }

    #[test]
    fn test_pv_function() {
        let ctx = EvalContext::new();
        // PV: 5% rate, 12 periods, -100 payment
        let result = eval("PV(0.05, 12, -100)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_ppmt() {
        let ctx = EvalContext::new();
        // PPMT: Principal payment for period 1 of 60-month loan at 5% for $10,000
        let result = eval("PPMT(0.05/12, 1, 60, 10000)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n < 0.0));
    }

    #[test]
    fn test_ppmt_zero_rate() {
        let ctx = EvalContext::new();
        // PPMT with rate=0: simple division
        let result = eval("PPMT(0, 1, 12, 1200)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n + 100.0).abs() < 0.01));
    }

    #[test]
    fn test_ipmt() {
        let ctx = EvalContext::new();
        // IPMT: Interest payment for period 1 of 60-month loan at 5% for $10,000
        let result = eval("IPMT(0.05/12, 1, 60, 10000)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n < 0.0));
    }

    #[test]
    fn test_ipmt_zero_rate() {
        let ctx = EvalContext::new();
        // IPMT with rate=0: no interest
        let result = eval("IPMT(0, 1, 12, 1200)", &ctx).unwrap();
        assert_eq!(result, Value::Number(0.0));
    }

    #[test]
    fn test_effect() {
        let ctx = EvalContext::new();
        // EFFECT: 6% nominal rate compounded monthly
        let result = eval("EFFECT(0.06, 12)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.06 && n < 0.07));
    }

    #[test]
    fn test_effect_error_negative_rate() {
        let ctx = EvalContext::new();
        let result = eval("EFFECT(-0.05, 12)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_effect_error_invalid_periods() {
        let ctx = EvalContext::new();
        let result = eval("EFFECT(0.05, 0)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_nominal() {
        let ctx = EvalContext::new();
        // NOMINAL: 6.17% effective rate compounded monthly
        let result = eval("NOMINAL(0.0617, 12)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.05 && n < 0.07));
    }

    #[test]
    fn test_nominal_error_negative_rate() {
        let ctx = EvalContext::new();
        let result = eval("NOMINAL(-0.05, 12)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_nominal_error_invalid_periods() {
        let ctx = EvalContext::new();
        let result = eval("NOMINAL(0.05, 0)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_pricedisc() {
        let ctx = EvalContext::new();
        // PRICEDISC: $100 face value, 5% discount, 180 days
        // Settlement=0, Maturity=180, Discount=0.05, Redemption=100
        let result = eval("PRICEDISC(0, 180, 0.05, 100)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 95.0 && n < 100.0));
    }

    #[test]
    fn test_pricedisc_error_invalid_dates() {
        let ctx = EvalContext::new();
        // Settlement after maturity
        let result = eval("PRICEDISC(180, 0, 0.05, 100)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_yielddisc() {
        let ctx = EvalContext::new();
        // YIELDDISC: $97.50 price for $100 redemption, 180 days
        let result = eval("YIELDDISC(0, 180, 97.50, 100)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_yielddisc_error_invalid_dates() {
        let ctx = EvalContext::new();
        let result = eval("YIELDDISC(180, 0, 97.50, 100)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_yielddisc_error_invalid_price() {
        let ctx = EvalContext::new();
        let result = eval("YIELDDISC(0, 180, 0, 100)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_accrint() {
        let ctx = EvalContext::new();
        // ACCRINT: $1000 par, 6% rate, 180 days, annual payment
        // Issue=0, FirstInterest=365, Settlement=180, Rate=0.06, Par=1000, Frequency=1
        let result = eval("ACCRINT(0, 365, 180, 0.06, 1000, 1)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_accrint_error_invalid_dates() {
        let ctx = EvalContext::new();
        // Issue after settlement
        let result = eval("ACCRINT(180, 365, 0, 0.06, 1000, 1)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_accrint_error_negative_rate() {
        let ctx = EvalContext::new();
        let result = eval("ACCRINT(0, 365, 180, -0.06, 1000, 1)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_accrint_error_invalid_par() {
        let ctx = EvalContext::new();
        let result = eval("ACCRINT(0, 365, 180, 0.06, 0, 1)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_accrint_error_invalid_frequency() {
        let ctx = EvalContext::new();
        // Frequency must be 1, 2, or 4
        let result = eval("ACCRINT(0, 365, 180, 0.06, 1000, 3)", &ctx);
        assert!(result.is_err());
    }
}
