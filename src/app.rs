use crate::articles::{articles, featured, latest_date, start_live_updates, Article};
use leptos::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement};

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

/// "2026-11-12" -> "12 November"
fn nice_date(iso: &str) -> String {
    let (y, rest) = iso.split_once('-').unwrap_or((iso, ""));
    let (m, d) = rest.split_once('-').unwrap_or((rest, ""));
    let m: usize = m.parse().unwrap_or(1);
    format!("{} {} {}", d, MONTHS[(m - 1).min(11)], y)
}

/// The ink trail: as the hand crosses the pour it leaves a grey-black line
/// that fades like a mark drying into the concrete, throwing off small ink
/// flecks as it goes. Still under reduced motion, and for touch.
fn mount_ink_trail(section: HtmlElement, canvas: HtmlCanvasElement) {
    const TRAIL_LIFE: f64 = 700.0; // ms a trail point stays on the sheet
    const FLECK_LIFE: f64 = 1100.0; // ms a fleck stays in the air

    // the sheet stays still for reduced motion
    let reduced = window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .map(|m| m.matches())
        .unwrap_or(false);
    if reduced {
        return;
    }

    // take the ink from the tokens, so the trail is the page's own grey-black
    let ink = window()
        .get_computed_style(&section)
        .ok()
        .flatten()
        .and_then(|s| s.get_property_value("--color-ink").ok())
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| "oklch(18% 0.005 270)".to_string());

    let Ok(Some(ctx)) = canvas
        .get_context("2d")
        .map(|c| c.and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok()))
    else {
        return;
    };

    let trail: Rc<RefCell<Vec<[f64; 3]>>> = Rc::new(RefCell::new(Vec::new()));
    let flecks: Rc<RefCell<Vec<[f64; 6]>>> = Rc::new(RefCell::new(Vec::new()));
    let running: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let frame: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));

    // fit the sheet to the section, now and on every resize
    let fit = {
        let canvas = canvas.clone();
        let section = section.clone();
        move || {
            let dpr = window().device_pixel_ratio();
            let rect = section.get_bounding_client_rect();
            canvas.set_width((rect.width() * dpr).round() as u32);
            canvas.set_height((rect.height() * dpr).round() as u32);
        }
    };
    fit();
    let on_resize = {
        let fit = fit.clone();
        Closure::<dyn FnMut(web_sys::Event)>::new(move |_| fit())
    };
    let _ = window().add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());

    // the frame: clear, draw the line through the last points, step the flecks
    let draw = {
        let trail = trail.clone();
        let flecks = flecks.clone();
        let running = running.clone();
        let frame = frame.clone();
        let ctx = ctx.clone();
        let section = section.clone();
        let ink = ink.clone();
        move || {
            let now = js_sys::Date::now();
            trail.borrow_mut().retain(|p| now - p[2] < TRAIL_LIFE);
            let dpr = window().device_pixel_ratio();
            let rect = section.get_bounding_client_rect();
            let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
            ctx.clear_rect(0.0, 0.0, rect.width(), rect.height());

            // the trail: a grey-black line through the last points, fading
            ctx.set_line_width(2.0);
            ctx.set_line_cap("round");
            ctx.set_line_join("round");
            ctx.set_stroke_style_str(&ink);
            for pair in trail.borrow().windows(2) {
                let age = (now - pair[1][2]) / TRAIL_LIFE;
                let a = (1.0 - age).clamp(0.0, 1.0) * 0.7;
                if a <= 0.02 {
                    continue;
                }
                ctx.set_global_alpha(a);
                let _ = ctx.begin_path();
                let _ = ctx.move_to(pair[0][0], pair[0][1]);
                let _ = ctx.line_to(pair[1][0], pair[1][1]);
                let _ = ctx.stroke();
            }

            // the flecks: small ink chips thrown off the line, fading as they fall
            flecks.borrow_mut().retain(|f| now - f[5] < FLECK_LIFE);
            for f in flecks.borrow_mut().iter_mut() {
                let age = ((now - f[5]) / FLECK_LIFE).clamp(0.0, 1.0);
                f[3] += 0.055; // gravity takes each chip down
                f[2] *= 0.985; // and drag slows it
                f[3] *= 0.985;
                f[0] += f[2];
                f[1] += f[3];
                f[4] += 0.06; // the chip's spin
                let s = 3.2 * (1.0 - age * 0.5);
                ctx.set_global_alpha((1.0 - age) * 0.85);
                ctx.set_fill_style_str(&ink);
                ctx.save();
                let _ = ctx.translate(f[0], f[1]);
                let _ = ctx.rotate(f[4]);
                let _ = ctx.fill_rect(-s / 2.0, -s / 2.0, s, s * 0.7); // a chip, not a square
                ctx.restore();
            }
            ctx.set_global_alpha(1.0);

            let quiet = trail.borrow().is_empty() && flecks.borrow().is_empty();
            if quiet {
                running.set(false); // the sheet rests until the hand returns
            } else if let Some(next) = frame.borrow().as_ref() {
                let _ = window().request_animation_frame(next.as_ref().unchecked_ref());
            }
        }
    };
    *frame.borrow_mut() = Some(Closure::wrap(Box::new(draw) as Box<dyn FnMut()>));

    let wake = {
        let running = running.clone();
        let frame = frame.clone();
        move || {
            if !running.get() {
                running.set(true);
                if let Some(next) = frame.borrow().as_ref() {
                    let _ = window().request_animation_frame(next.as_ref().unchecked_ref());
                }
            }
        }
    };

    // the hand: lay the trail where it passes, throw flecks off it
    let on_move = {
        let trail = trail.clone();
        let flecks = flecks.clone();
        let section = section.clone();
        let wake = wake.clone();
        Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
            if ev.pointer_type() != "mouse" {
                return;
            }
            let rect = section.get_bounding_client_rect();
            let x = f64::from(ev.client_x()) - rect.left();
            let y = f64::from(ev.client_y()) - rect.top();
            if x < 0.0 || y < 0.0 || x > rect.width() || y > rect.height() {
                return;
            }
            let mut tr = trail.borrow_mut();
            let moved = tr
                .last()
                .map(|p| (p[0] - x).hypot(p[1] - y))
                .unwrap_or(f64::INFINITY);
            if moved > 7.0 {
                tr.push([x, y, js_sys::Date::now()]);
                if tr.len() > 24 {
                    tr.remove(0);
                }
                // the flecks: one or two chips a visit, more when the hand is quick
                let mut fl = flecks.borrow_mut();
                let n = if moved > 30.0 { 2 } else { 1 };
                for _ in 0..n {
                    let ang = js_sys::Math::random() * std::f64::consts::TAU;
                    let v = 0.6 + js_sys::Math::random() * 1.8;
                    fl.push([
                        x,
                        y,
                        ang.cos() * v,
                        ang.sin() * v - 0.6,
                        js_sys::Math::random() * std::f64::consts::TAU,
                        js_sys::Date::now(),
                    ]);
                }
                let overflow = fl.len().saturating_sub(90);
                if overflow > 0 {
                    fl.drain(0..overflow);
                }
                wake();
            }
        })
    };
    let _ = section.add_event_listener_with_callback(
        "pointermove",
        on_move.as_ref().unchecked_ref::<js_sys::Function>(),
    );

    // the handlers live as long as the page
    std::mem::forget(on_move);
    std::mem::forget(on_resize);
}

