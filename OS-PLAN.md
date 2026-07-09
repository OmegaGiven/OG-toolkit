# OG-OS — plan to package this machine into an installable ISO

Goal: take this exact sway/OG-toolkit desktop stack (base Arch + custom
tools/configs/scripts built across the og-bar/og-settings/og-search/etc
sessions) and turn it into something installable on a new or repeat
machine — an ISO, not a dotfiles script that half-works depending on
what's already there.

Snapshot of the machine this plan is built from (2026-07-09): Arch Linux,
139 explicit official packages + 17 AUR packages (via `yay`), lightdm as
greeter, sway 1.11 as the compositor. Numbers below will drift as the
system does — this doc describes the *mechanism* for capturing state, not
a frozen package list.

## 1. Two viable approaches — pick one before building anything

| | **archiso respin** | **Custom installer image (calamares/airootfs)** |
|---|---|---|
| What it is | Arch's own `archiso` tool, profile = official releng profile + your packages/configs baked into `airootfs/` | archiso base, but boots into a graphical installer (Calamares) that partitions/installs for the user instead of a manual `arch-chroot` |
| Effort | Low-medium — mostly config, no new UI to build | Medium-high — need to package Calamares + a branding module |
| Result | Boots to a live sway session with everything installed; user partitions manually via a script you provide, `pacstrap`-alike | Boots to a "click next" installer, ends with a fully configured system, reboots into it |
| Best for | You, re-imaging your own boxes | "someone else" installs it unattended |

**Recommendation: start with the archiso respin.** It's the same tool
Arch's own installer ISO is built from, gets you a working artifact in
days not weeks, and the airootfs/package-list/hooks structure it produces
is exactly what a later Calamares module would consume anyway — nothing
built in phase 1 is thrown away if you go further later.

## 2. What actually needs to be captured

Everything below already exists on this machine in some form — the work
is extracting it into a buildable, versioned form instead of "whatever
state this disk happens to be in."

