# og-os

Plan + build inputs for packaging the OG-toolkit sway desktop (this
machine's setup) into an installable Arch-based ISO.

- `OS-PLAN.md` — the plan (approach, phases, open decisions).
- `packages.x86_64` — frozen snapshot of `pacman -Qqe` (official-repo explicit packages).
- `foreign-packages.txt` — frozen snapshot of `pacman -Qqm` (AUR/foreign packages).
- `INVENTORY-DATE.txt` — when the above snapshots were taken; re-freeze periodically, don't assume they stay current.

See OS-PLAN.md section 5 for phase order. This repo is phase 1 (inventory freeze) — PKGBUILDs and the archiso skeleton land in later phases.
