#!/usr/bin/env bash
# OG-OS installer — phase 4 of OS-PLAN.md. Partitions a disk, pacstraps
# base + the OG-suite (via the [ogos] repo baked onto this ISO at
# /root/ogos-repo — not the build machine's local-repo/, which won't
# exist on the installed target), creates the first sudo user, installs
# systemd-boot, and seeds a default sway session. Run from the live
# session as root (liveuser has NOPASSWD sudo).
#
# Usage:
#   install-ogos.sh                      interactive prompts
#   install-ogos.sh --disk /dev/sda --hostname ogos-box --user alice \
#                    --password hunter2 --yes
#                                         unattended (all flags required
#                                         together, --yes skips the
#                                         final destructive-confirm)
#
# --password on the command line is visible in `ps`/shell history —
# fine for throwaway qemu testing, not for anything you'd actually keep.
set -euo pipefail

DISK=""
HOSTNAME_VAL=""
USERNAME=""
PASSWORD=""
ASSUME_YES=0
FS_TYPE="ext4"

usage() {
    cat <<'EOF'
Usage: install-ogos.sh [--disk /dev/sdX] [--hostname NAME] [--user NAME]
                        [--password PASS] [--fs ext4|btrfs] [--yes]

Omit any of --disk/--hostname/--user/--password and you'll be prompted
for it interactively. --yes skips the final "this will erase X" confirm.
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --disk) DISK="$2"; shift 2 ;;
        --hostname) HOSTNAME_VAL="$2"; shift 2 ;;
        --user) USERNAME="$2"; shift 2 ;;
        --password) PASSWORD="$2"; shift 2 ;;
        --fs) FS_TYPE="$2"; shift 2 ;;
        --yes) ASSUME_YES=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
    esac
done

if [ "$(id -u)" -ne 0 ]; then
    echo "Must run as root (try: sudo $0)" >&2
    exit 1
fi

echo "== OG-OS installer =="

if [ -z "$DISK" ]; then
    echo
    echo "Available disks:"
    lsblk -d -o NAME,SIZE,MODEL -n | sed 's#^#  /dev/#'
    echo
    read -rp "Target disk (e.g. /dev/sda): " DISK
fi
if [ ! -b "$DISK" ]; then
    echo "error: $DISK is not a block device" >&2
    exit 1
fi

if [ -z "$HOSTNAME_VAL" ]; then
    read -rp "Hostname [ogos]: " HOSTNAME_VAL
    HOSTNAME_VAL="${HOSTNAME_VAL:-ogos}"
fi

if [ -z "$USERNAME" ]; then
    read -rp "Username for the first sudo user: " USERNAME
fi
if [ -z "$USERNAME" ]; then
    echo "error: username can't be empty" >&2
    exit 1
fi

if [ -z "$PASSWORD" ]; then
    while true; do
        read -rsp "Password for $USERNAME: " PASSWORD
        echo
        read -rsp "Confirm password: " PASSWORD_CONFIRM
        echo
        [ "$PASSWORD" = "$PASSWORD_CONFIRM" ] && break
        echo "Passwords didn't match, try again."
    done
fi

case "$FS_TYPE" in
    ext4|btrfs) ;;
    *) echo "error: --fs must be ext4 or btrfs" >&2; exit 1 ;;
esac

if [ "$ASSUME_YES" -ne 1 ]; then
    echo
    echo "!! This will ERASE ALL DATA on $DISK !!"
    read -rp "Type 'yes' to continue: " CONFIRM
    [ "$CONFIRM" = "yes" ] || { echo "Aborted."; exit 1; }
fi

# --- Partitioning ---
# GPT: 512MiB ESP (ef00) + rest as root.
echo "==> Partitioning $DISK"
sgdisk --zap-all "$DISK"
sgdisk -n1:0:+512MiB -t1:ef00 -c1:ESP "$DISK"
sgdisk -n2:0:0       -t2:8300 -c2:root "$DISK"
partprobe "$DISK"
sleep 2

# NVMe-style disks (/dev/nvme0n1) need a 'p' before the partition number;
# plain sdX/vdX disks don't.
if [[ "$DISK" =~ [0-9]$ ]]; then
    ESP_PART="${DISK}p1"
    ROOT_PART="${DISK}p2"
else
    ESP_PART="${DISK}1"
    ROOT_PART="${DISK}2"
fi

echo "==> Formatting"
mkfs.fat -F32 -n ESP "$ESP_PART"
if [ "$FS_TYPE" = "btrfs" ]; then
    mkfs.btrfs -f -L root "$ROOT_PART"
else
    mkfs.ext4 -F -L root "$ROOT_PART"
fi

echo "==> Mounting"
mount "$ROOT_PART" /mnt
mkdir -p /mnt/boot
mount "$ESP_PART" /mnt/boot

