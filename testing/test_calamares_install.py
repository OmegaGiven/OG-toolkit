#!/usr/bin/env python3
"""End-to-end Calamares click-through test for og-os.

Assumes a VM is already booted via boot-vm.sh and its env sourced, OR
pass --ssh-port/--monitor-port directly. Drives: login -> Welcome ->
Location -> Keyboard -> Partitions (erase whole disk) -> Users (create
a test account) -> Summary -> Install -> Finish, using gui_test.py's
ydotool-over-SSH input and swaymsg/log-based ground truth instead of
screenshot-guessing.

Usage:
    python3 test_calamares_install.py --ssh-port 2223 --monitor-port 4458 \\
        --username testuser --password testpass123 --hostname ogos-test
"""
import argparse
import time

from gui_test import GuestSession


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ssh-port", type=int, required=True)
    ap.add_argument("--monitor-port", type=int, required=True)
    ap.add_argument("--live-password", default="ogos")
    ap.add_argument("--username", default="testuser")
    ap.add_argument("--password", default="testpass123")
    ap.add_argument("--hostname", default="ogos-test")
    ap.add_argument("--screenshot-dir", default="/tmp/ogos-test-shots")
    args = ap.parse_args()

    import os
    os.makedirs(args.screenshot_dir, exist_ok=True)

    g = GuestSession(ssh_port=args.ssh_port, password=args.live_password)

    def shot(name):
        path = f"{args.screenshot_dir}/{name}.png"
        g.screenshot(args.monitor_port, path)
        print(f"  [screenshot: {path}]")

    print("== Waiting for SSH ==")
    g.wait_for_ssh(timeout=120)

    print("== Installing ydotool in guest ==")
    g.install_ydotool()

    print("== Logging in at greeter ==")
    g.login_greeter(args.live_password)
    g.wait_for_sway_socket(timeout=60)
    print("  sway session up")

    print("== Waiting for Calamares to auto-launch, then relaunching with logging ==")
    # The seed sway config auto-launches Calamares with no log capture
    # (its stdout goes nowhere). Kill that instance and relaunch via
    # launch_logged() so we have a real log to poll for ground truth
    # instead of guessing from screenshots or window titles (Calamares'
    # window title doesn't change per page).
    g.wait_for_window("io.calamares.calamares", timeout=60)
    g.ssh("sudo pkill -f calamares")
    time.sleep(1)
    log_path = "/tmp/calamares_test.log"
    g.launch_logged("calamares -d", log_path)
    win = g.wait_for_window("io.calamares.calamares", timeout=30)
    print(f"  Calamares window: {win['rect']}")
    shot("01-welcome")

    def click_next():
        """Click Next, then confirm the click had *some* effect by
        checking the log grew — ground truth, not a screenshot guess.
        Doesn't assert *which* page we're on, just that something
        happened; screenshots are still taken for human review of
        exactly where we ended up."""
        before = g.log_line_count(log_path)
        w = g.find_window("io.calamares.calamares")
        x, y = g.calamares_next_button(w)
        g.click(x, y)
        time.sleep(2)
        after = g.log_line_count(log_path)
        if after <= before:
            print(f"  WARNING: no new log lines after clicking Next "
                  f"({before} -> {after}) -- click may have missed")

    print("== Welcome -> Location ==")
    click_next()
    shot("02-location")

    print("== Location -> Keyboard ==")
    click_next()
    shot("03-keyboard")

    print("== Keyboard -> Partitions ==")
    click_next()
    time.sleep(2)
    shot("04-partitions-initial")

    # Partitions page: pick "Erase disk" (first radio button / choice).
    # Position calibrated relative to the window rect, same approach as
    # calamares_next_button — top choice sits near vertical-center-upper
    # of the content area. If this misses, screenshot 04 shows exactly
    # where to recalibrate.
    w = g.find_window("io.calamares.calamares")
    rect = w["rect"]
    erase_x = rect["x"] + int(rect["width"] * 0.5)
    erase_y = rect["y"] + int(rect["height"] * 0.25)
    print(f"  Clicking 'Erase disk' choice at ({erase_x}, {erase_y})")
    g.click(erase_x, erase_y)
    time.sleep(1)
    shot("05-partitions-erase-selected")

    print("== Partitions -> Users ==")
    click_next()
    time.sleep(2)
    shot("06-users-blank")

    # Users page: Full name field is usually the first focused field.
    # Tab through: fullname -> username -> hostname -> password -> confirm.
    print("== Filling Users page ==")
    w = g.find_window("io.calamares.calamares")
    rect = w["rect"]
    # Click into the first text field (full name) to guarantee focus,
    # then drive the rest with Tab -- more robust than guessing every
    # field's position.
    fullname_x = rect["x"] + int(rect["width"] * 0.5)
    fullname_y = rect["y"] + int(rect["height"] * 0.25)
    g.click(fullname_x, fullname_y)
    g.type_text("Test User")
    g.key("15:1", "15:0")  # Tab (KEY_TAB=15)
    g.type_text(args.username)
    g.key("15:1", "15:0")
    g.type_text(args.hostname)
    g.key("15:1", "15:0")
    g.type_text(args.password)
    g.key("15:1", "15:0")
    g.type_text(args.password)
    shot("07-users-filled")

    print("== Users -> Summary ==")
    click_next()
    shot("08-summary")

    print("== Summary -> Install (this actually runs the install job) ==")
    click_next()

    print("== Waiting for install job queue to finish (up to 10 min) ==")
    # No exact log string is asserted here (not verified against every
    # Calamares version) -- instead wait for log growth to go quiet for
    # a sustained period, which is what "the job queue finished and
    # we're sitting on the Finish page" looks like from outside. Prints
    # log tail throughout for a human (or the next iteration of this
    # script) to read the real completion markers off of and tighten
    # this check.
    deadline = time.time() + 600
    quiet_since = None
    last_count = g.log_line_count(log_path)
    while time.time() < deadline:
        time.sleep(10)
        count = g.log_line_count(log_path)
        if count > last_count:
            print(f"  log grew: {last_count} -> {count} lines")
            new_text = g.tail_log(log_path, last_count)
            print("  " + new_text.strip().replace("\n", "\n  ")[-500:])
            last_count = count
            quiet_since = None
        else:
            quiet_since = quiet_since or time.time()
            if time.time() - quiet_since > 60:
                print("  log quiet for 60s, assuming install finished")
                break
    shot("09-finish-or-timeout")
    print(f"== Done. Review screenshots in {args.screenshot_dir} and "
          f"full log via: g.tail_log('{log_path}') ==")


if __name__ == "__main__":
    main()
