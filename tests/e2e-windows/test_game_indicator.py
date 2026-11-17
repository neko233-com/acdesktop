import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import tkinter


if sys.platform != "win32":
    raise SystemExit("This desktop test requires Windows")

binary = Path(sys.argv[1]).resolve()
lease_value = os.environ.get("ACDESKTOP_INTERACTION_LEASE_HANDLE")
lease_handle = int(lease_value) if lease_value else None
if lease_handle is not None:
    os.set_handle_inheritable(lease_handle, False)

user32 = ctypes.WinDLL("user32", use_last_error=True)
gdi32 = ctypes.WinDLL("gdi32", use_last_error=True)
user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))
user32.GetAncestor.argtypes = [wintypes.HWND, wintypes.UINT]
user32.GetAncestor.restype = wintypes.HWND
user32.ClientToScreen.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.POINT)]
user32.GetDC.argtypes = [wintypes.HWND]
user32.GetDC.restype = wintypes.HDC
user32.ReleaseDC.argtypes = [wintypes.HWND, wintypes.HDC]
user32.GetDpiForSystem.restype = wintypes.UINT
gdi32.GetPixel.argtypes = [wintypes.HDC, ctypes.c_int, ctypes.c_int]
gdi32.GetPixel.restype = wintypes.DWORD
gdi32.CreateCompatibleDC.argtypes = [wintypes.HDC]
gdi32.CreateCompatibleDC.restype = wintypes.HDC
gdi32.CreateCompatibleBitmap.argtypes = [wintypes.HDC, ctypes.c_int, ctypes.c_int]
gdi32.CreateCompatibleBitmap.restype = wintypes.HBITMAP
gdi32.SelectObject.argtypes = [wintypes.HDC, wintypes.HGDIOBJ]
gdi32.SelectObject.restype = wintypes.HGDIOBJ
gdi32.BitBlt.argtypes = [wintypes.HDC, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, wintypes.HDC, ctypes.c_int, ctypes.c_int, wintypes.DWORD]
gdi32.DeleteObject.argtypes = [wintypes.HGDIOBJ]
gdi32.DeleteDC.argtypes = [wintypes.HDC]


class BitmapInfoHeader(ctypes.Structure):
    _fields_ = [("size", wintypes.DWORD), ("width", wintypes.LONG), ("height", wintypes.LONG),
                ("planes", wintypes.WORD), ("bits", wintypes.WORD), ("compression", wintypes.DWORD),
                ("image_size", wintypes.DWORD), ("xppm", wintypes.LONG), ("yppm", wintypes.LONG),
                ("used", wintypes.DWORD), ("important", wintypes.DWORD)]


class BitmapInfo(ctypes.Structure):
    _fields_ = [("header", BitmapInfoHeader), ("colors", wintypes.DWORD * 3)]


gdi32.GetDIBits.argtypes = [wintypes.HDC, wintypes.HBITMAP, wintypes.UINT, wintypes.UINT,
                            ctypes.c_void_p, ctypes.POINTER(BitmapInfo), wintypes.UINT]


def pixel(x, y):
    dc = user32.GetDC(None)
    if not dc:
        raise OSError("Could not capture desktop pixel")
    try:
        return gdi32.GetPixel(dc, x, y)
    finally:
        user32.ReleaseDC(None, dc)


def region(x, y, width, height):
    screen = user32.GetDC(None)
    memory = gdi32.CreateCompatibleDC(screen)
    bitmap = gdi32.CreateCompatibleBitmap(screen, width, height)
    if not screen or not memory or not bitmap:
        raise OSError("Could not allocate a desktop capture")
    old = gdi32.SelectObject(memory, bitmap)
    try:
        if not gdi32.BitBlt(memory, 0, 0, width, height, screen, x, y, 0x40CC0020):
            raise OSError("Could not capture the status label")
        gdi32.SelectObject(memory, old)
        info = BitmapInfo()
        info.header.size = ctypes.sizeof(BitmapInfoHeader)
        info.header.width = width
        info.header.height = -height
        info.header.planes = 1
        info.header.bits = 32
        data = ctypes.create_string_buffer(width * height * 4)
        if gdi32.GetDIBits(memory, bitmap, 0, height, data, ctypes.byref(info), 0) != height:
            raise OSError("Could not read the status label")
        return data.raw
    finally:
        gdi32.SelectObject(memory, old)
        gdi32.DeleteObject(bitmap)
        gdi32.DeleteDC(memory)
        user32.ReleaseDC(None, screen)


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


