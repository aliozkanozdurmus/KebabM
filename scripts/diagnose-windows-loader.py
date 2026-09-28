"""Read-only import/export diagnostics for failed native CI test executables."""
import ctypes
from ctypes import wintypes
from pathlib import Path
import sys
import pefile

kernel = ctypes.WinDLL("kernel32", use_last_error=True)
kernel.SetErrorMode(0x0001 | 0x0002)
kernel.LoadLibraryExW.argtypes = [wintypes.LPCWSTR, wintypes.HANDLE, wintypes.DWORD]
kernel.LoadLibraryExW.restype = wintypes.HMODULE
kernel.GetProcAddress.argtypes = [wintypes.HMODULE, ctypes.c_char_p]
kernel.GetProcAddress.restype = ctypes.c_void_p
kernel.GetModuleFileNameW.argtypes = [wintypes.HMODULE, wintypes.LPWSTR, wintypes.DWORD]

for executable in Path(sys.argv[1]).rglob("nexq_lib-*.exe"):
    print(f"Executable: {executable}", flush=True)
    seen = set()
    def inspect(path):
        path = Path(path).resolve()
        if str(path).lower() in seen:
            return
        seen.add(str(path).lower())
        pe = pefile.PE(str(path), fast_load=True)
        pe.parse_data_directories(directories=[1])
        for entry in getattr(pe, "DIRECTORY_ENTRY_IMPORT", []):
            name = entry.dll.decode()
            local = executable.parent / name
            library = str(local.resolve()) if local.exists() else name
            handle = kernel.LoadLibraryExW(library, None, 0)
            if not handle:
                print(f"LOAD FAILED: {name}: Windows error {ctypes.get_last_error()}", flush=True)
                handle = kernel.LoadLibraryExW(library, None, 1)
            if not handle:
                continue
            buffer = ctypes.create_unicode_buffer(32768)
            kernel.GetModuleFileNameW(handle, buffer, len(buffer))
            for item in entry.imports:
                symbol = item.name or ctypes.cast(ctypes.c_void_p(item.ordinal), ctypes.c_char_p)
                if not kernel.GetProcAddress(handle, symbol):
                    print(f"MISSING: {path.name} -> {buffer.value} :: {item.name or item.ordinal}", flush=True)
            inspect(buffer.value)
        pe.close()
    inspect(executable)
