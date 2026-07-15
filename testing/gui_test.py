#!/usr/bin/env python3
"""Reusable click-through testing library for og-os live-session GUI
testing (Calamares, or any other graphical app running in the live sway
session) via qemu + SSH + ydotool.

Why this exists, and the bug it works around
----------------------------------------------
Driving a Wayland compositor's GUI from outside a VM is fragile if you
synthesize input at the QEMU-monitor level (sendkey/mouse_move over the
VNC/QMP connection) — coordinate systems, focus routing, and absolute-
vs-relative pointer semantics all have to line up, and debugging *why*
a click didn't land is mostly guesswork from screenshots.

The reliable alternative: install `ydotool` *inside* the guest and
drive it over SSH. ydotool injects input at the kernel uinput level, so
it looks like real hardware to sway — no VNC/QEMU coordinate
translation involved.

The one real gotcha, found the hard way: ydotool's virtual absolute
pointer device declares a logical resolution that is HALF the real
screen resolution (e.g. 640x400 on a 1280x800 screen), not the real
pixel dimensions. Every `mousemove --absolute` coordinate must be
halved before sending, or clicks land at ~2x your intended target
(confirmed by watching the actual rendered cursor position land at
~2x the requested coordinates, then re-deriving the correct scale
factor and verifying against a visible cursor overlapping the intended
button before trusting a click). This module bakes that fix in via
click()/move() so callers always work in real screen-pixel coordinates.

Ground truth over pixel-guessing
---------------------------------
Don't infer success from screenshots alone. Use:
  - sway_tree() / find_window() — real window geometry + focus state
    (via `swaymsg -t get_tree`), to know exactly where a window's
    buttons are (Calamares' Back/Next/Cancel row is a fixed offset
    from the window's bottom-right corner regardless of tiling
    position — see calamares_next_button()).
  - tail_log() / wait_for_log() — if the app being tested writes a log
    (Calamares does, when launched with stdout redirected to a file),
    poll that instead of diffing screenshots to know when a page
    transition / job actually happened.

Usage
-----
    from gui_test import GuestSession

    g = GuestSession(ssh_port=22223, password="ogos")
    g.wait_for_ssh()
    g.install_ydotool()          # one-time per boot, not baked into the ISO
    g.login_greeter("ogos")      # types password + Enter at the lightdm greeter
    g.wait_for_sway_socket()

    win = g.find_window("io.calamares.calamares")
    x, y = g.calamares_next_button(win)
    g.click(x, y)
"""
from __future__ import annotations

import json
import subprocess
import time
from dataclasses import dataclass
from typing import Optional


