# Packaging

OS-specific files for system installation.

## SHAs: who computes them, when

The release tarballs change every tag, so the package manifests can't hard-code
a real SHA in `main`. Each ecosystem has its own convention:

| Path | Pattern | How it gets the real SHA |
|---|---|---|
| `homebrew/Formula/*.rb` | `sha256 "0…"` placeholder | `dawidd6/action-homebrew-bump-formula` runs on release, opens a PR against `ShatilKhan/homebrew-tap` with the computed SHA |
| `scoop/*.json` | `checkver` + `autoupdate` blocks | Scoop re-fetches from GitHub releases on `scoop update`; hash is computed client-side |
| `aur/PKGBUILD` | `sha256sums=('SKIP')` | AUR convention for tagged releases — trust upstream |
| `winget/manifest.yaml` | Reference template | Submitted to `microsoft/winget-pkgs` via PR using `winget-create`, which computes SHA from the binary URL |

The `homebrew` placeholder zeros are valid for the file's first commit; the
release workflow rewrites them on each tag.

## Linux — systemd-user

```
install -m 0644 packaging/systemd/knightingale.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now knightingale.service
```

## macOS — launchd LaunchAgent

```
install -m 0644 packaging/launchd/dev.shatilkhan.knightingale.plist \
    ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/dev.shatilkhan.knightingale.plist
```

## Windows — Run-key autostart

`knightingale install-service` (Phase 3) writes:
```
HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Knightingale =
    %LOCALAPPDATA%\Programs\knightingale\knightingale-daemon.exe
```