### 2a. Package manifest
- `pacman -Qqe` → official repo package list (139 today) → `packages.x86_64` for archiso.
- `pacman -Qqm` → AUR/foreign package list (17 today: yay, brave-bin, vesktop, alvr-bin, etc. — note `discord` on this machine isn't in this list, so it's coming from a configured sync repo, not AUR; check `pacman.conf` for which repo before assuming AUR-only handling covers it) — archiso can't pull AUR at build time by default; either:
  - (a) vendor a local pacman repo of prebuilt AUR packages baked into the ISO (`repo-add`, then a local `[ogos]` repo entry in the image's pacman.conf), or
  - (b) ship `yay` + a first-boot script that pulls AUR packages post-install (needs network at install time — acceptable for a personal respin, not for the "airgapped installer" ideal).
  - Recommendation: (a) for anything used at every boot (yay itself, mangohud, gamemode-adjacent), (b) as a documented fallback for the long tail.

### 2b. Dotfiles / configs
Already tracked ad hoc under `~/.config/` — needs a single "skeleton"
source of truth copied into new-user homes at install time (`/etc/skel`
equivalent) or an actual dotfiles repo checked out post-install:
- `~/.config/sway/config` (+ `hotkeys.sh`)
- `~/.config/mako/`, `~/.config/alacritty/`, `~/.config/waybar/` (legacy — superseded by og-bar, keep only if still referenced by anything, otherwise drop)
- `~/.config/sway-power/` (og-power-apply's own config)
- `~/.config/og-files/`, `~/.config/og-links/`
- og-bar's `bar-config.json`, og-settings' `config.json` (both via `og-config` — ship sane defaults, not this machine's personal tuning)

### 2c. Your own repos (the actual product)
All of these are separate git repos today under `~/.local/src/`:
`OG-toolkit`, `og-bar`, `sway-control` (og-settings), `omegagiven-search`
(og-search), `og-clip`, `og-note`, `og-notif-center`, `og-notify`,
`app-store`, `file-manager`, `galias`, `OmegaGiven-profile`.

For an ISO these need to stop being "clone and cargo build on first
boot" and become **actual built binaries baked into the image** (or into
the local pacman repo from 2a as real packages with PKGBUILDs) —
otherwise the ISO needs a Rust toolchain and a live internet connection
just to reach a usable desktop, which defeats "install and go."
Concretely: write a `PKGBUILD` per repo (they're just `cargo build
--release` + install steps, straightforward), build once, drop `.pkg.tar.zst`
into the local `[ogos]` repo alongside the vendored AUR packages.

### 2d. System-level config outside `/home`
- greeter: lightdm + lightdm-gtk-greeter, greeter background sync (already handled by `sway::sync_greeter_background`) — needs to run once at install time, not rely on a live user session.
- systemd units: user-level (`og-links.service`, pipewire wants, timers) — ship as part of a `/etc/skel`-style user unit seed, enabled via `systemctl --user preset` on first login. System-level custom units found today: `gpu-performance.service` — audit what it does and ship it as an actual package-installed unit, not a hand-edited file.
- `/etc/wolf/cfg/config.toml` patch (the uinput/cgroup fix from the Remote Play session) — if Wolf/gamescope-hosting is meant to ship by default, this needs to become a config *template* applied by an install hook, not a manual edit repeated per machine.
- sudoers: the scoped NOPASSWD entries being added for og-vpn-apply etc. — these need to ship as real files under `/etc/sudoers.d/` installed by package, with the *scripts* they authorize also owned by root and not world-writable (currently `~/.local/bin` — a NOPASSWD sudoers rule pointing at a path a normal user can edit is a privilege escalation hole, this must move to `/usr/local/bin` or a package-owned path before it ships to anyone else).

### 2e. Hardware breadth
This machine has amdgpu (`xf86-video-amdgpu`) but the pacman list also
includes nouveau/radeon/intel-media drivers — that's fine for a
general-purpose ISO (install everything, let the running kernel pick),
just confirm the package list intentionally stays hardware-agnostic
rather than trimmed to "what this box needs," since the whole point is
running on a "new or repeat system."

## 3. Proposed repo layout

New repo, e.g. `~/.local/src/og-os/`:

```
og-os/
├── profiledef.sh              # archiso profile metadata
├── packages.x86_64            # official repo package list (from 2a)
├── pacman.conf                 # base + local [ogos] repo entry
├── local-repo/                 # built .pkg.tar.zst for AUR + your own tools
│   └── (repo-add generated .db/.files)
├── pkgbuilds/                  # one dir per your-own-tool repo
│   ├── og-bar/PKGBUILD
│   ├── og-settings/PKGBUILD    # builds sway-control → og-settings
│   ├── og-search/PKGBUILD
│   └── ...
├── airootfs/                   # files copied verbatim onto the live/installed system
│   ├── etc/skel/.config/sway/config
│   ├── etc/skel/.config/...
│   ├── etc/systemd/system/gpu-performance.service
│   ├── etc/sudoers.d/og-vpn    # scoped NOPASSWD entry
│   └── root/install.sh         # or a customize_airootfs.sh archiso hook
└── OS-PLAN.md                  # this doc, or move it here once the repo exists
```

## 4. Build pipeline (once repo exists)

1. `mkarchiso -v -o out/ og-os/` — standard archiso build, pulls
   `packages.x86_64` from the official repos + `[ogos]` local repo for
   everything in `local-repo/`.
2. Result: a `.iso` in `out/` — `dd` to USB or boot in a VM to test.
3. First real install target: a VM (qemu, already have `qemu-desktop`
   installed) — validate partitioning script + first-boot config before
   ever touching real hardware.

## 5. Suggested phase order

1. **Inventory freeze** — `pacman -Qqe`/`pacman -Qqm` snapshot committed to the new repo, so there's a concrete starting package list instead of "whatever's on this disk right now."
2. **PKGBUILDs for the 12 own-repos** — makes them installable/updatable like any other package, independent of the ISO work (useful immediately, even before an ISO exists).
3. **archiso skeleton** — boots to a live sway session with the package list, no installer yet.
4. **Install script** — partition + pacstrap + airootfs overlay + first-boot systemd preset, run manually from the live session (this is "phase 1" from section 1).
5. **(Optional, later)** Calamares module if unattended installs for other people become the actual goal.

## 6. Open decisions (need your call before phase 2+)

- Drop or keep waybar/mako/alacritty configs now that og-bar/og-search exist? (legacy configs found in `~/.config/` — worth auditing which are still live vs dead.)
- Personal vs. distributable config split — ship your actual `bar-config.json`/hotkeys, or seed defaults and let install re-run og-settings' own onboarding? Recommend defaults + a "restore my config" import step, so the ISO isn't hardcoded to your machine's binding choices.
- AUR package vendoring (2a option a vs b) — how many of the 17 AUR packages are actually load-bearing for the *desktop experience* (yay, mangohud/gamemode) vs. personal apps (discord, vesktop, brave-bin, gaming launchers) that a distributable OS probably shouldn't force-bundle.
