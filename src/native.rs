// Native Windows integration implemented directly in Rust.
#![allow(dead_code)]

use std::ffi::{c_char, c_void};
use std::mem::{size_of, transmute, zeroed};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr::{null, null_mut};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

type Handle = *mut c_void;
type Bool = i32;
type Dword = u32;
type HResult = i32;

const ERROR_INVALID_PARAMETER: i32 = 87;
const ERROR_ALREADY_EXISTS: i32 = 183;
const ERROR_MORE_DATA: u32 = 234;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_TIMEOUT: u32 = 0x102;
const INFINITE: u32 = 0xffff_ffff;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const STARTF_USESHOWWINDOW: u32 = 0x0000_0001;
const STARTF_USESTDHANDLES: u32 = 0x0000_0100;
const SW_HIDE: u16 = 0;
const SW_SHOWNORMAL: i32 = 1;
const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_USER_CLASS: u32 = 1;
const TOKEN_ELEVATION_CLASS: u32 = 20;
const HANDLE_FLAG_INHERIT: u32 = 1;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const PROCESS_TERMINATE: u32 = 0x0001;
const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
const SEE_MASK_INVOKEIDLIST: u32 = 0x0000_000c;
const SEE_MASK_NOCLOSEPROCESS: u32 = 0x0000_0040;
const SEE_MASK_NOASYNC: u32 = 0x0000_0100;
const FO_DELETE: u32 = 3;
const FOF_SILENT: u16 = 0x0004;
const FOF_NOCONFIRMATION: u16 = 0x0010;
const FOF_ALLOWUNDO: u16 = 0x0040;
const FOF_NOERRORUI: u16 = 0x0400;
const SHGFI_ICON: u32 = 0x0000_0100;
const SHGFI_USEFILEATTRIBUTES: u32 = 0x0000_0010;
const SHGFI_LARGEICON: u32 = 0x0000_0000;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
const DI_NORMAL: u32 = 0x0003;
const COINIT_APARTMENTTHREADED: u32 = 0x2;
const COINIT_DISABLE_OLE1DDE: u32 = 0x4;
const CLSCTX_INPROC_SERVER: u32 = 0x1;
const STGM_READ: u32 = 0;
const SLGP_UNCPRIORITY: u32 = 0x2;
const SC_MANAGER_CONNECT: u32 = 0x1;
const SC_MANAGER_ENUMERATE_SERVICE: u32 = 0x4;
const SERVICE_QUERY_CONFIG: u32 = 0x0001;
const SERVICE_STOP: u32 = 0x0020;
const DELETE_ACCESS: u32 = 0x0001_0000;
const SERVICE_QUERY_STATUS: u32 = 0x0004;
const SERVICE_CONTROL_STOP: u32 = 0x0001;
const SERVICE_STOPPED: u32 = 1;
const SERVICE_WIN32: u32 = 0x0030;
const SERVICE_STATE_ALL: u32 = 0x0003;
const SC_ENUM_PROCESS_INFO: u32 = 0;
const CP_UTF8: u32 = 65001;
const CP_ACP: u32 = 0;
const CP_OEMCP: u32 = 1;
const MB_ERR_INVALID_CHARS: u32 = 8;
const MOVEFILE_DELAY_UNTIL_REBOOT: u32 = 4;

#[repr(C)]
struct SecurityAttributes {
    length: u32,
    security_descriptor: *mut c_void,
    inherit_handle: Bool,
}

#[repr(C)]
struct StartupInfoW {
    cb: u32,
    lp_reserved: *mut u16,
    lp_desktop: *mut u16,
    lp_title: *mut u16,
    dw_x: u32,
    dw_y: u32,
    dw_x_size: u32,
    dw_y_size: u32,
    dw_x_count_chars: u32,
    dw_y_count_chars: u32,
    dw_fill_attribute: u32,
    dw_flags: u32,
    w_show_window: u16,
    cb_reserved2: u16,
    lp_reserved2: *mut u8,
    h_std_input: Handle,
    h_std_output: Handle,
    h_std_error: Handle,
}

#[repr(C)]
struct ProcessInformation {
    h_process: Handle,
    h_thread: Handle,
    process_id: u32,
    thread_id: u32,
}

#[repr(C)]
struct ShellExecuteInfoW {
    cb_size: u32,
    f_mask: u32,
    hwnd: Handle,
    lp_verb: *const u16,
    lp_file: *const u16,
    lp_parameters: *const u16,
    lp_directory: *const u16,
    n_show: i32,
    h_inst_app: Handle,
    lp_id_list: *mut c_void,
    lp_class: *const u16,
    hkey_class: Handle,
    dw_hot_key: u32,
    h_icon_or_monitor: Handle,
    h_process: Handle,
}

#[repr(C)]
struct ShFileOperationW {
    hwnd: Handle,
    w_func: u32,
    p_from: *const u16,
    p_to: *const u16,
    f_flags: u16,
    f_any_operations_aborted: Bool,
    h_name_mappings: Handle,
    lpsz_progress_title: *const u16,
}

#[repr(C)]
struct ShellFileInfoW {
    h_icon: Handle,
    i_icon: i32,
    dw_attributes: u32,
    sz_display_name: [u16; 260],
    sz_type_name: [u16; 80],
}

#[repr(C)]
struct IconInfo {
    f_icon: Bool,
    x_hotspot: u32,
    y_hotspot: u32,
    hbm_mask: Handle,
    hbm_color: Handle,
}

#[repr(C)]
struct Bitmap {
    bm_type: i32,
    bm_width: i32,
    bm_height: i32,
    bm_width_bytes: i32,
    bm_planes: u16,
    bm_bits_pixel: u16,
    bm_bits: *mut c_void,
}

#[repr(C)]
struct BitmapInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[repr(C)]
struct RgbQuad {
    blue: u8,
    green: u8,
    red: u8,
    reserved: u8,
}

#[repr(C)]
struct BitmapInfo {
    bmi_header: BitmapInfoHeader,
    bmi_colors: [RgbQuad; 1],
}

#[repr(C)]
struct ProcessEntry32W {
    dw_size: u32,
    cnt_usage: u32,
    th32_process_id: u32,
    th32_default_heap_id: usize,
    th32_module_id: u32,
    cnt_threads: u32,
    th32_parent_process_id: u32,
    pc_pri_class_base: i32,
    dw_flags: u32,
    sz_exe_file: [u16; 260],
}

#[repr(C)]
struct ServiceStatus {
    service_type: u32,
    current_state: u32,
    controls_accepted: u32,
    win32_exit_code: u32,
    service_specific_exit_code: u32,
    check_point: u32,
    wait_hint: u32,
}

#[repr(C)]
struct ServiceStatusProcess {
    service_type: u32,
    current_state: u32,
    controls_accepted: u32,
    win32_exit_code: u32,
    service_specific_exit_code: u32,
    check_point: u32,
    wait_hint: u32,
    process_id: u32,
    service_flags: u32,
}

#[repr(C)]
struct EnumServiceStatusProcessW {
    service_name: *mut u16,
    display_name: *mut u16,
    status: ServiceStatusProcess,
}

#[repr(C)]
struct QueryServiceConfigW {
    service_type: u32,
    start_type: u32,
    error_control: u32,
    binary_path_name: *mut u16,
    load_order_group: *mut u16,
    tag_id: u32,
    dependencies: *mut u16,
    service_start_name: *mut u16,
    display_name: *mut u16,
}

#[repr(C)]
struct OsVersionInfoW {
    os_version_info_size: u32,
    major_version: u32,
    minor_version: u32,
    build_number: u32,
    platform_id: u32,
    csd_version: [u16; 128],
}