# --- pacstrap ---
# Temporary install-time pacman.conf: the live system's own /etc/pacman.conf
# (stock core/extra) plus the [ogos] repo baked onto this ISO at
# /root/ogos-repo. This conf is only used to drive pacstrap itself — the
# target's /etc/pacman.conf comes from whatever the 'pacman' package ships
# by default, so there's nothing to strip afterward: /root/ogos-repo
# wouldn't exist on the installed disk anyway.
INSTALL_PACMAN_CONF="/root/pacman-install.conf"
cp /etc/pacman.conf "$INSTALL_PACMAN_CONF"
cat >> "$INSTALL_PACMAN_CONF" <<'EOF'

[ogos]
SigLevel = Optional TrustAll
Server = file:///root/ogos-repo
EOF

echo "==> pacstrap (this takes a while)"
pacstrap -K -C "$INSTALL_PACMAN_CONF" /mnt \
    base base-devel linux linux-firmware amd-ucode intel-ucode sudo efibootmgr \
    sway sway-contrib swaybg swayidle swaylock lightdm lightdm-gtk-greeter \
    polkit xorg-xwayland xdg-desktop-portal-wlr xdg-desktop-portal-gtk xdg-utils \
    og-bar og-settings og-search og-clip og-notif-center og-notify og-files \
    og-links og-apps og-scripts \
    ttf-nerd-fonts-symbols ttf-nerd-fonts-symbols-common wl-clipboard grim slurp \
    alacritty mako \
    pipewire-alsa pipewire-jack pipewire-pulse alsa-utils alsa-firmware sof-firmware \
    bluez bluez-utils iwd wireless_tools openssh tailscale wireguard-tools \
    mesa-utils vulkan-tools vulkan-intel vulkan-radeon vulkan-nouveau \
    libva-intel-driver intel-media-driver libva-utils \
    xf86-video-amdgpu xf86-video-ati xf86-video-nouveau \
    git nano vim htop btop unzip wget less brightnessctl smartmontools \
    zram-generator flatpak mpv \
    cups cups-pdf ghostscript gsfonts gutenprint \
    yay zsh

genfstab -U /mnt >> /mnt/etc/fstab

# pacstrap only pulls package files into /mnt — it does NOT carry over
# anything from this live ISO's own airootfs overlay, including the seed
# sway config at /etc/skel here. Without this, useradd -m on the target
# has nothing to copy and the new user gets a bare home dir (confirmed:
# sway silently falls back to its own stock default session).
mkdir -p /mnt/etc/skel/.config/sway
cp /etc/skel/.config/sway/config /mnt/etc/skel/.config/sway/config

# --- Target system config (arch-chroot) ---
echo "==> Configuring installed system"

arch-chroot /mnt /bin/bash <<CHROOT_EOF
set -euo pipefail

ln -sf /usr/share/zoneinfo/UTC /etc/localtime
hwclock --systohc

echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen
locale-gen
echo "LANG=en_US.UTF-8" > /etc/locale.conf

echo "$HOSTNAME_VAL" > /etc/hostname
cat >> /etc/hosts <<HOSTS_EOF
127.0.0.1   localhost
::1         localhost
127.0.1.1   $HOSTNAME_VAL.localdomain $HOSTNAME_VAL
HOSTS_EOF

# Real, persistent [ogos] repo entry — unlike the throwaway
# pacman-install.conf above (file:///root/ogos-repo, an ISO-local path
# that doesn't exist once this is a real installed system), this is the
# one og-settings' Updates tab and any later `pacman -Syu` actually see.
# Without it, an installed system has zero update path for its own
# desktop suite: pacstrap only ever reads pacman-install.conf, and the
# target's own /etc/pacman.conf (pacman's stock default) has no [ogos]
# section at all unless something adds it here.
cat >> /etc/pacman.conf <<PACMAN_EOF

[ogos]
SigLevel = Optional TrustAll
Server = https://omegagiven.github.io/OG-os-repo/x86_64
PACMAN_EOF

mkinitcpio -P

useradd -m -G wheel -s /usr/bin/zsh "$USERNAME"
echo "$USERNAME:$PASSWORD" | chpasswd
passwd -l root
sed -i 's/^# %wheel ALL=(ALL:ALL) ALL/%wheel ALL=(ALL:ALL) ALL/' /etc/sudoers

systemctl enable lightdm.service
systemctl enable iwd.service
systemctl enable bluetooth.service
systemctl enable systemd-networkd.service
systemctl enable systemd-resolved.service
ln -sf /usr/lib/systemd/system/graphical.target /etc/systemd/system/default.target

bootctl --path=/boot install
cat > /boot/loader/loader.conf <<LOADER_EOF
default arch.conf
timeout 3
console-mode max
editor no
LOADER_EOF

ROOT_PARTUUID=\$(blkid -s PARTUUID -o value "$ROOT_PART")
mkdir -p /boot/loader/entries
cat > /boot/loader/entries/arch.conf <<ENTRY_EOF
title   OG-OS
linux   /vmlinuz-linux
initrd  /amd-ucode.img
initrd  /intel-ucode.img
initrd  /initramfs-linux.img
options root=PARTUUID=\$ROOT_PARTUUID rw
ENTRY_EOF
CHROOT_EOF

echo "==> Done. Unmount and reboot when ready:"
echo "    umount -R /mnt && reboot"
