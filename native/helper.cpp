// AppSweep native helper (C++ / Win32)
// Compiled by build.rs through the `cc` crate into a static library.
// All functions use a flat C ABI so they can be called from Rust.

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#ifndef UNICODE
#define UNICODE
#endif
#ifndef _UNICODE
#define _UNICODE
#endif

#include <windows.h>
#include <aclapi.h>
#include <accctrl.h>
#include <shellapi.h>
#include <shlobj.h>
#include <shobjidl.h>
#include <shlwapi.h>
#include <commoncontrols.h>
#include <objbase.h>
#include <sddl.h>
#include <tlhelp32.h>
#include <winsvc.h>
#include <string>
#include <vector>
#include <algorithm>

#pragma comment(lib, "comctl32.lib")
#pragma comment(lib, "ole32.lib")
#pragma comment(lib, "oleaut32.lib")
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "shlwapi.lib")
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "user32.lib")
#pragma comment(lib, "gdi32.lib")
#pragma comment(lib, "uuid.lib")

extern "C" {

// ---------------------------------------------------------------- utilities

void as_free(void* p) {
    if (p) CoTaskMemFree(p);
}

static void copy_w(const std::wstring& src, wchar_t* dst, int cap) {
    if (!dst || cap <= 0) return;
    int n = (int)src.size();
    if (n > cap - 1) n = cap - 1;
    if (n > 0) memcpy(dst, src.c_str(), (size_t)n * sizeof(wchar_t));
    dst[n] = 0;
}

static std::wstring long_path(const std::wstring& p) {
    if (p.size() >= 4 && p.compare(0, 4, L"\\\\?\\") == 0) return p;
    if (p.size() >= 2 && p[0] == L'\\' && p[1] == L'\\') return L"\\\\?\\UNC\\" + p.substr(2);
    return L"\\\\?\\" + p;
}

static bool enable_privilege(const wchar_t* name) {
    HANDLE tok = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &tok)) return false;
    LUID luid{};
    bool ok = false;
    if (LookupPrivilegeValueW(nullptr, name, &luid)) {
        TOKEN_PRIVILEGES tp{};
        tp.PrivilegeCount = 1;
        tp.Privileges[0].Luid = luid;
        tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        ok = AdjustTokenPrivileges(tok, FALSE, &tp, sizeof(tp), nullptr, nullptr) != FALSE;
    }
    CloseHandle(tok);
    return ok;
}

int as_is_elevated() {
    HANDLE tok = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &tok)) return 0;
    TOKEN_ELEVATION el{};
    DWORD sz = sizeof(el);
    int r = 0;
    if (GetTokenInformation(tok, TokenElevation, &el, sizeof(el), &sz)) r = el.TokenIsElevated ? 1 : 0;
    CloseHandle(tok);
    return r;
}

// Current user SID as string. Returns 0 on success.
int as_current_user_sid(wchar_t* out, int cap) {
    HANDLE tok = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &tok)) return (int)GetLastError();
    DWORD sz = 0;
    GetTokenInformation(tok, TokenUser, nullptr, 0, &sz);
    std::vector<BYTE> buf(sz ? sz : 64);
    if (!GetTokenInformation(tok, TokenUser, buf.data(), (DWORD)buf.size(), &sz)) {
        DWORD e = GetLastError();
        CloseHandle(tok);
        return (int)e;
    }
    TOKEN_USER* tu = (TOKEN_USER*)buf.data();
    LPWSTR s = nullptr;
    int rc = 0;
    if (ConvertSidToStringSidW(tu->User.Sid, &s)) {
        copy_w(std::wstring(s), out, cap);
        LocalFree(s);
    } else {
        rc = (int)GetLastError();
    }
    CloseHandle(tok);
    return rc;
}

int as_windows_build() {
    typedef LONG(WINAPI * RtlGetVersionPtr)(PRTL_OSVERSIONINFOW);
    HMODULE nt = GetModuleFileNameW ? GetModuleHandleW(L"ntdll.dll") : nullptr;
    if (!nt) return 0;
    auto fn = (RtlGetVersionPtr)GetProcAddress(nt, "RtlGetVersion");
    if (!fn) return 0;
    RTL_OSVERSIONINFOW vi{};
    vi.dwOSVersionInfoSize = sizeof(vi);
    if (fn(&vi) != 0) return 0;
    return (int)vi.dwBuildNumber;
}

// ---------------------------------------------------------------- icons

static bool hicon_to_bgra(HICON icon, int w, int h, BYTE** out, int* out_w, int* out_h, int* out_stride) {
    if (!icon || w <= 0 || h <= 0) return false;
    BITMAPINFO bi{};
    bi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bi.bmiHeader.biWidth = w;
    bi.bmiHeader.biHeight = -h; // top-down
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB;

    HDC screen = GetDC(nullptr);
    if (!screen) return false;
    void* bits = nullptr;
    HBITMAP dib = CreateDIBSection(screen, &bi, DIB_RGB_COLORS, &bits, nullptr, 0);
    HDC mem = CreateCompatibleDC(screen);
    ReleaseDC(nullptr, screen);
    if (!dib || !mem) {
        if (dib) DeleteObject(dib);
        if (mem) DeleteDC(mem);
        return false;
    }
    HGDIOBJ old = SelectObject(mem, dib);
    memset(bits, 0, (size_t)w * h * 4);
    BOOL drawn = DrawIconEx(mem, 0, 0, icon, w, h, 0, nullptr, DI_NORMAL);

    // Copy while the DIB is still alive: `bits` becomes invalid after DeleteObject.
    size_t bytes = (size_t)w * h * 4;
    BYTE* copy = nullptr;
    if (drawn) {
        copy = (BYTE*)CoTaskMemAlloc(bytes);
        if (copy) memcpy(copy, bits, bytes);
    }
    SelectObject(mem, old);
    DeleteDC(mem);
    DeleteObject(dib);
    if (!copy) return false;

    *out = copy;
    *out_w = w;
    *out_h = h;
    *out_stride = w * 4;
    return true;
}

