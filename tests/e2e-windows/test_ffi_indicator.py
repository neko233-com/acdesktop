import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import tkinter


LIBRARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path("target/debug/acdesktop_ffi.dll").resolve()
if sys.platform != "win32":
    raise SystemExit("This desktop test requires Windows")

lease_value = os.environ.get("ACDESKTOP_INTERACTION_LEASE_HANDLE")
lease_handle = int(lease_value) if lease_value else None
if lease_handle is not None:
    os.set_handle_inheritable(lease_handle, False)


def guarded_popen(command, **kwargs):
    if lease_handle is None:
        return subprocess.Popen(command, **kwargs)
    startup = subprocess.STARTUPINFO()
    startup.lpAttributeList = {"handle_list": [lease_handle]}
    os.set_handle_inheritable(lease_handle, True)
    try:
        return subprocess.Popen(command, startupinfo=startup, close_fds=True, **kwargs)
    finally:
        os.set_handle_inheritable(lease_handle, False)


def guarded_run(command, check=False, capture_output=False, **kwargs):
    if capture_output:
        kwargs.update(stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    with guarded_popen(command, **kwargs) as process:
        stdout, stderr = process.communicate()
        result = subprocess.CompletedProcess(command, process.returncode, stdout, stderr)
    if check:
        result.check_returncode()
    return result


user32 = ctypes.WinDLL("user32", use_last_error=True)
gdi32 = ctypes.WinDLL("gdi32", use_last_error=True)
user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))
user32.GetDpiForSystem.restype = wintypes.UINT
user32.GetSystemMetrics.argtypes = [ctypes.c_int]
user32.GetDC.argtypes = [wintypes.HWND]
user32.GetDC.restype = wintypes.HDC
user32.ReleaseDC.argtypes = [wintypes.HWND, wintypes.HDC]
gdi32.CreateCompatibleDC.argtypes = [wintypes.HDC]
gdi32.CreateCompatibleDC.restype = wintypes.HDC
gdi32.CreateCompatibleBitmap.argtypes = [wintypes.HDC, ctypes.c_int, ctypes.c_int]
gdi32.CreateCompatibleBitmap.restype = wintypes.HBITMAP
gdi32.SelectObject.argtypes = [wintypes.HDC, wintypes.HGDIOBJ]
gdi32.SelectObject.restype = wintypes.HGDIOBJ
gdi32.BitBlt.argtypes = [wintypes.HDC, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, wintypes.HDC, ctypes.c_int, ctypes.c_int, wintypes.DWORD]
gdi32.GetPixel.argtypes = [wintypes.HDC, ctypes.c_int, ctypes.c_int]
gdi32.GetPixel.restype = wintypes.DWORD
gdi32.DeleteObject.argtypes = [wintypes.HGDIOBJ]
gdi32.DeleteDC.argtypes = [wintypes.HDC]


class Point(ctypes.Structure):
    _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]


class MouseEvent(ctypes.Structure):
    _fields_ = [("kind", ctypes.c_int32), ("point", Point), ("button", ctypes.c_int32), ("click_count", ctypes.c_uint32)]


class ScreenPoint(ctypes.Structure):
    _fields_ = [("x", ctypes.c_long), ("y", ctypes.c_long)]


user32.WindowFromPoint.argtypes = [ScreenPoint]
user32.WindowFromPoint.restype = wintypes.HWND
user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel32.OpenProcess.restype = wintypes.HANDLE
kernel32.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
kernel32.WaitForSingleObject.restype = wintypes.DWORD
kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
kernel32.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
kernel32.CreateFileW.restype = wintypes.HANDLE


def pixel(x, y):
    screen = user32.GetDC(None)
    memory = gdi32.CreateCompatibleDC(screen)
    bitmap = gdi32.CreateCompatibleBitmap(screen, 1, 1)
    if not screen or not memory or not bitmap:
        raise OSError("Could not capture the desktop")
    old = gdi32.SelectObject(memory, bitmap)
    try:
        if not gdi32.BitBlt(memory, 0, 0, 1, 1, screen, x, y, 0x40CC0020):
            raise OSError("Could not copy the desktop pixel")
        return gdi32.GetPixel(memory, 0, 0)
    finally:
        gdi32.SelectObject(memory, old)
        gdi32.DeleteObject(bitmap)
        gdi32.DeleteDC(memory)
        user32.ReleaseDC(None, screen)


