"""The browser leg: the workbench the installed wheel serves, driven as a listener drives it.

`uncompose compare --project` is not a command that finishes — it serves a page
and exits when the listener concludes. So this leg is three things the CLI leg
never needs: which manifest refs to compare, a launch that catches the tokened
URL the session prints, and a Chromium drive that places a pin, engraves a
blind verdict, and concludes.

The seam is unchanged (ADR-0007): the session is the installed command, reached
through the installation's `PATH`, and everything asserted afterwards is read
from the rendered page, the record file, and the manifest.
"""

from __future__ import annotations

import os
import selectors
import signal
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Sequence

from .commands import environment
from .install import Installation, ToolchainMissing

# The prefix every URL Compare prints starts with: the workbench is never
# served off this host (uncompose-compare #72).
LOOPBACK = "http://127.0.0.1:"

# How long the session gets to load both candidates and print its URL. Loading
# decodes and transcodes each candidate, so it is not instant even on the
# half-second fixtures.
URL_TIMEOUT = 60.0

# How long the concluded session gets to write its record, hand it to
# `uncompose-project import`, and exit.
EXIT_TIMEOUT = 60.0

# What the listener does at the workbench. The stem is the one both runs
# produced, so a `<stem>@<derivation>` ref names each run's take on the same
# part; the note is deliberately anonymous, since naming a file in a blind
# session would be the listener leaking to themselves.
CANDIDATE_STEM = "vocals"
PIN_TEXT = "cleaner top end here"
PREFERRED_LABEL = "A"
CONFIDENCE = 4


class WorkbenchError(Exception):
    """The browser leg could not be driven, and why."""


@dataclass(frozen=True)
class Served:
    """A running `uncompose compare` session and the URL it printed."""

    argv: tuple[str, ...]
    process: subprocess.Popen
    url: str
    stderr_path: Path

    def stderr(self) -> str:
        return self.stderr_path.read_text() if self.stderr_path.exists() else ""

    def wait(self, timeout: float = EXIT_TIMEOUT) -> int:
        """Wait for the session to exit itself, as a concluded one does."""
        try:
            return self.process.wait(timeout=timeout)
        except subprocess.TimeoutExpired as expired:
            self.kill()
            raise WorkbenchError(
                f"{self.describe()} was still serving {timeout}s after the conclude"
            ) from expired

    def kill(self) -> None:
        _kill(self.process)

    def describe(self) -> str:
        return _describe(self.argv)


@dataclass(frozen=True)
class BrowserRun:
    """What the driven page showed — the listener's own view of the session."""

    url: str
    pin_text: str
    preferred_label: str
    confidence: int
    record_path: Path
    reveal: Mapping[str, str]
    registered: bool
    before_conclude: str
    after_conclude: str

    def describe(self) -> str:
        return (
            f"workbench at {self.url}\n"
            f"--- the closing screen ---\n{self.after_conclude}"
        )


@dataclass(frozen=True)
class WorkbenchLeg:
    """One blind session, from launch to the exit that means saved and registered."""

    served: Served
    browser: BrowserRun
    returncode: int
    stderr: str

    def describe(self) -> str:
        return (
            f"{self.served.describe()}\n{self.browser.describe()}\n"
            f"exit {self.returncode}\n--- stderr ---\n{self.stderr}"
        )


@dataclass(frozen=True)
class BlindVerdict:
    """The written record, read as the reconnection between labels and files.

    A blind listener judges `A` and `B`; the manifest records asset ids. The
    record is the only place the two meet, so every claim about what was
    preferred is resolved through it rather than assumed.
    """

    blind: bool
    labels: tuple[str, ...]
    assets: Mapping[str, str]
    paths: Mapping[str, str]
    preferred_label: str | None
    confidence: object | None

    @property
    def preferred_asset(self) -> str | None:
        return None if self.preferred_label is None else self.assets[self.preferred_label]

    @property
    def candidate_assets(self) -> tuple[str, ...]:
        return tuple(self.assets[label] for label in self.labels)


