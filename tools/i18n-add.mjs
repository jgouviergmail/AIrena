#!/usr/bin/env node
// Dev helper: merge a JSON file of new keys ({ fr: {...}, en: {...}, zh: {...} },
// nested by section) into the locale files. Existing keys are overwritten only
// when the input provides them. Usage: node tools/i18n-add.mjs additions.json
import { readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const LOCALES_DIR = join(dirname(fileURLToPath(import.meta.url)), "..", "src", "i18n", "locales");
const input = JSON.parse(readFileSync(process.argv[2], "utf8"));

const merge = (target, source) => {
  for (const [k, v] of Object.entries(source)) {
    if (v !== null && typeof v === "object") {
      target[k] = merge(target[k] ?? {}, v);
    } else {
      target[k] = v;
    }
  }
  return target;
};

for (const [locale, additions] of Object.entries(input)) {
  const file = join(LOCALES_DIR, `${locale}.json`);
  const current = JSON.parse(readFileSync(file, "utf8"));
  const merged = merge(current, additions);
  writeFileSync(file, JSON.stringify(merged, null, 2) + "\n");
  console.log(`merged ${locale}.json`);
}