def wait_for(predicate, description):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.02)
    raise AssertionError(description)


def helper_is_released(path):
    handle = kernel32.CreateFileW(str(path), 0x40000000, 0, None, 3, 0x80, None)
    if handle == ctypes.c_void_p(-1).value:
        return False
    kernel32.CloseHandle(handle)
    return True


def load(path):
    lib = ctypes.CDLL(str(path))
    lib.ad_adapter_create.restype = ctypes.c_void_p
    lib.ad_adapter_create_with_session.argtypes = [ctypes.c_char_p]
    lib.ad_adapter_create_with_session.restype = ctypes.c_void_p
    lib.ad_adapter_destroy.argtypes = [ctypes.c_void_p]
    lib.ad_close_app.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_bool]
    lib.ad_close_app.restype = ctypes.c_int
    lib.ad_last_error_message.restype = ctypes.c_char_p
    lib.ad_mouse_event.argtypes = [ctypes.c_void_p, ctypes.POINTER(MouseEvent)]
    lib.ad_mouse_event.restype = ctypes.c_int
    return lib


def click_own_fixture(lib):
    root = tkinter.Tk()
    root.title("acdesktop FFI indicator test")
    root.geometry("400x240+40+180")
    root.configure(background="white")
    root.attributes("-topmost", True)
    root.update()
    adapter = lib.ad_adapter_create()
    try:
        x = root.winfo_rootx() + root.winfo_width() // 2
        y = root.winfo_rooty() + root.winfo_height() // 2
        pid = wintypes.DWORD()
        user32.GetWindowThreadProcessId(user32.WindowFromPoint(ScreenPoint(x, y)), ctypes.byref(pid))
        if pid.value != os.getpid():
            raise AssertionError("Fixture is not the visible window at the click location")
        points = [(x + dx, y + dy) for dx in range(-30, 31, 6) for dy in range(-30, 31, 6)]
        baseline = [pixel(px, py) for px, py in points]
        result = lib.ad_mouse_event(adapter, ctypes.byref(MouseEvent(3, Point(x, y), 0, 1)))
        if result != 0:
            raise AssertionError(f"FFI click failed: {result}, {lib.ad_last_error_message()!r}")
        deadline = time.monotonic() + 0.6
        while time.monotonic() < deadline:
            changed = sum(pixel(px, py) != before for (px, py), before in zip(points, baseline))
            if changed > 5:
                return
        raise AssertionError("The FFI click did not draw a visible animation")
    finally:
        if adapter:
            lib.ad_adapter_destroy(adapter)
        root.destroy()


def crash_host_while_launching(scratch, x, y, label_color):
    source = scratch / "host-fixture.rs"
    exe = scratch / "host-fixture.exe"
    pidfile = scratch / "host-fixture.pid"
    completed = scratch / "host-call-completed"
    source.write_text('#![windows_subsystem = "windows"]\nfn main() { if let Ok(path) = std::env::var("ACDESKTOP_FFI_FIXTURE_PID") { let _ = std::fs::write(path, std::process::id().to_string()); } std::thread::sleep(std::time::Duration::from_secs(5)); }\n', encoding="utf-8")
    build_env = os.environ.copy()
    build_env.pop("ACDESKTOP_INTERACTION_LEASE_HANDLE", None)
    subprocess.run(["rustc", str(source), "-o", str(exe)], check=True, env=build_env)
    script = "import ctypes,sys,pathlib; d=ctypes.CDLL(sys.argv[1]); d.ad_adapter_create.restype=ctypes.c_void_p; d.ad_launch_app_exact.argtypes=[ctypes.c_void_p,ctypes.c_char_p,ctypes.c_uint64,ctypes.c_void_p]; a=d.ad_adapter_create(); out=(ctypes.c_uint64*128)(); result=d.ad_launch_app_exact(a,sys.argv[2].encode(),10000,out); pathlib.Path(sys.argv[3]).write_text(str(result)); __import__('time').sleep(10)"
    env = os.environ.copy()
    env["ACDESKTOP_FFI_FIXTURE_PID"] = str(pidfile)
    host = guarded_popen([sys.executable, "-c", script, str(LIBRARY), str(exe), str(completed)], env=env)
    fixture_handle = None
    try:
        wait_for(lambda: pidfile.exists() and pixel(x, y) == label_color, "The hosted FFI call did not start")
        if completed.exists() or host.poll() is not None:
            raise AssertionError("The hosted FFI call finished before its host was terminated")
        fixture_handle = kernel32.OpenProcess(0x100000, False, int(pidfile.read_text()))
        if not fixture_handle:
            raise OSError("Could not retain the fixture process handle")
        host.kill()
        host.wait(timeout=5)
        wait_for(lambda: pixel(x, y) != label_color, "The indicator survived its host process")
    finally:
        if host.poll() is None:
            host.kill()
            host.wait(timeout=5)
        if not fixture_handle and pidfile.exists():
            fixture_handle = kernel32.OpenProcess(0x100000, False, int(pidfile.read_text()))
        if fixture_handle:
            try:
                if kernel32.WaitForSingleObject(fixture_handle, 10000) != 0:
                    raise AssertionError("The short-lived fixture did not exit")
            finally:
                kernel32.CloseHandle(fixture_handle)