#[component]
pub fn App() -> impl IntoView {
    // the live sheet starts under the mount's owner, then fetches hourly
    start_live_updates();
    let week_iso = Signal::derive(move || latest_date().unwrap_or_default());
    let week_nice = Signal::derive(move || {
        latest_date()
            .map(|d| format!("Week of {}", nice_date(&d)))
            .unwrap_or_else(|| "The current week's sheet".to_string())
    });
    view! {
        <header class="slab">
            <div class="container slab__row">
                <a class="slab__wordmark" href="/">Proof Sheet</a>
                <p class="slab__date"><time datetime=week_iso>{move || week_nice.get()}</time></p>
            </div>
        </header>
        <main id="main">
            <Hero/>
            <Index/>
            <DatesBand/>
            <About/>
        </main>
        <Footer/>
    }
}

#[component]
fn Hero() -> impl IntoView {
    let hero_ref = NodeRef::<leptos::html::Section>::new();
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let wired = Rc::new(Cell::new(false));

    // after the pour is mounted, wire the trail onto it
    Effect::new({
        let hero_ref = hero_ref.clone();
        let canvas_ref = canvas_ref.clone();
        let wired = wired.clone();
        move |_| {
            let Some(section) = hero_ref.get() else { return };
            let Some(canvas) = canvas_ref.get() else { return };
            if wired.get() {
                return;
            }
            wired.set(true);
            mount_ink_trail(section, canvas);
        }
    });

    let top = Memo::new(move |_| featured());
    view! {
        <section class="hero" node_ref=hero_ref>
            <div class="hero__photo" aria-hidden="true">
                <div class="hero__photo-grey"></div>
                <canvas class="hero__fx" node_ref=canvas_ref></canvas>
            </div>
            <div class="container hero__grid">
                <div class="hero__text">
                    <h1>"Five stories, set and checked."</h1>
                    <p class="lead">
                        "The front page of Hacker News, proofed and linked at source. Nothing invented, nothing inflated."
                    </p>
                    <div class="hero__actions">
                        {move || {
                            top.get().map(|a| {
                                view! {
                                    <a class="btn btn--primary" href=a.url.clone() target="_blank" rel="noopener">
                                        "Read the top story"
                                    </a>
                                }
                            })
                        }}
                        <a class="btn btn--quiet" href="#index">"Open the sheet"</a>
                    </div>
                </div>
                {move || top.get().map(|a| view! { <FeaturedSheet article=a /> })}
            </div>
        </section>
    }
}

