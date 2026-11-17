use gloo_net::http::Request;
use leptos::prelude::*;
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Article {
    pub id: String,
    pub title: String,
    pub source: String,
    pub url: String,
    pub date: String,
    pub tag: String,
    pub summary: String,
    /// the HN thread, when the story came off the front page
    #[serde(default)]
    pub discussion: Option<String>,
}

#[derive(Deserialize)]
struct Sheet {
    articles: Vec<Article>,
}

const SHEET: &str = include_str!("../data/articles.json");
const HOUR: u64 = 60 * 60;

/// The sheet, as a signal: live data lands here and every view tracks it.
/// It opens on what was baked in at build, so the page is never blank.
static LIVE: OnceLock<RwSignal<Vec<Article>>> = OnceLock::new();

pub fn articles() -> RwSignal<Vec<Article>> {
    *LIVE.get_or_init(|| {
        RwSignal::new(
            serde_json::from_str::<Sheet>(SHEET)
                .map(|s| s.articles)
                .expect("data/articles.json parses"),
        )
    })
}

/// HN rank #1, the story the hero is cast around.
pub fn featured() -> Option<Article> {
    articles().get().into_iter().next()
}

/// The newest date on the sheet, the week the masthead prints.
pub fn latest_date() -> Option<String> {
    articles().get().iter().map(|a| a.date.clone()).max()
}

/// Open the sheet live: paint once from what was baked in, fetch now, then
/// every hour, so the page keeps up with the front page while it's open.
pub fn start_live_updates() {
    if let Some(first) = featured() {
        apply_palette(&first.tag);
    }
    leptos::task::spawn_local(async move {
        loop {
            load().await;
            gloo_timers::future::sleep(Duration::from_secs(HOUR)).await;
        }
    });
}

/// Hacker News first, then the served sheet, then what was baked in.
/// The page is never blank and the palette rides the top story.
async fn load() {
    let sheet = match hn_front_page().await {
        Some(live) if !live.is_empty() => live,
        _ => match fetch_local().await {
            Ok(local) if !local.is_empty() => local,
            _ => articles().get_untracked(),
        },
    };
    if sheet.is_empty() {
        return;
    }
    // the top story dyes the page
    let tag = sheet[0].tag.clone();
    articles().set(sheet);
    apply_palette(&tag);
}

/// The Hacker News front page, the top five, mapped into the sheet's format.
async fn hn_front_page() -> Option<Vec<Article>> {
    let ids: Vec<u32> = Request::get("https://hacker-news.firebaseio.com/v0/topstories.json")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let mut out = Vec::new();
    for id in ids.into_iter().take(5) {
        let item = match Request::get(&format!(
            "https://hacker-news.firebaseio.com/v0/item/{id}.json"
        ))
        .send()
        .await
        {
            Ok(r) => r,
            Err(_) => continue, // a dead item costs nothing, the sheet stands
        };
        let item: HnItem = match item.json::<HnItem>().await {
            Ok(i) => i,
            Err(_) => continue,
        };
        if item.title.trim().is_empty() {
            continue;
        }
        let url = item
            .url
            .unwrap_or_else(|| format!("https://news.ycombinator.com/item?id={id}"));
        let tag = infer_tag(&item.title, &url);
        out.push(Article {
            id: format!("hn-{id}"),
            title: item.title,
            source: source_of(&url),
            url,
            date: iso_date(item.time),
            tag,
            summary: format!(
                "{} points, {} comments on Hacker News.",
                item.score, item.descendants
            ),
            discussion: Some(format!("https://news.ycombinator.com/item?id={id}")),
        });
    }
    (!out.is_empty()).then_some(out)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct HnItem {
    title: String,
    url: Option<String>,
    score: i64,
    descendants: i64,
    time: i64,
}

/// The served sheet, the same JSON you curate, fetched same-origin.
async fn fetch_local() -> Result<Vec<Article>, gloo_net::Error> {
    Request::get("data/articles.json")
        .send()
        .await?
        .json::<Sheet>()
        .await
        .map(|s| s.articles)
}

/// The host of the link, the story's outlet: "blog.rust-lang.org", or HN itself.
fn source_of(url: &str) -> String {
    let bare = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let host = bare
        .split('/')
        .next()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_string();
    if host.is_empty() {
        "news.ycombinator.com".to_string()
    } else {
        host
    }
}

/// A tag for the chip row and the palette, read off the title and the link.
/// Whole words only, so "email" is not AI and "rustproof" is not Rust.
fn infer_tag(title: &str, url: &str) -> String {
    let hay = format!("{title} {url}").to_lowercase();
    let has = |w: &str| hay.split(|c: char| !c.is_ascii_alphanumeric()).any(|t| t == w);
    if has("rust") || has("cargo") {
        return "rust".into();
    }
    if has("python") || has("pypi") || has("django") {
        return "python".into();
    }
    if has("cve") || has("security") || has("exploit") || has("vulnerability") || has("malware")
        || has("breach") || has("ransomware") || has("hacking")
    {
        return "security".into();
    }
    if has("ai") || has("llm") || has("gpt") || has("openai") || has("anthropic") || has("claude")
        || has("gemini") || has("neural") || has("agent") || has("agents")
    {
        return "ai".into();
    }
    "tech".into()
}

/// Days since 1970 -> "YYYY-MM-DD", Hinnant's civil_from_days, no chrono.
fn iso_date(unix_secs: i64) -> String {
    let z = unix_secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// The accent per tag, so the page is dyed by the top story: rust orange,
/// python blue, ai violet, security red, the formwork orange otherwise.
fn palette(tag: &str) -> (&'static str, &'static str, &'static str) {
    match tag {
        "python" => ("oklch(58% 0.16 252)", "oklch(42% 0.15 252)", "oklch(70% 0.14 252)"),
        "ai" => ("oklch(58% 0.20 300)", "oklch(43% 0.18 300)", "oklch(68% 0.16 300)"),
        "security" => ("oklch(55% 0.20 25)", "oklch(41% 0.18 25)", "oklch(65% 0.17 25)"),
        // rust, tech, anything else: the formwork orange the tokens ship with
        _ => ("oklch(66% 0.21 38)", "oklch(48% 0.17 38)", "oklch(42% 0.15 38)"),
    }
}

/// Set the dye on the root element, one accent at a time.
fn apply_palette(tag: &str) {
    let (accent, deep, focus) = palette(tag);
    if let Some(el) = window().document().and_then(|d| d.document_element()) {
        if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
            let style = el.style();
            let _ = style.set_property("--color-accent", accent);
            let _ = style.set_property("--color-accent-deep", deep);
            let _ = style.set_property("--color-focus", focus);
        }
    }
}