def exercise_multiple_library_copies(scratch, x, y, label_color):
    libraries = []
    for name in ("library-a", "library-b"):
        directory = scratch / name
        directory.mkdir()
        library = directory / LIBRARY.name
        shutil.copyfile(LIBRARY, library)
        shutil.copyfile(LIBRARY.parent / "acdesktop.exe", directory / "acdesktop.exe")
        libraries.append(library)
    script = """
import ctypes, sys, threading
libraries = [ctypes.CDLL(path) for path in sys.argv[1:]]
barrier = threading.Barrier(3)
results = [None, None]
def run(index, lib):
    lib.ad_adapter_create.restype = ctypes.c_void_p
    lib.ad_adapter_destroy.argtypes = [ctypes.c_void_p]
    lib.ad_close_app.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_bool]
    lib.ad_close_app.restype = ctypes.c_int
    adapter = lib.ad_adapter_create()
    barrier.wait()
    results[index] = lib.ad_close_app(adapter, b'acdesktop-indicator-no-such-app.exe', False) if adapter else 0
    if adapter:
        lib.ad_adapter_destroy(adapter)
threads = [threading.Thread(target=run, args=(index, lib)) for index, lib in enumerate(libraries)]
for thread in threads: thread.start()
barrier.wait()
for thread in threads: thread.join()
print(results)
sys.exit(0 if results == [-3, -3] else 1)
"""
    result = guarded_run([sys.executable, "-c", script, *(str(path) for path in libraries)], capture_output=True, text=True, env=os.environ.copy())
    wait_for(lambda: all(helper_is_released(path.parent / "acdesktop.exe") for path in libraries), "A concurrent indicator process did not exit")
    wait_for(lambda: pixel(x, y) != label_color, "Concurrent library indicators did not disappear")
    if result.returncode:
        raise AssertionError(f"Concurrent library copies failed: {result.stdout} {result.stderr}")


def close_missing_app(lib):
    adapter = lib.ad_adapter_create()
    if not adapter:
        raise AssertionError(f"adapter creation failed: {lib.ad_last_error_message()!r}")
    try:
        return lib.ad_close_app(adapter, b"acdesktop-indicator-no-such-app.exe", False)
    finally:
        lib.ad_adapter_destroy(adapter)


def exercise_session(lib, x, y, label_color):
    helper = LIBRARY.parent / "acdesktop.exe"
    start = guarded_run([str(helper), "--desc", "FFI session", "session", "start"], check=True, capture_output=True, text=True)
    envelope = json.loads(start.stdout)
    if not envelope["ok"] or not envelope["data"]["control_indicator"]["rendered"]:
        raise AssertionError(f"Could not start the control session: {envelope}")
    session = envelope["data"]["session_id"]
    adapter = None
    try:
        wait_for(lambda: pixel(x, y) == label_color, "The session indicator was not visible")
        adapter = lib.ad_adapter_create_with_session(session.encode())
        if not adapter:
            raise AssertionError(f"Could not create the session adapter: {lib.ad_last_error_message()!r}")
        if lib.ad_close_app(adapter, b"acdesktop-indicator-no-such-app.exe", False) != -3:
            raise AssertionError("The session FFI action did not reach its expected result")
        lib.ad_adapter_destroy(adapter)
        adapter = None
        until = time.monotonic() + 1.2
        while time.monotonic() < until:
            if pixel(x, y) != label_color:
                raise AssertionError("The active session indicator disappeared after an FFI call")
            time.sleep(0.02)
    finally:
        if adapter:
            lib.ad_adapter_destroy(adapter)
        end = guarded_run([str(helper), "session", "end", session], capture_output=True, text=True)
        if end.returncode:
            raise AssertionError(f"Could not end the control session: {end.stdout} {end.stderr}")
    wait_for(lambda: pixel(x, y) != label_color, "The ended session indicator remained visible")