#[repr(C)]
struct RestorePointInfoW {
    event_type: u32,
    restore_point_type: u32,
    sequence_number: i64,
    description: [u16; 256],
}

#[repr(C)]
struct StateManagerStatus {
    event_type: u32,
    restore_point_type: u32,
    sequence_number: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct UnknownVTable {
    methods: [usize; 3],
}

#[repr(C)]
struct ShellLinkVTable {
    methods: [usize; 21],
}

#[repr(C)]
struct PersistFileVTable {
    methods: [usize; 9],
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> Handle;
    fn GetCurrentProcessId() -> u32;
    fn GetLastError() -> u32;
    fn CloseHandle(handle: Handle) -> Bool;
    fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
    fn GetExitCodeProcess(process: Handle, exit_code: *mut u32) -> Bool;
    fn CreateProcessW(
        application_name: *const u16,
        command_line: *mut u16,
        process_attributes: *const SecurityAttributes,
        thread_attributes: *const SecurityAttributes,
        inherit_handles: Bool,
        creation_flags: u32,
        environment: *mut c_void,
        current_directory: *const u16,
        startup_info: *const StartupInfoW,
        process_information: *mut ProcessInformation,
    ) -> Bool;
    fn CreatePipe(
        read_pipe: *mut Handle,
        write_pipe: *mut Handle,
        attributes: *const SecurityAttributes,
        size: u32,
    ) -> Bool;
    fn SetHandleInformation(handle: Handle, mask: u32, flags: u32) -> Bool;
    fn ReadFile(
        file: Handle,
        buffer: *mut c_void,
        bytes_to_read: u32,
        bytes_read: *mut u32,
        overlapped: *mut c_void,
    ) -> Bool;
    fn TerminateProcess(process: Handle, exit_code: u32) -> Bool;
    fn Sleep(milliseconds: u32);
    fn GetModuleHandleW(module_name: *const u16) -> Handle;
    fn GetProcAddress(module: Handle, proc_name: *const c_char) -> *mut c_void;
    fn LoadLibraryW(file_name: *const u16) -> Handle;
    fn FreeLibrary(module: Handle) -> Bool;
    fn LocalFree(memory: Handle) -> Handle;
    fn MultiByteToWideChar(
        code_page: u32,
        flags: u32,
        input: *const u8,
        input_len: i32,
        output: *mut u16,
        output_len: i32,
    ) -> i32;
    fn MoveFileExW(existing_name: *const u16, new_name: *const u16, flags: u32) -> Bool;
    fn GetFileAttributesW(file_name: *const u16) -> u32;
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> Handle;
    fn Process32FirstW(snapshot: Handle, entry: *mut ProcessEntry32W) -> Bool;
    fn Process32NextW(snapshot: Handle, entry: *mut ProcessEntry32W) -> Bool;
    fn OpenProcess(access: u32, inherit: Bool, process_id: u32) -> Handle;
    fn QueryFullProcessImageNameW(
        process: Handle,
        flags: u32,
        image_name: *mut u16,
        size: *mut u32,
    ) -> Bool;
}

#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(process: Handle, desired_access: u32, token: *mut Handle) -> Bool;
    fn GetTokenInformation(
        token: Handle,
        information_class: u32,
        information: *mut c_void,
        information_length: u32,
        return_length: *mut u32,
    ) -> Bool;
    fn ConvertSidToStringSidW(sid: *mut c_void, string_sid: *mut *mut u16) -> Bool;
    fn OpenSCManagerW(machine_name: *const u16, database_name: *const u16, desired_access: u32) -> Handle;
    fn EnumServicesStatusExW(
        manager: Handle,
        information_level: u32,
        service_type: u32,
        service_state: u32,
        services: *mut u8,
        buffer_size: u32,
        bytes_needed: *mut u32,
        services_returned: *mut u32,
        resume_handle: *mut u32,
        group_name: *const u16,
    ) -> Bool;
    fn OpenServiceW(manager: Handle, service_name: *const u16, desired_access: u32) -> Handle;
    fn QueryServiceConfigW(
        service: Handle,
        config: *mut QueryServiceConfigW,
        buffer_size: u32,
        bytes_needed: *mut u32,
    ) -> Bool;
    fn CloseServiceHandle(service: Handle) -> Bool;
    fn ControlService(service: Handle, control: u32, status: *mut ServiceStatus) -> Bool;
    fn QueryServiceStatus(service: Handle, status: *mut ServiceStatus) -> Bool;
    fn DeleteService(service: Handle) -> Bool;
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: Handle,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show_command: i32,
    ) -> isize;
    fn ShellExecuteExW(info: *mut ShellExecuteInfoW) -> Bool;
    fn SHFileOperationW(operation: *mut ShFileOperationW) -> i32;
    fn SHGetFileInfoW(
        path: *const u16,
        file_attributes: u32,
        file_info: *mut ShellFileInfoW,
        file_info_size: u32,
        flags: u32,
    ) -> usize;
    fn ExtractIconExW(
        file_name: *const u16,
        icon_index: i32,
        large_icon: *mut Handle,
        small_icon: *mut Handle,
        icon_count: u32,
    ) -> u32;
}

#[link(name = "user32")]
extern "system" {
    fn GetDC(hwnd: Handle) -> Handle;
    fn ReleaseDC(hwnd: Handle, dc: Handle) -> i32;
    fn DrawIconEx(
        dc: Handle,
        x: i32,
        y: i32,
        icon: Handle,
        width: i32,
        height: i32,
        step_if_animated: u32,
        flicker_free_dc: Handle,
        flags: u32,
    ) -> Bool;
    fn DestroyIcon(icon: Handle) -> Bool;
    fn GetIconInfo(icon: Handle, info: *mut IconInfo) -> Bool;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(dc: Handle) -> Handle;
    fn CreateDIBSection(
        dc: Handle,
        bitmap_info: *const BitmapInfo,
        usage: u32,
        bits: *mut *mut c_void,
        section: Handle,
        offset: u32,
    ) -> Handle;
    fn SelectObject(dc: Handle, object: Handle) -> Handle;
    fn DeleteObject(object: Handle) -> Bool;
    fn DeleteDC(dc: Handle) -> Bool;
    fn GetObjectW(object: Handle, size: i32, buffer: *mut c_void) -> i32;
}

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> HResult;
    fn CoUninitialize();
    fn CoCreateInstance(
        class_id: *const Guid,
        outer: *mut c_void,
        context: u32,
        interface_id: *const Guid,
        instance: *mut *mut c_void,
    ) -> HResult;
}

pub fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_ptr_to_string(value: *const u16) -> String {
    if value.is_null() {
        return String::new();
    }
    unsafe {
        let mut len = 0usize;
        while *value.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(value, len))
    }
}

fn long_path(path: &str) -> String {
    if path.starts_with(r"\\?\") {
        path.to_string()
    } else if let Some(unc) = path.strip_prefix(r"\\") {
        format!(r"\\?\UNC\{}", unc)
    } else {
        format!(r"\\?\{}", path)
    }
}

fn normalize_path(path: &str) -> String {
    long_path(path).replace('/', "\\").to_lowercase()
}

fn is_under_normalized(folder: &str, candidate: &str) -> bool {
    let folder = folder.trim_end_matches(|ch| ch == '\\' || ch == '/');
    if folder.is_empty() || candidate.len() < folder.len() || !candidate.starts_with(folder) {
        return false;
    }
    candidate.len() == folder.len()
        || candidate.as_bytes().get(folder.len()).is_some_and(|c| *c == b'\\' || *c == b'/')
}