// Extract the best available icon for a path. Returns 1 on success.
// Buffer must be released with as_free().
int as_extract_icon(const wchar_t* path, int size, BYTE** out, int* out_w, int* out_h, int* out_stride) {
    if (!path || !out || !out_w || !out_h || !out_stride) return 0;
    *out = nullptr;
    if (size <= 0) size = 64;

    SHFILEINFOW sfi{};
    bool have_index = SHGetFileInfoW(path, 0, &sfi, sizeof(sfi), SHGFI_SYSICONINDEX | SHGFI_LARGEICON) != 0;

    HICON icon = nullptr;

    if (have_index) {
        int want = size >= 200 ? SHIL_JUMBO : (size >= 40 ? SHIL_EXTRALARGE : SHIL_LARGE);
        IImageList* il = nullptr;
        if (SUCCEEDED(SHGetImageList(want, IID_IImageList, (void**)&il)) && il) {
            il->GetIcon(sfi.iIcon, ILD_TRANSPARENT, &icon);
            il->Release();
        }
    }
    if (!icon) {
        SHFILEINFOW sfi2{};
        if (SHGetFileInfoW(path, 0, &sfi2, sizeof(sfi2), SHGFI_ICON | SHGFI_LARGEICON)) icon = sfi2.hIcon;
    }
    if (!icon) {
        HICON large = nullptr;
        if (ExtractIconExW(path, 0, &large, nullptr, 1) > 0) icon = large;
    }
    if (!icon) {
        SHFILEINFOW sfi3{};
        if (SHGetFileInfoW(L"unknown", FILE_ATTRIBUTE_NORMAL, &sfi3, sizeof(sfi3), SHGFI_ICON | SHGFI_USEFILEATTRIBUTES)) icon = sfi3.hIcon;
    }
    if (!icon) return 0;

    int iw = size, ih = size;
    ICONINFO ii{};
    if (GetIconInfo(icon, &ii)) {
        BITMAP bm{};
        if (ii.hbmColor && GetObject(ii.hbmColor, sizeof(bm), &bm) == sizeof(bm)) {
            iw = bm.bmWidth > 0 ? bm.bmWidth : size;
            ih = bm.bmHeight > 0 ? bm.bmHeight : size;
        }
        if (ii.hbmColor) DeleteObject(ii.hbmColor);
        if (ii.hbmMask) DeleteObject(ii.hbmMask);
    }
    int target = size;
    if (iw > target) iw = target;
    if (ih > target) ih = target;

    bool ok = hicon_to_bgra(icon, iw, ih, out, out_w, out_h, out_stride);
    DestroyIcon(icon);
    return ok ? 1 : 0;
}

// ---------------------------------------------------------------- shell helpers

int as_open_in_explorer(const wchar_t* path, int select) {
    if (!path) return ERROR_INVALID_PARAMETER;
    std::wstring p(path);
    std::wstring args;
    if (select) {
        args = L"/select,\"" + p + L"\"";
    } else {
        args = L"\"" + p + L"\"";
    }
    HINSTANCE r = ShellExecuteW(nullptr, L"open", L"explorer.exe", args.c_str(), nullptr, SW_SHOWNORMAL);
    return (INT_PTR)r > 32 ? 0 : (int)(INT_PTR)r;
}

int as_show_properties(const wchar_t* path) {
    if (!path) return ERROR_INVALID_PARAMETER;
    CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    SHELLEXECUTEINFOW sei{};
    sei.cbSize = sizeof(sei);
    sei.fMask = SEE_MASK_INVOKEIDLIST;
    sei.lpVerb = L"properties";
    sei.lpFile = path;
    sei.nShow = SW_SHOWNORMAL;
    BOOL ok = ShellExecuteExW(&sei);
    int err = ok ? 0 : (int)GetLastError();
    CoUninitialize();
    return err;
}

int as_recycle_path(const wchar_t* path) {
    if (!path) return ERROR_INVALID_PARAMETER;
    std::wstring p = long_path(path);
    p.push_back(0);
    p.push_back(0);
    SHFILEOPSTRUCTW op{};
    op.wFunc = FO_DELETE;
    op.pFrom = p.c_str();
    op.fFlags = FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI;
    int r = SHFileOperationW(&op);
    return r;
}

// Run a raw command line (no shell window) and optionally wait.
int as_run_cmd(const wchar_t* cmdline, int wait, uint32_t* exit_code) {
    if (!cmdline) return ERROR_INVALID_PARAMETER;
    std::wstring cmd(cmdline);
    std::vector<wchar_t> buf(cmd.begin(), cmd.end());
    buf.push_back(0);
    STARTUPINFOW si{};
    si.cb = sizeof(si);
    si.dwFlags = STARTF_USESHOWWINDOW;
    si.wShowWindow = SW_HIDE;
    PROCESS_INFORMATION pi{};
    if (!CreateProcessW(nullptr, buf.data(), nullptr, nullptr, FALSE, CREATE_NO_WINDOW, nullptr, nullptr, &si, &pi)) {
        return (int)GetLastError();
    }
    if (wait) {
        WaitForSingleObject(pi.hProcess, INFINITE);
        DWORD code = 0;
        GetExitCodeProcess(pi.hProcess, &code);
        if (exit_code) *exit_code = code;
    } else if (exit_code) {
        *exit_code = 0;
    }
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
    return 0;
}

