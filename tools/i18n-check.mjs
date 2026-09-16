#!/usr/bin/env node
// Locale parity gate: every key present in the reference locale (fr) must exist
// in every other locale, and no locale may carry keys the reference lacks.
// Wired into `npm run build` so a missing translation fails the build.
import { readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const LOCALES_DIR = join(dirname(fileURLToPath(import.meta.url)), "..", "src", "i18n", "locales");
const REFERENCE = "fr";

const flatten = (obj, prefix = "") =>
  Object.entries(obj).flatMap(([k, v]) =>
    v !== null && typeof v === "object" ? flatten(v, `${prefix}${k}.`) : [`${prefix}${k}`],
  );

const load = (name) => JSON.parse(readFileSync(join(LOCALES_DIR, `${name}.json`), "utf8"));

const locales = readdirSync(LOCALES_DIR)
  .filter((f) => f.endsWith(".json"))
  .map((f) => f.replace(/\.json$/, ""));

const reference = new Set(flatten(load(REFERENCE)));
let failures = 0;

for (const locale of locales) {
  if (locale === REFERENCE) continue;
  const keys = new Set(flatten(load(locale)));
  const missing = [...reference].filter((k) => !keys.has(k));
  const extra = [...keys].filter((k) => !reference.has(k));
  if (missing.length || extra.length) {
    failures += missing.length + extra.length;
    console.error(`✖ ${locale}.json — ${missing.length} missing, ${extra.length} extra key(s)`);
    for (const k of missing) console.error(`   missing: ${k}`);
    for (const k of extra) console.error(`   extra:   ${k}`);
  } else {
    console.log(`✔ ${locale}.json — ${keys.size} keys, in sync with ${REFERENCE}.json`);
  }
}

if (failures > 0) {
  console.error(`\ni18n parity check failed (${failures} problem(s)).`);
  process.exit(1);
}
console.log(`i18n parity OK (${reference.size} keys × ${locales.length} locales).`);
