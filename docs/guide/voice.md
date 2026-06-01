# Voice input (speech-to-text)

The desktop app has a mic button in the composer. Click it to record from your
default input device (it turns red while listening), click again to stop — the
audio is encoded to WAV locally and transcribed via an OpenAI-compatible
`/audio/transcriptions` endpoint, then dropped into the input box for you to edit
and send.

Capture is done natively in the Rust backend with [`cpal`](https://crates.io/crates/cpal)
(WebKitGTK, the Linux webview, doesn't support the browser Speech APIs), so it
needs the ALSA dev library at build time — see [Installation](./installation).

## Choosing a model

Configure the provider and model under **Settings → Voice input**. It defaults to
the `openai` provider with **`gpt-4o-transcribe`** (OpenAI's current best
transcription model — lower word-error-rate than `whisper-1` at the same price).

| Provider | Example models | Notes |
| --- | --- | --- |
| `openai` | `gpt-4o-transcribe`, `gpt-4o-mini-transcribe`, `whisper-1` | `whisper-1` is the only Whisper id OpenAI hosts (Whisper V2) |
| `openrouter` | `openai/gpt-4o-transcribe`, `openai/whisper-1`, `google/chirp-3` | Uses OpenRouter's JSON audio API; needs the fully-qualified slug |
| `local` | `whisper-large-v3`, `whisper-large-v3-turbo` | Point a provider at a whisper.cpp / faster-whisper server for fully offline transcription |

::: tip
`whisper-large-v3` is **not** a hosted OpenAI model id — it's open-source weights
you self-host. Run it via a local server and use the `local` provider.
:::

## How the request is routed

`la-core` picks the request shape automatically from the provider's base URL:

- **`openrouter.ai`** → JSON body with base64-encoded audio (`input_audio.data`).
- **everything else** (OpenAI, whisper.cpp, faster-whisper, Groq) → a
  `multipart/form-data` upload with a `file` field.

Both return `{ "text": ... }`.