pub fn path_is_under(folder: &str, candidate: &str) -> bool {
    if folder.is_empty() || candidate.is_empty() {
        return false;
    }
    is_under_normalized(&normalize_path(folder), &normalize_path(candidate))
}

pub fn command_executable(command_line: &str) -> String {
    let bytes = command_line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'"') {
        index += 1;
    }

    let executable = if index < bytes.len() && index > 0 && bytes.first() == Some(&b'"') {
        let rest = &command_line[index..];
        match rest.find('"') {
            Some(end) => &rest[..end],
            None => rest,
        }
    } else {
        let mut end = command_line.len();
        for i in index..bytes.len().saturating_sub(1) {
            if matches!(bytes[i], b' ' | b'\t') && matches!(bytes[i + 1], b'/' | b'-') {
                end = i;
                break;
            }
        }
        &command_line[index..end]
    };

    let normalized = long_path(executable);
    normalized.strip_prefix(r"\\?\").unwrap_or(&normalized).to_string()
}

pub fn is_elevated() -> bool {
    unsafe {
        let mut token = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = 0u32;
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION_CLASS,
            &mut elevation as *mut u32 as *mut c_void,
            size_of::<u32>() as u32,
            &mut returned,
        ) != 0;
        CloseHandle(token);
        ok && elevation != 0
    }
}

pub fn windows_build() -> u32 {
    #[allow(non_camel_case_types)]
    type RtlGetVersionFn = unsafe extern "system" fn(*mut OsVersionInfoW) -> i32;
    unsafe {
        let module_name = wide("ntdll.dll");
        let module = GetModuleHandleW(module_name.as_ptr());
        if module.is_null() {
            return 0;
        }
        let proc = GetProcAddress(module, b"RtlGetVersion\0".as_ptr() as *const c_char);
        if proc.is_null() {
            return 0;
        }
        let get_version: RtlGetVersionFn = transmute(proc);
        let mut version: OsVersionInfoW = zeroed();
        version.os_version_info_size = size_of::<OsVersionInfoW>() as u32;
        if get_version(&mut version) == 0 {
            version.build_number
        } else {
            0
        }
    }
}

pub fn current_user_sid() -> String {
    unsafe {
        let mut token = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return String::new();
        }
        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TOKEN_USER_CLASS, null_mut(), 0, &mut needed);
        if needed == 0 {
            CloseHandle(token);
            return String::new();
        }
        let mut data = vec![0u8; needed as usize];
        if GetTokenInformation(
            token,
            TOKEN_USER_CLASS,
            data.as_mut_ptr() as *mut c_void,
            needed,
            &mut needed,
        ) == 0
        {
            CloseHandle(token);
            return String::new();
        }

        // TOKEN_USER begins with SID_AND_ATTRIBUTES, whose first field is a SID pointer.
        let sid = std::ptr::read_unaligned(data.as_ptr() as *const Handle);
        let mut text_sid: *mut u16 = null_mut();
        let ok = ConvertSidToStringSidW(sid, &mut text_sid) != 0;
        CloseHandle(token);
        if !ok || text_sid.is_null() {
            return String::new();
        }
        let result = wide_ptr_to_string(text_sid);
        let _ = LocalFree(text_sid as Handle);
        result
    }
}

pub struct RawIcon {
    pub w: usize,
    pub h: usize,
    /// RGBA, unmultiplied, ready for ColorImage::from_rgba_unmultiplied.
    pub rgba: Vec<u8>,
}

fn icon_to_bgra(icon: Handle, width: i32, height: i32) -> Option<Vec<u8>> {
    if icon.is_null() || width <= 0 || height <= 0 {
        return None;
    }
    let byte_len = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
    unsafe {
        let screen = GetDC(null_mut());
        if screen.is_null() {
            return None;
        }

        let bitmap_info = BitmapInfo {
            bmi_header: BitmapInfoHeader {
                bi_size: size_of::<BitmapInfoHeader>() as u32,
                bi_width: width,
                bi_height: -height,
                bi_planes: 1,
                bi_bit_count: 32,
                bi_compression: BI_RGB,
                bi_size_image: byte_len.min(u32::MAX as usize) as u32,
                bi_x_pels_per_meter: 0,
                bi_y_pels_per_meter: 0,
                bi_clr_used: 0,
                bi_clr_important: 0,
            },
            bmi_colors: [RgbQuad { blue: 0, green: 0, red: 0, reserved: 0 }],
        };

        let mut bits = null_mut();
        let dib = CreateDIBSection(screen, &bitmap_info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        let memory_dc = CreateCompatibleDC(screen);
        let _ = ReleaseDC(null_mut(), screen);
        if dib.is_null() || memory_dc.is_null() || bits.is_null() {
            if !dib.is_null() {
                let _ = DeleteObject(dib);
            }
            if !memory_dc.is_null() {
                let _ = DeleteDC(memory_dc);
            }
            return None;
        }

        let old_object = SelectObject(memory_dc, dib);
        std::ptr::write_bytes(bits as *mut u8, 0, byte_len);
        let drawn = DrawIconEx(memory_dc, 0, 0, icon, width, height, 0, null_mut(), DI_NORMAL) != 0;
        let mut bgra = vec![0u8; byte_len];
        if drawn {
            std::ptr::copy_nonoverlapping(bits as *const u8, bgra.as_mut_ptr(), byte_len);
        }
        if !old_object.is_null() {
            let _ = SelectObject(memory_dc, old_object);
        }
        let _ = DeleteDC(memory_dc);
        let _ = DeleteObject(dib);
        drawn.then_some(bgra)
    }
}

pub fn extract_icon(path: &str, size: u32) -> Option<RawIcon> {
    let requested = size.clamp(1, 512) as i32;
    let wide_path = wide(path);
    let mut icon = null_mut();

    unsafe {
        let mut info: ShellFileInfoW = zeroed();
        let found = SHGetFileInfoW(
            wide_path.as_ptr(),
            0,
            &mut info,
            size_of::<ShellFileInfoW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        ) != 0;
        if found && !info.h_icon.is_null() {
            icon = info.h_icon;
        }

        if icon.is_null() {
            let mut large = null_mut();
            if ExtractIconExW(wide_path.as_ptr(), 0, &mut large, null_mut(), 1) > 0 {
                icon = large;
            }
        }

        if icon.is_null() {
            let unknown = wide("unknown");
            let mut info: ShellFileInfoW = zeroed();
            if SHGetFileInfoW(
                unknown.as_ptr(),
                FILE_ATTRIBUTE_NORMAL,
                &mut info,
                size_of::<ShellFileInfoW>() as u32,
                SHGFI_ICON | SHGFI_USEFILEATTRIBUTES,
            ) != 0
            {
                icon = info.h_icon;
            }
        }
        if icon.is_null() {
            return None;
        }

        let mut actual_width = requested;
        let mut actual_height = requested;
        let mut icon_info: IconInfo = zeroed();
        if GetIconInfo(icon, &mut icon_info) != 0 {
            if !icon_info.hbm_color.is_null() {
                let mut bitmap: Bitmap = zeroed();
                if GetObjectW(
                    icon_info.hbm_color,
                    size_of::<Bitmap>() as i32,
                    &mut bitmap as *mut Bitmap as *mut c_void,
                ) == size_of::<Bitmap>() as i32
                {
                    if bitmap.bm_width > 0 {
                        actual_width = actual_width.min(bitmap.bm_width);
                    }
                    if bitmap.bm_height > 0 {
                        actual_height = actual_height.min(bitmap.bm_height);
                    }
                }
            }
            if !icon_info.hbm_color.is_null() {
                let _ = DeleteObject(icon_info.hbm_color);
            }
            if !icon_info.hbm_mask.is_null() {
                let _ = DeleteObject(icon_info.hbm_mask);
            }
        }

        let pixels = icon_to_bgra(icon, actual_width, actual_height);
        let _ = DestroyIcon(icon);
        let bgra = pixels?;
        let mut rgba = vec![0u8; bgra.len()];
        let mut any_alpha = false;
        for pixel in bgra.chunks_exact(4) {
            if pixel[3] != 0 {
                any_alpha = true;
                break;
            }
        }
        for (src, dst) in bgra.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
            dst[0] = src[2];
            dst[1] = src[1];
            dst[2] = src[0];
            dst[3] = if any_alpha { src[3] } else { 255 };
        }
        Some(RawIcon {
            w: actual_width as usize,
            h: actual_height as usize,
            rgba,
        })
    }
}

