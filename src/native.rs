// Native Windows integration implemented directly in Rust.
#![allow(dead_code)]

use std::ffi::{c_char, c_void};
use std::mem::{size_of, transmute, zeroed};
use std::path::{Component, Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

type Handle = *mut c_void;
type Bool = i32;
type HResult = i32;

const ERROR_INVALID_PARAMETER: i32 = 87;
const ERROR_GEN_FAILURE: i32 = 31;
const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
const ERROR_MORE_DATA: u32 = 234;
const WAIT_TIMEOUT: u32 = 0x102;
const INFINITE: u32 = 0xffff_ffff;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const STARTF_USESHOWWINDOW: u32 = 0x0000_0001;
const STARTF_USESTDHANDLES: u32 = 0x0000_0100;
const SW_HIDE: u16 = 0;
const SW_SHOWNORMAL: i32 = 1;
const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_ADJUST_PRIVILEGES: u32 = 0x0020;
const SE_PRIVILEGE_ENABLED: u32 = 0x0002;
const ERROR_NOT_ALL_ASSIGNED: u32 = 1300;
const SE_FILE_OBJECT: u32 = 1;
const OWNER_SECURITY_INFORMATION: u32 = 0x0001;
const GROUP_SECURITY_INFORMATION: u32 = 0x0002;
const DACL_SECURITY_INFORMATION: u32 = 0x0004;
const PROTECTED_DACL_SECURITY_INFORMATION: u32 = 0x8000_0000;
const SET_ACCESS: u32 = 2;
const TRUSTEE_IS_SID: u32 = 0;
const TRUSTEE_IS_USER: u32 = 1;
const TRUSTEE_IS_GROUP: u32 = 2;
const TRUSTEE_IS_WELL_KNOWN_GROUP: u32 = 5;
const NO_INHERITANCE: u32 = 0;
const DELETE_FILE_ACCESS: u32 = 0x0001_0000;
const FILE_DELETE_CHILD: u32 = 0x0000_0040;
const FILE_GENERIC_READ: u32 = 0x0012_0089;
const FILE_GENERIC_WRITE: u32 = 0x0012_0116;
const READ_CONTROL: u32 = 0x0002_0000;
const SYNCHRONIZE: u32 = 0x0010_0000;
const FILE_ALL_ACCESS: u32 = 0x001f_01ff;
const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
const SECURITY_BUILTIN_DOMAIN_RID: u32 = 0x20;
const DOMAIN_ALIAS_RID_ADMINS: u32 = 0x220;
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
const SHGFI_SYSICONINDEX: u32 = 0x0000_4000;
const SHIL_LARGE: i32 = 0;
const SHIL_EXTRALARGE: i32 = 2;
const SHIL_JUMBO: i32 = 4;
const ILD_TRANSPARENT: u32 = 0x0001;
const SHGFI_USEFILEATTRIBUTES: u32 = 0x0000_0010;
const SHGFI_LARGEICON: u32 = 0x0000_0000;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const OPEN_EXISTING: u32 = 3;
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
const FILE_ATTRIBUTE_TAG_INFO_CLASS: i32 = 9;
const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;
const SE_DACL_PROTECTED: u16 = 0x1000;
const UNPROTECTED_DACL_SECURITY_INFORMATION: u32 = 0x2000_0000;
const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
const DI_NORMAL: u32 = 0x0003;
const COINIT_MULTITHREADED: u32 = 0x0;
const COINIT_APARTMENTTHREADED: u32 = 0x2;
const COINIT_DISABLE_OLE1DDE: u32 = 0x4;
const SECURITY_DESCRIPTOR_REVISION: u32 = 1;
const COM_RIGHTS_EXECUTE: u32 = 0x1;
const COM_RIGHTS_EXECUTE_LOCAL: u32 = 0x2;
const RPC_C_AUTHN_LEVEL_PKT_PRIVACY: u32 = 6;
const RPC_C_IMP_LEVEL_IDENTIFY: u32 = 2;
const EOAC_DISABLE_AAA: u32 = 0x1000;
const EOAC_NO_CUSTOM_MARSHAL: u32 = 0x2000;
const SYSTEM_RESTORE_BEGIN_SYSTEM_CHANGE: u32 = 100;
const SYSTEM_RESTORE_END_SYSTEM_CHANGE: u32 = 101;
const SYSTEM_RESTORE_APPLICATION_UNINSTALL: u32 = 1;
const CLSCTX_INPROC_SERVER: u32 = 0x1;
const STGM_READ: u32 = 0;
const SLGP_UNCPRIORITY: u32 = 0x2;
const SC_MANAGER_CONNECT: u32 = 0x1;
const SC_MANAGER_ENUMERATE_SERVICE: u32 = 0x4;
const SERVICE_QUERY_CONFIG: u32 = 0x0001;
const SERVICE_STOP: u32 = 0x0020;
const DELETE_ACCESS: u32 = 0x0001_0000;
const SERVICE_QUERY_STATUS: u32 = 0x0004;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_FAILED: u32 = 0xFFFF_FFFF;
const ERROR_INVALID_HANDLE: u32 = 6;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_PATH_NOT_FOUND: u32 = 3;
const ERROR_INVALID_NAME: u32 = 123;
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
struct Luid {
    low_part: u32,
    high_part: i32,
}

#[repr(C)]
struct LuidAndAttributes {
    luid: Luid,
    attributes: u32,
}

#[repr(C)]
struct TokenPrivileges {
    privilege_count: u32,
    privileges: [LuidAndAttributes; 1],
}

#[repr(C)]
struct SidIdentifierAuthority {
    value: [u8; 6],
}

#[repr(C)]
struct AbsoluteSecurityDescriptor {
    revision: u8,
    sbz1: u8,
    control: u16,
    owner: *mut c_void,
    group: *mut c_void,
    sacl: *mut c_void,
    dacl: *mut c_void,
}

#[repr(C)]
struct AclHeader {
    revision: u8,
    sbz1: u8,
    size: u16,
    ace_count: u16,
    sbz2: u16,
}

#[repr(C)]
struct AceHeader {
    ace_type: u8,
    ace_flags: u8,
    ace_size: u16,
}

#[repr(C)]
struct AccessAllowedAce {
    header: AceHeader,
    mask: u32,
    sid_start: u32,
}

#[repr(C)]
struct TrusteeW {
    multiple_trustee: *mut c_void,
    multiple_trustee_operation: u32,
    trustee_form: u32,
    trustee_type: u32,
    name: *mut u16,
}

#[repr(C)]
struct ExplicitAccessW {
    access_permissions: u32,
    access_mode: u32,
    inheritance: u32,
    trustee: TrusteeW,
}

struct SavedSecurity {
    descriptor: *mut c_void,
    owner: *mut c_void,
    group: *mut c_void,
}

impl Drop for SavedSecurity {
    fn drop(&mut self) {
        if !self.descriptor.is_null() {
            unsafe {
                let _ = LocalFree(self.descriptor);
            }
        }
    }
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
struct FileAttributeTagInfo {
    file_attributes: u32,
    reparse_tag: u32,
}

struct DirectoryHandle(Vec<Handle>);

impl Drop for DirectoryHandle {
    fn drop(&mut self) {
        for handle in self.0.drain(..).rev() {
            if !handle.is_null() && handle != -1isize as Handle {
                unsafe {
                    let _ = CloseHandle(handle);
                }
            }
        }
    }
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

#[repr(C, packed)]
struct RestorePointInfoW {
    event_type: u32,
    restore_point_type: u32,
    sequence_number: i64,
    description: [u16; 256],
}

#[repr(C, packed)]
struct StateManagerStatus {
    // Windows reports the failure reason here, not in GetLastError.
    status: u32,
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
struct ImageListVTable {
    methods: [usize; 32],
}

#[repr(C)]
struct PersistFileVTable {
    methods: [usize; 9],
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> Handle;
    fn GetCurrentProcessId() -> u32;
    fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
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
    fn PeekNamedPipe(
        pipe: Handle,
        buffer: *mut c_void,
        buffer_size: u32,
        bytes_read: *mut u32,
        total_bytes_available: *mut u32,
        bytes_left_this_message: *mut u32,
    ) -> Bool;
    fn CreateFileW(
        file_name: *const u16,
        desired_access: u32,
        share_mode: u32,
        security_attributes: *mut c_void,
        creation_disposition: u32,
        flags_and_attributes: u32,
        template_file: Handle,
    ) -> Handle;
    fn GetFileInformationByHandleEx(
        file: Handle,
        information_class: i32,
        file_information: *mut c_void,
        buffer_size: u32,
    ) -> Bool;
    fn TerminateProcess(process: Handle, exit_code: u32) -> Bool;
    fn Sleep(milliseconds: u32);
    fn GetModuleHandleW(module_name: *const u16) -> Handle;
    fn GetProcAddress(module: Handle, proc_name: *const c_char) -> *mut c_void;
    fn LoadLibraryExW(file_name: *const u16, file: Handle, flags: u32) -> Handle;
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
    fn SetFileAttributesW(file_name: *const u16, attributes: u32) -> Bool;
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
    fn ConvertStringSidToSidW(string_sid: *const u16, sid: *mut *mut c_void) -> Bool;
    fn InitializeSecurityDescriptor(security_descriptor: *mut c_void, revision: u32) -> Bool;
    fn SetSecurityDescriptorOwner(
        security_descriptor: *mut c_void,
        owner: *mut c_void,
        owner_defaulted: Bool,
    ) -> Bool;
    fn SetSecurityDescriptorGroup(
        security_descriptor: *mut c_void,
        group: *mut c_void,
        group_defaulted: Bool,
    ) -> Bool;
    fn SetSecurityDescriptorDacl(
        security_descriptor: *mut c_void,
        dacl_present: Bool,
        dacl: *mut c_void,
        dacl_defaulted: Bool,
    ) -> Bool;
    fn LookupPrivilegeValueW(system_name: *const u16, name: *const u16, luid: *mut Luid) -> Bool;
    fn AdjustTokenPrivileges(
        token: Handle,
        disable_all: Bool,
        new_state: *mut TokenPrivileges,
        buffer_length: u32,
        previous_state: *mut TokenPrivileges,
        return_length: *mut u32,
    ) -> Bool;
    fn AllocateAndInitializeSid(
        authority: *const SidIdentifierAuthority,
        sub_authority_count: u8,
        sub0: u32,
        sub1: u32,
        sub2: u32,
        sub3: u32,
        sub4: u32,
        sub5: u32,
        sub6: u32,
        sub7: u32,
        sid: *mut *mut c_void,
    ) -> Bool;
    fn FreeSid(sid: *mut c_void) -> *mut c_void;
    fn CheckTokenMembership(token: Handle, sid: *mut c_void, is_member: *mut Bool) -> Bool;
    fn GetNamedSecurityInfoW(
        object_name: *mut u16,
        object_type: u32,
        security_info: u32,
        owner: *mut *mut c_void,
        group: *mut *mut c_void,
        dacl: *mut *mut c_void,
        sacl: *mut *mut c_void,
        security_descriptor: *mut *mut c_void,
    ) -> u32;
    fn SetNamedSecurityInfoW(
        object_name: *mut u16,
        object_type: u32,
        security_info: u32,
        owner: *mut c_void,
        group: *mut c_void,
        dacl: *mut c_void,
        sacl: *mut c_void,
    ) -> u32;
    fn GetSecurityDescriptorDacl(
        security_descriptor: *mut c_void,
        dacl_present: *mut Bool,
        dacl: *mut *mut c_void,
        dacl_defaulted: *mut Bool,
    ) -> Bool;
    fn GetSecurityDescriptorControl(
        security_descriptor: *mut c_void,
        control: *mut u16,
        revision: *mut u32,
    ) -> Bool;
    fn SetEntriesInAclW(
        count: u32,
        entries: *const ExplicitAccessW,
        old_acl: *mut c_void,
        new_acl: *mut *mut c_void,
    ) -> u32;
    fn GetAce(acl: *mut c_void, index: u32, ace: *mut *mut c_void) -> Bool;
    fn EqualSid(first_sid: *mut c_void, second_sid: *mut c_void) -> Bool;
    fn OpenSCManagerW(
        machine_name: *const u16,
        database_name: *const u16,
        desired_access: u32,
    ) -> Handle;
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
    fn SHGetImageList(
        image_list: i32,
        interface_id: *const Guid,
        result: *mut *mut c_void,
    ) -> HResult;
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

struct ComApartment;

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

fn enter_com_mta() -> Result<ComApartment, i32> {
    let result = unsafe { CoInitializeEx(null_mut(), COINIT_MULTITHREADED) };
    if result < 0 {
        Err(result)
    } else {
        Ok(ComApartment)
    }
}

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> HResult;
    fn CoInitializeSecurity(
        security_descriptor: *mut c_void,
        authentication_services: i32,
        reserved_services: *mut c_void,
        reserved: *mut c_void,
        authentication_level: u32,
        impersonation_level: u32,
        authentication_list: *mut c_void,
        capabilities: u32,
        reserved3: *mut c_void,
    ) -> HResult;
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
    let folder = folder.trim_end_matches(['\\', '/']);
    if folder.is_empty() || candidate.len() < folder.len() || !candidate.starts_with(folder) {
        return false;
    }
    candidate.len() == folder.len()
        || candidate
            .as_bytes()
            .get(folder.len())
            .is_some_and(|c| *c == b'\\' || *c == b'/')
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
    normalized
        .strip_prefix(r"\\?\")
        .unwrap_or(&normalized)
        .to_string()
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
        let proc = GetProcAddress(module, c"RtlGetVersion".as_ptr());
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
    let byte_len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
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
            bmi_colors: [RgbQuad {
                blue: 0,
                green: 0,
                red: 0,
                reserved: 0,
            }],
        };

        let mut bits = null_mut();
        let dib = CreateDIBSection(
            screen,
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            null_mut(),
            0,
        );
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
        let drawn = DrawIconEx(
            memory_dc,
            0,
            0,
            icon,
            width,
            height,
            0,
            null_mut(),
            DI_NORMAL,
        ) != 0;
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
    let requested = if size == 0 {
        64
    } else {
        size.clamp(1, 512) as i32
    };
    let wide_path = wide(path);
    let mut icon = null_mut();

    unsafe {
        let mut info: ShellFileInfoW = zeroed();
        let have_index = SHGetFileInfoW(
            wide_path.as_ptr(),
            0,
            &mut info,
            size_of::<ShellFileInfoW>() as u32,
            SHGFI_SYSICONINDEX | SHGFI_LARGEICON,
        ) != 0;
        if have_index {
            let image_list_kind = if requested >= 200 {
                SHIL_JUMBO
            } else if requested >= 40 {
                SHIL_EXTRALARGE
            } else {
                SHIL_LARGE
            };
            let mut image_list = null_mut();
            let result = SHGetImageList(image_list_kind, &IID_IMAGE_LIST, &mut image_list);
            if !image_list.is_null() {
                if result >= 0 {
                    let vtable = *(image_list as *mut *const ImageListVTable);
                    let get_icon: unsafe extern "system" fn(
                        *mut c_void,
                        i32,
                        u32,
                        *mut Handle,
                    ) -> HResult = transmute((*vtable).methods[10]);
                    let _ = get_icon(image_list, info.i_icon, ILD_TRANSPARENT, &mut icon);
                }
                release_com_object(image_list);
            }
        }

        if icon.is_null() {
            let mut fallback: ShellFileInfoW = zeroed();
            let found = SHGetFileInfoW(
                wide_path.as_ptr(),
                0,
                &mut fallback,
                size_of::<ShellFileInfoW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            ) != 0;
            if found {
                icon = fallback.h_icon;
            }
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
        for pixel in bgra.as_chunks::<4>().0 {
            if pixel[3] != 0 {
                any_alpha = true;
                break;
            }
        }
        for (src, dst) in bgra
            .as_chunks::<4>()
            .0
            .iter()
            .zip(rgba.as_chunks_mut::<4>().0.iter_mut())
        {
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
        if result > 32 {
            0
        } else {
            result as i32
        }
    }
}

pub fn show_properties(path: &str) -> i32 {
    let verb = wide("properties");
    let file = wide(path);
    let initialized = unsafe {
        CoInitializeEx(
            null_mut(),
            COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
        ) >= 0
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
        let result = ShellExecuteW(
            null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        );
        if result > 32 {
            0
        } else {
            result as i32
        }
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
            current_directory_wide
                .as_ref()
                .map_or(null(), |v| v.as_ptr()),
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

/// Waits for a started process and reads its exit code. A failed wait or a
/// failed exit-code query is a failure, not a silent zero: callers treat a
/// non-zero return as "did not finish", so it must never be faked.
unsafe fn finish_process(process: Handle, wait: bool, code: &mut u32) -> i32 {
    if !wait {
        return 0;
    }
    let waited = WaitForSingleObject(process, INFINITE);
    if waited != WAIT_OBJECT_0 {
        let error = GetLastError();
        return if error == 0 {
            WAIT_FAILED as i32
        } else {
            error as i32
        };
    }
    if GetExitCodeProcess(process, code) == 0 {
        let error = GetLastError();
        return if error == 0 {
            WAIT_FAILED as i32
        } else {
            error as i32
        };
    }
    0
}

fn windows_path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn resolve_program_path(program: &str) -> Result<String, i32> {
    let program = program.trim().trim_matches('"');
    if program.is_empty() {
        return Err(ERROR_INVALID_PARAMETER);
    }
    let supplied = Path::new(program);
    if supplied.is_absolute() {
        return Ok(program.to_string());
    }
    if program.contains('\\') || program.contains('/') {
        let current = std::env::current_dir().map_err(|e| e.raw_os_error().unwrap_or(2))?;
        return Ok(current.join(supplied).to_string_lossy().into_owned());
    }

    let mut candidates = vec![program.to_string()];
    if supplied.extension().is_none() {
        candidates.push(format!("{}.exe", program));
    }

    // Prefer Windows' System32 copy before PATH, and never allow CreateProcessW
    // to search the AppSweep directory or the caller's current working directory.
    for name in &candidates {
        if let Ok(path) = system_program_path(name) {
            if Path::new(&path).is_file() {
                return Ok(path);
            }
        }
    }

    let current_dir = std::env::current_dir().ok();
    let application_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let forbidden_dirs: Vec<String> = current_dir
        .iter()
        .chain(application_dir.iter())
        .map(|path| windows_path_key(path))
        .collect();

    for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        if !dir.is_absolute() {
            continue;
        }
        let resolved_dir = std::fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone());
        let dir_key = windows_path_key(&resolved_dir);
        if forbidden_dirs.iter().any(|blocked| blocked == &dir_key) {
            continue;
        }
        for name in &candidates {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate.to_string_lossy().into_owned());
            }
        }
    }

    Err(2) // ERROR_FILE_NOT_FOUND
}

pub fn run_cmd(command_line: &str, wait: bool) -> (i32, u32) {
    let executable = command_executable(command_line);
    let application = match resolve_program_path(&executable) {
        Ok(path) => path,
        Err(error) => return (error, 0),
    };
    let info = match start_process(Some(&application), command_line, None, true) {
        Ok(info) => info,
        Err(error) => return (error as i32, 0),
    };
    let mut code = 0u32;
    let failure = unsafe {
        let failure = finish_process(info.h_process, wait, &mut code);
        let _ = CloseHandle(info.h_thread);
        let _ = CloseHandle(info.h_process);
        failure
    };
    (failure, code)
}

fn quote_windows_argument(argument: &str) -> String {
    if !argument.is_empty()
        && !argument.ends_with('\\')
        && !argument
            .chars()
            .any(|c| matches!(c, ' ' | '\t' | '\n' | '\u{000b}' | '"'))
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
    let application = match resolve_program_path(file) {
        Ok(path) => path,
        Err(error) => return (error, 0),
    };
    let command_line = if params.is_empty() {
        quote_windows_argument(file)
    } else {
        format!("{} {}", quote_windows_argument(file), params)
    };
    // Pass an explicit, resolved application path so CreateProcessW cannot
    // choose an executable from the AppSweep or current working directory.
    let info = match start_process(Some(&application), &command_line, cwd, false) {
        Ok(info) => info,
        Err(error) => return (error as i32, 0),
    };
    let mut code = 0u32;
    let failure = unsafe {
        let failure = finish_process(info.h_process, wait, &mut code);
        let _ = CloseHandle(info.h_thread);
        let _ = CloseHandle(info.h_process);
        failure
    };
    (failure, code)
}

pub fn run_elevated(file: &str, params: &str, cwd: Option<&str>, wait: bool) -> (i32, u32) {
    let verb = wide("runas");
    let file_wide = wide(file);
    let params_wide = wide(params);
    let cwd_wide = cwd.map(wide);
    let initialized = unsafe {
        CoInitializeEx(
            null_mut(),
            COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
        ) >= 0
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
            if info.h_process.is_null() {
                // ShellExecuteExW can succeed without a process to wait for.
                if wait {
                    (ERROR_INVALID_HANDLE as i32, 0)
                } else {
                    (0, 0)
                }
            } else {
                let failure = finish_process(info.h_process, wait, &mut code);
                let _ = CloseHandle(info.h_process);
                (failure, code)
            }
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

fn capture_process_output(command_line: &str) -> Option<(Vec<u8>, u32)> {
    let mut read_pipe = null_mut();
    let mut write_pipe = null_mut();
    let security = SecurityAttributes {
        length: size_of::<SecurityAttributes>() as u32,
        security_descriptor: null_mut(),
        inherit_handle: 1,
    };
    unsafe {
        if CreatePipe(&mut read_pipe, &mut write_pipe, &security, 0) == 0 {
            return None;
        }
        if SetHandleInformation(read_pipe, HANDLE_FLAG_INHERIT, 0) == 0 {
            let _ = CloseHandle(read_pipe);
            let _ = CloseHandle(write_pipe);
            return None;
        }
    }

    let executable = command_executable(command_line);
    let application = match resolve_program_path(&executable) {
        Ok(path) => path,
        Err(_) => {
            unsafe {
                let _ = CloseHandle(read_pipe);
                let _ = CloseHandle(write_pipe);
            }
            return None;
        }
    };
    let application_wide = wide(&application);
    let mut command_wide = wide(command_line);
    let mut startup: StartupInfoW = unsafe { zeroed() };
    startup.cb = size_of::<StartupInfoW>() as u32;
    startup.dw_flags = STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
    startup.h_std_output = write_pipe;
    startup.h_std_error = write_pipe;
    startup.h_std_input = null_mut();
    startup.w_show_window = SW_HIDE;

    let mut info: ProcessInformation = unsafe { zeroed() };
    let created = unsafe {
        CreateProcessW(
            application_wide.as_ptr(),
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
    unsafe {
        let _ = CloseHandle(write_pipe);
    }
    if created == 0 {
        unsafe {
            let _ = CloseHandle(read_pipe);
        }
        return None;
    }

    const OUTPUT_LIMIT: usize = 512 * 1024;
    const POLL_MILLIS: u32 = 10;
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut data = Vec::with_capacity(8192);
    let mut chunk = [0u8; 4096];
    let mut pipe_open = true;
    let mut timed_out = false;
    let mut truncated = false;
    let mut capture_failed = false;

    loop {
        if Instant::now() >= deadline {
            timed_out = true;
            unsafe {
                let _ = TerminateProcess(info.h_process, 1);
                let _ = WaitForSingleObject(info.h_process, 1000);
            }
            break;
        }

        if pipe_open {
            let mut available = 0u32;
            let peeked = unsafe {
                PeekNamedPipe(
                    read_pipe,
                    null_mut(),
                    0,
                    null_mut(),
                    &mut available,
                    null_mut(),
                )
            };
            if peeked == 0 {
                let error = unsafe { GetLastError() };
                // ERROR_BROKEN_PIPE means all writers closed and the buffered
                // output has been drained; other errors make completeness unknown.
                if error != 109 {
                    capture_failed = true;
                }
                pipe_open = false;
            } else if available > 0 {
                let request = available.min(chunk.len() as u32);
                let mut received = 0u32;
                let read_ok = unsafe {
                    ReadFile(
                        read_pipe,
                        chunk.as_mut_ptr() as *mut c_void,
                        request,
                        &mut received,
                        null_mut(),
                    )
                };
                if read_ok == 0 || received == 0 {
                    capture_failed = true;
                    pipe_open = false;
                } else if data.len() < OUTPUT_LIMIT {
                    let retain = (OUTPUT_LIMIT - data.len()).min(received as usize);
                    data.extend_from_slice(&chunk[..retain]);
                    if retain < received as usize {
                        truncated = true;
                    }
                } else {
                    truncated = true;
                }
                // Continue draining after the capture limit so a verbose child
                // cannot block forever because its pipe buffer is full.
                continue;
            }
        }

        match unsafe { WaitForSingleObject(info.h_process, 0) } {
            WAIT_OBJECT_0 if !pipe_open => break,
            WAIT_OBJECT_0 => {
                // The process may exit before an inherited writer handle is
                // closed by a descendant. Keep draining until EOF or the same
                // deadline; never accept potentially incomplete inventory.
                unsafe { Sleep(POLL_MILLIS) };
            }
            WAIT_FAILED => {
                unsafe {
                    let _ = TerminateProcess(info.h_process, 1);
                    let _ = WaitForSingleObject(info.h_process, 1000);
                }
                capture_failed = true;
                break;
            }
            _ => unsafe { Sleep(POLL_MILLIS) },
        }
    }

    let mut exit_code = 1u32;
    let exited = unsafe {
        WaitForSingleObject(info.h_process, 0) == WAIT_OBJECT_0
            && GetExitCodeProcess(info.h_process, &mut exit_code) != 0
    };
    unsafe {
        let _ = CloseHandle(info.h_thread);
        let _ = CloseHandle(info.h_process);
        let _ = CloseHandle(read_pipe);
    }
    // A truncated output can look like a complete scheduled-task listing, which
    // could let cleanup delete the installation while leaving tasks behind.
    if timed_out || !exited || truncated || capture_failed {
        None
    } else {
        Some((data, exit_code))
    }
}

pub fn capture_cmd(command_line: &str) -> String {
    let Some((data, _)) = capture_process_output(command_line) else {
        return String::new();
    };
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
const IID_IMAGE_LIST: Guid = Guid {
    data1: 0x46eb_5926,
    data2: 0x582e,
    data3: 0x4017,
    data4: [0x9f, 0xdf, 0xe8, 0x99, 0x8d, 0xaa, 0x09, 0x50],
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
        CoInitializeEx(
            null_mut(),
            COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
        ) >= 0
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
        let get_arguments: unsafe extern "system" fn(*mut c_void, *mut u16, i32) -> HResult =
            unsafe { transmute((*link_vtable).methods[10]) };

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
    let ok = unsafe { QueryFullProcessImageNameW(process, 0, image.as_mut_ptr(), &mut length) };
    if ok == 0 || length == 0 {
        return None;
    }
    image.truncate(length as usize);
    Some(String::from_utf16_lossy(&image))
}

fn is_protected_or_unsupported_path(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty()
        || trimmed.starts_with(r"\\?\")
        || trimmed.starts_with(r"\\.\")
    {
        return true;
    }
    crate::model::is_protected_dir(Path::new(trimmed))
}

fn count_or_kill_processes(path: &str, terminate: bool) -> i32 {
    if is_protected_or_unsupported_path(path) || is_root_path(path) {
        return 0;
    }
    let folder = normalize_path(path)
        .trim_end_matches(['\\', '/'])
        .to_string();
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
            let access =
                PROCESS_QUERY_LIMITED_INFORMATION | if terminate { PROCESS_TERMINATE } else { 0 };
            let process = unsafe { OpenProcess(access, 0, entry.th32_process_id) };
            if !process.is_null() {
                if let Some(image) = process_image_path(process) {
                    if path_is_under(path, &image) {
                        if !terminate {
                            count += 1;
                        } else {
                            let terminated = unsafe { TerminateProcess(process, 1) != 0 };
                            if terminated
                                && unsafe { WaitForSingleObject(process, 1500) } == WAIT_OBJECT_0
                            {
                                count += 1;
                            }
                        }
                    }
                }
                unsafe {
                    let _ = CloseHandle(process);
                }
            }
        }
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    count
}

pub fn count_processes_in(path: &str) -> i32 {
    count_or_kill_processes(path, false)
}

pub fn kill_processes_in(path: &str) -> i32 {
    count_or_kill_processes(path, true)
}

pub fn kill_process_by_pid(pid: u32) -> i32 {
    unsafe {
        let process = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if process.is_null() {
            return GetLastError() as i32;
        }
        let result = if TerminateProcess(process, 1) != 0 {
            0
        } else {
            GetLastError() as i32
        };
        let _ = CloseHandle(process);
        result
    }
}

/// `None` means service inventory could not be read completely. Do not
/// equate an access/configuration/enumeration error with an empty service list.
pub fn services_under(path: &str) -> Option<Vec<String>> {
    if path.trim().is_empty() {
        return None;
    }
    let folder = normalize_path(path)
        .trim_end_matches(['\\', '/'])
        .to_string();
    if folder.is_empty() {
        return None;
    }

    unsafe {
        let manager = OpenSCManagerW(null(), null(), SC_MANAGER_ENUMERATE_SERVICE);
        if manager.is_null() {
            return None;
        }

        let mut names = Vec::new();
        let mut resume = 0u32;
        loop {
            let mut capacity = 64 * 1024usize;
            let (buffer, ok, error, returned) = loop {
                let mut buffer = vec![0u64; capacity.div_ceil(size_of::<u64>())];
                let mut needed = 0u32;
                let mut returned = 0u32;
                let allocated_bytes = buffer.len() * size_of::<u64>();
                let ok = EnumServicesStatusExW(
                    manager,
                    SC_ENUM_PROCESS_INFO,
                    SERVICE_WIN32,
                    SERVICE_STATE_ALL,
                    buffer.as_mut_ptr() as *mut u8,
                    allocated_bytes.min(u32::MAX as usize) as u32,
                    &mut needed,
                    &mut returned,
                    &mut resume,
                    null(),
                ) != 0;
                if ok {
                    break (buffer, true, 0u32, returned);
                }
                let error = GetLastError();
                if error == ERROR_MORE_DATA && returned == 0 && needed as usize > capacity {
                    // One unusually large service-list page needs a larger buffer.
                    capacity = needed as usize;
                    continue;
                }
                break (buffer, false, error, returned);
            };

            if !ok && error != ERROR_MORE_DATA {
                let _ = CloseServiceHandle(manager);
                return None;
            }
            if !ok && returned == 0 {
                // A repeated ERROR_MORE_DATA with no progress cannot prove that
                // the complete inventory was examined.
                let _ = CloseServiceHandle(manager);
                return None;
            }

            let services = std::slice::from_raw_parts(
                buffer.as_ptr() as *const EnumServiceStatusProcessW,
                returned as usize,
            );
            for item in services {
                if item.service_name.is_null() {
                    let _ = CloseServiceHandle(manager);
                    return None;
                }
                let service_name = wide_ptr_to_string(item.service_name);
                let service = OpenServiceW(
                    manager,
                    item.service_name,
                    SERVICE_QUERY_CONFIG,
                );
                if service.is_null() {
                    let _ = CloseServiceHandle(manager);
                    return None;
                }

                let mut config_needed = 0u32;
                let _ = QueryServiceConfigW(
                    service,
                    null_mut(),
                    0,
                    &mut config_needed,
                );
                if config_needed == 0 {
                    let _ = CloseServiceHandle(service);
                    let _ = CloseServiceHandle(manager);
                    return None;
                }
                let mut config_buffer =
                    vec![0u64; (config_needed as usize).div_ceil(size_of::<u64>())];
                let config_bytes = config_buffer.len() * size_of::<u64>();
                let config_ok = QueryServiceConfigW(
                    service,
                    config_buffer.as_mut_ptr() as *mut QueryServiceConfigW,
                    config_bytes.min(u32::MAX as usize) as u32,
                    &mut config_needed,
                ) != 0;
                if !config_ok {
                    let _ = CloseServiceHandle(service);
                    let _ = CloseServiceHandle(manager);
                    return None;
                }

                let config = &*(config_buffer.as_ptr() as *const QueryServiceConfigW);
                let binary_path = wide_ptr_to_string(config.binary_path_name as *const u16);
                let executable = command_executable(&binary_path);
                if path_is_under(path, &executable) {
                    names.push(service_name);
                }
                let _ = CloseServiceHandle(service);
            }

            if ok {
                if resume == 0 {
                    break;
                }
                // A successful call normally resets the resume handle. A
                // nonzero handle here is unexpected; returning unknown is safer
                // than spinning forever or reporting a partial list.
                let _ = CloseServiceHandle(manager);
                return None;
            }
            // ERROR_MORE_DATA: resume now points to the next unread service.
        }
        let _ = CloseServiceHandle(manager);
        Some(names)
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
    let path_wide = wide(&long_path(&path.to_string_lossy()));
    unsafe {
        let attributes = GetFileAttributesW(path_wide.as_ptr());
        if attributes != u32::MAX && attributes & FILE_ATTRIBUTE_READONLY != 0 {
            let _ = SetFileAttributesW(path_wide.as_ptr(), attributes & !FILE_ATTRIBUTE_READONLY);
        }
    }
}

/// Fail closed if Win32 cannot determine whether the path is a reparse point.
pub(crate) fn is_reparse_point(path: &Path) -> bool {
    let path_wide = wide(&long_path(&path.to_string_lossy()));
    unsafe {
        let attributes = GetFileAttributesW(path_wide.as_ptr());
        attributes == u32::MAX || attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
}

fn enable_privilege(name: &str) -> bool {
    let process = unsafe { GetCurrentProcess() };
    let mut token = null_mut();
    unsafe {
        if OpenProcessToken(process, TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
    }

    let privilege_name = wide(name);
    let mut luid = Luid {
        low_part: 0,
        high_part: 0,
    };
    let looked_up =
        unsafe { LookupPrivilegeValueW(null(), privilege_name.as_ptr(), &mut luid) != 0 };
    if !looked_up {
        unsafe {
            let _ = CloseHandle(token);
        }
        return false;
    }

    let mut privileges = TokenPrivileges {
        privilege_count: 1,
        privileges: [LuidAndAttributes {
            luid,
            attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    let adjusted = unsafe {
        AdjustTokenPrivileges(
            token,
            0,
            &mut privileges,
            size_of::<TokenPrivileges>() as u32,
            null_mut(),
            null_mut(),
        ) != 0
    };
    let error = unsafe { GetLastError() };
    unsafe {
        let _ = CloseHandle(token);
    }
    adjusted && error != ERROR_NOT_ALL_ASSIGNED
}

fn allocate_admins_sid() -> Option<*mut c_void> {
    let nt_authority = SidIdentifierAuthority {
        value: [0, 0, 0, 0, 0, 5],
    };
    let mut sid = null_mut();
    let allocated = unsafe {
        AllocateAndInitializeSid(
            &nt_authority,
            2,
            SECURITY_BUILTIN_DOMAIN_RID,
            DOMAIN_ALIAS_RID_ADMINS,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut sid,
        )
    };
    if allocated != 0 && !sid.is_null() {
        Some(sid)
    } else {
        None
    }
}

fn save_security(path: &Path) -> Option<SavedSecurity> {
    let path_wide = wide(&long_path(&path.to_string_lossy()));
    let mut owner = null_mut();
    let mut group = null_mut();
    let mut dacl = null_mut();
    let mut descriptor = null_mut();
    let result = unsafe {
        GetNamedSecurityInfoW(
            path_wide.as_ptr() as *mut u16,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            &mut group,
            &mut dacl,
            null_mut(),
            &mut descriptor,
        )
    };
    if result == 0 && !descriptor.is_null() {
        Some(SavedSecurity {
            descriptor,
            owner,
            group,
        })
    } else {
        if !descriptor.is_null() {
            unsafe {
                let _ = LocalFree(descriptor);
            }
        }
        None
    }
}

fn restore_security(path: &Path, saved: &SavedSecurity) -> bool {
    if saved.descriptor.is_null() {
        return false;
    }
    let mut present = 0;
    let mut defaulted = 0;
    let mut dacl = null_mut();
    if unsafe {
        GetSecurityDescriptorDacl(saved.descriptor, &mut present, &mut dacl, &mut defaulted)
    } == 0
    {
        return false;
    }
    let path_wide = wide(&long_path(&path.to_string_lossy()));
    let mut control = 0u16;
    let mut revision = 0u32;
    if unsafe { GetSecurityDescriptorControl(saved.descriptor, &mut control, &mut revision) } == 0 {
        return false;
    }
    let dacl_protection = if control & SE_DACL_PROTECTED != 0 {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    // Always restore the DACL state, including a NULL/absent DACL, and preserve
    // whether inheritance was protected instead of silently changing it.
    let security_info = OWNER_SECURITY_INFORMATION
        | GROUP_SECURITY_INFORMATION
        | DACL_SECURITY_INFORMATION
        | dacl_protection;
    unsafe {
        SetNamedSecurityInfoW(
            path_wide.as_ptr() as *mut u16,
            SE_FILE_OBJECT,
            security_info,
            saved.owner,
            saved.group,
            if present != 0 { dacl } else { null_mut() },
            null_mut(),
        ) == 0
    }
}

fn take_ownership(path: &Path) -> bool {
    let _ = enable_privilege("SeTakeOwnershipPrivilege");
    let _ = enable_privilege("SeRestorePrivilege");
    let _ = enable_privilege("SeBackupPrivilege");

    let Some(owners_sid) = allocate_admins_sid() else {
        return false;
    };
    let path_wide = wide(&long_path(&path.to_string_lossy()));
    unsafe {
        let _ = SetNamedSecurityInfoW(
            path_wide.as_ptr() as *mut u16,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            owners_sid,
            null_mut(),
            null_mut(),
            null_mut(),
        );
        let _ = FreeSid(owners_sid);
    }

    let Some(membership_sid) = allocate_admins_sid() else {
        return false;
    };
    let mut is_admin = 0;
    let membership_checked =
        unsafe { CheckTokenMembership(null_mut(), membership_sid, &mut is_admin) != 0 };
    unsafe {
        let _ = FreeSid(membership_sid);
    }
    if !membership_checked || is_admin == 0 {
        return false;
    }

    let Some(admins_sid) = allocate_admins_sid() else {
        return false;
    };
    let entry = ExplicitAccessW {
        access_permissions: DELETE_FILE_ACCESS
            | FILE_DELETE_CHILD
            | FILE_GENERIC_READ
            | FILE_GENERIC_WRITE
            | READ_CONTROL
            | SYNCHRONIZE,
        access_mode: SET_ACCESS,
        inheritance: NO_INHERITANCE,
        trustee: TrusteeW {
            multiple_trustee: null_mut(),
            multiple_trustee_operation: 0,
            trustee_form: TRUSTEE_IS_SID,
            trustee_type: TRUSTEE_IS_GROUP,
            name: admins_sid as *mut u16,
        },
    };
    let mut acl = null_mut();
    let acl_result = unsafe { SetEntriesInAclW(1, &entry, null_mut(), &mut acl) };
    let result = if acl_result == 0 && !acl.is_null() {
        unsafe {
            let result = SetNamedSecurityInfoW(
                path_wide.as_ptr() as *mut u16,
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                acl,
                null_mut(),
            );
            let _ = LocalFree(acl);
            result == 0
        }
    } else {
        false
    };
    unsafe {
        let _ = FreeSid(admins_sid);
    }
    result
}

fn acl_snapshot_and_relax(path: &Path) -> Option<SavedSecurity> {
    // Enable backup/restore rights before trying to snapshot the descriptor.
    // Never alter ownership or the DACL if capturing the original state fails.
    let _ = enable_privilege("SeBackupPrivilege");
    let _ = enable_privilege("SeRestorePrivilege");
    let saved = save_security(path)?;
    let _ = take_ownership(path);
    Some(saved)
}

fn restore_acl_snapshot(path: &Path, snapshot: &SavedSecurity) -> bool {
    // Do not gate restoration on Path::exists(): access-denied metadata can
    // report false even while the object survives and its ACL must be restored.
    restore_security(path, snapshot)
}

fn remove_object(path: &Path) -> std::io::Result<()> {
    let path_wide = wide(&long_path(&path.to_string_lossy()));
    let attributes = unsafe { GetFileAttributesW(path_wide.as_ptr()) };
    if attributes != u32::MAX && attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        std::fs::remove_dir(path)
    } else {
        std::fs::remove_file(path).or_else(|_| std::fs::remove_dir(path))
    }
}

fn record_delete_failure(stats: &mut DeleteStats, path: &Path, reason: &str) {
    stats.failed += 1;
    if stats.first_error.is_empty() {
        stats.first_error = format!("{} ({})", path.display(), reason);
    }
}

fn open_guarded_directory_component(path: &Path) -> std::io::Result<Handle> {
    let path_wide = wide(&path.to_string_lossy());
    // Do not allow deletion/rename sharing: while this handle is open, the path
    // component cannot be replaced. OPEN_REPARSE_POINT prevents following a
    // junction at this component.
    let handle = unsafe {
        CreateFileW(
            path_wide.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        )
    };
    if handle.is_null() || handle == -1isize as Handle {
        return Err(std::io::Error::from_raw_os_error(unsafe { GetLastError() } as i32));
    }
    let mut attributes = FileAttributeTagInfo {
        file_attributes: 0,
        reparse_tag: 0,
    };
    let queried = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FILE_ATTRIBUTE_TAG_INFO_CLASS,
            &mut attributes as *mut FileAttributeTagInfo as *mut c_void,
            size_of::<FileAttributeTagInfo>() as u32,
        ) != 0
    };
    if !queried {
        let error = std::io::Error::from_raw_os_error(unsafe { GetLastError() } as i32);
        unsafe {
            let _ = CloseHandle(handle);
        }
        return Err(error);
    }
    if attributes.file_attributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || attributes.file_attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        unsafe {
            let _ = CloseHandle(handle);
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path component is not a normal directory",
        ));
    }
    Ok(handle)
}

fn open_directory_guard(path: &Path) -> std::io::Result<DirectoryHandle> {
    // Pin every component from the volume/share root to the target while using
    // path-based Rust enumeration. This prevents an attacker from swapping an
    // ancestor for a junction between validation and read_dir. Any parent-dir
    // component is rejected instead of being normalized across the guarded path.
    let mut current = PathBuf::new();
    let mut handles: Vec<Handle> = Vec::new();
    for component in path.components() {
        let should_open = match component {
            Component::Prefix(prefix) => {
                current.push(prefix.as_os_str());
                false
            }
            Component::RootDir => {
                current.push(component.as_os_str());
                true
            }
            Component::CurDir => false,
            Component::ParentDir => {
                for handle in handles.drain(..).rev() {
                    unsafe {
                        let _ = CloseHandle(handle);
                    }
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "parent components are not permitted in a deletion path",
                ));
            }
            Component::Normal(_) => {
                current.push(component.as_os_str());
                true
            }
        };
        if should_open {
            match open_guarded_directory_component(&current) {
                Ok(handle) => handles.push(handle),
                Err(error) => {
                    for handle in handles.drain(..).rev() {
                        unsafe {
                            let _ = CloseHandle(handle);
                        }
                    }
                    return Err(error);
                }
            }
        }
    }
    if handles.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "deletion path is not absolute",
        ));
    }
    Ok(DirectoryHandle(handles))
}

fn delete_tree(path: &Path, stats: &mut DeleteStats, depth: u32) {
    if depth > 64 {
        record_delete_failure(stats, path, "maximum recursion depth exceeded");
        return;
    }
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            record_delete_failure(stats, path, &format!("metadata query failed: {}", error));
            return;
        }
    };

    if metadata.file_type().is_symlink() || is_reparse_point(path) {
        // Never mutate attributes through a junction/symlink: remove the
        // reparse point itself, and fail closed if Windows refuses it.
        if remove_object(path).is_ok() {
            stats.deleted += 1;
        } else {
            record_delete_failure(stats, path, "could not remove reparse point");
        }
        return;
    }

    if metadata.is_dir() {
        let mut early_snapshot: Option<SavedSecurity> = None;
        let mut directory_guard = match open_directory_guard(path) {
            Ok(handle) => Some(handle),
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                early_snapshot = acl_snapshot_and_relax(path);
                match open_directory_guard(path) {
                    Ok(handle) => Some(handle),
                    Err(retry_error) => {
                        if let Some(snapshot) = early_snapshot.as_ref() {
                            let _ = restore_acl_snapshot(path, snapshot);
                        }
                        record_delete_failure(
                            stats,
                            path,
                            &format!("could not safely open directory: {}", retry_error),
                        );
                        return;
                    }
                }
            }
            Err(error) => {
                record_delete_failure(
                    stats,
                    path,
                    &format!("could not safely open directory: {}", error),
                );
                return;
            }
        };

        // The verified directory handle remains open without FILE_SHARE_DELETE
        // while read_dir and recursive calls use the same path. This blocks
        // junction/rename swaps during traversal; failure is fail-closed.
        let mut listing = std::fs::read_dir(path);
        if matches!(&listing, Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied) {
            if early_snapshot.is_none() {
                early_snapshot = acl_snapshot_and_relax(path);
            }
            listing = std::fs::read_dir(path);
        }
        let entries = match listing {
            Ok(entries) => entries,
            Err(error) => {
                drop(directory_guard.take());
                if let Some(snapshot) = early_snapshot.as_ref() {
                    let restored = restore_acl_snapshot(path, snapshot);
                    record_delete_failure(
                        stats,
                        path,
                        if restored {
                            "directory enumeration failed; permissions restored"
                        } else {
                            "directory enumeration failed; permissions may not be restored"
                        },
                    );
                } else {
                    record_delete_failure(
                        stats,
                        path,
                        &format!("directory enumeration failed: {}", error),
                    );
                }
                return;
            }
        };

        let mut enumeration_failed = false;
        for entry in entries {
            match entry {
                Ok(entry) => delete_tree(&entry.path(), stats, depth + 1),
                Err(error) => {
                    enumeration_failed = true;
                    record_delete_failure(
                        stats,
                        path,
                        &format!("directory entry enumeration failed: {}", error),
                    );
                }
            }
        }
        drop(directory_guard.take());

        if enumeration_failed {
            if let Some(snapshot) = early_snapshot.as_ref() {
                let _ = restore_acl_snapshot(path, snapshot);
            }
            return;
        }

        clear_readonly(path);
        match std::fs::remove_dir(path) {
            Ok(()) => stats.deleted += 1,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                let snapshot = early_snapshot.or_else(|| acl_snapshot_and_relax(path));
                clear_readonly(path);
                match std::fs::remove_dir(path) {
                    Ok(()) => stats.deleted += 1,
                    Err(retry_error) => {
                        let restored = snapshot
                            .as_ref()
                            .is_some_and(|saved| restore_acl_snapshot(path, saved));
                        record_delete_failure(
                            stats,
                            path,
                            &if snapshot.is_some() && !restored {
                                format!(
                                    "directory removal failed and permissions were not restored: {}",
                                    retry_error
                                )
                            } else {
                                format!("directory removal failed: {}", retry_error)
                            },
                        );
                    }
                }
            }
            Err(error) => {
                if let Some(snapshot) = early_snapshot.as_ref() {
                    let _ = restore_acl_snapshot(path, snapshot);
                }
                record_delete_failure(stats, path, &format!("directory removal failed: {}", error));
            }
        }
    } else {
        clear_readonly(path);
        match std::fs::remove_file(path) {
            Ok(()) => stats.deleted += 1,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                let snapshot = acl_snapshot_and_relax(path);
                clear_readonly(path);
                match std::fs::remove_file(path) {
                    Ok(()) => stats.deleted += 1,
                    Err(retry_error) => {
                        let restored = snapshot
                            .as_ref()
                            .is_some_and(|saved| restore_acl_snapshot(path, saved));
                        record_delete_failure(
                            stats,
                            path,
                            &if snapshot.is_some() && !restored {
                                format!(
                                    "file removal failed and permissions were not restored: {}",
                                    retry_error
                                )
                            } else {
                                format!("file removal failed: {}", retry_error)
                            },
                        );
                    }
                }
            }
            Err(error) => {
                record_delete_failure(stats, path, &format!("file removal failed: {}", error));
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
    let normalized = path.replace('/', "\\");
    let trimmed = normalized.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        return true;
    }

    if let Some(extended) = trimmed.strip_prefix(r"\\?\") {
        if extended
            .get(..11)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("GLOBALROOT\\"))
            || extended
                .get(..7)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Device\\"))
        {
            return true;
        }
        if extended
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("UNC\\"))
        {
            return extended[4..]
                .split('\\')
                .filter(|part| !part.is_empty())
                .count()
                <= 2;
        }
        if is_root_path(extended) {
            return true;
        }
        return extended
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Volume{"))
            && !extended.contains('\\');
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() == 2 && bytes[1] == b':' {
        return true;
    }
    if trimmed.starts_with(r"\\") {
        let parts = trimmed
            .trim_start_matches('\\')
            .split('\\')
            .filter(|part| !part.is_empty())
            .count();
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
    } else if is_protected_or_unsupported_path(path) {
        stats.failed = 1;
        stats.first_error = format!("Refusing to delete a protected or unsupported path: {}", path);
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
    let world_authority = SidIdentifierAuthority {
        value: [0, 0, 0, 0, 0, 1],
    };
    let mut everyone = null_mut();
    let allocated = unsafe {
        AllocateAndInitializeSid(&world_authority, 1, 0, 0, 0, 0, 0, 0, 0, 0, &mut everyone)
    };
    if allocated == 0 || everyone.is_null() {
        return -1;
    }

    let mut object_name = wide(&long_path(path));
    let mut dacl = null_mut();
    let mut descriptor = null_mut();
    let result = unsafe {
        GetNamedSecurityInfoW(
            object_name.as_mut_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut descriptor,
        )
    };
    if result != 0 {
        unsafe {
            let _ = FreeSid(everyone);
            if !descriptor.is_null() {
                let _ = LocalFree(descriptor);
            }
        }
        return -1;
    }

    let mut present = 0;
    let mut defaulted = 0;
    let mut actual_dacl = null_mut();
    let descriptor_valid = unsafe {
        GetSecurityDescriptorDacl(descriptor, &mut present, &mut actual_dacl, &mut defaulted) != 0
    };
    let mut verdict = 0;
    if descriptor_valid && present != 0 && !actual_dacl.is_null() {
        let header = unsafe { &*(actual_dacl as *const AclHeader) };
        for index in 0..u32::from(header.ace_count) {
            let mut ace = null_mut();
            if unsafe { GetAce(actual_dacl, index, &mut ace) } == 0 || ace.is_null() {
                continue;
            }
            let header = unsafe { &*(ace as *const AceHeader) };
            if header.ace_type != ACCESS_ALLOWED_ACE_TYPE {
                continue;
            }
            let allowed = unsafe { &*(ace as *const AccessAllowedAce) };
            let ace_sid = std::ptr::addr_of!(allowed.sid_start) as *mut c_void;
            if unsafe { EqualSid(ace_sid, everyone) } != 0
                && (allowed.mask & FILE_ALL_ACCESS) == FILE_ALL_ACCESS
            {
                verdict = 1;
                break;
            }
        }
    }

    unsafe {
        let _ = LocalFree(descriptor);
        let _ = FreeSid(everyone);
    }
    verdict
}

/// Self-test only: add an explicit, non-inheriting Everyone full-control ACE.
pub fn grant_everyone_full_control(path: &str) -> i32 {
    let world_authority = SidIdentifierAuthority {
        value: [0, 0, 0, 0, 0, 1],
    };
    let mut everyone = null_mut();
    let allocated = unsafe {
        AllocateAndInitializeSid(&world_authority, 1, 0, 0, 0, 0, 0, 0, 0, 0, &mut everyone)
    };
    if allocated == 0 || everyone.is_null() {
        return unsafe { GetLastError() as i32 };
    }

    let entry = ExplicitAccessW {
        access_permissions: FILE_ALL_ACCESS,
        access_mode: SET_ACCESS,
        inheritance: NO_INHERITANCE,
        trustee: TrusteeW {
            multiple_trustee: null_mut(),
            multiple_trustee_operation: 0,
            trustee_form: TRUSTEE_IS_SID,
            trustee_type: TRUSTEE_IS_WELL_KNOWN_GROUP,
            name: everyone as *mut u16,
        },
    };
    let mut acl = null_mut();
    let acl_result = unsafe { SetEntriesInAclW(1, &entry, null_mut(), &mut acl) };
    if acl_result != 0 || acl.is_null() {
        unsafe {
            let _ = FreeSid(everyone);
        }
        return acl_result as i32;
    }

    let mut object_name = wide(&long_path(path));
    let result = unsafe {
        SetNamedSecurityInfoW(
            object_name.as_mut_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            acl,
            null_mut(),
        )
    };
    unsafe {
        let _ = LocalFree(acl);
        let _ = FreeSid(everyone);
    }
    result as i32
}

pub fn path_exists(path: &str) -> bool {
    let path_wide = wide(&long_path(path));
    unsafe { GetFileAttributesW(path_wide.as_ptr()) != u32::MAX }
}

/// `Some(true)` there, `Some(false)` confirmed missing, `None` inaccessible.
/// Access denied must not read as "already gone".
pub fn path_state(path: &str) -> Option<bool> {
    // An empty path is nothing to ask about, not proof of absence.
    if path.trim().is_empty() {
        return None;
    }
    let w = wide(&long_path(path));
    if unsafe { GetFileAttributesW(w.as_ptr()) } != u32::MAX {
        return Some(true);
    }
    // Only the "not found" codes mean gone; access denied stays unknown.
    match unsafe { GetLastError() } {
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND | ERROR_INVALID_NAME => Some(false),
        _ => None,
    }
}

pub fn service_exists(name: &str) -> Option<bool> {
    let w = wide(name);
    unsafe {
        let manager = OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT);
        if manager.is_null() {
            return None;
        }
        let service = OpenServiceW(manager, w.as_ptr(), SERVICE_QUERY_STATUS);
        let found = !service.is_null();
        let reason = GetLastError();
        if found {
            let _ = CloseServiceHandle(service);
        }
        let _ = CloseServiceHandle(manager);
        if found {
            Some(true)
        } else if reason == 1060 {
            // ERROR_SERVICE_DOES_NOT_EXIST; anything else is "could not ask".
            Some(false)
        } else {
            None
        }
    }
}

/// `Some(true)` the package is still registered for the current user,
/// `Some(false)` confirmed gone, `None` could not ask.
pub fn package_registered(full_name: &str) -> Option<bool> {
    use windows::core::HSTRING;
    use windows::Management::Deployment::PackageManager;
    let sid = current_user_sid();
    if sid.is_empty() {
        return None;
    }
    let pm = PackageManager::new().ok()?;
    let list = pm
        .FindPackagesByUserSecurityId(&HSTRING::from(sid.as_str()))
        .ok()?;
    for pkg in list {
        // A missing ID on one rogue entry is not proof the rest are gone.
        let Ok(id) = pkg.Id() else { continue };
        let Ok(name) = id.FullName() else { continue };
        if name.to_string_lossy().eq_ignore_ascii_case(full_name) {
            return Some(true);
        }
    }
    Some(false)
}

pub fn task_exists(name: &str) -> Option<bool> {
    let command = format!("schtasks.exe /Query /TN \"{}\"", name);
    let (launched, code) = run_cmd(&command, true);
    if launched != 0 {
        return None;
    }
    // schtasks exits 0 for a task that exists and non-zero for one that does not.
    Some(code == 0)
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

/// Returns the error code (0 on success) and the point's sequence number.
static RESTORE_POINT_COM_SECURITY_READY: AtomicBool = AtomicBool::new(false);

fn allocate_sid_with_rid(rid: u32) -> Option<*mut c_void> {
    let authority = SidIdentifierAuthority {
        value: [0, 0, 0, 0, 0, 5],
    };
    let mut sid = null_mut();
    let allocated = unsafe {
        AllocateAndInitializeSid(
            &authority,
            1,
            rid,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut sid,
        )
    };
    if allocated != 0 && !sid.is_null() {
        Some(sid)
    } else {
        None
    }
}

/// Configure the process-wide COM ACL required by System Restore. Run this on
/// a short-lived MTA thread before the UI initializes COM, so the main thread
/// remains free to use its own apartment model.
pub fn initialize_restore_point_com_security() -> bool {
    if RESTORE_POINT_COM_SECURITY_READY.load(Ordering::Acquire) {
        return true;
    }

    let initialized = std::thread::Builder::new()
        .name("appsweep-com-security".into())
        .spawn(|| unsafe {
            let com_result = CoInitializeEx(null_mut(), COINIT_MULTITHREADED);
            if com_result < 0 {
                return false;
            }

            let result = (|| {
                let Some(admins_sid) = allocate_admins_sid() else {
                    return false;
                };
                let Some(local_service_sid) = allocate_sid_with_rid(19) else {
                    let _ = FreeSid(admins_sid);
                    return false;
                };
                let Some(network_service_sid) = allocate_sid_with_rid(20) else {
                    let _ = FreeSid(local_service_sid);
                    let _ = FreeSid(admins_sid);
                    return false;
                };
                let Some(system_sid) = allocate_sid_with_rid(18) else {
                    let _ = FreeSid(network_service_sid);
                    let _ = FreeSid(local_service_sid);
                    let _ = FreeSid(admins_sid);
                    return false;
                };

                let self_sid_text = wide(&current_user_sid());
                let mut self_sid = null_mut();
                let self_sid_ok =
                    !self_sid_text.is_empty()
                        && ConvertStringSidToSidW(self_sid_text.as_ptr(), &mut self_sid) != 0;
                if !self_sid_ok || self_sid.is_null() {
                    let _ = FreeSid(system_sid);
                    let _ = FreeSid(network_service_sid);
                    let _ = FreeSid(local_service_sid);
                    let _ = FreeSid(admins_sid);
                    return false;
                }

                let mut trustees = [
                    ExplicitAccessW {
                        access_permissions: COM_RIGHTS_EXECUTE | COM_RIGHTS_EXECUTE_LOCAL,
                        access_mode: SET_ACCESS,
                        inheritance: NO_INHERITANCE,
                        trustee: TrusteeW {
                            multiple_trustee: null_mut(),
                            multiple_trustee_operation: 0,
                            trustee_form: TRUSTEE_IS_SID,
                            trustee_type: TRUSTEE_IS_WELL_KNOWN_GROUP,
                            name: admins_sid as *mut u16,
                        },
                    },
                    ExplicitAccessW {
                        access_permissions: COM_RIGHTS_EXECUTE | COM_RIGHTS_EXECUTE_LOCAL,
                        access_mode: SET_ACCESS,
                        inheritance: NO_INHERITANCE,
                        trustee: TrusteeW {
                            multiple_trustee: null_mut(),
                            multiple_trustee_operation: 0,
                            trustee_form: TRUSTEE_IS_SID,
                            trustee_type: TRUSTEE_IS_WELL_KNOWN_GROUP,
                            name: local_service_sid as *mut u16,
                        },
                    },
                    ExplicitAccessW {
                        access_permissions: COM_RIGHTS_EXECUTE | COM_RIGHTS_EXECUTE_LOCAL,
                        access_mode: SET_ACCESS,
                        inheritance: NO_INHERITANCE,
                        trustee: TrusteeW {
                            multiple_trustee: null_mut(),
                            multiple_trustee_operation: 0,
                            trustee_form: TRUSTEE_IS_SID,
                            trustee_type: TRUSTEE_IS_WELL_KNOWN_GROUP,
                            name: network_service_sid as *mut u16,
                        },
                    },
                    ExplicitAccessW {
                        access_permissions: COM_RIGHTS_EXECUTE | COM_RIGHTS_EXECUTE_LOCAL,
                        access_mode: SET_ACCESS,
                        inheritance: NO_INHERITANCE,
                        trustee: TrusteeW {
                            multiple_trustee: null_mut(),
                            multiple_trustee_operation: 0,
                            trustee_form: TRUSTEE_IS_SID,
                            trustee_type: TRUSTEE_IS_USER,
                            name: self_sid as *mut u16,
                        },
                    },
                    ExplicitAccessW {
                        access_permissions: COM_RIGHTS_EXECUTE | COM_RIGHTS_EXECUTE_LOCAL,
                        access_mode: SET_ACCESS,
                        inheritance: NO_INHERITANCE,
                        trustee: TrusteeW {
                            multiple_trustee: null_mut(),
                            multiple_trustee_operation: 0,
                            trustee_form: TRUSTEE_IS_SID,
                            trustee_type: TRUSTEE_IS_WELL_KNOWN_GROUP,
                            name: system_sid as *mut u16,
                        },
                    },
                ];

                let mut acl = null_mut();
                if SetEntriesInAclW(
                    trustees.len() as u32,
                    trustees.as_mut_ptr(),
                    null_mut(),
                    &mut acl,
                ) != 0
                    || acl.is_null()
                {
                    let _ = LocalFree(acl);
                    let _ = LocalFree(self_sid as Handle);
                    let _ = FreeSid(system_sid);
                    let _ = FreeSid(network_service_sid);
                    let _ = FreeSid(local_service_sid);
                    let _ = FreeSid(admins_sid);
                    return false;
                }

                let mut descriptor: AbsoluteSecurityDescriptor = zeroed();
                let descriptor_ptr = &mut descriptor as *mut AbsoluteSecurityDescriptor as *mut c_void;
                let descriptor_ok = InitializeSecurityDescriptor(
                    descriptor_ptr,
                    SECURITY_DESCRIPTOR_REVISION,
                ) != 0
                    && SetSecurityDescriptorOwner(descriptor_ptr, admins_sid, 0) != 0
                    && SetSecurityDescriptorGroup(descriptor_ptr, admins_sid, 0) != 0
                    && SetSecurityDescriptorDacl(descriptor_ptr, 1, acl, 0) != 0;
                let initialized = descriptor_ok
                    && CoInitializeSecurity(
                        descriptor_ptr,
                        -1,
                        null_mut(),
                        null_mut(),
                        RPC_C_AUTHN_LEVEL_PKT_PRIVACY,
                        RPC_C_IMP_LEVEL_IDENTIFY,
                        null_mut(),
                        EOAC_DISABLE_AAA | EOAC_NO_CUSTOM_MARSHAL,
                        null_mut(),
                    ) >= 0;

                let _ = LocalFree(acl);
                let _ = LocalFree(self_sid as Handle);
                let _ = FreeSid(system_sid);
                let _ = FreeSid(network_service_sid);
                let _ = FreeSid(local_service_sid);
                let _ = FreeSid(admins_sid);
                initialized
            })();

            CoUninitialize();
            result
        })
        .ok()
        .and_then(|handle| handle.join().ok())
        .unwrap_or(false);

    if initialized {
        RESTORE_POINT_COM_SECURITY_READY.store(true, Ordering::Release);
    }
    initialized
}

pub fn create_restore_point(description: &str) -> (i32, i64) {
    if !RESTORE_POINT_COM_SECURITY_READY.load(Ordering::Acquire) {
        return (ERROR_GEN_FAILURE, 0);
    }
    let _com = match enter_com_mta() {
        Ok(apartment) => apartment,
        Err(error) => return (error, 0),
    };
    type RestoreFn =
        unsafe extern "system" fn(*mut RestorePointInfoW, *mut StateManagerStatus) -> Bool;
    unsafe {
        let library = wide("srclient.dll");
        let module = LoadLibraryExW(
            library.as_ptr(),
            null_mut(),
            LOAD_LIBRARY_SEARCH_SYSTEM32,
        );
        if module.is_null() {
            return (GetLastError() as i32, 0);
        }
        let symbol = GetProcAddress(module, c"SRSetRestorePointW".as_ptr());
        if symbol.is_null() {
            let _ = FreeLibrary(module);
            return (127, 0); // ERROR_PROC_NOT_FOUND
        }
        let restore: RestoreFn = transmute(symbol);
        let mut info: RestorePointInfoW = zeroed();
        info.event_type = SYSTEM_RESTORE_BEGIN_SYSTEM_CHANGE;
        info.restore_point_type = SYSTEM_RESTORE_APPLICATION_UNINSTALL;
        let mut status: StateManagerStatus = zeroed();
        let mut description_wide = [0u16; 256];
        for (index, unit) in description.encode_utf16().take(255).enumerate() {
            description_wide[index] = unit;
        }
        info.description = description_wide;
        let ok = restore(&mut info, &mut status) != 0;
        // On failure the reason is in nStatus; GetLastError is not the source.
        let error = if ok { 0 } else { status.status as i32 };
        // Frequency suppression reuses an existing point and returns its
        // sequence, so a non-zero sequence only proves a point exists.
        // Copy packed fields by value; never create references to unaligned fields.
        let sequence = if ok { status.sequence_number } else { 0 };
        let _ = FreeLibrary(module);
        (error, sequence)
    }
}


/// End the System Restore change notification after the cleanup operation.
/// The sequence returned by BEGIN_SYSTEM_CHANGE must be passed back here.
pub fn end_restore_point(sequence_number: i64) -> i32 {
    if !RESTORE_POINT_COM_SECURITY_READY.load(Ordering::Acquire) {
        return ERROR_GEN_FAILURE;
    }
    let _com = match enter_com_mta() {
        Ok(apartment) => apartment,
        Err(error) => return error,
    };
    type RestoreFn =
        unsafe extern "system" fn(*mut RestorePointInfoW, *mut StateManagerStatus) -> Bool;
    unsafe {
        let library = wide("srclient.dll");
        let module = LoadLibraryExW(
            library.as_ptr(),
            null_mut(),
            LOAD_LIBRARY_SEARCH_SYSTEM32,
        );
        if module.is_null() {
            return GetLastError() as i32;
        }
        let symbol = GetProcAddress(module, c"SRSetRestorePointW".as_ptr());
        if symbol.is_null() {
            let _ = FreeLibrary(module);
            return 127;
        }
        let restore: RestoreFn = transmute(symbol);
        let mut info: RestorePointInfoW = zeroed();
        info.event_type = SYSTEM_RESTORE_END_SYSTEM_CHANGE;
        info.restore_point_type = SYSTEM_RESTORE_APPLICATION_UNINSTALL;
        info.sequence_number = sequence_number;
        let mut status: StateManagerStatus = zeroed();
        let ok = restore(&mut info, &mut status) != 0;
        let result = if ok {
            0
        } else if status.status != 0 {
            status.status as i32
        } else {
            ERROR_GEN_FAILURE
        };
        let _ = FreeLibrary(module);
        result
    }
}

pub fn delete_scheduled_task(name: &str) -> i32 {
    let program_path = match system_program_path("schtasks.exe") {
        Ok(path) => path,
        Err(error) => return error,
    };
    let command = format!(
        "{} /Delete /TN {} /F",
        quote_windows_argument(&program_path),
        quote_windows_argument(name)
    );
    let (launch_or_wait_error, exit_code) = run_cmd(&command, true);
    if launch_or_wait_error != 0 {
        launch_or_wait_error
    } else {
        i32::try_from(exit_code).unwrap_or(i32::MAX)
    }
}

fn system_program_path(program: &str) -> Result<String, i32> {
    let mut system_dir = vec![0u16; 32_768];
    let system_dir_len = unsafe {
        GetSystemDirectoryW(system_dir.as_mut_ptr(), system_dir.len() as u32)
    } as usize;
    if system_dir_len == 0 {
        return Err(unsafe { GetLastError() } as i32);
    }
    if system_dir_len >= system_dir.len() {
        return Err(ERROR_INSUFFICIENT_BUFFER);
    }
    system_dir.truncate(system_dir_len);
    let system_dir = String::from_utf16_lossy(&system_dir);
    Ok(format!("{}\\{}", system_dir.trim_end_matches(['\\', '/']), program))
}

fn capture_command_oem(program: &str, arguments: &[&str]) -> Option<String> {
    // Administrative utilities are resolved from System32, never from an
    // application or current working directory that might be user-writable.
    let program_path = system_program_path(program).ok()?;

    // Quote every token according to CreateProcessW rules and reuse the bounded
    // pipe reader. A launch failure, timeout, or non-zero exit is not an empty list.
    let command_line = std::iter::once(program_path.as_str())
        .chain(arguments.iter().copied())
        .map(quote_windows_argument)
        .collect::<Vec<_>>()
        .join(" ");
    let (output, exit_code) = capture_process_output(&command_line)?;
    if exit_code != 0 {
        return None;
    }
    Some(
        decode_multibyte(&output, CP_OEMCP, 0)
            .or_else(|| decode_multibyte(&output, CP_ACP, 0))
            .unwrap_or_else(|| String::from_utf8_lossy(&output).into_owned()),
    )
}

/// Quoted or whitespace-separated tokens of a command line.
fn command_arguments(command_line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for ch in command_line.chars() {
        match ch {
            '"' => quoted = !quoted,
            ' ' | '\t' if !quoted => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// A task belongs to the app when its command starts an executable in the app
/// folder, or hands a path in that folder to a script host. A sibling such as
/// `Demo2` is not inside `Demo`.
fn command_targets_folder(folder: &str, command_line: &str) -> bool {
    let executable = command_executable(command_line);
    if executable.is_empty() {
        return false;
    }
    if path_is_under(folder, &executable) {
        return true;
    }
    let name = executable
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or_default()
        .to_lowercase();
    if !matches!(
        name.as_str(),
        "cmd.exe"
            | "powershell.exe"
            | "pwsh.exe"
            | "wscript.exe"
            | "cscript.exe"
            | "mshta.exe"
            | "python.exe"
            | "py.exe"
            | "node.exe"
    ) {
        return false;
    }
    command_arguments(command_line)
        .iter()
        .any(|argument| path_is_under(folder, argument))
}

/// Parse Windows CSV output without depending on localized column labels.
/// `schtasks /Query /FO CSV /V /NH` keeps the documented column order while
/// suppressing localized headings; TaskName is column 1 and Task To Run column 8.
fn parse_csv_rows(input: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(ch);
            }
            continue;
        }

        match ch {
            '"' if field.is_empty() => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' | '\n' => {
                if ch == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                if row.iter().any(|value| !value.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            _ => field.push(ch),
        }
    }

    if quoted {
        return None;
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        if row.iter().any(|value| !value.is_empty()) {
            rows.push(row);
        }
    }
    Some(rows)
}

fn parse_tasks(listing: &str, folder: &str) -> Option<Vec<String>> {
    let rows = parse_csv_rows(listing)?;
    let mut result = Vec::new();
    for row in rows {
        // Fail closed if a row is malformed: silently skipping it could leave a
        // task pointing into an installation directory that Purge then removes.
        if row.len() < 9 {
            return None;
        }
        let task_name = row[1].trim();
        let command = row[8].trim();
        if !task_name.is_empty() && command_targets_folder(folder, command) {
            result.push(task_name.to_string());
        }
    }
    Some(result)
}

/// `None` means task discovery was incomplete, not that there were no tasks.
pub fn tasks_under(path: &str) -> Option<Vec<String>> {
    if path.trim().is_empty() {
        return None;
    }
    let listing = capture_command_oem("schtasks.exe", &["/Query", "/FO", "CSV", "/V", "/NH"])?;
    if listing.trim().is_empty() {
        return None;
    }
    parse_tasks(&listing, path)
}

pub fn path_size(path: &str) -> u64 {
    fn size(path: &Path, depth: u8) -> u64 {
        if depth > 8 {
            return 0;
        }
        let Ok(metadata) = std::fs::symlink_metadata(path) else {
            return 0;
        };
        if metadata.file_type().is_symlink() || is_reparse_point(path) {
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
    fn task_discovery_rejects_blank_paths_and_uses_path_aware_csv_fields() {
        assert_eq!(tasks_under("   "), None);
        let listing = r#""localhost","\Demo\Sibling","N/A","Ready","Interactive","N/A","0","User","C:\Apps\Foobar\tool.exe"
"localhost","\Demo\Owned","N/A","Ready","Interactive","N/A","0","User","C:\Apps\Foo\tool.exe"
"#;
        assert_eq!(
            parse_tasks(listing, r"C:\Apps\Foo"),
            Some(vec![r"\Demo\Owned".to_string()])
        );
        assert_eq!(parse_tasks("", r"C:\Apps\Foo"), Some(Vec::new()));
        assert_eq!(parse_tasks(r#""host","incomplete""#, r"C:\Apps\Foo"), None);
        assert_eq!(parse_tasks(r#""host","unterminated"#, r"C:\Apps\Foo"), None);
    }

    #[test]
    fn unsupported_namespace_paths_fail_closed_for_destructive_operations() {
        assert!(is_protected_or_unsupported_path(r"\\?\C:\Windows"));
        assert!(is_protected_or_unsupported_path(r"\\.\C:\Windows"));
        assert!(is_protected_or_unsupported_path(""));
        assert!(is_protected_or_unsupported_path(r"\\?\GLOBALROOT\Device\HarddiskVolume1"));
    }

    #[test]
    fn restore_point_structs_match_the_windows_pack_one_layout() {
        assert_eq!(std::mem::size_of::<RestorePointInfoW>(), 528);
        assert_eq!(std::mem::offset_of!(RestorePointInfoW, sequence_number), 8);
        assert_eq!(std::mem::offset_of!(RestorePointInfoW, description), 16);
        assert_eq!(std::mem::size_of::<StateManagerStatus>(), 12);
        assert_eq!(std::mem::offset_of!(StateManagerStatus, sequence_number), 4);
    }

    #[test]
    fn wide_strings_are_nul_terminated_utf16() {
        assert_eq!(wide("ab"), vec![0x61, 0x62, 0]);
        assert_eq!(wide(""), vec![0]);
        assert_eq!(wide("ا"), vec![0x0627, 0]);
        assert_eq!(wide("😀"), vec![0xD83D, 0xDE00, 0]);
    }

    #[test]
    fn long_path_conversion_preserves_drive_and_unc_paths() {
        assert_eq!(long_path(r"C:\Apps\Demo"), r"\\?\C:\Apps\Demo");
        assert_eq!(
            long_path(r"\\server\share\Demo"),
            r"\\?\UNC\server\share\Demo"
        );
        assert_eq!(long_path(r"\\?\C:\Apps\Demo"), r"\\?\C:\Apps\Demo");
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
        assert!(path_is_under(
            r"\\server\share\Foo",
            r"\\server\share\Foo\a.exe"
        ));
    }

    #[test]
    fn deletion_guard_rejects_drive_and_unc_roots() {
        assert!(is_root_path(r"C:\"));
        assert!(is_root_path("C:"));
        assert!(is_root_path(r"\\"));
        assert!(is_root_path(r"\\server\share"));
        assert!(is_root_path(r"\\?\C:\"));
        assert!(is_root_path(r"\\?\UNC\server\share"));
        assert!(is_root_path(r"\\?\unc\server\share"));
        assert!(is_root_path(
            r"\\?\Volume{01234567-89ab-cdef-0123-456789abcdef}\"
        ));
        assert!(is_root_path(
            r"\\?\GLOBALROOT\Device\HarddiskVolumeShadowCopy1"
        ));
        assert!(is_root_path(r"\\?\Device\HarddiskVolume1"));
        assert!(!is_root_path(r"C:\Apps\Demo"));
        assert!(!is_root_path(r"\\server\share\Demo"));
        assert!(!is_root_path(r"\\?\UNC\server\share\Demo"));
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
    fn a_task_belongs_to_the_folder_only_through_its_command() {
        let folder = r"C:\Apps\Demo";
        assert!(command_targets_folder(
            folder,
            r"C:\Apps\Demo\app.exe --silent"
        ));
        assert!(command_targets_folder(
            folder,
            r#"cmd.exe /c "C:\Apps\Demo\run.bat""#
        ));
        assert!(command_targets_folder(
            folder,
            r"powershell.exe -File C:\Apps\Demo\start.ps1"
        ));
        // A sibling folder that merely shares the prefix is not inside it.
        assert!(!command_targets_folder(folder, r"C:\Apps\Demo2\app.exe"));
        assert!(!command_targets_folder(
            folder,
            r#"cmd.exe /c echo C:\Apps\Demo2\x"#
        ));
        assert!(!command_targets_folder(folder, r"C:\Other\app.exe"));
        assert!(!command_targets_folder(folder, "cmd.exe /c echo hello"));
        assert!(!command_targets_folder(folder, ""));
    }

    #[test]
    fn task_listing_matches_the_command_and_keeps_every_result() {
        let folder = r"C:\Apps\Demo";
        // TaskName, Author and Comment mention the folder; only the command can
        // make a task the app's.
        let listing = [
            r"TaskName: \Decoy",
            r"Author: C:\Apps\Demo\owner",
            r"Comment: started by C:\Apps\Demo",
            r"Task To Run: C:\Other\app.exe",
            "",
            r"TaskName: \Real",
            r"Task To Run: C:\Apps\Demo\app.exe /run",
            "",
        ]
        .join("\r\n");
        assert_eq!(parse_tasks(&listing, folder), vec![r"\Real"]);

        // More than 64 matches used to be dropped silently.
        let mut many = Vec::new();
        for index in 0..70 {
            many.push(format!(r"TaskName: \T{index}"));
            many.push(r"Task To Run: C:\Apps\Demo\app.exe".to_string());
            many.push(String::new());
        }
        assert_eq!(parse_tasks(&many.join("\r\n"), folder).len(), 70);
    }

    #[test]
    fn windows_argument_quoting_preserves_spaces_and_trailing_slashes() {
        assert_eq!(quote_windows_argument("plain"), "plain");
        assert_eq!(
            quote_windows_argument("task with spaces"),
            r#""task with spaces""#
        );
        assert_eq!(
            quote_windows_argument(r"C:\Apps\Demo\"),
            r#""C:\Apps\Demo\\""#
        );
    }
}
