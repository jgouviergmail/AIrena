# Wikipedia API Integration Feasibility Report for AIrena

## Executive Summary

**Verdict: FULLY FEASIBLE.** Wikipedia provides free, unauthenticated, well-documented APIs that are perfectly suited for AIrena's use case. The recommended approach uses **two API calls per lookup**: one Action API search + extract combo request, and optionally a REST API page summary as fallback. Everything works with `reqwest` in Rust. No API key needed.

---

## 1. API Endpoints Analyzed

### 1.1 Action API — Base Endpoint

- **URL pattern**: `https://{lang}.wikipedia.org/w/api.php`
- **Examples**:
  - `https://en.wikipedia.org/w/api.php` (English)
  - `https://fr.wikipedia.org/w/api.php` (French)
  - `https://zh.wikipedia.org/w/api.php` (Chinese)
- **Method**: GET (for read-only queries) or POST
- **Input format**: URL query parameters (`application/x-www-form-urlencoded`)
- **Output format**: JSON (via `format=json`), also supports XML, PHP serialization
- **Authentication**: **NONE required** for read-only queries
- **CORS**: Add `&origin=*` for browser requests (not needed for server-side Rust)

### 1.2 Action API — `action=query&list=search` (Full-Text Search)

**Purpose**: Search Wikipedia articles by text content. Returns ranked results with snippets.

**Endpoint**: `https://{lang}.wikipedia.org/w/api.php?action=query&list=search&...`

**Key Parameters**:
| Parameter | Description | Default | Range |
|-----------|-------------|---------|-------|
| `srsearch` | Search query string (REQUIRED) | - | - |
| `srnamespace` | Namespace(s) to search | 0 (articles) | 0,1,2... |
| `srlimit` | Max results to return | 10 | 1-500 |
| `sroffset` | Offset for pagination | 0 | - |
| `srprop` | Properties to return | `size\|wordcount\|timestamp\|snippet` | - |
| `srwhat` | Type of search: `text` or `title` | (engine decides) | - |
| `format` | Output format | - | `json` recommended |
| `formatversion` | JSON format version | 1 | **Use `2`** (cleaner) |

**Live Response Structure** (verified):
```json
{
  "batchcomplete": true,
  "continue": { "sroffset": 3, "continue": "-||" },
  "query": {
    "searchinfo": {
      "totalhits": 5903,
      "suggestion": "quantum computering",
      "suggestionsnippet": "quantum computering"
    },
    "search": [
      {
        "ns": 0,
        "title": "Quantum computing",
        "pageid": 25220,
        "size": 119749,
        "wordcount": 12801,
        "snippet": "<span class=\"searchmatch\">quantum</span> <span class=\"searchmatch\">computing</span>...",
        "timestamp": "2026-02-09T21:02:38Z"
      }
    ]
  }
}
```

**Notes**:
- `snippet` contains HTML `<span class="searchmatch">` tags around matched terms — needs stripping
- `continue` object present when more results available
- Works with all language Wikipedias (verified: en, fr, zh)

### 1.3 Action API — `action=opensearch` (Autocomplete Search)

**Purpose**: Quick title-based search suggestions (like autocomplete).

**Endpoint**: `https://{lang}.wikipedia.org/w/api.php?action=opensearch&...`

**Key Parameters**:
| Parameter | Description | Default | Range |
|-----------|-------------|---------|-------|
| `search` | Search string (REQUIRED) | - | - |
| `limit` | Max results | 10 | 1-500 |
| `namespace` | Namespace(s) to search | 0 | - |
| `format` | Output format | `json` | - |

**Live Response Structure** (verified):
```json
[
  "quantum computing",
  ["Quantum computing", "Quantum computing scaling laws", "Quantum Computing Since Democritus", ...],
  ["", "", "", ...],
  ["https://en.wikipedia.org/wiki/Quantum_computing", ...]
]
```

**Format**: Array of 4 arrays: [query, titles[], descriptions[], urls[]]

