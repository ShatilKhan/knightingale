# Smoke eval corpus

Tiny fixtures used by `knightingale eval` in CI and during local testing.

To use:

```sh
knightingale eval --corpus eval-corpus/
```

Each `.wav` (16 kHz mono PCM) needs a matching `.txt` with the gold reference
transcript.

For real benchmarking, point `--corpus` at a larger set
(LibriSpeech `test-clean`, FLEURS, custom recordings, etc).
