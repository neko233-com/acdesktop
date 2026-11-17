import ctypes
from ctypes import wintypes
import csv
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid


if sys.platform != "win32":
    raise SystemExit("This desktop test requires Windows")

binary = Path(sys.argv[1]).resolve()
lease_value = os.environ.get("ACDESKTOP_INTERACTION_LEASE_HANDLE")
lease_handle = int(lease_value) if lease_value else None
if lease_handle is not None:
    os.set_handle_inheritable(lease_handle, False)

user32 = ctypes.WinDLL("user32", use_last_error=True)
callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
user32.EnumWindows.argtypes = [callback_type, wintypes.LPARAM]
user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
user32.IsWindowVisible.argtypes = [wintypes.HWND]
user32.IsWindow.argtypes = [wintypes.HWND]
user32.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]


def find_window(name):
    found = []

    @callback_type
    def visit(hwnd, _parameter):
        title = ctypes.create_unicode_buffer(1024)
        user32.GetWindowTextW(hwnd, title, len(title))
        if user32.IsWindowVisible(hwnd) and name in title.value:
            found.append(hwnd)
        return True

    user32.EnumWindows(visit, 0)
    if len(found) > 1:
        raise RuntimeError("The fixture window title is ambiguous")
    return found[0] if found else None


def launch(command):
    options = {"stdout": subprocess.PIPE, "stderr": subprocess.PIPE}
    if lease_handle is not None:
        startup = subprocess.STARTUPINFO()
        startup.lpAttributeList = {"handle_list": [lease_handle]}
        options.update(startupinfo=startup, close_fds=True)
        os.set_handle_inheritable(lease_handle, True)
        try:
            return subprocess.Popen(command, **options)
        finally:
            os.set_handle_inheritable(lease_handle, False)
    return subprocess.Popen(command, **options)


def existing_notepad_processes():
    result = subprocess.run(
        ["tasklist.exe", "/FI", "IMAGENAME eq notepad.exe", "/FO", "CSV", "/NH"],
        check=True, capture_output=True, text=True, errors="replace",
    )
    return {int(row[1]) for row in csv.reader(io.StringIO(result.stdout))
            if len(row) > 1 and row[0].lower() == "notepad.exe" and row[1].isdigit()}


with tempfile.TemporaryDirectory(prefix="acdesktop-game-notepad-") as directory:
    base = Path(directory)
    os.environ["ACDESKTOP_HOME"] = str(base / "state")
    os.environ.pop("ACDESKTOP_SESSION", None)
    name = f"acdesktop-game-{uuid.uuid4().hex}.txt"
    document = base / name
    document.write_text("game-input", encoding="utf-8")
    if existing_notepad_processes():
        raise RuntimeError("An existing Notepad process was found; refusing to disturb its windows")
    notepad = subprocess.Popen(["notepad.exe", str(document)], close_fds=True)
    hwnd = None
    owns_window = False
    deadline = time.monotonic() + 15
    try:
        while hwnd is None and time.monotonic() < deadline:
            hwnd = find_window(name)
            if hwnd is None:
                time.sleep(0.05)
        if hwnd is None:
            raise TimeoutError("Notepad did not open the fixture document")
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        owns_window = owner.value in existing_notepad_processes()
        if not owns_window:
            raise RuntimeError("Notepad reused another process; refusing to control or close its window")
        plan = base / "plan.json"
        plan.write_text(json.dumps({
            "duration_ms": 3000,
            "fps": 10,
            "events": [
                {"at_ms": 500, "action": "key", "key": "space"},
                {"at_ms": 1000, "action": "key-down", "key": "ctrl"},
                {"at_ms": 1100, "action": "key", "key": "s"},
                {"at_ms": 1200, "action": "key-up", "key": "ctrl"},
            ],
        }), encoding="utf-8")
        process = launch([
            str(binary), "game", "--window-id", f"w-{hwnd}", "--plan", str(plan),
            "--out", str(base / "out"), "--headed",
        ])
        try:
            stdout, stderr = process.communicate(timeout=30)
        except subprocess.TimeoutExpired:
            process.kill()
            process.communicate()
            raise
        result = json.loads(stdout)
        if process.returncode or not result.get("ok"):
            raise RuntimeError(f"Game session failed: {result}; stderr={stderr.decode(errors='replace')}")
        deadline = time.monotonic() + 5
        while document.read_text(encoding="utf-8-sig") == "game-input" and time.monotonic() < deadline:
            time.sleep(0.05)
        content = document.read_text(encoding="utf-8-sig")
        if content.replace(" ", "") != "game-input" or content.count(" ") != 1:
            raise AssertionError(f"Notepad did not save the expected input: {content!r}")
        print("Game input was independently observed in Notepad's saved document")
    finally:
        if owns_window and hwnd and user32.IsWindow(hwnd):
            user32.PostMessageW(hwnd, 0x0010, 0, 0)
            deadline = time.monotonic() + 5
            while user32.IsWindow(hwnd) and time.monotonic() < deadline:
                time.sleep(0.05)