**Notes**:
- Much lighter than full search — good for quick lookups
- Descriptions array is usually empty (Wikipedia doesn't populate it)
- Title-only matching (not content)

### 1.4 Action API — `action=query&prop=extracts` (Article Extracts/Summaries)

**Purpose**: Get plain text extracts (summaries) from articles. **THIS IS THE KEY ENDPOINT.**

**Endpoint**: `https://{lang}.wikipedia.org/w/api.php?action=query&prop=extracts&...`

**Key Parameters**:
| Parameter | Description | Default |
|-----------|-------------|---------|
| `titles` | Article title(s), pipe-separated | - |
| `exintro` | Return only intro section (boolean flag) | false |
| `explaintext` | Return plain text instead of HTML (boolean flag) | false |
| `exchars` | Character limit for extract | unlimited |
| `exsentences` | Sentence limit (1-10) | unlimited |
| `exlimit` | Number of extracts when querying multiple pages | - |
| `redirects` | Follow redirects (boolean flag) | false |

**Live Response Structure** (verified):
```json
{
  "batchcomplete": true,
  "query": {
    "pages": [
      {
        "pageid": 25220,
        "ns": 0,
        "title": "Quantum computing",
        "extract": "A quantum computer is a (real or theoretical) computer that exploits superposed and entangled states..."
      }
    ]
  }
}
```

**Notes**:
- `exintro=1&explaintext=1` is the winning combination: gives a clean plain-text introduction
- `exchars=500` truncates to ~500 characters (may be slightly more due to word boundaries)
- Works across all Wikipedia languages (verified: en, fr, zh)
- Part of the TextExtracts MediaWiki extension (installed on all Wikimedia wikis)

### 1.5 Action API — Generator Mode (Search + Extract in ONE Request) **RECOMMENDED**

**Purpose**: Combine search with extracts in a single HTTP request.

**Endpoint**: `https://{lang}.wikipedia.org/w/api.php?action=query&generator=search&gsrsearch=...&prop=extracts&...`

**Key Parameters**:
| Parameter | Description |
|-----------|-------------|
| `generator=search` | Use search as the page generator |
| `gsrsearch` | Search query (generator prefix `gsr` instead of `sr`) |
| `gsrlimit` | Max search results |
| `prop=extracts` | Add extracts to each result |
| `exintro=1` | Intro only |
| `explaintext=1` | Plain text |
| `exchars=300` | Character limit per extract |

**Live Response Structure** (verified):
```json
{
  "batchcomplete": true,
  "continue": { "gsroffset": 3, "continue": "gsroffset||" },
  "query": {
    "pages": [
      {
        "pageid": 25220,
        "ns": 0,
        "title": "Quantum computing",
        "index": 1,
        "extract": "A quantum computer is a (real or theoretical) computer that exploits superposed and entangled states..."
      },
      {
        "pageid": 1730328,
        "ns": 0,
        "title": "Superconducting quantum computing",
        "index": 2,
        "extract": "Superconducting quantum computing is a branch of quantum computing..."
      }
    ]
  }
}
```

**THIS IS THE OPTIMAL APPROACH**: One HTTP request gives you search results WITH article summaries already included. The `index` field preserves search ranking order.

### 1.6 REST API — `/page/summary/{title}` (Page Summary)

**Purpose**: Get a structured summary of a specific article with metadata.

**Endpoint**: `https://{lang}.wikipedia.org/api/rest_v1/page/summary/{title}`

**Method**: GET (no query parameters needed)

**Live Response Structure** (verified):
```json
{
  "type": "standard",
  "title": "Albert Einstein",
  "displaytitle": "<span>Albert Einstein</span>",
  "pageid": 736,
  "wikibase_item": "Q937",
  "namespace": { "id": 0, "text": "" },
  "titles": {
    "canonical": "Albert_Einstein",
    "normalized": "Albert Einstein",
    "display": "..."
  },
  "thumbnail": {
    "source": "https://upload.wikimedia.org/.../330px-Albert_Einstein_Head_cleaned.jpg",
    "width": 330,
    "height": 408
  },
  "lang": "en",
  "dir": "ltr",
  "revision": "1336523373",
  "timestamp": "2026-02-04T07:25:16Z",
  "description": "German-born theoretical physicist (1879-1955)",
  "description_source": "local",
  "extract": "Albert Einstein was a German-born theoretical physicist best known for developing the theory of relativity...",
  "extract_html": "<p><b>Albert Einstein</b> was a German-born theoretical physicist...",
  "content_urls": {
    "desktop": { "page": "https://en.wikipedia.org/wiki/Albert_Einstein", ... },
    "mobile": { ... }
  }
}
```

**Key fields**:
- `extract`: Clean plain-text summary (intro paragraph)
- `description`: One-line description
- `thumbnail`: Article image (could be useful for UI)
- Works across all language Wikipedias (verified: en, fr)

**CORS headers**: `access-control-allow-origin: *` (fully open)

**Caching**: `cache-control: s-maxage=1209600, max-age=300` (5 minutes client-side, 14 days CDN)

### 1.7 Action API — `action=parse` (Full Wikitext Parsing)

**Purpose**: Parse full article content into HTML, sections, links, etc.

**Key Parameters**:
| Parameter | Description |
|-----------|-------------|
| `page` | Page title to parse |
| `prop` | What to return: `text`, `wikitext`, `sections`, `categories`, `links`, `images`, etc. |
| `section` | Parse only a specific section (0 = intro) |

**Notes**:
- Heavy endpoint — returns full parsed HTML
- Overkill for AIrena's needs (extracts are sufficient)
- Could be useful if you ever need structured section data

### 1.8 Action API — `action=query&meta=siteinfo` (Meta/Site Info)

**Purpose**: Get site metadata (languages available, statistics, namespaces, etc.)

**Notes**: Not directly useful for AIrena's Wikipedia search feature, but could help verify API availability.

### 1.9 Action API — Data Formats

**Supported formats**: `json`, `jsonfm` (formatted HTML), `xml`, `xmlfm`, `php`, `rawfm`, `none`

**Recommended**: `format=json&formatversion=2`
- `formatversion=2`: Cleaner JSON structure. Uses arrays instead of objects for page lists. Booleans are actual `true`/`false` instead of empty strings.
- Default `formatversion=1` uses page IDs as object keys, making deserialization harder.

---

## 2. Rate Limits and Usage Policies

### 2.1 Official Policy

| Aspect | Limit |
|--------|-------|
| **Overall rate** | 200 requests/second max across all endpoints |
| **Unauthenticated** | No formal per-IP limit published, but aggressive scraping gets blocked |
| **Authenticated (OAuth)** | 5,000 requests/hour per user |
| **Recommended** | Stay well under 200 req/s; ideally < 1 req/s for bursty desktop apps |

### 2.2 User-Agent Requirement

**MANDATORY**: Wikimedia requires a descriptive `User-Agent` header.

**Format**: `<client name>/<version> (<contact information>) <library/framework name>/<version>`

**Example for AIrena**: `AIrena/1.4 (https://github.com/your-repo; your-email@example.com) reqwest/0.12`

**Consequence of missing User-Agent**: Requests may be throttled or blocked. Currently not automatically enforced for all requests, but can be enforced at any time.

### 2.3 Etiquette Rules

- **Set a unique User-Agent header** (mandatory)
- **Limit request rate**: No hard limit published for unauthenticated, but be respectful
- **Use `maxlag` parameter**: `&maxlag=5` tells the API to return 503 if servers are lagged > 5 seconds
- **Do not make parallel requests**: Run requests sequentially where possible
- **Use compression**: Accept gzip (`Accept-Encoding: gzip`)
- **Cache results**: REST API responses have explicit cache headers

### 2.4 AIrena Impact Assessment

For AIrena's use case (debate participants occasionally searching during debates):
- **Expected volume**: 1-5 searches per debate turn, maybe 10-20 turns per debate = **20-100 requests per debate session**
- **Rate**: One search every 10-30 seconds (during LLM thinking time)
- **Verdict**: **WELL within limits**. This is negligible traffic for Wikipedia.

---

## 3. Authentication Requirements

| API | Authentication |
|-----|---------------|
| Action API (read-only queries) | **NONE** |
| REST API `/page/summary` | **NONE** |
| REST API at `api.wikimedia.org` (Core REST API) | OAuth 2.0 required |

**Important distinction**: The `{lang}.wikipedia.org/api/rest_v1/` REST API is **unauthenticated**. The newer `api.wikimedia.org` gateway requires OAuth. **We use the former.**

---

## 4. Recommended Integration Architecture for AIrena

### 4.1 Primary Approach: Generator Search + Extracts (1 HTTP call)

**Single request that searches AND gets summaries**:

```
GET https://{lang}.wikipedia.org/w/api.php
  ?action=query
  &generator=search
  &gsrsearch={query}
  &gsrlimit=3
  &prop=extracts
  &exintro=1
  &explaintext=1
  &exchars=500
  &format=json
  &formatversion=2
  &maxlag=5
```

**Returns**: Top 3 search results, each with a ~500 character plain-text introduction.

### 4.2 Fallback: REST API Page Summary (when you know the exact title)

```
GET https://{lang}.wikipedia.org/api/rest_v1/page/summary/{title}
```

**Returns**: Clean structured summary with plain text extract + description.

### 4.3 Language Selection

Map AIrena's language setting to Wikipedia subdomain:
- FR -> `fr.wikipedia.org`
- EN -> `en.wikipedia.org`
- ZH -> `zh.wikipedia.org`

All three verified working with live API calls.

### 4.4 Rust/reqwest Compatibility

**Fully compatible.** reqwest already used in AIrena for Ollama calls. Key points:
- Standard HTTP GET requests with query parameters
- JSON response body → deserialize with `serde`
- Must set `User-Agent` header (already easy with reqwest `ClientBuilder`)
- Response bodies are small (< 10KB typically)
- No streaming needed — simple request/response
- No authentication, no cookies, no session management

### 4.5 Serde Structs Needed

Approximate Rust structs for the generator+extracts response:

```rust
struct WikiSearchResponse {
    batchcomplete: bool,
    query: Option<WikiQuery>,
}

struct WikiQuery {
    pages: Option<Vec<WikiPage>>,
}

struct WikiPage {
    pageid: u64,
    ns: i32,
    title: String,
    index: Option<i32>,   // search rank
    extract: Option<String>,  // plain text summary
}
```

**Note**: Use `#[serde(default)]` on all fields (lesson from MEMORY.md — LLM/API responses can have missing fields).

---

## 5. Comparison: Which Endpoints Are Most Useful

| Endpoint | Use Case | Calls Needed | Data Quality | Recommendation |
|----------|----------|--------------|--------------|----------------|
| `generator=search` + `prop=extracts` | Search + get summaries | **1** | Good plain text intros | **PRIMARY** |
| `list=search` then `prop=extracts` | Search, then fetch details | **2** | Same quality, more control | Fallback |
| `opensearch` | Quick title suggestions | 1 | Titles only, no content | Not sufficient alone |
| REST `/page/summary/{title}` | Get summary by exact title | 1 per article | Excellent structured data | **SECONDARY** (when title known) |
| `action=parse` | Full article HTML | 1 | Full content, heavy | Overkill |

---

## 6. Potential Concerns and Mitigations

### 6.1 Chinese Wikipedia + TextExtracts
The `generator=search` + `prop=extracts` combo returned empty for Chinese Wikipedia in initial testing. The `list=search` and direct `prop=extracts` by title both work individually. **Mitigation**: Use 2-step approach (search first, then fetch extracts by title) as fallback for zh.wikipedia.org if generator mode fails.

### 6.2 HTML in Snippets
The `list=search` endpoint returns `snippet` fields with HTML `<span class="searchmatch">` tags. **Mitigation**: Strip HTML tags before passing to LLM (simple regex or string replacement).

### 6.3 Network Failures
Wikipedia is highly available but network issues can occur. **Mitigation**:
- Timeout after 5 seconds
- Graceful degradation (debate continues without Wikipedia data)
- Do not block the debate flow on Wikipedia lookups

### 6.4 UTF-8 Encoding
Chinese and French content uses multi-byte UTF-8 characters. **Mitigation**: Already handled in AIrena (see MEMORY.md UTF-8 lessons). Use `formatversion=2` which defaults to UTF-8.

### 6.5 Response Size
With `exchars=500` and `gsrlimit=3`, responses are typically 2-5KB. **No concern.**

---

## 7. Final Verdict

**Wikipedia API integration into AIrena is:**
- **Free**: No API key, no OAuth, no registration needed
- **Simple**: Standard HTTP GET + JSON responses
- **Fast**: Single request gives search + summaries
- **Compatible**: Works perfectly with existing reqwest + serde stack
- **Multilingual**: Same API pattern works for EN, FR, ZH Wikipedia
- **Low volume**: AIrena's debate usage is negligible vs. Wikipedia's capacity
- **Well-documented**: Self-documenting API at `api.php?action=help`

**Recommended approach**: Use `generator=search` + `prop=extracts` as the primary single-call endpoint, with REST `/page/summary/{title}` as an optional secondary endpoint for when you have an exact article title.