// Run a command line and capture its stdout (UTF-8 or ANSI, converted to wide).
int as_capture_cmd(const wchar_t* cmdline, wchar_t* out, int cap) {
    if (out && cap > 0) out[0] = 0;
    if (!cmdline || !out || cap <= 0) return ERROR_INVALID_PARAMETER;

    SECURITY_ATTRIBUTES sa{};
    sa.nLength = sizeof(sa);
    sa.bInheritHandle = TRUE;
    HANDLE rd = nullptr, wr = nullptr;
    if (!CreatePipe(&rd, &wr, &sa, 0)) return (int)GetLastError();
    SetHandleInformation(rd, HANDLE_FLAG_INHERIT, 0);

    STARTUPINFOW si{};
    si.cb = sizeof(si);
    si.dwFlags = STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
    si.hStdOutput = wr;
    si.hStdError = wr;
    si.hStdInput = nullptr;
    si.wShowWindow = SW_HIDE;
    PROCESS_INFORMATION pi{};

    std::wstring cmd(cmdline);
    std::vector<wchar_t> buf(cmd.begin(), cmd.end());
    buf.push_back(0);
    BOOL ok = CreateProcessW(nullptr, buf.data(), nullptr, nullptr, TRUE, CREATE_NO_WINDOW,
                             nullptr, nullptr, &si, &pi);
    CloseHandle(wr);
    if (!ok) {
        DWORD e = GetLastError();
        CloseHandle(rd);
        return (int)e;
    }

    std::string data;
    char chunk[4096];
    DWORD got = 0;
    while (ReadFile(rd, chunk, sizeof(chunk), &got, nullptr) && got > 0) {
        data.append(chunk, got);
        if (data.size() > 512 * 1024) break;
    }
    WaitForSingleObject(pi.hProcess, 20000);
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
    CloseHandle(rd);

    // Try UTF-8 first, fall back to the ANSI code page.
    int need = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, data.data(), (int)data.size(), nullptr, 0);
    UINT cp = CP_UTF8;
    DWORD flags = MB_ERR_INVALID_CHARS;
    if (need <= 0) {
        cp = CP_ACP;
        flags = 0;
        need = MultiByteToWideChar(cp, flags, data.data(), (int)data.size(), nullptr, 0);
    }
    if (need <= 0) return 0;
    std::wstring w(need, L'\0');
    MultiByteToWideChar(cp, flags, data.data(), (int)data.size(), &w[0], need);
    copy_w(w, out, cap);
    return 0;
}

int as_run_shell(const wchar_t* file, const wchar_t* params, const wchar_t* cwd, int wait, uint32_t* exit_code) {
    if (!file) return ERROR_INVALID_PARAMETER;
    std::wstring f(file), p(params ? params : L""), d(cwd ? cwd : L"");
    STARTUPINFOW si{};
    si.cb = sizeof(si);
    PROCESS_INFORMATION pi{};
    std::wstring cmd = L"\"" + f + L"\" " + p;
    std::vector<wchar_t> buf(cmd.begin(), cmd.end());
    buf.push_back(0);
    if (!CreateProcessW(f.c_str(), buf.data(), nullptr, nullptr, FALSE, 0, nullptr,
                        d.empty() ? nullptr : d.c_str(), &si, &pi)) {
        return (int)GetLastError();
    }
    if (wait) {
        WaitForSingleObject(pi.hProcess, INFINITE);
        DWORD code = 0;
        GetExitCodeProcess(pi.hProcess, &code);
        if (exit_code) *exit_code = code;
    } else if (exit_code) {
        *exit_code = 0;
    }
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
    return 0;
}

int as_run_elevated(const wchar_t* file, const wchar_t* params, const wchar_t* cwd, int wait, uint32_t* exit_code) {
    if (!file) return ERROR_INVALID_PARAMETER;
    CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    SHELLEXECUTEINFOW sei{};
    sei.cbSize = sizeof(sei);
    sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    sei.lpVerb = L"runas";
    sei.lpFile = file;
    sei.lpParameters = params;
    sei.lpDirectory = cwd;
    sei.nShow = SW_SHOWNORMAL;
    BOOL ok = ShellExecuteExW(&sei);
    int err = ok ? 0 : (int)GetLastError();
    if (ok && sei.hProcess) {
        if (wait) {
            WaitForSingleObject(sei.hProcess, INFINITE);
            DWORD code = 0;
            GetExitCodeProcess(sei.hProcess, &code);
            if (exit_code) *exit_code = code;
        } else if (exit_code) {
            *exit_code = 0;
        }
        CloseHandle(sei.hProcess);
    }
    CoUninitialize();
    return err;
}

// ---------------------------------------------------------------- shortcut resolution

int as_resolve_shortcut(const wchar_t* lnk, wchar_t* target, int tcap, wchar_t* args, int acap, wchar_t* workdir, int wcap) {
    if (!lnk) return ERROR_INVALID_PARAMETER;
    HRESULT hr = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    IShellLinkW* link = nullptr;
    int rc = 0;
    hr = CoCreateInstance(CLSID_ShellLink, nullptr, CLSCTX_INPROC_SERVER, IID_IShellLinkW, (void**)&link);
    if (SUCCEEDED(hr) && link) {
        IPersistFile* pf = nullptr;
        if (SUCCEEDED(link->QueryInterface(IID_IPersistFile, (void**)&pf)) && pf) {
            if (SUCCEEDED(pf->Load(lnk, STGM_READ))) {
                wchar_t buf[MAX_PATH * 2];
                if (SUCCEEDED(link->GetPath(buf, MAX_PATH * 2, nullptr, SLGP_UNCPRIORITY))) copy_w(buf, target, tcap);
                if (SUCCEEDED(link->GetArguments(buf, MAX_PATH * 2))) copy_w(buf, args, acap);
                if (SUCCEEDED(link->GetWorkingDirectory(buf, MAX_PATH * 2))) copy_w(buf, workdir, wcap);
            } else {
                rc = 1;
            }
            pf->Release();
        } else {
            rc = 2;
        }
        link->Release();
    } else {
        rc = 3;
    }
    if (SUCCEEDED(hr)) CoUninitialize();
    return rc;
}

// ---------------------------------------------------------------- processes

static std::wstring lower(std::wstring s) {
    std::transform(s.begin(), s.end(), s.begin(), [](wchar_t c) { return (wchar_t)towlower(c); });
    return s;
}

// True when `candidate` is the folder itself or a file inside it. A plain prefix
// test also matched siblings: "C:\Apps\Foo" claimed "C:\Apps\Foobar\other.exe".
static bool under_folder(const std::wstring& folder, const std::wstring& candidate) {
    if (folder.empty()) return false;
    if (candidate.size() < folder.size()) return false;
    if (candidate.compare(0, folder.size(), folder) != 0) return false;
    if (candidate.size() == folder.size()) return true;
    return candidate[folder.size()] == L'\\' || candidate[folder.size()] == L'/';
}