pub fn open_in_explorer(path: &str, select: bool) -> i32 {
    let explorer = wide("explorer.exe");
    let verb = wide("open");
    let argument = if select {
        format!("/select,\"{}\"", path)
    } else {
        format!("\"{}\"", path)
    };
    let arguments = wide(&argument);
    unsafe {
        let result = ShellExecuteW(
            null_mut(),
            verb.as_ptr(),
            explorer.as_ptr(),
            arguments.as_ptr(),
            null(),
            SW_SHOWNORMAL,
        );
        if result > 32 { 0 } else { result as i32 }
    }
}

pub fn show_properties(path: &str) -> i32 {
    let verb = wide("properties");
    let file = wide(path);
    let initialized = unsafe {
        CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) >= 0
    };
    let mut info: ShellExecuteInfoW = unsafe { zeroed() };
    info.cb_size = size_of::<ShellExecuteInfoW>() as u32;
    info.f_mask = SEE_MASK_INVOKEIDLIST;
    info.lp_verb = verb.as_ptr();
    info.lp_file = file.as_ptr();
    info.n_show = SW_SHOWNORMAL;
    let result = unsafe {
        if ShellExecuteExW(&mut info) != 0 {
            0
        } else {
            GetLastError() as i32
        }
    };
    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}

pub fn recycle(path: &str) -> i32 {
    let normalized = long_path(path);
    let mut from = wide(&normalized);
    from.push(0);
    let mut operation: ShFileOperationW = unsafe { zeroed() };
    operation.w_func = FO_DELETE;
    operation.p_from = from.as_ptr();
    operation.f_flags = FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI;
    unsafe { SHFileOperationW(&mut operation) }
}

pub fn shell_open(what: &str) -> i32 {
    let verb = wide("open");
    let target = wide(what);
    unsafe {
        let result = ShellExecuteW(null_mut(), verb.as_ptr(), target.as_ptr(), null(), null(), SW_SHOWNORMAL);
        if result > 32 { 0 } else { result as i32 }
    }
}

fn make_startup_info(hidden: bool) -> StartupInfoW {
    let mut startup: StartupInfoW = unsafe { zeroed() };
    startup.cb = size_of::<StartupInfoW>() as u32;
    if hidden {
        startup.dw_flags = STARTF_USESHOWWINDOW;
        startup.w_show_window = SW_HIDE;
    }
    startup
}

fn start_process(
    application: Option<&str>,
    command_line: &str,
    current_directory: Option<&str>,
    hidden: bool,
) -> Result<ProcessInformation, u32> {
    let application_wide = application.map(wide);
    let mut command_wide = wide(command_line);
    let current_directory_wide = current_directory.map(wide);
    let startup = make_startup_info(hidden);
    let mut info: ProcessInformation = unsafe { zeroed() };

    let ok = unsafe {
        CreateProcessW(
            application_wide.as_ref().map_or(null(), |v| v.as_ptr()),
            command_wide.as_mut_ptr(),
            null(),
            null(),
            0,
            if hidden { CREATE_NO_WINDOW } else { 0 },
            null_mut(),
            current_directory_wide.as_ref().map_or(null(), |v| v.as_ptr()),
            &startup,
            &mut info,
        )
    };
    if ok == 0 {
        Err(unsafe { GetLastError() })
    } else {
        Ok(info)
    }
}

pub fn run_cmd(command_line: &str, wait: bool) -> (i32, u32) {
    let info = match start_process(None, command_line, None, true) {
        Ok(info) => info,
        Err(error) => return (error as i32, 0),
    };
    let mut code = 0u32;
    unsafe {
        if wait {
            let _ = WaitForSingleObject(info.h_process, INFINITE);
            let _ = GetExitCodeProcess(info.h_process, &mut code);
        }
        let _ = CloseHandle(info.h_thread);
        let _ = CloseHandle(info.h_process);
    }
    (0, code)
}

fn quote_windows_argument(argument: &str) -> String {
    if !argument.is_empty()
        && !argument.chars().any(|c| matches!(c, ' ' | '\t' | '\n' | '\u{000b}' | '"'))
    {
        return argument.to_string();
    }
    let mut out = String::from("\"");
    let mut backslashes = 0usize;
    for ch in argument.chars() {
        if ch == '\\' {
            backslashes += 1;
        } else if ch == '"' {
            out.push_str(&"\\".repeat(backslashes * 2 + 1));
            out.push('"');
            backslashes = 0;
        } else {
            out.push_str(&"\\".repeat(backslashes));
            backslashes = 0;
            out.push(ch);
        }
    }
    out.push_str(&"\\".repeat(backslashes * 2));
    out.push('"');
    out
}

pub fn run_shell(file: &str, params: &str, cwd: Option<&str>, wait: bool) -> (i32, u32) {
    let command_line = if params.is_empty() {
        quote_windows_argument(file)
    } else {
        format!("{} {}", quote_windows_argument(file), params)
    };
    let info = match start_process(Some(file), &command_line, cwd, false) {
        Ok(info) => info,
        Err(error) => return (error as i32, 0),
    };
    let mut code = 0u32;
    unsafe {
        if wait {
            let _ = WaitForSingleObject(info.h_process, INFINITE);
            let _ = GetExitCodeProcess(info.h_process, &mut code);
        }
        let _ = CloseHandle(info.h_thread);
        let _ = CloseHandle(info.h_process);
    }
    (0, code)
}

pub fn run_elevated(file: &str, params: &str, cwd: Option<&str>, wait: bool) -> (i32, u32) {
    let verb = wide("runas");
    let file_wide = wide(file);
    let params_wide = wide(params);
    let cwd_wide = cwd.map(wide);
    let initialized = unsafe {
        CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) >= 0
    };
    let mut info: ShellExecuteInfoW = unsafe { zeroed() };
    info.cb_size = size_of::<ShellExecuteInfoW>() as u32;
    info.f_mask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    info.lp_verb = verb.as_ptr();
    info.lp_file = file_wide.as_ptr();
    info.lp_parameters = params_wide.as_ptr();
    info.lp_directory = cwd_wide.as_ref().map_or(null(), |v| v.as_ptr());
    info.n_show = SW_SHOWNORMAL;

    let (rc, mut exit_code) = unsafe {
        if ShellExecuteExW(&mut info) == 0 {
            (GetLastError() as i32, 0)
        } else {
            let mut code = 0u32;
            if !info.h_process.is_null() {
                if wait {
                    let _ = WaitForSingleObject(info.h_process, INFINITE);
                    let _ = GetExitCodeProcess(info.h_process, &mut code);
                }
                let _ = CloseHandle(info.h_process);
            }
            (0, code)
        }
    };
    if initialized {
        unsafe { CoUninitialize() };
    }
    if !wait {
        exit_code = 0;
    }
    (rc, exit_code)
}