def candidate_refs(manifest: dict, stem: str = CANDIDATE_STEM) -> tuple[str, str]:
    """The two `<stem>@<derivation>` refs naming each run's take on `stem`.

    The refs are the ones the walkthrough teaches a user to type, resolved from
    the manifest rather than hard-coded, so the leg follows whatever ids
    `import` actually minted for the two runs.
    """
    derivations = manifest.get("derivations", [])
    if len(derivations) != 2:
        raise WorkbenchError(
            f"a blind session compares two candidates, so the project needs two "
            f"derivations; the manifest has {len(derivations)}"
        )

    assets = {asset["id"]: asset for asset in manifest["assets"]}
    refs = []
    for derivation in derivations:
        outputs = [assets[asset_id] for asset_id in derivation["outputs"]]
        named = [asset for asset in outputs if Path(asset["path"]).stem == stem]
        if len(named) != 1:
            filenames = ", ".join(Path(asset["path"]).name for asset in outputs)
            raise WorkbenchError(
                f"derivation {derivation['id']!r} has {len(named)} outputs named "
                f"{stem!r}, so it cannot furnish one candidate. Its outputs: {filenames}"
            )
        refs.append(f"{stem}@{derivation['id']}")
    return (refs[0], refs[1])


def workbench_argv(project: str | Path, refs: Sequence[str]) -> list[str]:
    """The command a user types to audition two candidates blind."""
    return ["uncompose", "compare", "--project", str(project), *refs, "--blind"]


def spawn_served(
    argv: Sequence[str],
    *,
    cwd: str | Path,
    env: Mapping[str, str],
    stderr_path: Path,
    timeout: float = URL_TIMEOUT,
) -> Served:
    """Launch a serving command and return it with the loopback URL it printed.

    The CLI contract says the first line of stdout is the tokened URL, so a
    session that prints anything else, dies first, or says nothing is a failed
    launch — never a hang, and never a `Served` the caller has to re-check.
    """
    argv = tuple(str(arg) for arg in argv)
    with open(stderr_path, "wb") as stderr_file:
        process = subprocess.Popen(
            argv,
            cwd=str(cwd),
            env=dict(env),
            stdout=subprocess.PIPE,
            stderr=stderr_file,
            # Its own process group, so a session that outlives the leg is
            # reaped whole rather than left serving in the background.
            start_new_session=True,
        )

    def failed(why: str) -> WorkbenchError:
        _kill(process)
        stderr = stderr_path.read_text() if stderr_path.exists() else ""
        return WorkbenchError(f"{_describe(argv)}\n{why}\n--- stderr ---\n{stderr}")

    try:
        line = _first_line(process.stdout, timeout)
    except WorkbenchError as silent:
        raise failed(str(silent)) from silent
    if not line:
        raise failed(f"exited (code {process.poll()}) before printing a URL")
    if not line.startswith(LOOPBACK):
        raise failed(f"printed {line!r}, which is not a loopback URL")
    return Served(argv=argv, process=process, url=line, stderr_path=stderr_path)


def _describe(argv: Sequence[str]) -> str:
    return f"$ {' '.join(argv)}"


def _first_line(stream, timeout: float) -> str:
    """The first newline-terminated line, or "" at EOF; raises on the deadline."""
    deadline = time.monotonic() + timeout
    selector = selectors.DefaultSelector()
    selector.register(stream, selectors.EVENT_READ)
    buffered = b""
    try:
        while b"\n" not in buffered:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not selector.select(remaining):
                raise WorkbenchError(f"nothing was printed within {timeout}s")
            chunk = os.read(stream.fileno(), 4096)
            if not chunk:
                return ""
            buffered += chunk
    finally:
        selector.close()
    return buffered.split(b"\n", 1)[0].decode(errors="replace").strip()