// Resolve either an executable path or a service command line to the executable
// it starts, honouring a quoted path and the usual unquoted forms.
static std::wstring command_executable(const std::wstring& cmd) {
    size_t i = 0;
    while (i < cmd.size() && (cmd[i] == L' ' || cmd[i] == L'\t' || cmd[i] == L'"')) i++;
    std::wstring out;
    if (i < cmd.size() && i > 0 && cmd[0] == L'"') {
        // Quoted: everything up to the closing quote, even with spaces inside.
        size_t end = cmd.find(L'"', i);
        out = cmd.substr(i, end == std::wstring::npos ? std::wstring::npos : end - i);
    } else {
        // Unquoted: a space followed by a '/' or '-' starts the arguments.
        size_t end = cmd.size();
        for (size_t k = i; k < cmd.size(); k++) {
            if ((cmd[k] == L' ' || cmd[k] == L'\t') && k + 1 < cmd.size() &&
                (cmd[k + 1] == L'/' || cmd[k + 1] == L'-')) {
                end = k;
                break;
            }
        }
        out = cmd.substr(i, end - i);
    }
    return long_path(out);
}

// Public for the self-test: does the cleanup planner consider `candidate` a
// process image inside `folder`?
int as_path_is_under(const wchar_t* folder, const wchar_t* candidate) {
    if (!folder || !candidate) return 0;
    std::wstring f = lower(long_path(folder));
    while (!f.empty() && (f.back() == L'\\' || f.back() == L'/')) f.pop_back();
    return under_folder(f, lower(long_path(candidate))) ? 1 : 0;
}

// Public for the self-test: the executable a command line starts.
int as_command_executable(const wchar_t* cmd, wchar_t* out, int cap) {
    if (!cmd || !out || cap <= 0) return ERROR_INVALID_PARAMETER;
    out[0] = 0;
    std::wstring exe = command_executable(cmd);
    // The service matcher does not need the \\?\ form, and the self-test wants
    // a path it can read back.
    if (exe.compare(0, 4, L"\\\\?\\") == 0) exe = exe.substr(4);
    copy_w(exe, out, cap);
    return 0;
}

static int count_or_kill_processes(const wchar_t* path, int do_kill) {
    if (!path) return 0;
    std::wstring prefix = lower(long_path(path));
    if (!prefix.empty() && prefix.back() == L'\\') prefix.pop_back();
    int count = 0;
    DWORD self = GetCurrentProcessId();
    HANDLE snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snap == INVALID_HANDLE_VALUE) return 0;
    PROCESSENTRY32W pe{};
    pe.dwSize = sizeof(pe);
    if (Process32FirstW(snap, &pe)) {
        do {
            if (pe.th32ProcessID == self || pe.th32ProcessID == 0) continue;
            HANDLE hp = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | (do_kill ? PROCESS_TERMINATE : 0),
                                    FALSE, pe.th32ProcessID);
            if (!hp) continue;
            wchar_t img[MAX_PATH * 2];
            DWORD sz = MAX_PATH * 2;
            if (QueryFullProcessImageNameW(hp, 0, img, &sz)) {
                std::wstring p = lower(long_path(img));
                if (under_folder(prefix, p)) {
                    count++;
                    if (do_kill) {
                        TerminateProcess(hp, 1);
                        WaitForSingleObject(hp, 1500);
                    }
                }
            }
            CloseHandle(hp);
        } while (Process32NextW(snap, &pe));
    }
    CloseHandle(snap);
    return count;
}

int as_count_processes_in_path(const wchar_t* path) { return count_or_kill_processes(path, 0); }
int as_kill_processes_in_path(const wchar_t* path) { return count_or_kill_processes(path, 1); }

// ---------------------------------------------------------------- services

// Enumerate services whose ImagePath points inside `path`. One name per line.
int as_list_services_under(const wchar_t* path, wchar_t* out, int cap) {
    if (out && cap > 0) out[0] = 0;
    if (!path) return ERROR_INVALID_PARAMETER;
    std::wstring prefix = lower(long_path(path));
    if (!prefix.empty() && prefix.back() == L'\\') prefix.pop_back();

    SC_HANDLE scm = OpenSCManagerW(nullptr, nullptr, SC_MANAGER_ENUMERATE_SERVICE);
    if (!scm) return (int)GetLastError();
    DWORD needed = 0, count = 0, resume = 0;
    EnumServicesStatusExW(scm, SC_ENUM_PROCESS_INFO, SERVICE_WIN32, SERVICE_STATE_ALL, nullptr, 0,
                          &needed, &count, &resume, nullptr);
    std::vector<BYTE> buf(needed ? needed : 4096);
    std::wstring result;
    if (EnumServicesStatusExW(scm, SC_ENUM_PROCESS_INFO, SERVICE_WIN32, SERVICE_STATE_ALL, buf.data(),
                             (DWORD)buf.size(), &needed, &count, &resume, nullptr)) {
        auto* items = (ENUM_SERVICE_STATUS_PROCESSW*)buf.data();
        for (DWORD i = 0; i < count; i++) {
            SC_HANDLE svc = OpenServiceW(scm, items[i].lpServiceName, SERVICE_QUERY_CONFIG);
            if (!svc) continue;
            DWORD sz = 0;
            QueryServiceConfigW(svc, nullptr, 0, &sz);
            std::vector<BYTE> cfg(sz ? sz : 1024);
            if (QueryServiceConfigW(svc, (LPQUERY_SERVICE_CONFIGW)cfg.data(), (DWORD)cfg.size(), &sz)) {
                auto* qc = (LPQUERY_SERVICE_CONFIGW)cfg.data();
                if (qc->lpBinaryPathName) {
                    std::wstring img = lower(command_executable(qc->lpBinaryPathName));
                    if (under_folder(prefix, img)) {
                        if (!result.empty()) result.push_back(L'\n');
                        result += items[i].lpServiceName;
                    }
                }
            }
            CloseServiceHandle(svc);
        }
    }
    CloseServiceHandle(scm);
    copy_w(result, out, cap);
    return 0;
}