fn decode_multibyte(data: &[u8], code_page: u32, flags: u32) -> Option<String> {
    if data.is_empty() {
        return Some(String::new());
    }
    let byte_len = i32::try_from(data.len()).ok()?;
    unsafe {
        let needed = MultiByteToWideChar(code_page, flags, data.as_ptr(), byte_len, null_mut(), 0);
        if needed <= 0 {
            return None;
        }
        let mut result = vec![0u16; needed as usize];
        let converted = MultiByteToWideChar(
            code_page,
            flags,
            data.as_ptr(),
            byte_len,
            result.as_mut_ptr(),
            needed,
        );
        if converted <= 0 {
            return None;
        }
        result.truncate(converted as usize);
        Some(String::from_utf16_lossy(&result))
    }
}

pub fn capture_cmd(command_line: &str) -> String {
    let mut read_pipe = null_mut();
    let mut write_pipe = null_mut();
    let security = SecurityAttributes {
        length: size_of::<SecurityAttributes>() as u32,
        security_descriptor: null_mut(),
        inherit_handle: 1,
    };
    unsafe {
        if CreatePipe(&mut read_pipe, &mut write_pipe, &security, 0) == 0 {
            return String::new();
        }
        if SetHandleInformation(read_pipe, HANDLE_FLAG_INHERIT, 0) == 0 {
            let _ = CloseHandle(read_pipe);
            let _ = CloseHandle(write_pipe);
            return String::new();
        }
    }

    let mut command_wide = wide(command_line);
    let startup = {
        let mut startup: StartupInfoW = unsafe { zeroed() };
        startup.cb = size_of::<StartupInfoW>() as u32;
        startup.dw_flags = STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
        startup.h_std_output = write_pipe;
        startup.h_std_error = write_pipe;
        startup.h_std_input = null_mut();
        startup.w_show_window = SW_HIDE;
        startup
    };
    let mut info: ProcessInformation = unsafe { zeroed() };
    let created = unsafe {
        CreateProcessW(
            null(),
            command_wide.as_mut_ptr(),
            null(),
            null(),
            1,
            CREATE_NO_WINDOW,
            null_mut(),
            null(),
            &startup,
            &mut info,
        )
    };
    unsafe { let _ = CloseHandle(write_pipe); }
    if created == 0 {
        unsafe { let _ = CloseHandle(read_pipe); }
        return String::new();
    }

    let mut data = Vec::with_capacity(8192);
    let mut chunk = [0u8; 4096];
    while data.len() < 512 * 1024 {
        let mut received = 0u32;
        let ok = unsafe {
            ReadFile(
                read_pipe,
                chunk.as_mut_ptr() as *mut c_void,
                chunk.len() as u32,
                &mut received,
                null_mut(),
            )
        };
        if ok == 0 || received == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..received as usize]);
    }

    let wait = unsafe { WaitForSingleObject(info.h_process, 20_000) };
    if wait == WAIT_TIMEOUT {
        unsafe {
            let _ = TerminateProcess(info.h_process, 1);
            let _ = WaitForSingleObject(info.h_process, 1000);
        }
    }
    unsafe {
        let _ = CloseHandle(info.h_thread);
        let _ = CloseHandle(info.h_process);
        let _ = CloseHandle(read_pipe);
    }

    decode_multibyte(&data, CP_UTF8, MB_ERR_INVALID_CHARS)
        .or_else(|| decode_multibyte(&data, CP_ACP, 0))
        .unwrap_or_default()
}

const CLSID_SHELL_LINK: Guid = Guid {
    data1: 0x0002_1401,
    data2: 0,
    data3: 0,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};
const IID_SHELL_LINK_W: Guid = Guid {
    data1: 0x0002_14f9,
    data2: 0,
    data3: 0,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};
const IID_PERSIST_FILE: Guid = Guid {
    data1: 0x0000_010b,
    data2: 0,
    data3: 0,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

unsafe fn release_com_object(object: *mut c_void) {
    if object.is_null() {
        return;
    }
    let vtable = *(object as *mut *const UnknownVTable);
    let release: unsafe extern "system" fn(*mut c_void) -> u32 = transmute((*vtable).methods[2]);
    let _ = release(object);
}

pub fn resolve_shortcut(link_path: &str) -> Option<(String, String, String)> {
    let initialized = unsafe {
        CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) >= 0
    };
    let mut link: *mut c_void = null_mut();
    let create_hr = unsafe {
        CoCreateInstance(
            &CLSID_SHELL_LINK,
            null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_SHELL_LINK_W,
            &mut link,
        )
    };
    if create_hr < 0 || link.is_null() {
        if initialized {
            unsafe { CoUninitialize() };
        }
        return None;
    }

    let link_vtable = unsafe { *(link as *mut *const ShellLinkVTable) };
    let query_interface: unsafe extern "system" fn(
        *mut c_void,
        *const Guid,
        *mut *mut c_void,
    ) -> HResult = unsafe { transmute((*link_vtable).methods[0]) };
    let mut persist: *mut c_void = null_mut();
    let query_hr = unsafe { query_interface(link, &IID_PERSIST_FILE, &mut persist) };
    if query_hr < 0 || persist.is_null() {
        unsafe { release_com_object(link) };
        if initialized {
            unsafe { CoUninitialize() };
        }
        return None;
    }

    let persist_vtable = unsafe { *(persist as *mut *const PersistFileVTable) };
    let load: unsafe extern "system" fn(*mut c_void, *const u16, u32) -> HResult =
        unsafe { transmute((*persist_vtable).methods[5]) };
    let link_wide = wide(link_path);
    let load_hr = unsafe { load(persist, link_wide.as_ptr(), STGM_READ) };

    let mut target = vec![0u16; 4096];
    let mut arguments = vec![0u16; 4096];
    let mut working_directory = vec![0u16; 4096];
    let mut result = None;
    if load_hr >= 0 {
        let get_path: unsafe extern "system" fn(
            *mut c_void,
            *mut u16,
            i32,
            *mut c_void,
            u32,
        ) -> HResult = unsafe { transmute((*link_vtable).methods[3]) };
        let get_working_directory: unsafe extern "system" fn(
            *mut c_void,
            *mut u16,
            i32,
        ) -> HResult = unsafe { transmute((*link_vtable).methods[8]) };
        let get_arguments: unsafe extern "system" fn(
            *mut c_void,
            *mut u16,
            i32,
        ) -> HResult = unsafe { transmute((*link_vtable).methods[10]) };

        let _ = unsafe {
            get_path(
                link,
                target.as_mut_ptr(),
                target.len() as i32,
                null_mut(),
                SLGP_UNCPRIORITY,
            )
        };
        let _ = unsafe { get_arguments(link, arguments.as_mut_ptr(), arguments.len() as i32) };
        let _ = unsafe {
            get_working_directory(
                link,
                working_directory.as_mut_ptr(),
                working_directory.len() as i32,
            )
        };
        let to_string = |buffer: &[u16]| {
            let end = buffer.iter().position(|u| *u == 0).unwrap_or(buffer.len());
            String::from_utf16_lossy(&buffer[..end])
        };
        result = Some((
            to_string(&target),
            to_string(&arguments),
            to_string(&working_directory),
        ));
    }

    unsafe {
        release_com_object(persist);
        release_com_object(link);
        if initialized {
            CoUninitialize();
        }
    }
    result
}