with tempfile.TemporaryDirectory(prefix="acdesktop-ffi-indicator-") as scratch:
    os.environ["ACDESKTOP_HOME"] = str(Path(scratch) / "state")
    os.environ["ACDESKTOP_NO_AUTO_UPDATE"] = "1"
    scale = user32.GetDpiForSystem() / 96
    width = user32.GetSystemMetrics(0)
    x = int(width / 2 + min(200 * scale, width / 4))
    y = int(48 * scale)
    before = pixel(x, y)
    label_color = 0x291C17
    if before == label_color:
        raise AssertionError("The desktop oracle already has the indicator color")
    lib = load(LIBRARY)
    result = close_missing_app(lib)
    if result != -3:
        raise AssertionError(f"Expected app-not-found after showing the indicator, got {result}")
    wait_for(lambda: pixel(x, y) == label_color, "The FFI control indicator was not visible")
    wait_for(lambda: pixel(x, y) != label_color, "The FFI control indicator did not disappear")
    exercise_session(lib, x, y, label_color)
    click_own_fixture(lib)
    wait_for(lambda: pixel(x, y) != label_color, "The click indicator did not disappear")
    crash_host_while_launching(Path(scratch), x, y, label_color)
    exercise_multiple_library_copies(Path(scratch), x, y, label_color)
    isolated = Path(scratch) / "without-helper"
    isolated.mkdir()
    isolated_lib = isolated / LIBRARY.name
    shutil.copyfile(LIBRARY, isolated_lib)
    script = "import ctypes,sys; d=ctypes.CDLL(sys.argv[1]); d.ad_adapter_create.restype=ctypes.c_void_p; d.ad_close_app.argtypes=[ctypes.c_void_p,ctypes.c_char_p,ctypes.c_bool]; d.ad_close_app.restype=ctypes.c_int; d.ad_last_error_delivery_semantics.argtypes=[ctypes.c_void_p]; a=d.ad_adapter_create(); result=d.ad_close_app(a,b'no-such-app.exe',False) if a else 0; semantics=(ctypes.c_int32*2)(); d.ad_last_error_delivery_semantics(semantics); sys.exit(0 if result==-12 and semantics[0]==1 else 1)"
    guarded_run([sys.executable, "-c", script, str(isolated_lib)], check=True, env=os.environ.copy())
    named_host = Path(scratch) / "named-host"
    named_host.mkdir()
    host_exe = named_host / "acdesktop.exe"
    host_dll = named_host / LIBRARY.name
    shutil.copyfile(sys.executable, host_exe)
    shutil.copyfile(LIBRARY, host_dll)
    env = os.environ.copy()
    env["PYTHONHOME"] = sys.base_prefix
    env["PATH"] = os.pathsep.join([str(Path(sys.executable).parent), sys.base_prefix, env.get("PATH", "")])
    script = "import ctypes,sys; d=ctypes.CDLL(sys.argv[1]); d.ad_adapter_create.restype=ctypes.c_void_p; d.ad_close_app.argtypes=[ctypes.c_void_p,ctypes.c_char_p,ctypes.c_bool]; d.ad_close_app.restype=ctypes.c_int; d.ad_last_error_message.restype=ctypes.c_char_p; a=d.ad_adapter_create(); result=d.ad_close_app(a,b'no-such-app.exe',False) if a else 0; message=d.ad_last_error_message() or b''; sys.exit(0 if result==-12 and b'host process image' in message else 1)"
    guarded_run([str(host_exe), "-c", script, str(host_dll)], check=True, env=env)
    host_alias = named_host / "host-alias.exe"
    try:
        os.link(host_exe, host_alias)
    except OSError:
        print("Hard-link host identity check skipped: the temporary filesystem does not support it")
    else:
        guarded_run([str(host_alias), "-c", script, str(host_dll)], check=True, env=env)
    print("Windows FFI indicator passed lifecycle, click, host-exit, helper identity, and concurrent-library checks")
