// Refreshes the fallback sheet from the Hacker News front page, so
// data/articles.json holds the current copy, not a stale one. The mapping is
// the same one the app applies to its live fetch (source = the link's host,
// date from the item's time, tag read off the title and link, summary =
// points and comments, plus the discussion thread).
//
//   node data/fetch-hn.mjs
//
// If the fetch comes up short, the existing file is left untouched.
import { readFile, writeFile } from "node:fs/promises";

const API = "https://hacker-news.firebaseio.com/v0";

const json = async (url) => {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`${r.status} ${url}`);
  return r.json();
};

// whole words only, so "email" is not AI and "rustproof" is not Rust —
// the same word list and order articles.rs reads
const inferTag = (title, url) => {
  const hay = `${title} ${url}`.toLowerCase();
  const has = (w) => hay.split(/[^a-z0-9]+/).includes(w);
  if (has("rust") || has("cargo")) return "rust";
  if (has("python") || has("pypi") || has("django")) return "python";
  if (
    ["cve", "security", "exploit", "vulnerability", "malware", "breach", "ransomware", "hacking"].some(has)
  ) {
    return "security";
  }
  if (
    ["ai", "llm", "gpt", "openai", "anthropic", "claude", "gemini", "neural", "agent", "agents"].some(has)
  ) {
    return "ai";
  }
  return "tech";
};

const sourceOf = (url) => {
  const bare = url.includes("://") ? url.slice(url.indexOf("://") + 3) : url;
  const host = bare.split("/")[0].replace(/^www\./, "");
  return host || "news.ycombinator.com";
};

const iso = (secs) => new Date((secs ?? 0) * 1000).toISOString().slice(0, 10);

const ids = await json(`${API}/topstories.json`);
const items = [];
for (const id of (ids ?? []).slice(0, 5)) {
  try {
    const it = await json(`${API}/item/${id}.json`);
    if (!it || !it.title?.trim()) continue;
    const url = it.url ?? `https://news.ycombinator.com/item?id=${id}`;
    items.push({
      id: `hn-${id}`,
      title: it.title,
      source: sourceOf(url),
      url,
      date: iso(it.time),
      tag: inferTag(it.title, url),
      summary: `${it.score ?? 0} points, ${it.descendants ?? 0} comments on Hacker News.`,
      discussion: `https://news.ycombinator.com/item?id=${id}`,
    });
  } catch {
    // a dead item costs nothing, the sheet stands
  }
}

if (items.length < 3) {
  console.error("HN fetch came up short; data/articles.json left as it was");
  process.exit(0);
}

// write only when the sheet actually changed, so a trunk pre-build hook does
// not touch the file on every build and feed its own watcher a loop
const file = new URL("./articles.json", import.meta.url);
const next = JSON.stringify({ articles: items }, null, 2) + "\n";
const current = await readFile(file, "utf8").catch(() => null);
if (current === next) {
  console.log("data/articles.json already current; left untouched");
  process.exit(0);
}

await writeFile(file, next);
console.log(`data/articles.json refreshed: ${items.length} stories off the front page`);