fn process_image_path(process: Handle) -> Option<String> {
    let mut image = vec![0u16; 32768];
    let mut length = image.len() as u32;
    let ok = unsafe {
        QueryFullProcessImageNameW(process, 0, image.as_mut_ptr(), &mut length)
    };
    if ok == 0 || length == 0 {
        return None;
    }
    image.truncate(length as usize);
    Some(String::from_utf16_lossy(&image))
}

fn count_or_kill_processes(path: &str, terminate: bool) -> i32 {
    if path.trim().is_empty() {
        return 0;
    }
    let folder = normalize_path(path).trim_end_matches(|ch| ch == '\\' || ch == '/').to_string();
    if folder.is_empty() {
        return 0;
    }
    let self_id = unsafe { GetCurrentProcessId() };
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot == -1isize as Handle {
        return 0;
    }

    let mut entry: ProcessEntry32W = unsafe { zeroed() };
    entry.dw_size = size_of::<ProcessEntry32W>() as u32;
    let mut count = 0i32;
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while has_entry {
        if entry.th32_process_id != self_id && entry.th32_process_id != 0 {
            let access = PROCESS_QUERY_LIMITED_INFORMATION
                | if terminate { PROCESS_TERMINATE } else { 0 };
            let process = unsafe { OpenProcess(access, 0, entry.th32_process_id) };
            if !process.is_null() {
                if let Some(image) = process_image_path(process) {
                    if path_is_under(path, &image) {
                        count += 1;
                        if terminate {
                            unsafe {
                                let _ = TerminateProcess(process, 1);
                                let _ = WaitForSingleObject(process, 1500);
                            }
                        }
                    }
                }
                unsafe { let _ = CloseHandle(process); }
            }
        }
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    unsafe { let _ = CloseHandle(snapshot); }
    count
}

pub fn count_processes_in(path: &str) -> i32 {
    count_or_kill_processes(path, false)
}

pub fn kill_processes_in(path: &str) -> i32 {
    count_or_kill_processes(path, true)
}

pub fn services_under(path: &str) -> Vec<String> {
    if path.trim().is_empty() {
        return Vec::new();
    }
    let folder = normalize_path(path).trim_end_matches(|ch| ch == '\\' || ch == '/').to_string();
    if folder.is_empty() {
        return Vec::new();
    }

    unsafe {
        let manager = OpenSCManagerW(null(), null(), SC_MANAGER_ENUMERATE_SERVICE);
        if manager.is_null() {
            return Vec::new();
        }

        let mut needed = 0u32;
        let mut returned = 0u32;
        let mut resume = 0u32;
        let _ = EnumServicesStatusExW(
            manager,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            null_mut(),
            0,
            &mut needed,
            &mut returned,
            &mut resume,
            null(),
        );
        let mut buffer = vec![0u8; (needed as usize).max(4096)];
        let ok = EnumServicesStatusExW(
            manager,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            buffer.as_mut_ptr(),
            buffer.len().min(u32::MAX as usize) as u32,
            &mut needed,
            &mut returned,
            &mut resume,
            null(),
        ) != 0;
        let mut names = Vec::new();

        if ok {
            let services = std::slice::from_raw_parts(
                buffer.as_ptr() as *const EnumServiceStatusProcessW,
                returned as usize,
            );
            for item in services {
                if item.service_name.is_null() {
                    continue;
                }
                let service = OpenServiceW(manager, item.service_name, SERVICE_QUERY_CONFIG);
                if service.is_null() {
                    continue;
                }

                let mut config_needed = 0u32;
                let _ = QueryServiceConfigW(service, null_mut(), 0, &mut config_needed);
                if config_needed > 0 {
                    let mut config_buffer = vec![0u8; config_needed as usize];
                    if QueryServiceConfigW(
                        service,
                        config_buffer.as_mut_ptr() as *mut QueryServiceConfigW,
                        config_buffer.len().min(u32::MAX as usize) as u32,
                        &mut config_needed,
                    ) != 0
                    {
                        let config = &*(config_buffer.as_ptr() as *const QueryServiceConfigW);
                        let executable = command_executable(wide_ptr_to_string(config.binary_path_name as *const u16).as_str());
                        if path_is_under(path, &executable) {
                            names.push(wide_ptr_to_string(item.service_name));
                        }
                    }
                }
                let _ = CloseServiceHandle(service);
            }
        }
        let _ = CloseServiceHandle(manager);
        names
    }
}

pub fn stop_delete_service(name: &str) -> i32 {
    let name_wide = wide(name);
    unsafe {
        let manager = OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT);
        if manager.is_null() {
            return GetLastError() as i32;
        }
        let service = OpenServiceW(
            manager,
            name_wide.as_ptr(),
            SERVICE_STOP | DELETE_ACCESS | SERVICE_QUERY_STATUS,
        );
        if service.is_null() {
            let error = GetLastError() as i32;
            let _ = CloseServiceHandle(manager);
            return error;
        }

        let mut status: ServiceStatus = zeroed();
        let _ = ControlService(service, SERVICE_CONTROL_STOP, &mut status);
        for _ in 0..20 {
            let mut current: ServiceStatus = zeroed();
            if QueryServiceStatus(service, &mut current) == 0 {
                break;
            }
            if current.current_state == SERVICE_STOPPED {
                break;
            }
            Sleep(150);
        }

        let result = if DeleteService(service) != 0 {
            0
        } else {
            GetLastError() as i32
        };
        let _ = CloseServiceHandle(service);
        let _ = CloseServiceHandle(manager);
        result
    }
}

struct DeleteStats {
    deleted: i32,
    failed: i32,
    first_error: String,
}

fn clear_readonly(path: &Path) {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        let mut permissions = metadata.permissions();
        if permissions.readonly() {
            permissions.set_readonly(false);
            let _ = std::fs::set_permissions(path, permissions);
        }
    }
}

fn is_directory_reparse_point(path: &Path) -> bool {
    let path = wide(&long_path(&path.to_string_lossy()));
    let attributes = unsafe { GetFileAttributesW(path.as_ptr()) };
    attributes != u32::MAX
        && (attributes & FILE_ATTRIBUTE_DIRECTORY) != 0
}

fn powershell_base64(script: &str) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut utf16 = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    let mut encoded = String::with_capacity(utf16.len().div_ceil(3) * 4);
    let mut index = 0usize;
    while index < utf16.len() {
        let a = utf16[index];
        let b = utf16.get(index + 1).copied();
        let c = utf16.get(index + 2).copied();
        encoded.push(TABLE[(a >> 2) as usize] as char);
        encoded.push(TABLE[(((a & 0x03) << 4) | (b.unwrap_or(0) >> 4)) as usize] as char);
        match b {
            Some(b) => encoded.push(TABLE[(((b & 0x0f) << 2) | (c.unwrap_or(0) >> 6)) as usize] as char),
            None => encoded.push('='),
        }
        match c {
            Some(c) => encoded.push(TABLE[(c & 0x3f) as usize] as char),
            None => encoded.push('='),
        }
        index += 3;
    }
    encoded
}

fn ps_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn run_powershell(script: &str) -> Result<Output, String> {
    let encoded = powershell_base64(script);
    Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encoded,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| error.to_string())
}

