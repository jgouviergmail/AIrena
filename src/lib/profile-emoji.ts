// Profile name → emoji mapping for predefined profiles
const PROFILE_EMOJIS: Record<string, string> = {
  "Le Scientifique": "🔬",
  "Le Philosophe": "🤔",
  "L'Avocat du Diable": "😈",
  "Le Créatif": "🎨",
  "Le Pragmatique": "🔧",
  "L'Optimiste": "☀️",
  "Le Critique": "🧐",
  "L'Informaticien": "💻",
  "La Féministe": "♀️",
  "Le Masculiniste": "♂️",
  "Le Complotiste": "🔺",
  "L'Humoriste": "🃏",
  "Le Pilier de Bar": "🍺",
  "Le Politicien": "🏛️",
  "L'Extra-terrestre": "👽",
  "Le Chien": "🐕",
  "Le Chat": "🐈",
  "Le Pessimiste": "😩",
  "Dieu": "✝️",
  "Satan": "😈",
  "La Singularité": "🤖",
  // New profiles
  "L'Historien": "📜",
  "Le Biologiste": "🧬",
  "Le Géographe": "🌍",
  "Le Mathématicien": "📐",
  "Le Physicien": "⚛️",
  "Le Chimiste": "🧪",
  "Le Climatologue": "🌡️",
  "Le Product Owner": "📋",
  "Le Chef de Projet": "📊",
  "Le Marketing": "📢",
  "Le Hackeur": "🔓",
  "Le DevOps": "🐳",
  "Le RSSI": "🛡️",
  "Le Comptable": "🧮",
  "Le Financier": "💰",
  "Le Tradeur": "📈",
  // Personnalités
  "Socrate": "🏛️",
  "Friedrich Nietzsche": "🔨",
  "Voltaire": "✒️",
  "Nicolas Machiavel": "🦊",
  "Sun Tzu": "⚔️",
  "Napoléon Bonaparte": "🫡",
  "Charles Darwin": "🐢",
  "Albert Einstein": "💡",
  "Karl Marx": "✊",
  "Winston Churchill": "🎩",
  // Arbitre profiles
  "Le Modérateur Impartial": "⚖️",
  "Le Provocateur": "🔥",
  "Le Maïeuticien": "❓",
  "Le Juge Strict": "🔨",
  "L'Animateur TV": "📺",
  "Le Thérapeute": "💚",
  "Le Roi Philosophe": "👑",
  "L'Agent du Chaos": "🌀",
  "Le Directeur Scientifique": "🔬",
  "La Grand-mère": "🧶",
};

// Keyword → emoji mapping for custom profiles (checked against name + systemPrompt)
const KEYWORD_EMOJIS: [RegExp, string][] = [
  [/scien|recherch|labor|données|preuve|hypothèse|scientist|research|data|evidence/i, "🔬"],
  [/philos|éthique|moral|penseur|philosopher|ethics/i, "🤔"],
  [/diable|contre-pied|provocat|challeng|devil|advocate/i, "😈"],
  [/créati|imagin|artist|métaphor|innov|creative|imagination/i, "🎨"],
  [/pragmati|concret|réalist|faisab|pratique|realistic|feasib|practical/i, "🔧"],
  [/optimist|positif|constructi|opportunit|positive|constructive/i, "☀️"],
  [/critiqu|analyti|exigean|rigoureu|critic|analytic|demanding/i, "🧐"],
  [/histori|passé|héritage|historian|history|heritage/i, "📜"],
  [/économi|financ|marché|econom|finance|market/i, "📊"],
  [/tech|ingénieur|program|code|numériq|engineer|digital/i, "💻"],
  [/médec|santé|bio|médical|doctor|health|medical/i, "⚕️"],
  [/jurist|droit|loi|légal|lawyer|law|legal/i, "⚖️"],
  [/psycho|ment|esprit|conscience|psycholog|mind|conscious/i, "🧠"],
  [/écolo|environn|nature|climat|ecolog|environment|climate/i, "🌍"],
  [/musiq|cinéma|littér|poète|music|cinema|liter|poet|théâtr|theat/i, "🎭"],
  [/sport|athlèt|compétit|perform|athlet|competit/i, "🏆"],
  [/stratèg|militair|défense|strateg|military|defense/i, "♟️"],
  [/communi|social|média|journali|communic|media/i, "📢"],
  [/éduca|enseign|pédago|apprent|educ|teach|pedagog/i, "📚"],
  [/humor|comiq|drôle|satir|humor|comic|funny|satir/i, "🃏"],
  [/diplomat|négocia|mediati|consensus|diplomat|negotiat/i, "🤝"],
  [/visionnair|futur|prospect|anticip|visionary|future/i, "🔮"],
  [/féminis|patriar|genre|intersect|sexism|feminist|gender/i, "♀️"],
  [/masculinis|droits des hommes|men.s rights/i, "♂️"],
  [/comploti|conspira|officiel|ficelles|illumina|conspiracy/i, "🔺"],
  [/bourré|pilier|comptoir|alcool|bar|drunk|ivre/i, "🍺"],
  [/politici|langue de bois|élu|député|politic|elected/i, "🏛️"],
  [/extra.terrestre|alien|planète|ovni|ufo|martien/i, "👽"],
  [/\bchien\b|canin|loyal|aboie|dog\b|bark|woof/i, "🐕"],
  [/\bchat\b|félin|miaou|hautain|cat\b|meow|feline/i, "🐈"],
  [/pessimis|défaitis|sombre|doom|pessimist|gloomy/i, "😩"],
  [/\bdieu\b|éternel|tout.puissant|omniscient|god\b|almighty/i, "✝️"],
  [/\bsatan\b|diaboliq|ténèbres|tentateur|lucifer|devil|demon/i, "😈"],
];

const FALLBACK_EMOJIS = ["💬", "🗣️", "💡", "🎯", "⭐", "🌟", "📌", "🔥"];

/** Determine a representative emoji for a profile based on name and system prompt. */
export function getProfileEmoji(name: string, systemPrompt: string): string {
  // 1. Exact match on predefined profile names
  if (PROFILE_EMOJIS[name]) return PROFILE_EMOJIS[name];

  // 2. Keyword matching on name + system prompt
  const text = `${name} ${systemPrompt}`;
  for (const [regex, emoji] of KEYWORD_EMOJIS) {
    if (regex.test(text)) return emoji;
  }

  // 3. Deterministic fallback based on name hash
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  return FALLBACK_EMOJIS[Math.abs(hash) % FALLBACK_EMOJIS.length];
}

/** Curated emoji palette for manual selection */
export const EMOJI_PALETTE = [
  // Personalities & roles
  "🔬", "🤔", "😈", "🎨", "🔧", "☀️", "🧐",
  // New personalities
  "💻", "♀️", "♂️", "🔺", "🃏", "🍺", "🏛️",
  "👽", "🐕", "🐈", "😩", "✝️",
  // Domains
  "📜", "📊", "⚕️", "⚖️", "🧠", "🌍",
  "🎭", "🏆", "♟️", "📢", "📚", "🤝", "🔮",
  // Generic & fun
  "💬", "🗣️", "💡", "🎯", "⭐", "🌟", "🔥",
  "🦊", "🐺", "🦁", "🐉", "🦅", "🐙", "🤖",
];

/** Special role emojis */
export const ROLE_EMOJIS = {
  IArbitre: "⚖️",
  user: "👤",
} as const;