with tempfile.TemporaryDirectory(prefix="acdesktop-game-indicator-") as directory:
    base = Path(directory)
    os.environ["ACDESKTOP_HOME"] = str(base / "state")
    os.environ.pop("ACDESKTOP_SESSION", None)
    root = tkinter.Tk()
    root.title("acdesktop game indicator fixture")
    root.geometry("500x350+100+150")
    received = []
    root.bind_all("<KeyPress-space>", lambda _event: received.append("space"))
    root.bind_all("<ButtonPress-1>", lambda _event: received.append("click"))
    root.update()
    root.focus_force()
    root.update()
    window = user32.GetAncestor(root.winfo_id(), 2)
    if not window:
        raise RuntimeError("Could not determine fixture window")
    origin = wintypes.POINT(0, 0)
    if not user32.ClientToScreen(window, ctypes.byref(origin)):
        raise OSError("Could not determine fixture client origin")
    pulse_points = [(origin.x + 150 + dx, origin.y + 100 + dy) for dx, dy in
                    [(-16, 0), (-12, 0), (10, 0), (20, 0), (0, 16), (0, -16)]]
    pulse_baseline = [pixel(x, y) for x, y in pulse_points]
    edge_x = min(600, user32.GetSystemMetrics(0) - 100)
    baseline = pixel(edge_x, 1)
    scale = user32.GetDpiForSystem() / 96
    center = user32.GetSystemMetrics(0) // 2
    label_rect = (center - round(160 * scale), round(84 * scale), round(320 * scale), round(27 * scale))
    keys_baseline = None
    plan = base / "plan.json"
    plan.write_text(json.dumps({
        "duration_ms": 3000,
        "fps": 10,
        "events": [
            {"at_ms": 700, "action": "key", "key": "space"},
            {"at_ms": 1500, "action": "mouse-click", "x": 150, "y": 100},
        ],
    }), encoding="utf-8")
    process = launch([
        str(binary), "--desc", "Game feedback test", "game", "--window-id", f"w-{window}",
        "--plan", str(plan), "--out", str(base / "out"), "--headed",
    ])
    observed_edge = False
    observed_pulse = False
    observed_key_label = False
    deadline = time.monotonic() + 30
    try:
        while process.poll() is None and time.monotonic() < deadline:
            root.update()
            observed_edge |= pixel(edge_x, 1) != baseline
            if observed_edge and not received and keys_baseline is None:
                keys_baseline = region(*label_rect)
            if "space" in received and keys_baseline is not None and not observed_key_label:
                observed_key_label = region(*label_rect) != keys_baseline
            if "click" in received:
                observed_pulse |= any(pixel(x, y) != before for (x, y), before in zip(pulse_points, pulse_baseline))
            time.sleep(0.02)
        if process.poll() is None:
            process.kill()
            raise TimeoutError("Game session did not finish")
        stdout, stderr = process.communicate()
        result = json.loads(stdout)
        if process.returncode or not result.get("ok"):
            raise RuntimeError(f"Game session failed: {result}; stderr={stderr.decode(errors='replace')}")
        print("Game input timing:", json.dumps(result["data"]["summary"]["input_metrics"]))
        if not observed_edge:
            raise AssertionError("The screen-edge indicator was not observed")
        if not observed_pulse:
            raise AssertionError(f"The game click did not produce a visible effect (key_label={observed_key_label}, baseline={keys_baseline is not None}, received={received})")
        if not observed_key_label:
            raise AssertionError("The game key did not produce a visible status label")
        if "space" not in received or "click" not in received:
            raise AssertionError(f"Fixture did not receive both input events: {received}")
        edge_deadline = time.monotonic() + 5
        while pixel(edge_x, 1) != baseline and time.monotonic() < edge_deadline:
            root.update()
            time.sleep(0.02)
        if pixel(edge_x, 1) != baseline:
            raise AssertionError("The screen-edge indicator did not disappear")
        observation = base / "observe.json"
        observation.write_text('{"duration_ms":1000,"fps":10,"events":[]}', encoding="utf-8")
        process = launch([
            str(binary), "game", "--window-id", f"w-{window}", "--plan", str(observation),
            "--out", str(base / "observation"),
        ])
        deadline = time.monotonic() + 30
        while process.poll() is None and time.monotonic() < deadline:
            root.update()
            if pixel(edge_x, 1) != baseline:
                raise AssertionError("An observation-only session showed a control indicator")
            time.sleep(0.02)
        if process.poll() is None:
            process.kill()
            raise TimeoutError("Observation session did not finish")
        stdout, stderr = process.communicate()
        observed = json.loads(stdout)
        if process.returncode or not observed.get("ok"):
            raise RuntimeError(f"Observation session failed: {observed}; stderr={stderr.decode(errors='replace')}")
        print("Game indicator and real keyboard/mouse input passed")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        root.destroy()
