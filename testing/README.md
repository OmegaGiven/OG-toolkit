# og-os click-through testing tool

Automated GUI testing for the og-os live session (Calamares, or any
other graphical app running under sway in the live ISO), built after
several sessions of fighting QEMU-monitor synthetic input (sendkey/
mouse_move over VNC) proved too fragile to trust — coordinate systems,
focus routing, and absolute-vs-relative pointer semantics didn't line
up reliably, and failures were mostly unexplainable from screenshots
alone.

## What this is

- `boot-vm.sh` — boots an og-os ISO in qemu with standardized VNC,
  monitor, and SSH-forwarding ports for a test run.
- `gui_test.py` — the reusable library: SSH into the guest, install
  `ydotool` there (kernel-level input injection via `/dev/uinput` —
  looks like real hardware to sway, unlike QEMU-monitor synthetic
  input), and drive it. Also wraps `swaymsg -t get_tree` for real
  window geometry/focus (ground truth, not a guess from a screenshot)
  and screenshot capture via the qemu monitor for human review.
- `test_calamares_install.py` — a concrete test script that logs in
  and attempts to click through Calamares' full install flow.

## What's actually proven solid

- **SSH + ydotool harness**: reliable. Boots, installs ydotool fresh
  each run (it's a testing-session dependency, never baked into the
  production ISO), logs in at the greeter by typing the password —
  this is real, verified UI state change (confirmed by watching a
  wrong password produce "Your password is incorrect" on screen, then
  a real password land in a working sway session).
- **`swaymsg` ground truth**: reliable. `find_window()` returns exact
  window rect + focus state — this is how the "Next button is always
  at a fixed fraction of the window's bottom-right corner" positioning
  in `calamares_next_button()` was derived and confirmed (cursor
  visibly lands exactly on the button in every test run).
- **The coordinate bug, found and fixed**: ydotool's virtual absolute
  pointer device declares a logical resolution that is **half** the
  real screen resolution (e.g. 640x400 on a 1280x800 screen), not the
  real pixel dimensions. Every target coordinate must be halved before
  sending, or the cursor lands at ~2x the intended position. Confirmed
  by watching the actual rendered cursor land at ~2x requested
  coordinates, deriving the 0.5 scale factor, and then verifying the
  cursor visually overlapping the intended button before trusting a
  click. This fix is baked into `move()`/`click()` — callers always
  work in real screen pixels.
- **Window focus and mouse-move-only interactions**: reliable. Clicking
  inside a different window (e.g. og-notif-center's search box) does
  shift sway's window focus correctly and consistently.

## What's NOT solid yet — the open problem

**Clicking an actual button inside Calamares' Qt window does not
reliably register**, even with:
- confirmed-correct coordinates (cursor visibly sits exactly on the
  "Next" button in the screenshot taken immediately after clicking)
- confirmed window focus (via `swaymsg`, the Calamares window shows
  `focused: true`)
- a single SSH round-trip for move+click (ruled out cross-connection
  timing jitter as the cause)
- a 0.4s settle delay between the move and the click event

One click did succeed once, interactively, with a human-paced pause
(move → screenshot → visually confirm → click, each a separate manual
step). No combination tried since — including `click_verified()`'s
tight move-then-click — has reproduced that reliably in a scripted run.
Symptoms observed: the page never advances (confirmed by screenshot),
and typed text intended for a later page's form fields sometimes lands
in whatever widget silently still has focus on the *current* page
(e.g. a language combobox's type-ahead-select, which switched the
whole UI to Turkish mid-test — a real, funny, and useful bug report in
its own right about how easy it is to fat-finger a live install if a
Next click is ever silently dropped for a real user too).

**Leading theories, not yet confirmed:**
1. Qt6's wayland platform plugin may have specific requirements around
   pointer *enter* events before accepting a button press on a
   just-hovered widget — a teleporting absolute-position jump (no
   intermediate motion samples) might not satisfy that, unlike a real
   mouse dragging across the surface.
2. ydotool's click may need to be split into explicit down/up events
   with real time between them (`ydotool click` bundles both in one
   call) rather than relying on its own internal timing.
3. Something specific to running Calamares as root (via `sudo env ...`)
   while the wayland compositor session is owned by `liveuser` — this
   hasn't broken window rendering, focus, or keyboard input, but maybe
   affects pointer button event delivery specifically in a way not yet
   understood.

## Next steps for whoever picks this up

- Try `ydotool click` as explicit `keydown`/`keyup`-equivalent button
  events with a deliberate 100-200ms hold, instead of the bundled
  `0xC0` shorthand.
- Try sending a few incremental relative mouse-motion events ending at
  the target, instead of one absolute jump, to see if "real motion
  before the click" is what Qt/wayland wants.
- Try running Calamares as `liveuser` (no `sudo`) against a
  passwordless-sudo *inside* Calamares' own privilege-escalation
  path instead of pre-escalating the whole process, to rule out
  theory 3.
- Whatever fixes it, add a regression case to `test_calamares_install.py`
  that this README's theories can be checked off against.

## Usage

```sh
./boot-vm.sh                                   # boots newest ISO in ../out/
source /tmp/ogos-vm-XXXXXX/env.sh              # get the printed ports
python3 test_calamares_install.py \
    --ssh-port "$OGOS_SSH_PORT" \
    --monitor-port "$OGOS_MONITOR_PORT" \
    --username testuser --password testpass123 --hostname ogos-test
```

Screenshots land in `/tmp/ogos-test-shots/` by default — review them,
they're the actual ground truth for "what page did we end up on" until
the click-reliability problem above is solved.