fn acl_snapshot_and_relax(path: &Path) -> Option<PathBuf> {
    let snapshot = std::env::temp_dir().join(format!(
        "appsweep-acl-{}-{}.sddl",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    let p = ps_literal(&path.to_string_lossy());
    let s = ps_literal(&snapshot.to_string_lossy());
    let script = format!(
        "$ErrorActionPreference='Stop'; $p={p}; $s={s}; try {{ \
         $acl=Get-Acl -LiteralPath $p; \
         [IO.File]::WriteAllText($s,$acl.GetSecurityDescriptorSddlForm([Security.AccessControl.AccessControlSections]::All),[Text.Encoding]::Unicode); \
         & takeown.exe /F $p /A *> $null; if ($LASTEXITCODE -ne 0) {{ throw 'takeown failed' }}; \
         & icacls.exe $p /inheritance:r /grant:r '*S-1-5-32-544:F' *> $null; if ($LASTEXITCODE -ne 0) {{ throw 'icacls failed' }}; \
         exit 0 }} catch {{ if (Test-Path -LiteralPath $s) {{ try {{ $acl=Get-Acl -LiteralPath $p; $sddl=[IO.File]::ReadAllText($s,[Text.Encoding]::Unicode); $acl.SetSecurityDescriptorSddlForm($sddl,[Security.AccessControl.AccessControlSections]::All); Set-Acl -LiteralPath $p -AclObject $acl }} catch {{}} }}; exit 1 }}"
    );
    let result = run_powershell(&script).ok()?;
    if result.status.success() && snapshot.exists() {
        Some(snapshot)
    } else {
        let _ = std::fs::remove_file(snapshot);
        None
    }
}

fn restore_acl_snapshot(path: &Path, snapshot: &Path) {
    if path.exists() {
        let p = ps_literal(&path.to_string_lossy());
        let s = ps_literal(&snapshot.to_string_lossy());
        let script = format!(
            "$ErrorActionPreference='Stop'; $p={p}; $s={s}; \
             if ((Test-Path -LiteralPath $p) -and (Test-Path -LiteralPath $s)) {{ \
             $acl=Get-Acl -LiteralPath $p; $sddl=[IO.File]::ReadAllText($s,[Text.Encoding]::Unicode); \
             $acl.SetSecurityDescriptorSddlForm($sddl,[Security.AccessControl.AccessControlSections]::All); \
             Set-Acl -LiteralPath $p -AclObject $acl }}"
        );
        let _ = run_powershell(&script);
    }
    let _ = std::fs::remove_file(snapshot);
}

fn remove_object(path: &Path) -> std::io::Result<()> {
    if is_directory_reparse_point(path) {
        std::fs::remove_dir(path)
    } else if path.is_dir() {
        std::fs::remove_dir(path)
    } else {
        std::fs::remove_file(path)
    }
}

fn delete_tree(path: &Path, stats: &mut DeleteStats, depth: u32) {
    if depth > 64 {
        return;
    }
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(_) => {
            stats.failed += 1;
            if stats.first_error.is_empty() {
                stats.first_error = path.to_string_lossy().into_owned();
            }
            return;
        }
    };

    if metadata.file_type().is_symlink() {
        if remove_object(path).is_ok() {
            stats.deleted += 1;
        } else if stats.first_error.is_empty() {
            stats.failed += 1;
            stats.first_error = format!("{} (link)", path.display());
        } else {
            stats.failed += 1;
        }
        return;
    }

    if metadata.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                delete_tree(&entry.path(), stats, depth + 1);
            }
        }
        clear_readonly(path);
        match std::fs::remove_dir(path) {
            Ok(()) => stats.deleted += 1,
            Err(_) => {
                let snapshot = acl_snapshot_and_relax(path);
                if snapshot.is_some() {
                    clear_readonly(path);
                    if std::fs::remove_dir(path).is_ok() {
                        stats.deleted += 1;
                        let _ = std::fs::remove_file(snapshot.as_ref().unwrap());
                        return;
                    }
                    restore_acl_snapshot(path, snapshot.as_ref().unwrap());
                }
                stats.failed += 1;
                if stats.first_error.is_empty() {
                    stats.first_error = format!("{} (dir)", path.display());
                }
            }
        }
    } else {
        clear_readonly(path);
        match std::fs::remove_file(path) {
            Ok(()) => stats.deleted += 1,
            Err(_) => {
                let snapshot = acl_snapshot_and_relax(path);
                if snapshot.is_some() {
                    clear_readonly(path);
                    if std::fs::remove_file(path).is_ok() {
                        stats.deleted += 1;
                        let _ = std::fs::remove_file(snapshot.as_ref().unwrap());
                        return;
                    }
                    restore_acl_snapshot(path, snapshot.as_ref().unwrap());
                }
                stats.failed += 1;
                if stats.first_error.is_empty() {
                    stats.first_error = format!("{} (file)", path.display());
                }
            }
        }
    }
}

pub struct DeleteReport {
    pub deleted: i32,
    pub failed: i32,
    pub first_error: String,
}

fn is_root_path(path: &str) -> bool {
    let normalized = path.replace('/', "\\\\");
    let trimmed = normalized.trim_end_matches(|ch| ch == '\\' || ch == '/');
    if trimmed.is_empty() {
        return true;
    }
    let bytes = trimmed.as_bytes();
    if bytes.len() == 2 && bytes[1] == b':' {
        return true;
    }
    if trimmed.starts_with("\\\\\\\\") {
        let parts = trimmed.trim_start_matches('\\\\').split('\\\\').filter(|part| !part.is_empty()).count();
        return parts <= 2;
    }
    false
}

pub fn force_delete(path: &str) -> DeleteReport {
    let mut stats = DeleteStats {
        deleted: 0,
        failed: 0,
        first_error: String::new(),
    };
    if is_root_path(path) {
        stats.failed = 1;
        stats.first_error = format!("Refusing to delete a filesystem root: {}", path);
    } else {
        let mut raw = path.to_string();
        while raw.ends_with('\\') || raw.ends_with('/') {
            raw.pop();
        }
        if !raw.is_empty() {
            let long = long_path(&raw);
            delete_tree(Path::new(&long), &mut stats, 0);
        }
    }
    DeleteReport {
        deleted: stats.deleted,
        failed: stats.failed,
        first_error: stats.first_error,
    }
}

/// Self-test helper: returns 1 when an allowed Everyone ACE grants full control,
/// 0 when none is present, and -1 when the ACL could not be read.
pub fn grants_everyone_full_control(path: &str) -> i32 {
    let p = ps_literal(path);
    let script = format!(
        "$ErrorActionPreference='Stop'; $acl=Get-Acl -LiteralPath {p}; $sid='S-1-1-0'; \
         $full=[int][IO.FileSystemRights]::FullControl; $found=$false; \
         foreach ($rule in $acl.Access) {{ try {{ \
         if ($rule.AccessControlType -eq [Security.AccessControl.AccessControlType]::Allow -and \
         $rule.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value -eq $sid -and \
         (([int]$rule.FileSystemRights -band $full) -eq $full)) {{ $found=$true }} \
         }} catch {{}} }}; if ($found) {{ '1' }} else {{ '0' }}"
    );
    match run_powershell(&script) {
        Ok(output) if output.status.success() => {
            if String::from_utf8_lossy(&output.stdout).trim() == "1" { 1 } else { 0 }
        }
        _ => -1,
    }
}

/// Self-test only: add an explicit, non-inheriting Everyone full-control ACE.
pub fn grant_everyone_full_control(path: &str) -> i32 {
    let p = ps_literal(path);
    let script = format!(
        "$ErrorActionPreference='Stop'; $p={p}; $acl=Get-Acl -LiteralPath $p; \
         $sid=New-Object Security.Principal.SecurityIdentifier('S-1-1-0'); \
         $rule=New-Object Security.AccessControl.FileSystemAccessRule($sid,[IO.FileSystemRights]::FullControl,[Security.AccessControl.InheritanceFlags]::None,[Security.AccessControl.PropagationFlags]::None,[Security.AccessControl.AccessControlType]::Allow); \
         $acl.SetAccessRule($rule); Set-Acl -LiteralPath $p -AclObject $acl"
    );
    match run_powershell(&script) {
        Ok(output) if output.status.success() => 0,
        Ok(output) => output.status.code().unwrap_or(1),
        Err(_) => 1,
    }
}