def _kill(process: subprocess.Popen) -> None:
    """Reap the session and everything it started, whether or not it is still up."""
    try:
        os.killpg(os.getpgid(process.pid), signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()


def read_verdict(record: dict) -> BlindVerdict:
    """Read a comparison record as what the listener decided, in their labels."""
    candidates = record["candidates"]
    labels = tuple(candidate["label"] for candidate in candidates)
    preferred = record.get("result", {}).get("preference")
    if preferred is not None and preferred not in labels:
        raise WorkbenchError(
            f"the record prefers {preferred!r}, which is none of its candidates {labels}"
        )
    return BlindVerdict(
        blind=record["mode"] != "ab",
        labels=labels,
        assets={c["label"]: c.get("asset") for c in candidates},
        paths={c["label"]: c["path"] for c in candidates},
        preferred_label=preferred,
        confidence=record.get("result", {}).get("confidence"),
    )


def drive_blind_session(
    url: str,
    *,
    pin_text: str = PIN_TEXT,
    prefer: str = PREFERRED_LABEL,
    confidence: int = CONFIDENCE,
    timeout_ms: int = 60_000,
) -> BrowserRun:
    """Place a pin, engrave a blind verdict, and conclude — in Chromium.

    Chromium only: the engine-dependent half of the workbench is its audio
    contract, and Compare already runs that across three engines. What this leg
    adds is the flow through the *installed* wheel's page, which is the same in
    every engine.

    The browser is not part of the installation under test, so it runs with the
    machine's own environment; the session it drives is the installed command,
    hermetic as ever.
    """
    with _chromium(timeout_ms) as page:
        page.goto(url, wait_until="load")
        page.get_by_test_id("workbench").wait_for(state="visible")

        # Pin a moment worth marking: seek into the middle of the stage, then
        # write the note and pin it on the live candidate with Enter.
        stage = page.get_by_test_id("stage-waveform")
        box = stage.bounding_box()
        page.mouse.click(box["x"] + box["width"] * 0.5, box["y"] + box["height"] / 2)
        composer = page.get_by_test_id("composer")
        composer.click()
        composer.fill(pin_text)
        composer.press("Enter")

        # Engrave the verdict: a preference and a confidence, saved. Save is the
        # engrave gate — closing the modal any other way discards it.
        page.get_by_test_id("open-verdict").click()
        page.get_by_test_id(f"prefer-{prefer}").click()
        page.get_by_test_id(f"confidence-{confidence}").click()
        page.get_by_test_id("save-verdict").click()
        page.get_by_test_id("verdict-modal").wait_for(state="detached")

        # Everything the listener could see while still blind, captured before
        # the one irreversible event that reveals the identities.
        before_conclude = page.locator("body").inner_text()

        page.get_by_test_id("conclude").click()
        page.get_by_test_id("conclude-path").wait_for(state="visible")
        record_path = page.get_by_test_id("conclude-path").locator("code").inner_text()
        page.get_by_test_id("reveal").wait_for(state="visible")
        reveal = {
            label: page.get_by_test_id(f"reveal-{label}").inner_text()
            for label in ("A", "B")
        }
        return BrowserRun(
            url=url,
            pin_text=pin_text,
            preferred_label=prefer,
            confidence=confidence,
            record_path=Path(record_path.strip()),
            reveal=reveal,
            registered=page.get_by_test_id("registration-ok").count() == 1,
            before_conclude=before_conclude,
            after_conclude=page.locator("body").inner_text(),
        )


class _chromium:
    """A Chromium page, or a stated missing browser — never a stray process."""

    def __init__(self, timeout_ms: int) -> None:
        self.timeout_ms = timeout_ms

    def __enter__(self):
        try:
            from playwright.sync_api import Error as PlaywrightError
            from playwright.sync_api import sync_playwright
        except ImportError as missing:
            raise ToolchainMissing(
                "playwright is not installed; the browser leg needs it "
                "(`uv sync` in acceptance/)"
            ) from missing

        self._playwright = sync_playwright().start()
        try:
            self._browser = self._playwright.chromium.launch()
        except PlaywrightError as unlaunchable:
            self._playwright.stop()
            raise ToolchainMissing(
                f"chromium could not be launched ({unlaunchable}); "
                "install it with `uv run playwright install chromium`"
            ) from unlaunchable
        page = self._browser.new_page()
        page.set_default_timeout(self.timeout_ms)
        return page

    def __exit__(self, *exc_info) -> None:
        self._browser.close()
        self._playwright.stop()


def run_workbench_leg(
    installation: Installation,
    project: Path,
    refs: Sequence[str],
    *,
    driver=drive_blind_session,
) -> WorkbenchLeg:
    """Launch a blind session over `refs`, drive it, and wait for it to close.

    A concluded project session writes its record, hands it to
    `uncompose-project import`, and exits: exit 0 is the tool's own statement
    that the verdict was saved *and* registered, so the leg waits for it rather
    than killing a server it is done with.
    """
    argv = workbench_argv(project, refs)
    served = spawn_served(
        argv,
        cwd=project,
        env=environment(installation),
        stderr_path=installation.home / "compare-stderr.log",
    )
    try:
        browser = driver(served.url)
        returncode = served.wait()
    finally:
        served.kill()
    return WorkbenchLeg(
        served=served,
        browser=browser,
        returncode=returncode,
        stderr=served.stderr(),
    )
