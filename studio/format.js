// How numbers look on screen. The numbers themselves are Forge's: this file
// rounds them for display and draws them, and never derives one from another.

// Whether a number is shown as a percentage. One rule, used by every place
// that shows or edits a number:
// - a unit decides when there is one: "%", "percent", "percentage" or "pct"
//   means a fraction shown as a percentage (Forge's VARIANCE_PCT gives 0.05
//   for 5%, and the examples write 0.72 with unit "%"); any other unit means
//   never a percentage, whatever the name;
// - with no unit, a name with one of these words in it (whole words, split on
//   "_" and "."), and only for a value of 1 or less either way: a rate of 1.38
//   is an exchange rate or a price, not 138%.
const PERCENT_UNITS = new Set(["%", "percent", "percentage", "pct"]);
const PERCENT_WORDS = new Set(["rate", "rates", "growth", "margin", "share", "pct", "percent", "percentage", "increase", "inflation", "wacc"]);
const YEARISH = /(^|_)(year|years)$/i;

export function isPercent(name = "", unit = null, value = undefined) {
  if (unit !== null && unit !== undefined && String(unit).trim() !== "") return PERCENT_UNITS.has(String(unit).trim().toLowerCase());
  const words = String(name).toLowerCase().split(/[._]/);
  if (!words.some((w) => PERCENT_WORDS.has(w))) return false;
  return value === undefined || (typeof value === "number" && Number.isFinite(value) && Math.abs(value) <= 1);
}

export function fmt(v, name = "", unit = null) {
  if (v === null || v === undefined) return "—";
  if (Array.isArray(v)) return v.length + " values";
  if (typeof v === "string") return v;
  if (typeof v === "boolean") return v ? "TRUE" : "FALSE";
  if (typeof v !== "number" || !Number.isFinite(v)) return String(v);
  const minus = v < 0 ? "−" : "";
  const a = Math.abs(v);
  if (isPercent(name, unit, v)) {
    const opts = a < 0.01 && a > 0 ? { maximumSignificantDigits: 3 } : { maximumFractionDigits: a < 0.1 ? 2 : 1 };
    return minus + (a * 100).toLocaleString("en-US", opts) + "%";
  }
  if (YEARISH.test(name) && Number.isInteger(v) && v >= 1900 && v <= 2200) return String(v);
  const digits = a >= 1000 ? (Number.isInteger(a) ? 0 : 2) : a >= 1 ? (a >= 10 ? 2 : 3) : 4;
  return minus + a.toLocaleString("en-US", { maximumFractionDigits: digits });
}

export function short(v, name = "", unit = null) {
  if (typeof v !== "number") return String(v);
  const a = Math.abs(v), minus = v < 0 ? "−" : "";
  if (isPercent(name, unit, v)) return fmt(v, name, unit);
  if (a >= 1e6) return minus + (a / 1e6).toFixed(a >= 1e7 ? 1 : 2) + "M";
  if (a >= 1e4) return minus + (a / 1e3).toFixed(a >= 1e5 ? 0 : 1) + "k";
  return fmt(Number(v.toFixed(2)), name, unit);
}

// The range of an input's slider: from 0 (or the negative) to twice the value,
// in steps of about a hundredth of its order of magnitude.
export function sliderRange(v, name = "", unit = null) {
  if (!Number.isFinite(v)) return { min: 0, max: 100, step: 1 };
  const a = Math.abs(v);
  if (isPercent(name, unit, v) || (a < 1 && !Number.isInteger(v))) {
    return { min: 0, max: Math.max(Number((a * 3).toPrecision(2)), 0.1), step: 0.0001 };
  }
  if (v === 0) return { min: 0, max: 1000, step: 1 };
  const magnitude = Number(a.toExponential().split("e")[1]);
  const step = Number(`1e${magnitude - 2}`);
  return { min: v < 0 ? -2 * a : 0, max: 2 * a, step: Number.isInteger(v) ? Math.max(1, step) : step };
}

export function spark(vals) {
  const w = 96, h = 30, pad = 3;
  if (!vals.length) return "";
  const lo = Math.min(...vals), hi = Math.max(...vals), span = hi - lo || 1;
  const x = (i) => pad + (i * (w - 2 * pad)) / Math.max(1, vals.length - 1);
  const y = (v) => h - pad - ((v - lo) / span) * (h - 2 * pad);
  const pts = vals.map((v, i) => `${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
  const zero = lo < 0 && hi > 0 ? `<line x1="${pad}" x2="${w - pad}" y1="${y(0).toFixed(1)}" y2="${y(0).toFixed(1)}" stroke="var(--line)" stroke-width="1" stroke-dasharray="2 2"/>` : "";
  const last = vals.length - 1;
  return `<svg class="spark" viewBox="0 0 ${w} ${h}" aria-hidden="true">${zero}<polygon points="${x(0)},${h - pad} ${pts} ${x(last)},${h - pad}" fill="var(--accent-soft)"/><polyline points="${pts}" fill="none" stroke="var(--accent)" stroke-width="1.6" stroke-linejoin="round"/><circle cx="${x(last).toFixed(1)}" cy="${y(vals[last]).toFixed(1)}" r="2.6" fill="var(--accent)"/></svg>`;
}

// A number typed by a person: "1,250", "−3" and "1e-5" are read. A "%" at the
// end means hundredths, so "4.5 %" is 0.045, the way a percentage is held in
// a model (and shown on its card); the decimal point is moved in the digits,
// so no rounding is added. Anything else is null.
export function readNumber(text) {
  let t = String(text).trim().replace(/[,\s]/g, "").replace("−", "-");
  const percent = t.endsWith("%");
  if (percent) t = t.slice(0, -1);
  if (!/^[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?$/.test(t)) return null;
  if (!percent) return Number(t);
  const [mantissa, exponent = "0"] = t.split(/[eE]/);
  return Number(`${mantissa}e${Number(exponent) - 2}`);
}