int as_stop_delete_service(const wchar_t* name) {    if (!name) return ERROR_INVALID_PARAMETER;
    SC_HANDLE scm = OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT);
    if (!scm) return (int)GetLastError();
    SC_HANDLE svc = OpenServiceW(scm, name, SERVICE_STOP | DELETE | SERVICE_QUERY_STATUS);
    int rc = 0;
    if (!svc) {
        rc = (int)GetLastError();
        CloseServiceHandle(scm);
        return rc;
    }
    SERVICE_STATUS st{};
    ControlService(svc, SERVICE_CONTROL_STOP, &st);
    for (int i = 0; i < 20; i++) {
        SERVICE_STATUS cur{};
        if (!QueryServiceStatus(svc, &cur)) break;
        if (cur.dwCurrentState == SERVICE_STOPPED) break;
        Sleep(150);
    }
    if (!DeleteService(svc)) rc = (int)GetLastError();
    CloseServiceHandle(svc);
    CloseServiceHandle(scm);
    return rc;
}

// ---------------------------------------------------------------- delete / purge

static void clear_readonly(const std::wstring& p) {
    DWORD attr = GetFileAttributesW(p.c_str());
    if (attr != INVALID_FILE_ATTRIBUTES && (attr & FILE_ATTRIBUTE_READONLY)) {
        SetFileAttributesW(p.c_str(), attr & ~FILE_ATTRIBUTE_READONLY);
    }
}

// The original security descriptor is captured before any change and put back
// if the delete still fails right after the relaxation.
struct SavedSecurity {
    PSECURITY_DESCRIPTOR sd = nullptr;
    PSID owner = nullptr;
    PSID owner_group = nullptr;

    ~SavedSecurity() {
        if (sd) LocalFree(sd);
    }
};

static bool save_security(const std::wstring& p, SavedSecurity& out) {
    PACL dacl = nullptr;
    return GetNamedSecurityInfoW((LPWSTR)p.c_str(), SE_FILE_OBJECT,
                                 OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION |
                                     DACL_SECURITY_INFORMATION,
                                 &out.owner, &out.owner_group, &dacl, nullptr, &out.sd) ==
           ERROR_SUCCESS;
}

static bool restore_security(const std::wstring& p, const SavedSecurity& saved) {
    if (!saved.sd) return false;
    BOOL present = FALSE, dacl_defaulted = FALSE;
    PACL dacl = nullptr;
    GetSecurityDescriptorDacl(saved.sd, &present, &dacl, &dacl_defaulted);
    return SetNamedSecurityInfoW((LPWSTR)p.c_str(), SE_FILE_OBJECT,
                                 OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION |
                                     (present ? DACL_SECURITY_INFORMATION : 0),
                                 saved.owner, saved.owner_group, present ? dacl : nullptr, nullptr) ==
           ERROR_SUCCESS;
}

// Returns false when the object could not be relaxed. Never widens the
// permissions beyond the current user, and drops the inherited ACEs so a
// surviving folder cannot hand out access to its children.
static bool take_ownership(const std::wstring& p) {
    enable_privilege(L"SeTakeOwnershipPrivilege");
    enable_privilege(L"SeRestorePrivilege");
    enable_privilege(L"SeBackupPrivilege");
    PSID admins = nullptr;
    SID_IDENTIFIER_AUTHORITY nt = SECURITY_NT_AUTHORITY;
    if (AllocateAndInitializeSid(&nt, 2, SECURITY_BUILTIN_DOMAIN_RID, DOMAIN_ALIAS_RID_ADMINS, 0, 0, 0, 0, 0, 0, &admins)) {
        SetNamedSecurityInfoW((LPWSTR)p.c_str(), SE_FILE_OBJECT, OWNER_SECURITY_INFORMATION, admins, nullptr, nullptr, nullptr);
        FreeSid(admins);
    }
    // The deleting process already runs as the owner after the step above, so the
    // narrower grant is "the current user", not Everyone, and it does not
    // inherit: a failed delete must not leave child objects more open than before.
    PSID user = nullptr;
    SECURITY_INFORMATION info = DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION;
    EXPLICIT_ACCESSW ea{};
    ea.grfAccessPermissions = DELETE | FILE_DELETE_CHILD | FILE_GENERIC_READ | FILE_GENERIC_WRITE | READ_CONTROL | SYNCHRONIZE;
    ea.grfAccessMode = SET_ACCESS;
    ea.grfInheritance = NO_INHERITANCE;
    ea.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    ea.Trustee.TrusteeType = TRUSTEE_IS_USER;
    SID_IDENTIFIER_AUTHORITY nt_auth = SECURITY_NT_AUTHORITY;
    BOOL is_user = FALSE;
    if (AllocateAndInitializeSid(&nt_auth, 2, SECURITY_BUILTIN_DOMAIN_RID, DOMAIN_ALIAS_RID_ADMINS, 0, 0, 0, 0, 0, 0, &user) &&
        CheckTokenMembership(nullptr, user, &is_user)) {
        // Running elevated: the process token is the Administrators group, so that
        // group is the narrowest grant that lets this process delete.
        FreeSid(user);
        user = nullptr;
        if (!is_user) {
            return false;
        }
        if (!AllocateAndInitializeSid(&nt_auth, 2, SECURITY_BUILTIN_DOMAIN_RID, DOMAIN_ALIAS_RID_ADMINS, 0, 0, 0, 0, 0, 0, &user)) {
            return false;
        }
        ea.Trustee.TrusteeType = TRUSTEE_IS_GROUP;
    } else {
        if (user) {
            FreeSid(user);
            user = nullptr;
        }
        // No safe narrow grant without knowing who is deleting; stop rather than
        // fall back to Everyone.
        return false;
    }
    ea.Trustee.ptstrName = (LPWSTR)user;
    PACL acl = nullptr;
    DWORD rc = SetEntriesInAclW(1, &ea, nullptr, &acl);
    bool ok = false;
    if (rc == ERROR_SUCCESS && acl) {
        ok = SetNamedSecurityInfoW((LPWSTR)p.c_str(), SE_FILE_OBJECT, info, nullptr, nullptr, acl, nullptr) == ERROR_SUCCESS;
        LocalFree(acl);
    }
    FreeSid(user);
    return ok;
}

