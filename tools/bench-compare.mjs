#!/usr/bin/env node
// Compare two prompt-bench reports produced by `cargo test --lib bench_prompts -- --ignored`
// (JSON form). Prints a per-scenario delta table; with --strict, exits 1 when a
// scenario regresses on a guarded metric (markdown leaks, refusals, errors up;
// repetition up by more than the tolerance; name usage or intention compliance down).
// Usage: node tools/bench-compare.mjs <before.json> <after.json> [--strict]
import { readFileSync } from "node:fs";

const TOLERANCE = 0.05;
const [, , beforePath, afterPath, flag] = process.argv;
if (!beforePath || !afterPath) {
  console.error("usage: node tools/bench-compare.mjs <before.json> <after.json> [--strict]");
  process.exit(2);
}
const strict = flag === "--strict";
const load = (p) => JSON.parse(readFileSync(p, "utf8"));
const before = load(beforePath);
const after = load(afterPath);

const METRICS = [
  ["interventions", "Interventions", null],
  ["repetition", "Répétition", "lower"],
  ["nameUsageRate", "Noms", "higher"],
  ["markdownLeaks", "Fuites MD", "zero"],
  ["refusals", "Refus", "zero"],
  ["avgLengthChars", "Longueur", null],
  ["intentionCompliance", "Intention", "higher"],
  ["errorEvents", "Erreurs", "zero"],
  ["avgSpeakerGapMs", "Écart (ms)", "lower"],
  ["argmapMaxDepth", "Prof. max", "higher"],
  ["argmapDeepShare", "Part ≥ 2", "higher"],
  ["argmapUnanswered", "Objections", "lower"],
  ["reactionsPerIntervention", "Réactions / interv.", null],
  ["reactionInsightfulShare", "Part 💡", "lower"],
  ["reactionCriticalShare", "Part critique", "higher"],
  ["emotionPeak", "Pic émotionnel", "lower"],
  ["emotionSaturatedShare", "Part saturée", "lower"],
];

const fmt = (v) => (v === null || v === undefined ? "—" : Number.isInteger(v) ? String(v) : v.toFixed(2));
const byName = (report) => new Map(report.scenarios.map((s) => [s.name, s.metrics]));
const b = byName(before);
const a = byName(after);

let regressions = 0;
console.log(`Avant : ${before.date} ${before.provider}\nAprès : ${after.date} ${after.provider}\n`);
for (const [name, am] of a) {
  const bm = b.get(name);
  console.log(`## ${name}${bm ? "" : " (nouveau scénario)"}`);
  console.log("| Métrique | Avant | Après | Δ | Verdict |\n|---|---|---|---|---|");
  for (const [key, label, direction] of METRICS) {
    const x = bm ? bm[key] : null;
    const y = am[key];
    const both = x !== null && x !== undefined && y !== null && y !== undefined;
    const delta = both ? y - x : null;
    let verdict = "";
    if (both && direction) {
      const worse =
        (direction === "lower" && delta > TOLERANCE * Math.max(1, Math.abs(x))) ||
        (direction === "higher" && delta < -TOLERANCE) ||
        (direction === "zero" && y > x);
      verdict = worse ? "⚠ régression" : "ok";
      if (worse) regressions += 1;
    }
    console.log(`| ${label} | ${fmt(x)} | ${fmt(y)} | ${delta === null ? "—" : fmt(delta)} | ${verdict} |`);
  }
  console.log("");
}
if (regressions > 0) {
  console.log(`${regressions} régression(s) détectée(s).`);
  if (strict) process.exit(1);
} else {
  console.log("Aucune régression.");
}
