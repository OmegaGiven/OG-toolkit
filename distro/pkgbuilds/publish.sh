#!/usr/bin/env bash
# Builds every OG-suite PKGBUILD, assembles the [ogos] pacman repo, and
# publishes it two places:
#   1. distro/local-repo/ -> distro/airootfs/root/ogos-repo/ (baked into
#      the ISO itself, so a fresh install works fully offline — see
#      install-ogos.sh's pacman-install.conf, which points here).
#   2. https://omegagiven.github.io/OG-os-repo/x86_64/ (a real hosted
#      repo an *installed* system's own /etc/pacman.conf points at
#      permanently — without this, an installed system has no update
#      path for its own desktop suite at all).
#
# Both copies come from the same build, same run — never let them drift.
#
# Requires: the OG-os-repo GitHub repo already cloned somewhere and its
# path passed as $1 (or set OGOS_REPO_CLONE), push access via `gh`/git
# credentials already configured.
set -euo pipefail

DISTRO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKGBUILDS_DIR="$DISTRO_DIR/pkgbuilds"
LOCAL_REPO="$DISTRO_DIR/local-repo"
BAKED_REPO="$DISTRO_DIR/airootfs/root/ogos-repo"
OGOS_REPO_CLONE="${1:-${OGOS_REPO_CLONE:-}}"

if [ -z "$OGOS_REPO_CLONE" ]; then
    echo "usage: $0 <path to a clone of OmegaGiven/OG-os-repo>" >&2
    echo "  (or set OGOS_REPO_CLONE env var)" >&2
    exit 1
fi
if [ ! -d "$OGOS_REPO_CLONE/.git" ]; then
    echo "error: $OGOS_REPO_CLONE is not a git clone (run: gh repo clone OmegaGiven/OG-os-repo)" >&2
    exit 1
fi

PACKAGES=(og-settings og-apps og-bar og-clip og-files og-notif-center og-notify og-search og-links og-scripts yay calamares)
# yay/calamares are third-party AUR builds, not OG-toolkit monorepo apps —
# they don't rsync from anywhere, their own PKGBUILD sources() pull
# straight from GitHub release tarballs. Same build/repo-add/publish loop
# works for them unmodified since nothing here is OG-toolkit-specific
# beyond the rsync step each individual PKGBUILD already handles itself.
# calamares in particular is a real C++/Qt6 build — expect this loop to
# take significantly longer whenever it needs a rebuild (a new upstream
# release, not just an OG-suite app change).

echo "==> Building ${#PACKAGES[@]} packages"
mkdir -p "$LOCAL_REPO" "$BAKED_REPO"
for p in "${PACKAGES[@]}"; do
    echo "--- $p ---"
    dir="$PKGBUILDS_DIR/$p"
    rm -rf "$dir/src" "$dir/pkg" "$dir"/*.pkg.tar.zst "$dir"/*.pkg.tar.zst.sig 2>/dev/null || true
    ( cd "$dir" && makepkg -f --noconfirm --skipchecksums )
    cp "$dir"/*.pkg.tar.zst "$LOCAL_REPO/"
    # Debug packages (gdb-add-index split-debug output) aren't part of the
    # base install list and just double the repo's size for no one — the
    # real og-* binaries above are all a fresh install or update needs.
    rm -f "$LOCAL_REPO"/*-debug-*.pkg.tar.zst
    rm -rf "$dir/src" "$dir/pkg"
done

echo "==> repo-add"
( cd "$LOCAL_REPO" && repo-add ogos.db.tar.gz ./*.pkg.tar.zst )
# repo-add's ogos.db/ogos.files are symlinks to the versioned .tar.gz —
# fine for local-repo/ (file:// access follows symlinks), but GitHub
# Pages does not reliably serve symlinks, so the copy going there needs
# real files. Resolve them once, here, rather than at every publish site.
for l in ogos.db ogos.files; do
    if [ -L "$LOCAL_REPO/$l" ]; then
        cp --remove-destination "$(readlink -f "$LOCAL_REPO/$l")" "$LOCAL_REPO/$l"
    fi
done

echo "==> Baking into airootfs/root/ogos-repo (offline install path)"
rm -f "$BAKED_REPO"/*.pkg.tar.zst "$BAKED_REPO"/*.db* "$BAKED_REPO"/*.files*
cp "$LOCAL_REPO"/*.pkg.tar.zst "$LOCAL_REPO"/ogos.db.tar.gz "$LOCAL_REPO"/ogos.files.tar.gz "$LOCAL_REPO"/ogos.db "$LOCAL_REPO"/ogos.files "$BAKED_REPO/"

echo "==> Publishing to $OGOS_REPO_CLONE (GitHub Pages, permanent update path)"
mkdir -p "$OGOS_REPO_CLONE/x86_64"
rm -f "$OGOS_REPO_CLONE"/x86_64/*.pkg.tar.zst "$OGOS_REPO_CLONE"/x86_64/*.db* "$OGOS_REPO_CLONE"/x86_64/*.files*
cp "$LOCAL_REPO"/*.pkg.tar.zst "$LOCAL_REPO"/ogos.db.tar.gz "$LOCAL_REPO"/ogos.files.tar.gz "$LOCAL_REPO"/ogos.db "$LOCAL_REPO"/ogos.files "$OGOS_REPO_CLONE/x86_64/"
touch "$OGOS_REPO_CLONE/.nojekyll"

( cd "$OGOS_REPO_CLONE" \
  && git add -A \
  && (git diff --cached --quiet && echo "nothing changed, skipping commit/push" \
      || (git commit -q -m "Publish OG-OS suite packages $(date +%Y%m%d)" \
          && git push)) )

echo "==> Done. Verify: curl -sI https://omegagiven.github.io/OG-os-repo/x86_64/ogos.db"
