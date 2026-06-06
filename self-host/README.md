# Self-hosted STT

Run an OpenAI-compatible STT server on your machine, point Knightingale at it,
no API key required.

## Speaches

```sh
cd self-host/speaches
docker compose up -d
```

Set in `~/.config/knightingale/.env`:

```
KNIGHTINGALE_PROVIDER=custom
CUSTOM_BASE_URL=http://localhost:8000/v1
CUSTOM_API_KEY=anything
CUSTOM_MODEL=Systran/faster-whisper-large-v3
```

The first transcription will be slow while the model downloads inside the
container. After that it's near-instant on a GPU.

## CPU-only

Drop `deploy.resources.reservations.devices` and use the `:latest-cpu` tag.
Speaches falls back to faster-whisper on CPU.

## Alternatives

Anything that speaks `/v1/audio/transcriptions` works:

- [whisper.cpp `server`](https://github.com/ggml-org/whisper.cpp/tree/master/examples/server)
- [LocalAI](https://localai.io/)
- [vLLM Whisper](https://docs.vllm.ai/)