struct DelStats {
    int deleted = 0;
    int failed = 0;
    std::wstring first_error;
};

static void delete_tree(const std::wstring& raw, DelStats& st, int depth) {
    if (depth > 64) return;
    std::wstring p = long_path(raw);
    DWORD attr = GetFileAttributesW(p.c_str());
    if (attr == INVALID_FILE_ATTRIBUTES) return;

    if (attr & FILE_ATTRIBUTE_REPARSE_POINT) {
        // Never follow junctions/symlinks: just remove the link itself.
        clear_readonly(p);
        if (RemoveDirectoryW(p.c_str()) || DeleteFileW(p.c_str())) st.deleted++;
        else st.failed++;
        return;
    }

    if (attr & FILE_ATTRIBUTE_DIRECTORY) {
        WIN32_FIND_DATAW fd{};
        std::wstring pattern = p + L"\\*";
        HANDLE h = FindFirstFileW(pattern.c_str(), &fd);
        if (h != INVALID_HANDLE_VALUE) {
            do {
                if (wcscmp(fd.cFileName, L".") == 0 || wcscmp(fd.cFileName, L"..") == 0) continue;
                delete_tree(raw + L"\\" + fd.cFileName, st, depth + 1);
            } while (FindNextFileW(h, &fd));
            FindClose(h);
        }
        clear_readonly(p);
        if (RemoveDirectoryW(p.c_str())) {
            st.deleted++;
        } else {
            // One more attempt after taking ownership. Restore the original
            // security on failure: a failed delete must not leave the folder
            // more open than it was.
            SavedSecurity saved;
            bool have = save_security(raw, saved);
            take_ownership(raw);
            clear_readonly(p);
            if (RemoveDirectoryW(p.c_str())) {
                st.deleted++;
            } else {
                if (have) restore_security(raw, saved);
                st.failed++;
                if (st.first_error.empty()) st.first_error = raw + L" (dir)";
            }
        }
    } else {
        clear_readonly(p);
        if (DeleteFileW(p.c_str())) {
            st.deleted++;
        } else {
            SavedSecurity saved;
            bool have = save_security(raw, saved);
            take_ownership(raw);
            clear_readonly(p);
            if (DeleteFileW(p.c_str())) {
                st.deleted++;
            } else {
                if (have) restore_security(raw, saved);
                st.failed++;
                if (st.first_error.empty()) st.first_error = raw + L" (file)";
            }
        }
    }
}

// Force-delete a path recursively. Returns 0 when everything was removed.
int as_force_delete(const wchar_t* path, int* deleted, int* failed, wchar_t* log, int logcap) {    if (!path) return ERROR_INVALID_PARAMETER;
    enable_privilege(L"SeTakeOwnershipPrivilege");
    enable_privilege(L"SeRestorePrivilege");
    enable_privilege(L"SeBackupPrivilege");
    DelStats st{};
    std::wstring raw(path);
    while (!raw.empty() && (raw.back() == L'\\' || raw.back() == L'/')) raw.pop_back();
    delete_tree(raw, st, 0);
    if (deleted) *deleted = st.deleted;
    if (failed) *failed = st.failed;
    if (log) copy_w(st.first_error, log, logcap);
    return st.failed == 0 ? 0 : 1;
}

// Self-test only: does the DACL on `path` allow Everyone full control? Returns
// 1 if it does (which is what issue #5 forbids), 0 if it does not, -1 on error.
int as_grants_everyone_full_control(const wchar_t* path) {
    if (!path) return -1;
    PSID everyone = nullptr;
    SID_IDENTIFIER_AUTHORITY world = SECURITY_WORLD_SID_AUTHORITY;
    if (!AllocateAndInitializeSid(&world, 1, SECURITY_WORLD_RID, 0, 0, 0, 0, 0, 0, 0, &everyone)) return -1;
    PACL dacl = nullptr;
    PSECURITY_DESCRIPTOR sd = nullptr;
    DWORD rc = GetNamedSecurityInfoW((LPWSTR)long_path(path).c_str(), SE_FILE_OBJECT,
                                     DACL_SECURITY_INFORMATION, nullptr, nullptr, &dacl, nullptr, &sd);
    if (rc != ERROR_SUCCESS) {
        FreeSid(everyone);
        return -1;
    }
    BOOL present = FALSE, defaulted = FALSE;
    PACL actual = nullptr;
    int verdict = 0;
    if (GetSecurityDescriptorDacl(sd, &present, &actual, &defaulted) && present && actual) {
        for (WORD i = 0; i < actual->AceCount; i++) {
            LPVOID ace = nullptr;
            if (!GetAce(actual, i, &ace)) continue;
            if (((ACE_HEADER*)ace)->AceType != ACCESS_ALLOWED_ACE_TYPE) continue;
            auto* a = (ACCESS_ALLOWED_ACE*)ace;
            if (EqualSid((PSID)&a->SidStart, everyone) && (a->Mask & FILE_ALL_ACCESS) == FILE_ALL_ACCESS) {
                verdict = 1;
                break;
            }
        }
    }
    LocalFree(sd);
    FreeSid(everyone);
    return verdict;
}

// Self-test only: plant an Everyone-full-control ACE, so the detector above can
// be proven to work. Returns 0 on success.
int as_grant_everyone_full_control(const wchar_t* path) {
    if (!path) return ERROR_INVALID_PARAMETER;
    PSID everyone = nullptr;
    SID_IDENTIFIER_AUTHORITY world = SECURITY_WORLD_SID_AUTHORITY;
    if (!AllocateAndInitializeSid(&world, 1, SECURITY_WORLD_RID, 0, 0, 0, 0, 0, 0, 0, &everyone))
        return (int)GetLastError();
    EXPLICIT_ACCESSW ea{};
    ea.grfAccessPermissions = FILE_ALL_ACCESS;
    ea.grfAccessMode = SET_ACCESS;
    ea.grfInheritance = NO_INHERITANCE;
    ea.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    ea.Trustee.TrusteeType = TRUSTEE_IS_WELL_KNOWN_GROUP;
    ea.Trustee.ptstrName = (LPWSTR)everyone;
    PACL acl = nullptr;
    DWORD rc = SetEntriesInAclW(1, &ea, nullptr, &acl);
    if (rc == ERROR_SUCCESS && acl) {
        rc = SetNamedSecurityInfoW((LPWSTR)long_path(path).c_str(), SE_FILE_OBJECT,
                                   DACL_SECURITY_INFORMATION, nullptr, nullptr, acl, nullptr);
        LocalFree(acl);
    }
    FreeSid(everyone);
    return (int)rc;
}