#[component]
fn FeaturedSheet(article: Article) -> impl IntoView {
    // the masthead steps down as the headline runs long, so the plate holds one screen
    let count = article.title.chars().count();
    let tier = if count <= 18 {
        "mast__title"
    } else if count <= 42 {
        "mast__title mast__title--mid"
    } else {
        "mast__title mast__title--long"
    };
    view! {
        <figure class="sheet" aria-label="The featured story, set in Oswald over Archivo">
            <figcaption class="sheet__label">
                <span>"Featured"</span>
                <span>{article.source.clone()}</span>
            </figcaption>
            <div class="mast">
                <span class="guide guide--cap guide--anim" aria-hidden="true"></span>
                <span class="readout readout--cap">"cap height 700"</span>
                <h2 class=tier>{article.title.clone()}</h2>
                <span class="guide guide--base guide--anim" aria-hidden="true"></span>
                <span class="readout readout--base">"baseline 0"</span>
            </div>
            <p class="sheet__meta">
                <time datetime=article.date.clone()>{nice_date(&article.date)}</time>
                <span class="sheet__tag">{article.tag.clone()}</span>
            </p>
        </figure>
    }
}

#[component]
fn Index() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let tag_filter: RwSignal<Option<String>> = RwSignal::new(None);
    let open_id: RwSignal<Option<String>> = RwSignal::new(None);

    let filtered = Memo::new(move |_| {
        let q = search.get().to_lowercase();
        let tag = tag_filter.get();
        let mut items = articles().get();
        items.sort_by(|a, b| b.date.cmp(&a.date));
        items
            .iter()
            .filter(|a| {
                tag.as_deref()
                    .is_none_or(|t| a.tag == t)
                    && (q.is_empty()
                        || a.title.to_lowercase().contains(&q)
                        || a.source.to_lowercase().contains(&q))
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    // the chips read the sheet: whatever tags the live data carries
    let tags = Memo::new(move |_| {
        let mut tags: Vec<String> = articles().get().iter().map(|a| a.tag.clone()).collect();
        tags.sort();
        tags.dedup();
        tags
    });

    view! {
        <section class="sheet-index" id="index">
            <div class="container">
                <div class="sheet-index__head">
                    <h2>"This week's sheet"</h2>
                    <p class="lead">"Straight off the front page, refreshed every hour. Search, filter, open a row, read at source."</p>
                </div>
                <div class="sheet-index__controls">
                    <input
                        class="field"
                        type="search"
                        placeholder="Search titles and sources"
                        aria-label="Search the sheet"
                        bind:value=search
                    />
                    <div class="chips" role="group" aria-label="Filter by tag">
                        <TagChip label="All".to_string() tag=None filter=tag_filter/>
                        <For each=move || tags.get() key=|t| t.clone() let:tag>
                            <TagChip label=tag.clone() tag=Some(tag.clone()) filter=tag_filter/>
                        </For>
                    </div>
                </div>
                <ul class="set">
                    <For
                        each=move || filtered.get()
                        key=|a| a.id.clone()
                        let:article
                    >
                        <Row article open_id/>
                    </For>
                </ul>
                <Show
                    when=move || !articles().get().is_empty() && filtered.get().is_empty()
                    fallback=|| ()
                >
                    <div class="empty">
                        <p>"Nothing on the sheet matches. Clear the search or pick another tag."</p>
                        <button
                            type="button"
                            class="btn btn--quiet"
                            on:click=move |_| {
                                search.set(String::new());
                                tag_filter.set(None);
                            }
                        >
                            "Clear the filters"
                        </button>
                    </div>
                </Show>
            </div>
        </section>
    }
}

#[component]
fn TagChip(label: String, tag: Option<String>, filter: RwSignal<Option<String>>) -> impl IntoView {
    let tag_active = tag.clone();
    let tag_pressed = tag.clone();
    let is_active = move || filter.get().as_deref() == tag_active.as_deref();
    let pressed = move || filter.get().as_deref() == tag_pressed.as_deref();
    let set = move |_| {
        let t = tag.clone();
        filter.set(if filter.get().as_deref() == t.as_deref() { None } else { t });
    };
    view! {
        <button type="button" class="chip" class:is-active=is_active on:click=set aria-pressed=pressed>
            {label}
        </button>
    }
}

#[component]
fn Row(article: Article, open_id: RwSignal<Option<String>>) -> impl IntoView {
    let id_open = article.id.clone();
    let id_toggle = article.id.clone();
    let open = Memo::new(move |_| open_id.get().as_deref() == Some(id_open.as_str()));
    let toggle = move |_| {
        if open.get() {
            open_id.set(None);
        } else {
            open_id.set(Some(id_toggle.clone()));
        }
    };
    view! {
        <li class="row" class:is-open=open>
            <h3 class="row__title">
                <button
                    type="button"
                    class="row__head"
                    on:click=toggle
                    aria-expanded=open
                >
                    <span class="row__name">{article.title.clone()}</span>
                    <span class="row__meta">
                        <time datetime=article.date.clone()>{nice_date(&article.date)}</time>
                        <span>{article.source.clone()}</span>
                    </span>
                </button>
            </h3>
            <Show when=move || open.get()>
                <div class="row__panel">
                    <p>{article.summary.clone()}</p>
                    <div class="row__links">
                        <a class="row__link" href=article.url.clone() target="_blank" rel="noopener">
                            "Read at source"
                        </a>
                        {article
                            .discussion
                            .clone()
                            .map(|d| {
                                view! {
                                    <a class="row__link" href=d target="_blank" rel="noopener">
                                        "The discussion"
                                    </a>
                                }
                            })}
                    </div>
                </div>
            </Show>
        </li>
    }
}

#[component]
fn DatesBand() -> impl IntoView {
    let stats = Memo::new(move |_| {
        let mut items = articles().get();
        items.sort_by(|a, b| b.date.cmp(&a.date));
        let outlets = items
            .iter()
            .map(|a| a.source.clone())
            .collect::<std::collections::BTreeSet<String>>()
            .len();
        let days = items
            .iter()
            .map(|a| a.date.clone())
            .collect::<std::collections::BTreeSet<String>>()
            .len();
        (items, outlets, days)
    });
    move || {
        let (items, outlets, days) = stats.get();
        let stories = items.len();
        view! {
            <section class="dates-band">
                <div class="container dates-band__inner">
                    <p class="run">
                        <span>{stories}</span>" stories, "<span>{outlets}</span>" outlets, "<span>{days}</span>" posting days"
                    </p>
                    <ul class="rail" tabindex="0" aria-label="The outlets of the week, by date posted">
                        {items
                            .iter()
                            .map(|a| {
                                view! {
                                    <li class="rail__item">
                                        <span class="rail__outlet">{a.source.clone()}</span>
                                        <time datetime=a.date.clone()>{nice_date(&a.date)}</time>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                </div>
            </section>
        }
    }
}

#[component]
fn About() -> impl IntoView {
    view! {
        <section class="about" id="about">
            <div class="container about__grid">
                <div class="about__text">
                    <h2>"Set in Rust"</h2>
                    <p class="lead">
                        "This page is a Leptos app compiled to WebAssembly. The sheet is fetched live from the Hacker News front page and refreshed every hour; the served JSON is its fallback."
                    </p>
                </div>
                <dl class="about__spec">
                    <div class="about__row">
                        <dt>"Runtime"</dt>
                        <dd>"Leptos, compiled to WebAssembly"</dd>
                    </div>
                    <div class="about__row">
                        <dt>"Data"</dt>
                        <dd>"Hacker News front page, the top five"</dd>
                    </div>
                    <div class="about__row">
                        <dt>"Network"</dt>
                        <dd>"one fetch an hour, no key"</dd>
                    </div>
                </dl>
            </div>
        </section>
    }
}

#[component]
fn Footer() -> impl IntoView {
    let signoff = Signal::derive(move || {
        latest_date()
            .map(|d| {
                format!(
                    "Set on {}. {} stories, every link real.",
                    nice_date(&d),
                    articles().get().len()
                )
            })
            .unwrap_or_else(|| "Set fresh from the front page. Every link real.".to_string())
    });
    view! {
        <footer class="close">
            <div class="container close__grid">
                <p class="close__wordmark">"Proof Sheet"</p>
                <div class="close__aside">
                    <p class="close__signoff">{move || signoff.get()}</p>
                    <nav class="close__links" aria-label="Footer">
                        <a href="#index">"The sheet"</a>
                        <a href="#about">"About"</a>
                        <a href="https://github.com/itsVrushal/Proof-Sheet" target="_blank" rel="noopener">"GitHub"</a>
                        <a href="#main">"Back to top"</a>
                    </nav>
                </div>
            </div>
            <div class="container">
                <p class="close__credits" data-credits>
                    "Crafted in Rust & WASM by "
                    <a href="https://github.com/itsVrushal" target="_blank" rel="noopener">
                        "Vrushal Patil"
                    </a>
                    " · "
                    <a href="https://github.com/itsVrushal/Proof-Sheet" target="_blank" rel="noopener">
                        "GitHub"
                    </a>
                </p>
                <p class="close__credits" style="margin-top: var(--space-xs);" data-credits>
                    "Photographs: "
                    <a href="https://commons.wikimedia.org/wiki/File:P%C3%B6rtschach_10.-Oktober-Stra%C3%9Fe_27_Telekommunikationsgeb%C3%A4ude_Sichtbeton_30012022_2068.jpg" target="_blank" rel="noopener">
                        "Johann Jaritz"
                    </a>
                    " (CC BY-SA 4.0), via Wikimedia Commons"
                </p>
            </div>
        </footer>
    }
}
