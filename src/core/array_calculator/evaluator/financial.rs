//! Financial functions: PMT, FV, PV, NPV, IRR, NPER, RATE, SLN, DB, DDB, MIRR, XIRR, XNPV

use super::{
    collect_numeric_values, evaluate, require_args, require_args_range, EvalContext, EvalError,
    Expr, Value,
};

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
        }

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
        }

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
        }

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
                    }
                    Value::Number(n) => {
                        npv += n / (1.0 + rate).powi(period);
                        period += 1;
                    }
                    _ => {}
                }
            }

            Value::Number(npv)
        }

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
        }

        "XIRR" => {
            require_args(name, args, 2)?;
            let values = collect_numeric_values(&args[..1], ctx)?;
            let dates_val = evaluate(&args[1], ctx)?;
            let dates: Vec<f64> = match dates_val {
                Value::Array(arr) => arr.iter().filter_map(|v| v.as_number()).collect(),
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
        }

        "XNPV" => {
            require_args(name, args, 3)?;
            let rate = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("XNPV requires rate"))?;
            let values = collect_numeric_values(&args[1..2], ctx)?;
            let dates_val = evaluate(&args[2], ctx)?;
            let dates: Vec<f64> = match dates_val {
                Value::Array(arr) => arr.iter().filter_map(|v| v.as_number()).collect(),
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
        }

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
        }

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
        }

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
        }

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
        }

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
        }

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
        }

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
}