int as_path_exists(const wchar_t* path) {
    if (!path) return 0;
    DWORD a = GetFileAttributesW(long_path(path).c_str());
    return a == INVALID_FILE_ATTRIBUTES ? 0 : 1;
}

// 1 = there, 0 = confirmed missing, -1 = could not tell. Only FILE_NOT_FOUND
// and PATH_NOT_FOUND mean "gone": access denied is not absence.
int as_path_state(const wchar_t* path) {
    if (!path) return -1;
    DWORD a = GetFileAttributesW(long_path(path).c_str());
    if (a != INVALID_FILE_ATTRIBUTES) return 1;
    DWORD e = GetLastError();
    return (e == ERROR_FILE_NOT_FOUND || e == ERROR_PATH_NOT_FOUND) ? 0 : -1;
}

int as_service_exists(const wchar_t* name) {
    if (!name) return -1;
    SC_HANDLE sc = OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT);
    if (!sc) return -1;
    SC_HANDLE svc = OpenServiceW(sc, name, SERVICE_QUERY_STATUS);
    int rc;
    if (svc) {
        rc = 1;
        CloseServiceHandle(svc);
    } else {
        rc = GetLastError() == ERROR_SERVICE_DOES_NOT_EXIST ? 0 : -1;
    }
    CloseServiceHandle(sc);
    return rc;
}

// schtasks exit code 0 means the query succeeded, so the task is still there.
int as_task_exists(const wchar_t* name) {
    if (!name) return -1;
    std::wstring cmd = L"schtasks.exe /Query /TN \"" + std::wstring(name) + L"\"";
    uint32_t code = 0;
    int rc = as_run_cmd(cmd.c_str(), 1, &code);
    if (rc != 0) return -1;
    return code == 0 ? 1 : 0;
}

// Schedule deletion at next reboot (last resort).
int as_delete_on_reboot(const wchar_t* path) {
    if (!path) return ERROR_INVALID_PARAMETER;
    std::wstring p = long_path(path);
    if (MoveFileExW(p.c_str(), nullptr, MOVEFILE_DELAY_UNTIL_REBOOT)) return 0;
    return (int)GetLastError();
}

// ---------------------------------------------------------------- system restore

typedef BOOL(WINAPI* SRSetRestorePointWFn)(void*, void*);

int as_create_restore_point(const wchar_t* description) {
    HMODULE dll = LoadLibraryW(L"srclient.dll");
    if (!dll) return (int)GetLastError();
    auto fn = (SRSetRestorePointWFn)GetProcAddress(dll, "SRSetRestorePointW");
    int rc = 0;
    if (!fn) {
        rc = ERROR_PROC_NOT_FOUND;
    } else {
        // RESTOREPOINTINFOW { DWORD dwEventType; DWORD dwRestorePtType; INT64 llSequenceNumber; WCHAR szDescription[256]; }
        struct RP {
            DWORD dwEventType;
            DWORD dwRestorePtType;
            INT64 llSequenceNumber;
            WCHAR szDescription[256];
        };
        struct SM {
            DWORD dwEventType;
            DWORD dwRestorePtType;
            INT64 llSequenceNumber;
        };
        RP rp{};
        SM sm{};
        rp.dwEventType = 100;      // BEGIN_SYSTEM_CHANGE
        rp.dwRestorePtType = 0;    // APPLICATION_INSTALL
        rp.llSequenceNumber = 0;
        wcsncpy_s(rp.szDescription, description ? description : L"AppSweep", _TRUNCATE);
        // SRSetRestorePointW takes exactly two arguments:
        //   BOOL SRSetRestorePointW(PRESTOREPOINTINFOW, PSTATEMGRSTATUS)
        if (!fn(&rp, &sm)) rc = (int)GetLastError();
    }
    FreeLibrary(dll);
    return rc;
}

// ---------------------------------------------------------------- misc

int as_delete_scheduled_task(const wchar_t* name) {
    if (!name) return ERROR_INVALID_PARAMETER;
    std::wstring cmd = L"schtasks.exe /Delete /TN \"" + std::wstring(name) + L"\" /F";
    // as_run_cmd only reports whether the process started, so the exit code has
    // to be handled here: 0 is the only success. Otherwise a refused delete
    // (access denied, task gone) looked exactly like a clean one.
    uint32_t code = 1;
    int rc = as_run_cmd(cmd.c_str(), 1, &code);
    return rc == 0 && code == 0 ? 0 : ERROR_ACCESS_DENIED;
}

