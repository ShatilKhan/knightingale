# Packaging

OS-specific files for system installation.

## Linux — systemd-user

Drop into `~/.config/systemd/user/`:
```
install -m 0644 packaging/systemd/knightingale.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now knightingale.service
```

`knightingale install-service` (Phase 1) automates the above.

## macOS — launchd LaunchAgent

```
install -m 0644 packaging/launchd/dev.shatilkhan.knightingale.plist \
    ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/dev.shatilkhan.knightingale.plist
```

## Windows

Set the Run-key via PowerShell at install time. `knightingale install-service` writes:
```
HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Knightingale =
    %LOCALAPPDATA%\Programs\knightingale\knightingale-daemon.exe
```
