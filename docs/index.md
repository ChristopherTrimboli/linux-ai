---
layout: home

hero:
  name: Linux AI Companion
  text: AI that lives on your Linux desktop
  tagline: A Linux-first AI chat companion with tool-based access to your computer — desktop app and CLI. Free and open source.
  image:
    src: /favicon.png
    alt: Linux AI Companion
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: Installation
      link: /guide/installation
    - theme: alt
      text: View on GitHub
      link: https://github.com/ChristopherTrimboli/linux-ai

features:
  - icon: 🐧
    title: Linux-first
    details: Built for the Linux desktop (Ubuntu/GNOME), not an afterthought port of a cross-platform app.
  - icon: 🛠️
    title: Acts on your computer
    details: Shell, files, search, and system info via built-in tools — each mutating action gated behind explicit approval.
  - icon: 🎙️
    title: Voice input
    details: Native microphone capture transcribed via OpenAI, OpenRouter, or a fully local whisper server.
  - icon: 🔌
    title: Bring your own model
    details: Anthropic, OpenAI, OpenRouter, or any OpenAI-compatible / local endpoint (Ollama, LM Studio, …).
  - icon: 💻
    title: Desktop + CLI
    details: A Tauri 2 / Svelte 5 desktop app and an `ai` command line, sharing one Rust core.
  - icon: 🔒
    title: Local & private
    details: Conversations in local SQLite, API keys in your OS keyring. Only the provider you pick is ever contacted.
---