@dataclass
class GuestSession:
    ssh_port: int
    password: str
    user: str = "liveuser"
    host: str = "localhost"
    ydotool_socket: str = "/tmp/.ydotool_socket"
    _sway_socket: Optional[str] = None

    # --- low-level SSH ---

    def ssh(self, remote_cmd: str, timeout: int = 30) -> subprocess.CompletedProcess:
        cmd = [
            "sshpass", "-p", self.password,
            "ssh", "-o", "StrictHostKeyChecking=no",
            "-o", "UserKnownHostsFile=/dev/null",
            "-p", str(self.ssh_port),
            f"{self.user}@{self.host}",
            remote_cmd,
        ]
        return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)

    def wait_for_ssh(self, timeout: int = 120) -> None:
        deadline = time.time() + timeout
        while time.time() < deadline:
            r = self.ssh("echo ok", timeout=5)
            if r.returncode == 0 and "ok" in r.stdout:
                return
            time.sleep(2)
        raise TimeoutError("SSH never came up")

    # --- one-time-per-boot setup ---

    def install_ydotool(self) -> None:
        """ydotool is a testing-session dependency, not something baked
        into the production ISO — install it fresh each boot."""
        self.ssh("sudo pacman -Sy --noconfirm ydotool", timeout=60)
        self.ssh("sudo usermod -aG input liveuser")
        self.ssh(
            f"sudo setsid ydotoold < /dev/null > /tmp/ydotoold.log 2>&1 & disown; "
            f"sleep 1; sudo chmod 666 {self.ydotool_socket}"
        )

    def wait_for_sway_socket(self, timeout: int = 60) -> str:
        deadline = time.time() + timeout
        while time.time() < deadline:
            r = self.ssh("ls /run/user/1000/sway-ipc.* 2>&1")
            if r.returncode == 0 and "sway-ipc" in r.stdout:
                self._sway_socket = r.stdout.strip().split("\n")[0]
                return self._sway_socket
            time.sleep(2)
        raise TimeoutError("sway socket never appeared — did login succeed?")

    @property
    def sway_socket(self) -> str:
        if not self._sway_socket:
            self.wait_for_sway_socket()
        return self._sway_socket  # type: ignore[return-value]

    # --- input injection (the part that has to be exactly right) ---

    def _ydotool(self, args: str) -> None:
        r = self.ssh(f"YDOTOOL_SOCKET={self.ydotool_socket} ydotool {args}")
        if r.returncode != 0:
            raise RuntimeError(f"ydotool {args} failed: {r.stderr}")

    def move(self, x: int, y: int) -> None:
        """Move the pointer to a REAL screen pixel coordinate.

        Halves x/y before sending: ydotool's virtual absolute device's
        logical resolution is half the real screen resolution. Verify
        this still holds if ydotool/wlroots versions change — confirm
        by moving to a known button and screenshotting before trusting
        click() blindly on a new setup (see module docstring).
        """
        self._ydotool(f"mousemove --absolute -x {x // 2} -y {y // 2}")

    def click(self, x: int, y: int, button: str = "0xC0") -> None:
        """Move to (x, y) in real screen pixels, then click.
        button: 0xC0 = left click (down+up). See `ydotool click --help`
        for the full bitmask table (right/middle/etc).

        Move and click are sent as ONE ssh round-trip (not two separate
        connections with a local sleep sandwiched between) — found this
        matters in practice: two separate self.ssh() calls per click
        was unreliable (silently missed real UI buttons in a scripted
        run despite identical coordinates that worked fine when typed
        interactively with a pause in between), while a single ssh call
        with a remote `sleep` between the two ydotool invocations was
        not. Suspect ssh connection-setup jitter between the two calls
        was occasionally landing the click before the compositor had
        processed the preceding move. Prefer click_verified() over this
        for anything that matters — it screenshots after clicking so
        you have actual evidence, not just hope.
        """
        cmd = (
            f"YDOTOOL_SOCKET={self.ydotool_socket} ydotool mousemove --absolute "
            f"-x {x // 2} -y {y // 2} && sleep 0.4 && "
            f"YDOTOOL_SOCKET={self.ydotool_socket} ydotool click {button}"
        )
        r = self.ssh(cmd)
        if r.returncode != 0:
            raise RuntimeError(f"click at ({x},{y}) failed: {r.stderr}")

    def click_verified(self, x: int, y: int, monitor_port: int, out_path: str,
                        button: str = "0xC0") -> str:
        """click() then screenshot immediately after — use this instead
        of bare click() whenever a test needs to actually trust the
        click landed, rather than assume it. Returns the screenshot
        path for the caller (or a human) to inspect."""
        self.click(x, y, button)
        time.sleep(1)
        self.screenshot(monitor_port, out_path)
        return out_path

    def type_text(self, text: str) -> None:
        self._ydotool(f"type {text}")

    def key(self, *keycodes_and_state: str) -> None:
        """Raw key event pairs, e.g. key("28:1", "28:0") for Enter
        press+release. Look up keycodes in linux/input-event-codes.h."""
        self._ydotool("key " + " ".join(keycodes_and_state))

    def press_enter(self) -> None:
        self.key("28:1", "28:0")

    def login_greeter(self, password: str) -> None:
        """Types the password at lightdm's greeter (username is already
        selected — single-user live ISO) and hits Enter."""
        self.type_text(password)
        self.press_enter()

    # --- ground truth: window tree ---

    def sway_tree(self) -> dict:
        r = self.ssh(f"SWAYSOCK={self.sway_socket} swaymsg -t get_tree")
        # ssh may prepend a "Warning: Permanently added..." line on first
        # connection — skip to the first '{'.
        text = r.stdout
        start = text.find("{")
        return json.loads(text[start:])

    def find_window(self, app_id: str) -> Optional[dict]:
        """Returns the sway container node for a window by app_id,
        including its real on-screen rect and focus state — ground
        truth, not a guess from a screenshot."""
        def walk(n):
            if n.get("app_id") == app_id:
                return n
            for c in n.get("nodes", []) + n.get("floating_nodes", []):
                found = walk(c)
                if found:
                    return found
            return None
        return walk(self.sway_tree())

    def wait_for_window(self, app_id: str, timeout: int = 30) -> dict:
        deadline = time.time() + timeout
        while time.time() < deadline:
            w = self.find_window(app_id)
            if w:
                return w
            time.sleep(1)
        raise TimeoutError(f"window with app_id={app_id!r} never appeared")

    # --- ground truth: app log, not pixel-diffing ---

    def launch_logged(self, command: str, log_path: str = "/tmp/gui_test_app.log") -> None:
        """Launch a command in the live session with stdout/stderr
        redirected to a file you can poll with wait_for_log() — far
        more reliable than screenshot diffing for "did the click do
        anything" verification."""
        self.ssh(
            f"sudo setsid sh -c "
            f"'XDG_RUNTIME_DIR=/run/user/1000 WAYLAND_DISPLAY=wayland-1 "
            f"QT_QPA_PLATFORM=wayland {command} > {log_path} 2>&1' "
            f"< /dev/null > /dev/null 2>&1 & disown"
        )

    def log_line_count(self, log_path: str) -> int:
        r = self.ssh(f"wc -l < {log_path} 2>&1")
        try:
            return int(r.stdout.strip().split()[0])
        except (ValueError, IndexError):
            return 0

    def tail_log(self, log_path: str, from_line: int = 0) -> str:
        r = self.ssh(f"tail -n +{from_line + 1} {log_path} 2>&1")
        return r.stdout

    def wait_for_log(self, log_path: str, pattern: str, from_line: int = 0,
                      timeout: int = 30) -> str:
        """Poll a log file until `pattern` appears in new lines, or
        raise TimeoutError. Returns the new log content."""
        deadline = time.time() + timeout
        while time.time() < deadline:
            new_text = self.tail_log(log_path, from_line)
            if pattern in new_text:
                return new_text
            time.sleep(1)
        raise TimeoutError(f"{pattern!r} never appeared in {log_path}")

    # --- screenshots (for human review / debugging, not as ground truth) ---

    def screenshot(self, monitor_port: int, out_path: str) -> None:
        """Grab a screenshot via the qemu monitor for visual review.
        Use sway_tree()/wait_for_log() to actually *drive* a test —
        this is for producing evidence a human can look at."""
        import socket
        ppm_path = out_path + ".ppm"
        with socket.create_connection(("127.0.0.1", monitor_port), timeout=5) as s:
            s.recv(4096)  # banner
            s.sendall(f"screendump {ppm_path}\n".encode())
            time.sleep(1.5)
            s.recv(4096)
        subprocess.run(["convert", ppm_path, out_path], capture_output=True)

    # --- known widget-position helpers ---

    @staticmethod
    def calamares_next_button(window: dict) -> tuple[int, int]:
        """Calamares' Back/Next/Cancel row sits at a fixed offset from
        the window's bottom-right corner regardless of where the
        window is tiled on screen — computed from the window rect
        (ground truth from find_window()) rather than a hardcoded
        absolute screen position, so it survives different tiling
        layouts. Offsets calibrated against a 640x778 window; re-check
        with a screenshot crop if Calamares' UI chrome changes size.
        """
        rect = window["rect"]
        x = rect["x"] + int(rect["width"] * 0.746)
        y = rect["y"] + int(rect["height"] * 0.957)
        return x, y