// List scheduled tasks whose action references the given directory prefix.
// Returns newline-separated task paths ("\Folder\Name"), or empty when none.
int as_list_tasks_under(const wchar_t* path, wchar_t* out, int cap) {
    if (out && cap > 0) out[0] = 0;
    if (!path || !out || cap <= 0) return ERROR_INVALID_PARAMETER;

    // schtasks /FO LIST /V prints one "Key: value" per line and a blank line
    // between records. That is far more robust than parsing CSV, whose fields
    // may contain embedded commas, quotes and newlines.
    std::wstring cmd = L"schtasks.exe /Query /FO LIST /V";
    std::wstring text;
    {
        SECURITY_ATTRIBUTES sa{};
        sa.nLength = sizeof(sa);
        sa.bInheritHandle = TRUE;
        HANDLE rd = nullptr, wr = nullptr;
        if (!CreatePipe(&rd, &wr, &sa, 0)) return (int)GetLastError();
        SetHandleInformation(rd, HANDLE_FLAG_INHERIT, 0);

        STARTUPINFOW si{};
        si.cb = sizeof(si);
        si.dwFlags = STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
        si.hStdOutput = wr;
        si.hStdError = wr;
        si.wShowWindow = SW_HIDE;
        PROCESS_INFORMATION pi{};
        std::vector<wchar_t> cmdline(cmd.begin(), cmd.end());
        cmdline.push_back(0);
        if (!CreateProcessW(nullptr, cmdline.data(), nullptr, nullptr, TRUE,
                            CREATE_NO_WINDOW, nullptr, nullptr, &si, &pi)) {
            CloseHandle(rd);
            CloseHandle(wr);
            return (int)GetLastError();
        }
        CloseHandle(wr);
        // schtasks writes bytes in the OEM code page, not UTF-16.
        std::string raw;
        {
            char buf[1 << 16];
            DWORD got = 0;
            while (ReadFile(rd, buf, sizeof(buf), &got, nullptr) && got > 0) {
                raw.append(buf, got);
                if (raw.size() > 8u * 1024u * 1024u) break;
            }
        }
        WaitForSingleObject(pi.hProcess, 15000);
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
        CloseHandle(rd);

        if (!raw.empty()) {
            int need = MultiByteToWideChar(CP_OEMCP, 0, raw.c_str(), (int)raw.size(), nullptr, 0);
            if (need > 0) {
                text.resize((size_t)need);
                MultiByteToWideChar(CP_OEMCP, 0, raw.c_str(), (int)raw.size(), &text[0], need);
            }
        }
    }

    std::wstring needle = lower(long_path(path));
    if (needle.size() > 4 && needle.rfind(L"\\\\?\\", 0) == 0) needle = needle.substr(4);

    // Records are separated by blank lines; each record holds "Key: value"
    // lines. "Task To Run" holds the command that must reference the folder.
    std::wstring result;
    int count = 0;
    std::wstring task_name;
    bool task_hit = false;

    auto flush = [&]() {
        if (!task_name.empty() && task_hit) {
            if (!result.empty()) result += L"\n";
            result += task_name;
            ++count;
        }
        task_name.clear();
        task_hit = false;
    };

    size_t pos = 0;
    while (pos < text.size()) {
        size_t eol = text.find(L'\n', pos);
        if (eol == std::wstring::npos) eol = text.size();
        std::wstring line = text.substr(pos, eol - pos);
        pos = eol + 1;
        while (!line.empty() && (line.back() == L'\r' || line.back() == L' ')) line.pop_back();

        if (line.empty()) {
            flush();
            continue;
        }
        size_t colon = line.find(L": ");
        if (colon == std::wstring::npos) colon = line.find(L':');
        if (colon == std::wstring::npos) continue;
        std::wstring key = line.substr(0, colon);
        std::wstring val = colon + 1 < line.size() ? line.substr(colon + 1) : L"";
        while (!val.empty() && val.front() == L' ') val.erase(val.begin());

        if (key == L"TaskName") {
            task_name = val;
        } else if (key == L"Task To Run") {
            // Only the command counts. Matching any field pulled in tasks whose
            // author, comment or trigger merely mentioned the folder.
            task_hit = lower(val).find(needle) != std::wstring::npos;
        }
    }
    flush();

    // The buffer is fixed, so a very long list would end mid-task-name. Say so
    // instead of handing back a truncated name that no delete would match.
    if (result.size() >= (size_t)cap) return ERROR_BUFFER_OVERFLOW;
    wmemcpy(out, result.c_str(), result.size() + 1);
    return 0;
}

int as_kill_process_by_pid(unsigned long pid) {
    HANDLE h = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
    if (!h) return (int)GetLastError();
    BOOL ok = TerminateProcess(h, 1);
    int err = ok ? 0 : (int)GetLastError();
    CloseHandle(h);
    return err;
}

// Directory size in bytes (shallow recursion, capped).
static unsigned long long dir_size(const std::wstring& raw, int depth) {
    if (depth > 8) return 0;
    unsigned long long total = 0;
    std::wstring p = long_path(raw);
    WIN32_FIND_DATAW fd{};
    HANDLE h = FindFirstFileW((p + L"\\*").c_str(), &fd);
    if (h == INVALID_HANDLE_VALUE) return 0;
    do {
        if (wcscmp(fd.cFileName, L".") == 0 || wcscmp(fd.cFileName, L"..") == 0) continue;
        if (fd.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) continue;
        if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
            total += dir_size(raw + L"\\" + fd.cFileName, depth + 1);
        } else {
            total += ((unsigned long long)fd.nFileSizeHigh << 32) | fd.nFileSizeLow;
        }
    } while (FindNextFileW(h, &fd));
    FindClose(h);
    return total;
}

unsigned long long as_path_size(const wchar_t* path) {
    if (!path) return 0;
    std::wstring p(path);
    DWORD a = GetFileAttributesW(long_path(p).c_str());
    if (a == INVALID_FILE_ATTRIBUTES) return 0;
    if (a & FILE_ATTRIBUTE_DIRECTORY) return dir_size(p, 0);
    WIN32_FILE_ATTRIBUTE_DATA fad{};
    if (!GetFileAttributesExW(long_path(p).c_str(), GetFileExInfoStandard, &fad)) return 0;
    return ((unsigned long long)fad.nFileSizeHigh << 32) | fad.nFileSizeLow;
}

int as_ensure_dir(const wchar_t* path) {
    if (!path) return ERROR_INVALID_PARAMETER;
    if (CreateDirectoryW(path, nullptr)) return 0;
    DWORD e = GetLastError();
    return e == ERROR_ALREADY_EXISTS ? 0 : (int)e;
}

// Open a URL / file with the default handler.
int as_shell_open(const wchar_t* what) {
    if (!what) return ERROR_INVALID_PARAMETER;
    HINSTANCE r = ShellExecuteW(nullptr, L"open", what, nullptr, nullptr, SW_SHOWNORMAL);
    return (INT_PTR)r > 32 ? 0 : (int)(INT_PTR)r;
}

} // extern "C"
