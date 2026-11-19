# Proof-Sheet 📰

> A client-side editorial newspaper for Hacker News built in **Rust** (Leptos 0.8) compiled to **WebAssembly** — zero JavaScript frameworks, zero backend server.

[![Live Demo](https://img.shields.io/badge/Demo-proof--sheet.vercel.app-orange?style=for-the-badge&logo=vercel)](https://proof--sheet.vercel.app/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-black?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Leptos](https://img.shields.io/badge/Leptos-0.8%20CSR-red?style=for-the-badge)](https://leptos.dev/)
[![WASM](https://img.shields.io/badge/WebAssembly-WASM-purple?style=for-the-badge&logo=webassembly)](https://webassembly.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)

---

## ⚡ Overview

**Proof-Sheet** reimagines Hacker News reading through the lens of a classic, physical print newspaper. Instead of a typical React or Next.js SPA, the entire application is written in **Rust** and compiled directly into a single client-side WebAssembly binary served as static assets on Vercel.

🔗 **Try the live deployment:** [proof--sheet.vercel.app](https://proof--sheet.vercel.app/)

---

## ✨ Key Features

- **100% Client-Side Rust (CSR)**: Built with Leptos 0.8 reactive primitives (`create_signal`, `create_resource`) compiling to native WebAssembly with no JavaScript framework runtime.
- **Offline-Resilient Top-5 Fetching**: Consumes the official Hacker News API via `gloo-net`. Includes an embedded static JSON fallback snapshot so the page renders immediately even on cold-starts or rate-limited networks, seamlessly hydrating when the live feed resolves.
- **Physical "Drying Ink" Canvas 2D Trail**: A high-DPR HTML5 Canvas 2D interactive cursor that renders ink splatters fading over time like real newspaper printing. Respects user accessibility by automatically disabling under `prefers-reduced-motion`.
- **Dynamic Story Palette & Tag Inference**: Automatically parses story titles to infer topics and dynamically derives an accent tint inspired by the lead front-page story.
- **Zero-Server Static Deployment**: Shipped as pure static assets compiled by Trunk, completely eliminating cloud server hosting costs.

---

## 🏛️ Architecture & Data Flow

```text
Hacker News API ─────────► [gloo-net HTTP] ──────────┐
                                                     ▼
Embedded Snapshot (JSON) ─► [Fallback Trigger] ──► Leptos Reactive Signals
                                                     │
                                                     ├─► DOM Canvas 2D (Ink Physics)
                                                     │
                                                     └─► Virtual Tree ──► WASM Mount
```

---

## 🛠️ Tech Stack

| Layer | Technology |
| :--- | :--- |
| **Language** | Rust 2021 Edition |
| **Frontend Framework** | [Leptos 0.8](https://leptos.dev/) (Client-Side Rendering) |
| **Binary Target** | `wasm32-unknown-unknown` |
| **Build & Bundler** | [Trunk](https://trunkrs.dev/) + `wasm-bindgen` |
| **Async Networking** | `gloo-net` + `wasm-bindgen-futures` |
| **Graphics & FX** | HTML5 Canvas 2D via `web-sys` |
| **Hosting** | Vercel (Static WASM Assets) |

---

## 🚀 Local Development

### Prerequisites

Ensure you have Rust and the WebAssembly compilation target installed:

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Add WASM target
rustup target add wasm32-unknown-unknown

# 3. Install Trunk bundler
cargo install trunk
```

### Running Locally

```bash
# Clone the repository
git clone https://github.com/itsVrushal/Proof-Sheet.git
cd Proof-Sheet

# Start local dev server with hot-reload
trunk serve --open
```

The app will be available at `http://127.0.0.1:8080/`.

### Production Build

```bash
trunk build --release
```
The optimized bundle will be generated inside the `/dist` directory, ready to be deployed to Vercel, Cloudflare Pages, or GitHub Pages.

---

## 💡 Engineering Highlights & Lessons Learned

- **Handling glibc / musl Mismatch**: Solved cross-compilation linking discrepancies in headless Linux CI environments by using standardized Trunk build containers, ensuring repeatable WASM artifact generation.
- **Canvas 2D Performance in WASM**: Interfacing with the DOM from WASM can incur bridge overhead. Batching Canvas draw operations inside `requestAnimationFrame` preserved smooth 60 FPS cursor trails without CPU spikes.

---

## 👤 Author

**Vrushal Patil**
- **Portfolio & Other Projects**: [vrushalpatil.vercel.app](https://vrushalpatil.vercel.app)
- **GitHub**: [@itsVrushal](https://github.com/itsVrushal)
- **LinkedIn**: [in/vrushal-patil](https://linkedin.com/in/vrushal-patil)

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
