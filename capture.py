"""Renode capture sequence for Minesweeper (not yet run).

Run from xous-dev-toolkit:
    python3 scripts/renode_capture.py --app mines --script ../precursor-mines/capture.py

Contract: run(ctl, outdir) -> number of failed steps. The app is already launched.
"""
import os


def run(ctl, outdir) -> int:
    failures = 0

    def shot(name):
        nonlocal failures
        if not ctl.try_screenshot(os.path.join(outdir, name)):
            failures += 1

    shot("01_menu.png")
    ctl.inject_line("")                 # Enter: Beginner
    ctl.sleep(1.0)
    shot("02_board.png")
    ctl.inject_line("")                 # first dig (always safe)
    ctl.sleep(1.5)
    shot("03_first_dig.png")
    ctl.timed_key("Right", after=0.5)
    ctl.timed_key("Down", after=0.5)
    ctl.inject_key("f")                 # flag
    ctl.sleep(1.0)
    shot("04_flag.png")
    ctl.timed_key("BackSpace", after=1.0)   # asks to abandon
    shot("05_confirm.png")
    ctl.timed_key("BackSpace", after=1.0)   # back to the menu
    ctl.inject_key("h")
    ctl.sleep(1.0)
    shot("06_best_times.png")
    return failures
