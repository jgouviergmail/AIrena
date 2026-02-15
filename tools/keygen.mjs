#!/usr/bin/env node

/**
 * AIrena License Key Generator
 *
 * Standalone script using Node.js native crypto (zero npm dependencies).
 *
 * Usage:
 *   node tools/keygen.mjs generate --email user@example.com --duration 720
 *   node tools/keygen.mjs inspect <KEY>
 *
 * First run generates keypair + AES key in tools/.keys.json and
 * prints hex constants to paste into src-tauri/src/constants.rs.
 */

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { createCipheriv, createDecipheriv, randomBytes, generateKeyPairSync, sign, verify, createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const KEYS_PATH = join(__dirname, ".keys.json");

// ── Key management ──────────────────────────────────────────────────

function ensureKeys() {
  if (existsSync(KEYS_PATH)) {
    return JSON.parse(readFileSync(KEYS_PATH, "utf-8"));
  }

  console.log("Generating new keypair + AES key...\n");

  // Ed25519 keypair
  const { publicKey, privateKey } = generateKeyPairSync("ed25519", {
    publicKeyEncoding: { type: "spki", format: "der" },
    privateKeyEncoding: { type: "pkcs8", format: "der" },
  });

  // Extract raw 32-byte public key from SPKI DER (12-byte header for Ed25519)
  const rawPublicKey = publicKey.subarray(12);

  // AES-256 key
  const aesKey = randomBytes(32);

  const keys = {
    ed25519_public_hex: rawPublicKey.toString("hex"),
    ed25519_private_der_b64: privateKey.toString("base64"),
    aes_key_hex: aesKey.toString("hex"),
  };

  writeFileSync(KEYS_PATH, JSON.stringify(keys, null, 2), "utf-8");

  console.log("Keys saved to tools/.keys.json\n");
  console.log("=== Paste these into src-tauri/src/constants.rs ===\n");
  console.log(`pub const LICENSE_ED25519_PUBLIC_KEY_HEX: &str = "${keys.ed25519_public_hex}";`);
  console.log(`pub const LICENSE_AES_KEY_HEX: &str = "${keys.aes_key_hex}";`);
  console.log("");

  return keys;
}

function loadKeys() {
  if (!existsSync(KEYS_PATH)) {
    return ensureKeys();
  }
  return JSON.parse(readFileSync(KEYS_PATH, "utf-8"));
}

// ── Generate ────────────────────────────────────────────────────────

function generateKey(email, durationHours) {
  const keys = loadKeys();

  // 1. Build payload
  const payload = JSON.stringify({
    v: 1,
    e: email,
    t: Math.floor(Date.now() / 1000),
    d: durationHours,
    n: randomBytes(8).toString("hex"),
  });
  const payloadBytes = Buffer.from(payload, "utf-8");

  // 2. Sign with Ed25519
  const privateDer = Buffer.from(keys.ed25519_private_der_b64, "base64");
  const privateKeyObj = {
    key: privateDer,
    format: "der",
    type: "pkcs8",
  };
  const signature = sign(null, payloadBytes, privateKeyObj);

  // 3. Concatenate payload + signature
  const plaintext = Buffer.concat([payloadBytes, signature]);

  // 4. AES-256-GCM encrypt
  const aesKey = Buffer.from(keys.aes_key_hex, "hex");
  const nonce = randomBytes(12);
  const cipher = createCipheriv("aes-256-gcm", aesKey, nonce);
  const encrypted = Buffer.concat([cipher.update(plaintext), cipher.final()]);
  const authTag = cipher.getAuthTag();

  // 5. blob = nonce(12) + ciphertext + tag(16)
  const blob = Buffer.concat([nonce, encrypted, authTag]);

  // 6. Base64 standard → segment with dashes
  const b64 = blob.toString("base64");
  const segments = b64.match(/.{1,5}/g).join("-");
  const key = `AIRENA-${segments}`;

  // Compute max discussions
  const maxDiscussions = Math.ceil((50 * durationHours) / 24);
  const expiresAt = new Date((JSON.parse(payload).t + durationHours * 3600) * 1000);

  console.log("\n=== Generated License Key ===\n");
  console.log(key);
  console.log(`\nEmail:            ${email}`);
  console.log(`Duration:         ${durationHours}h (${(durationHours / 24).toFixed(1)} days)`);
  console.log(`Max discussions:  ${maxDiscussions}`);
  console.log(`Expires at:       ${expiresAt.toISOString()}`);
  console.log(`SHA-256:          ${createHash("sha256").update(key).digest("hex")}`);

  return key;
}

// ── Inspect ─────────────────────────────────────────────────────────

function inspectKey(key) {
  const keys = loadKeys();

  // 1. Strip prefix
  if (!key.startsWith("AIRENA-")) {
    console.error("ERROR: Key must start with AIRENA-");
    process.exit(1);
  }
  const withoutPrefix = key.slice("AIRENA-".length);

  // 2. Remove dashes → Base64 decode
  const b64 = withoutPrefix.replace(/-/g, "");
  const blob = Buffer.from(b64, "base64");

  if (blob.length < 93) {
    console.error("ERROR: Key too short");
    process.exit(1);
  }

  // 3. Split nonce + ciphertext+tag
  const nonce = blob.subarray(0, 12);
  const ciphertextWithTag = blob.subarray(12);

  // 4. AES-256-GCM decrypt
  const aesKey = Buffer.from(keys.aes_key_hex, "hex");
  // GCM tag is the last 16 bytes
  const ciphertext = ciphertextWithTag.subarray(0, ciphertextWithTag.length - 16);
  const authTag = ciphertextWithTag.subarray(ciphertextWithTag.length - 16);

  const decipher = createDecipheriv("aes-256-gcm", aesKey, nonce);
  decipher.setAuthTag(authTag);

  let plaintext;
  try {
    plaintext = Buffer.concat([decipher.update(ciphertext), decipher.final()]);
  } catch {
    console.error("ERROR: Decryption failed (invalid or tampered key)");
    process.exit(1);
  }

  // 5. Split payload + signature
  if (plaintext.length < 65) {
    console.error("ERROR: Decrypted content too short");
    process.exit(1);
  }
  const payloadBytes = plaintext.subarray(0, plaintext.length - 64);
  const sigBytes = plaintext.subarray(plaintext.length - 64);

  // 6. Ed25519 verify
  // Reconstruct SPKI DER: 12-byte header + 32-byte raw public key
  const spkiHeader = Buffer.from("302a300506032b6570032100", "hex");
  const rawPubKey = Buffer.from(keys.ed25519_public_hex, "hex");
  const spkiDer = Buffer.concat([spkiHeader, rawPubKey]);
  const pubKeyObj = {
    key: spkiDer,
    format: "der",
    type: "spki",
  };

  const sigValid = verify(null, payloadBytes, pubKeyObj, sigBytes);

  // 7. Parse payload
  const payload = JSON.parse(payloadBytes.toString("utf-8"));
  const expiresAt = new Date((payload.t + payload.d * 3600) * 1000);
  const createdAt = new Date(payload.t * 1000);
  const now = new Date();
  const maxDiscussions = Math.ceil((50 * payload.d) / 24);
  const expired = now > expiresAt;

  console.log("\n=== License Key Inspection ===\n");
  console.log(`Signature:        ${sigValid ? "VALID" : "INVALID"}`);
  console.log(`Version:          ${payload.v}`);
  console.log(`Email:            ${payload.e}`);
  console.log(`Created:          ${createdAt.toISOString()}`);
  console.log(`Duration:         ${payload.d}h (${(payload.d / 24).toFixed(1)} days)`);
  console.log(`Expires:          ${expiresAt.toISOString()}`);
  console.log(`Max discussions:  ${maxDiscussions}`);
  console.log(`Nonce:            ${payload.n}`);
  console.log(`Status:           ${expired ? "EXPIRED" : "ACTIVE"}`);
  console.log(`SHA-256:          ${createHash("sha256").update(key).digest("hex")}`);
}

// ── CLI ─────────────────────────────────────────────────────────────

const args = process.argv.slice(2);
const command = args[0];

if (command === "generate") {
  let email = "";
  let duration = 0;

  for (let i = 1; i < args.length; i++) {
    if (args[i] === "--email" && args[i + 1]) {
      email = args[++i];
    } else if (args[i] === "--duration" && args[i + 1]) {
      duration = parseInt(args[++i], 10);
    }
  }

  if (!email || !duration) {
    console.error("Usage: node tools/keygen.mjs generate --email <email> --duration <hours>");
    process.exit(1);
  }

  generateKey(email, duration);
} else if (command === "inspect") {
  const key = args[1];
  if (!key) {
    console.error("Usage: node tools/keygen.mjs inspect <KEY>");
    process.exit(1);
  }
  inspectKey(key);
} else if (command === "init") {
  ensureKeys();
} else {
  console.log("AIrena License Key Generator\n");
  console.log("Commands:");
  console.log("  generate --email <email> --duration <hours>  Generate a new license key");
  console.log("  inspect <KEY>                                Inspect/decode a license key");
  console.log("  init                                         Generate keypair + AES key only");
}