pub fn path_exists(path: &str) -> bool {
    std::fs::symlink_metadata(long_path(path)).is_ok()
}

pub fn delete_on_reboot(path: &str) -> i32 {
    let normalized = long_path(path);
    let path_wide = wide(&normalized);
    unsafe {
        if MoveFileExW(path_wide.as_ptr(), null(), MOVEFILE_DELAY_UNTIL_REBOOT) != 0 {
            0
        } else {
            GetLastError() as i32
        }
    }
}

pub fn create_restore_point(description: &str) -> i32 {
    type RestoreFn = unsafe extern "system" fn(*mut RestorePointInfoW, *mut StateManagerStatus) -> Bool;
    unsafe {
        let library = wide("srclient.dll");
        let module = LoadLibraryW(library.as_ptr());
        if module.is_null() {
            return GetLastError() as i32;
        }
        let symbol = GetProcAddress(module, b"SRSetRestorePointW\0".as_ptr() as *const c_char);
        if symbol.is_null() {
            let _ = FreeLibrary(module);
            return 127; // ERROR_PROC_NOT_FOUND
        }
        let restore: RestoreFn = transmute(symbol);
        let mut info: RestorePointInfoW = zeroed();
        info.event_type = 100; // BEGIN_SYSTEM_CHANGE
        info.restore_point_type = 0; // APPLICATION_INSTALL
        let mut status: StateManagerStatus = zeroed();
        status.event_type = info.event_type;
        status.restore_point_type = info.restore_point_type;
        for (index, unit) in description.encode_utf16().take(info.description.len() - 1).enumerate() {
            info.description[index] = unit;
        }
        if description.is_empty() {
            let fallback = wide("AppSweep");
            for (index, unit) in fallback.iter().take(info.description.len() - 1).enumerate() {
                if *unit == 0 { break; }
                info.description[index] = *unit;
            }
        }
        let ok = restore(&mut info, &mut status) != 0;
        let error = if ok { 0 } else { GetLastError() as i32 };
        let _ = FreeLibrary(module);
        error
    }
}

pub fn delete_scheduled_task(name: &str) -> i32 {
    let command = format!(
        "schtasks.exe /Delete /TN {} /F",
        quote_windows_argument(name)
    );
    run_cmd(&command, true).0
}

fn capture_command_oem(program: &str, arguments: &[&str]) -> String {
    let mut command = Command::new(program);
    command.args(arguments).creation_flags(CREATE_NO_WINDOW);
    let Ok(output) = command.output() else {
        return String::new();
    };
    decode_multibyte(&output.stdout, CP_OEMCP, 0)
        .or_else(|| decode_multibyte(&output.stdout, CP_ACP, 0))
        .unwrap_or_else(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn tasks_under(path: &str) -> Vec<String> {
    let mut needle = long_path(path).to_lowercase();
    if let Some(unprefixed) = needle.strip_prefix(r"\\?\") {
        needle = unprefixed.to_string();
    }
    let text = capture_command_oem("schtasks.exe", &["/Query", "/FO", "LIST", "/V"]);
    let mut result = Vec::new();
    let mut task_name = String::new();
    let mut task_hit = false;

    let flush = |task_name: &mut String, task_hit: &mut bool, result: &mut Vec<String>| {
        if !task_name.is_empty() && *task_hit && result.len() < 64 {
            result.push(std::mem::take(task_name));
        } else {
            task_name.clear();
        }
        *task_hit = false;
    };

    for original_line in text.lines() {
        let line = original_line.trim_end_matches(|ch| ch == '\r' || ch == ' ');
        if line.is_empty() {
            flush(&mut task_name, &mut task_hit, &mut result);
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        let key = &line[..colon];
        let value = line[colon + 1..].trim_start();
        if key == "TaskName" {
            task_name = value.to_string();
        } else if value.to_lowercase().contains(&needle) {
            task_hit = true;
        }
    }
    flush(&mut task_name, &mut task_hit, &mut result);
    result
}

pub fn path_size(path: &str) -> u64 {
    fn size(path: &Path, depth: u8) -> u64 {
        if depth > 8 {
            return 0;
        }
        let Ok(metadata) = std::fs::symlink_metadata(path) else {
            return 0;
        };
        if metadata.file_type().is_symlink() {
            return 0;
        }
        if metadata.is_file() {
            return metadata.len();
        }
        if !metadata.is_dir() {
            return 0;
        }
        let Ok(entries) = std::fs::read_dir(path) else {
            return 0;
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| size(&entry.path(), depth + 1))
            .sum()
    }
    size(Path::new(&long_path(path)), 0)
}

pub fn ensure_dir(path: &str) -> i32 {
    match std::fs::create_dir(path) {
        Ok(()) => 0,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => 0,
        Err(error) => error.raw_os_error().unwrap_or(ERROR_INVALID_PARAMETER),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_strings_are_nul_terminated_utf16() {
        assert_eq!(wide("ab"), vec![0x61, 0x62, 0]);
        assert_eq!(wide(""), vec![0]);
        assert_eq!(wide("ا"), vec![0x0627, 0]);
        assert_eq!(wide("😀"), vec![0xD83D, 0xDE00, 0]);
    }

    #[test]
    fn a_process_inside_a_sibling_folder_is_not_related() {
        assert!(path_is_under(r"C:\Apps\Foo", r"C:\Apps\Foo\app.exe"));
        assert!(path_is_under(r"C:\Apps\Foo", r"C:\Apps\Foo"));
        assert!(path_is_under(r"C:\Apps\Foo\", r"C:\Apps\Foo\app.exe"));
        assert!(path_is_under(r"c:\apps\foo", r"C:\Apps\Foo\App.EXE"));
        assert!(!path_is_under(r"C:\Apps\Foo", r"C:\Apps\Foobar\other.exe"));
        assert!(!path_is_under(r"C:\Apps\Foo", r"C:\Apps\Foo2"));
        assert!(!path_is_under(r"C:\Apps\Foo", r"C:\Apps\Fo"));
        assert!(!path_is_under(r"C:\Apps\Foo", r"D:\Tools\Foo\a.exe"));
        assert!(!path_is_under(r"C:\Apps\Foo", ""));
        assert!(path_is_under(r"\\server\share\Foo", r"\\server\share\Foo\a.exe"));
    }

    #[test]
    fn a_service_command_line_resolves_to_its_executable() {
        assert_eq!(
            command_executable(r#""C:\Apps\Foo\foo.exe" --service"#),
            r"C:\Apps\Foo\foo.exe"
        );
        assert_eq!(
            command_executable(r"C:\Apps\Foo\foo.exe -k netsvcs"),
            r"C:\Apps\Foo\foo.exe"
        );
        assert_eq!(
            command_executable(r#""C:\Program Files\Foo Bar\a b.exe" /run"#),
            r"C:\Program Files\Foo Bar\a b.exe"
        );
        assert_eq!(
            command_executable(r#"C:\Apps\Foobar\other.exe -c "C:\Apps\Foo""#),
            r"C:\Apps\Foobar\other.exe"
        );
    }

    #[test]
    fn windows_argument_quoting_preserves_spaces_and_trailing_slashes() {
        assert_eq!(quote_windows_argument("plain"), "plain");
        assert_eq!(quote_windows_argument("task with spaces"), r#""task with spaces""#);
        assert_eq!(quote_windows_argument(r"C:\Apps\Demo\"), r#""C:\Apps\Demo\\""#);
    }
}
