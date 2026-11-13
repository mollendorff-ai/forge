// How Forge Studio decides a number is a percentage (studio/format.js,
// isPercent), and how it reads a number a person types (readNumber).
//
// A unit decides when the model gives one; with no unit, the name decides,
// for a value of 1 or less. The cases are the ones that showed misleading
// numbers before (exchange_rate 1.38 as 138%) and the ones that must stay.
//
//   node studio/tests/format.mjs

import { fmt, short, sliderRange, isPercent, readNumber } from "../format.js";

const failures = [];
const same = (what, got, want) => { if (got !== want) failures.push(`${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`); };

// The percent decision, through fmt
same("exchange_rate 1.38, no unit", fmt(1.38, "exchange_rate"), "1.38");
same("hourly_rate 4.5, no unit", fmt(4.5, "hourly_rate"), "4.5");
same("margin 3, no unit", fmt(3, "margin"), "3");
same("market_share_units 2, no unit", fmt(2, "market_share_units"), "2");
same("renewal_increase 0.04, no unit", fmt(0.04, "renewal_increase"), "4%");
same("unit %: a fraction shown as a percentage", fmt(0.72, "contribution_margin_ratio", "%"), "72%");
same("unit %, above 1", fmt(1.25, "growth_multiple", "%"), "125%");
same("money unit: never a percentage", fmt(0.03, "fee_rate", "CAD"), "0.03");
same("money unit, a rate's name", fmt(0.5, "rate", "$/month"), "0.5");
same("whole words: share_of_pay", fmt(0.33, "share_of_pay"), "33%");
same("whole words: corporate_tax is not a rate", fmt(0.26, "corporate_tax"), "0.26");
same("monthly_growth 0.03", fmt(0.03, "monthly_growth"), "3%");
same("negative rate", fmt(-0.05, "revenue_pct"), "−5%");

// The same decision everywhere it is used
same("isPercent, unit wins over the name", isPercent("card_fee_rate", "CAD", 0.026), false);
same("isPercent, percent unit wins over the name", isPercent("units", "percent", 0.2), true);
same("isPercent, name only", isPercent("card_fee_rate", null, 0.026), true);
same("short, money unit", short(0.03, "fee_rate", "CAD"), "0.03");
same("short, percent", short(0.03, "fee_rate"), "3%");
same("short, exchange rate", short(1.38, "exchange_rate"), "1.38");
same("sliderRange, exchange rate is not a percentage", sliderRange(1.38, "exchange_rate").step !== 0.0001, true);
same("sliderRange, percent unit", sliderRange(0.72, "x", "%").step, 0.0001);

// What a person types
same('"4.5 %" is 0.045, not 4.5', readNumber("4.5 %"), 0.045);
same('"4.5%"', readNumber("4.5%"), 0.045);
same('"1.1%" has no rounding added', readNumber("1.1%"), 0.011);
same('"12%"', readNumber("12%"), 0.12);
same('"−3"', readNumber("−3"), -3);
same('"1e-5"', readNumber("1e-5"), 0.00001);
same('"1,250"', readNumber("1,250"), 1250);
same('"abc"', readNumber("abc"), null);
// A French keyboard: a decimal comma, spaces between groups
same('"0,03" is 0.03, not 3', readNumber("0,03"), 0.03);
same('"3,5"', readNumber("3,5"), 3.5);
same('"1,5"', readNumber("1,5"), 1.5);
same('"12,34"', readNumber("12,34"), 12.34);
same('"4,5 %" is 0.045', readNumber("4,5 %"), 0.045);
same('"4,5%"', readNumber("4,5%"), 0.045);
same('"1 234,5"', readNumber("1 234,5"), 1234.5);
same('"1 234,5" with a no-break space', readNumber("1\u00a0234,5"), 1234.5);
same('"1 234,5" with a narrow no-break space', readNumber("1\u202f234,5"), 1234.5);
same('"1.234,56"', readNumber("1.234,56"), 1234.56);
same('"1,234.56"', readNumber("1,234.56"), 1234.56);
same('"1,234,567"', readNumber("1,234,567"), 1234567);
same('"1.234.567"', readNumber("1.234.567"), 1234567);
same('"0,250" is a decimal, a thousand never starts with 0', readNumber("0,250"), 0.25);
same('"-0,5"', readNumber("-0,5"), -0.5);
same('"1,2e3"', readNumber("1,2e3"), 1200);
same('"1,2,3,4" is not a number', readNumber("1,2,3,4"), null);
same('"1.2.3" is not a number', readNumber("1.2.3"), null);
same('"1,2.3,4" is not a number', readNumber("1,2.3,4"), null);
same('"," alone', readNumber(","), null);
same('"%"', readNumber("%"), null);

if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log("Number display: the unit decides a percentage, the name only without one and only up to 1; typed percentages are read as hundredths.");
