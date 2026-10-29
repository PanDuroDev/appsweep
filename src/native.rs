// FFI bindings to native/helper.cpp
#![allow(dead_code)]

use std::ffi::c_void;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

extern "C" {
    fn as_free(p: *mut c_void);
    fn as_is_elevated() -> i32;
    fn as_current_user_sid(out: *mut u16, cap: i32) -> i32;
    fn as_windows_build() -> i32;

    fn as_extract_icon(
        path: *const u16,
        size: i32,
        out: *mut *mut u8,
        out_w: *mut i32,
        out_h: *mut i32,
        out_stride: *mut i32,
    ) -> i32;

    fn as_open_in_explorer(path: *const u16, select: i32) -> i32;
    fn as_show_properties(path: *const u16) -> i32;
    fn as_recycle_path(path: *const u16) -> i32;
    fn as_shell_open(what: *const u16) -> i32;

    fn as_run_cmd(cmdline: *const u16, wait: i32, exit_code: *mut u32) -> i32;
    fn as_run_shell(
        file: *const u16,
        params: *const u16,
        cwd: *const u16,
        wait: i32,
        exit_code: *mut u32,
    ) -> i32;
    fn as_run_elevated(
        file: *const u16,
        params: *const u16,
        cwd: *const u16,
        wait: i32,
        exit_code: *mut u32,
    ) -> i32;
    fn as_capture_cmd(cmdline: *const u16, out: *mut u16, cap: i32) -> i32;

    fn as_resolve_shortcut(
        lnk: *const u16,
        target: *mut u16,
        tcap: i32,
        args: *mut u16,
        acap: i32,
        workdir: *mut u16,
        wcap: i32,
    ) -> i32;

    fn as_count_processes_in_path(path: *const u16) -> i32;
    fn as_kill_processes_in_path(path: *const u16) -> i32;
    fn as_path_is_under(folder: *const u16, candidate: *const u16) -> i32;
    fn as_command_executable(cmd: *const u16, out: *mut u16, cap: i32) -> i32;
    fn as_list_services_under(path: *const u16, out: *mut u16, cap: i32) -> i32;
    fn as_stop_delete_service(name: *const u16) -> i32;

    fn as_force_delete(
        path: *const u16,
        deleted: *mut i32,
        failed: *mut i32,
        log: *mut u16,
        logcap: i32,
    ) -> i32;
    fn as_path_exists(path: *const u16) -> i32;
    fn as_path_state(path: *const u16) -> i32;
    fn as_service_exists(name: *const u16) -> i32;
    fn as_task_exists(name: *const u16) -> i32;
    fn as_grants_everyone_full_control(path: *const u16) -> i32;
    fn as_grant_everyone_full_control(path: *const u16) -> i32;
    fn as_delete_on_reboot(path: *const u16) -> i32;
    fn as_create_restore_point(desc: *const u16) -> i32;
    fn as_delete_scheduled_task(name: *const u16) -> i32;
    fn as_list_tasks_under(path: *const u16, out: *mut u16, cap: i32) -> i32;
    fn as_path_size(path: *const u16) -> u64;
    fn as_ensure_dir(path: *const u16) -> i32;
}

fn take_string(f: impl FnOnce(*mut u16, i32) -> i32, cap: usize) -> String {
    let mut buf = vec![0u16; cap];
    f(buf.as_mut_ptr(), cap as i32);
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

pub fn is_elevated() -> bool {
    unsafe { as_is_elevated() != 0 }
}

pub fn windows_build() -> u32 {
    unsafe { as_windows_build() as u32 }
}

pub fn current_user_sid() -> String {
    take_string(|out, cap| unsafe { as_current_user_sid(out, cap) }, 256)
}

pub struct RawIcon {
    pub w: usize,
    pub h: usize,
    /// RGBA, unmultiplied, ready for `ColorImage::from_rgba_unmultiplied`.
    pub rgba: Vec<u8>,
}

pub fn extract_icon(path: &str, size: u32) -> Option<RawIcon> {
    let wpath = wide(path);
    let mut ptr: *mut u8 = std::ptr::null_mut();
    let (mut w, mut h, mut stride) = (0i32, 0i32, 0i32);
    let ok = unsafe {
        as_extract_icon(
            wpath.as_ptr(),
            size as i32,
            &mut ptr,
            &mut w,
            &mut h,
            &mut stride,
        )
    };
    if ok == 0 || ptr.is_null() || w <= 0 || h <= 0 {
        return None;
    }
    let (wu, hu, su) = (w as usize, h as usize, stride as usize);
    let mut bgra = vec![0u8; wu * hu * 4];
    unsafe {
        std::ptr::copy_nonoverlapping(ptr, bgra.as_mut_ptr(), bgra.len());
        as_free(ptr as *mut c_void);
    }
    if su != wu * 4 {
        return None;
    }
    let mut any_alpha = false;
    for i in 0..(wu * hu) {
        if bgra[i * 4 + 3] != 0 {
            any_alpha = true;
            break;
        }
    }
    let mut rgba = vec![0u8; wu * hu * 4];
    for i in 0..(wu * hu) {
        let b = bgra[i * 4];
        let g = bgra[i * 4 + 1];
        let r = bgra[i * 4 + 2];
        let a = if any_alpha { bgra[i * 4 + 3] } else { 255 };
        rgba[i * 4] = r;
        rgba[i * 4 + 1] = g;
        rgba[i * 4 + 2] = b;
        rgba[i * 4 + 3] = a;
    }
    Some(RawIcon { w: wu, h: hu, rgba })
}

pub fn open_in_explorer(path: &str, select: bool) -> i32 {
    let w = wide(path);
    unsafe { as_open_in_explorer(w.as_ptr(), select as i32) }
}

pub fn show_properties(path: &str) -> i32 {
    let w = wide(path);
    unsafe { as_show_properties(w.as_ptr()) }
}

pub fn recycle(path: &str) -> i32 {
    let w = wide(path);
    unsafe { as_recycle_path(w.as_ptr()) }
}

pub fn shell_open(what: &str) -> i32 {
    let w = wide(what);
    unsafe { as_shell_open(w.as_ptr()) }
}

pub fn run_cmd(cmdline: &str, wait: bool) -> (i32, u32) {
    let w = wide(cmdline);
    let mut code = 0u32;
    let rc = unsafe { as_run_cmd(w.as_ptr(), wait as i32, &mut code) };
    (rc, code)
}

pub fn run_shell(file: &str, params: &str, cwd: Option<&str>, wait: bool) -> (i32, u32) {
    let f = wide(file);
    let p = wide(params);
    let c = wide(cwd.unwrap_or(""));
    let mut code = 0u32;
    let rc = unsafe {
        as_run_shell(
            f.as_ptr(),
            p.as_ptr(),
            if cwd.is_some() {
                c.as_ptr()
            } else {
                std::ptr::null()
            },
            wait as i32,
            &mut code,
        )
    };
    (rc, code)
}

pub fn run_elevated(file: &str, params: &str, cwd: Option<&str>, wait: bool) -> (i32, u32) {
    let f = wide(file);
    let p = wide(params);
    let c = wide(cwd.unwrap_or(""));
    let mut code = 0u32;
    let rc = unsafe {
        as_run_elevated(
            f.as_ptr(),
            p.as_ptr(),
            if cwd.is_some() {
                c.as_ptr()
            } else {
                std::ptr::null()
            },
            wait as i32,
            &mut code,
        )
    };
    (rc, code)
}

pub fn capture_cmd(cmdline: &str) -> String {
    take_string(
        |out, cap| unsafe {
            let w = wide(cmdline);
            as_capture_cmd(w.as_ptr(), out, cap)
        },
        1024 * 256,
    )
}

pub fn resolve_shortcut(lnk: &str) -> Option<(String, String, String)> {
    let w = wide(lnk);
    let mut t = vec![0u16; 2048];
    let mut a = vec![0u16; 2048];
    let mut d = vec![0u16; 2048];
    let rc = unsafe {
        as_resolve_shortcut(
            w.as_ptr(),
            t.as_mut_ptr(),
            2048,
            a.as_mut_ptr(),
            2048,
            d.as_mut_ptr(),
            2048,
        )
    };
    if rc != 0 {
        return None;
    }
    let s = |v: &Vec<u16>| {
        let end = v.iter().position(|&c| c == 0).unwrap_or(v.len());
        String::from_utf16_lossy(&v[..end])
    };
    Some((s(&t), s(&a), s(&d)))
}

pub fn count_processes_in(path: &str) -> i32 {
    let w = wide(path);
    unsafe { as_count_processes_in_path(w.as_ptr()) }
}

pub fn kill_processes_in(path: &str) -> i32 {
    let w = wide(path);
    unsafe { as_kill_processes_in_path(w.as_ptr()) }
}

/// Is `candidate` the folder itself or a file inside it? Same rule the helper
/// uses before it counts or kills a process.
pub fn path_is_under(folder: &str, candidate: &str) -> bool {
    let f = wide(folder);
    let c = wide(candidate);
    unsafe { as_path_is_under(f.as_ptr(), c.as_ptr()) != 0 }
}

pub fn command_executable(cmd: &str) -> String {
    let w = wide(cmd);
    take_string(
        |out, cap| unsafe { as_command_executable(w.as_ptr(), out, cap) },
        1024,
    )
}

pub fn services_under(path: &str) -> Vec<String> {
    let s = take_string(
        |out, cap| unsafe {
            let w = wide(path);
            as_list_services_under(w.as_ptr(), out, cap)
        },
        64 * 1024,
    );
    s.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

pub fn stop_delete_service(name: &str) -> i32 {
    let w = wide(name);
    unsafe { as_stop_delete_service(w.as_ptr()) }
}

/// Capped by the helper's buffer. A list longer than that comes back short, and
/// the folder's remaining tasks are simply not offered for cleanup.
pub fn tasks_under(path: &str) -> Vec<String> {
    let s = take_string(
        |out, cap| unsafe {
            let w = wide(path);
            as_list_tasks_under(w.as_ptr(), out, cap)
        },
        64 * 1024,
    );
    s.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

pub struct DeleteReport {
    pub deleted: i32,
    pub failed: i32,
    pub first_error: String,
}

pub fn force_delete(path: &str) -> DeleteReport {
    let w = wide(path);
    let mut deleted = 0i32;
    let mut failed = 0i32;
    let mut log = vec![0u16; 2048];
    unsafe {
        as_force_delete(
            w.as_ptr(),
            &mut deleted,
            &mut failed,
            log.as_mut_ptr(),
            2048,
        );
    }
    let end = log.iter().position(|&c| c == 0).unwrap_or(log.len());
    DeleteReport {
        deleted,
        failed,
        first_error: String::from_utf16_lossy(&log[..end]),
    }
}

/// Self-test helper: 1 when the object's DACL still grants Everyone full
/// control, 0 when it does not, -1 when it could not be read.
pub fn grants_everyone_full_control(path: &str) -> i32 {
    let w = wide(path);
    unsafe { as_grants_everyone_full_control(w.as_ptr()) }
}

/// Self-test only: plant an Everyone-full-control ACE so the detector above can
/// be proven to work.
pub fn grant_everyone_full_control(path: &str) -> i32 {
    let w = wide(path);
    unsafe { as_grant_everyone_full_control(w.as_ptr()) }
}

pub fn path_exists(path: &str) -> bool {
    let w = wide(path);
    unsafe { as_path_exists(w.as_ptr()) != 0 }
}

/// `Some(true)` there, `Some(false)` confirmed missing, `None` inaccessible.
/// Access denied must not read as "already gone".
pub fn path_state(path: &str) -> Option<bool> {
    let w = wide(path);
    match unsafe { as_path_state(w.as_ptr()) } {
        1 => Some(true),
        0 => Some(false),
        _ => None,
    }
}

pub fn service_exists(name: &str) -> Option<bool> {
    let w = wide(name);
    match unsafe { as_service_exists(w.as_ptr()) } {
        1 => Some(true),
        0 => Some(false),
        _ => None,
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
    let w = wide(name);
    match unsafe { as_task_exists(w.as_ptr()) } {
        1 => Some(true),
        0 => Some(false),
        _ => None,
    }
}

pub fn delete_on_reboot(path: &str) -> i32 {
    let w = wide(path);
    unsafe { as_delete_on_reboot(w.as_ptr()) }
}

pub fn create_restore_point(desc: &str) -> i32 {
    let w = wide(desc);
    unsafe { as_create_restore_point(w.as_ptr()) }
}

pub fn delete_scheduled_task(name: &str) -> i32 {
    let w = wide(name);
    unsafe { as_delete_scheduled_task(w.as_ptr()) }
}

pub fn path_size(path: &str) -> u64 {
    let w = wide(path);
    unsafe { as_path_size(w.as_ptr()) }
}

pub fn ensure_dir(path: &str) -> i32 {
    let w = wide(path);
    unsafe { as_ensure_dir(w.as_ptr()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_strings_are_nul_terminated_utf16() {
        assert_eq!(wide("ab"), vec![0x61, 0x62, 0]);
        assert_eq!(wide(""), vec![0]);
        assert_eq!(wide("ا"), vec![0x0627, 0]);
        // Non-BMP characters must become a surrogate pair, not be dropped.
        assert_eq!(wide("😀"), vec![0xD83D, 0xDE00, 0]);
    }

    #[test]
    fn a_process_inside_a_sibling_folder_is_not_related() {
        // Issue #4: a prefix test made "C:\\Apps\\Foo" claim "C:\\Apps\\Foobar".
        assert!(path_is_under("C:\\Apps\\Foo", "C:\\Apps\\Foo\\app.exe"));
        assert!(path_is_under("C:\\Apps\\Foo", "C:\\Apps\\Foo"));
        assert!(path_is_under("C:\\Apps\\Foo\\", "C:\\Apps\\Foo\\app.exe"));
        assert!(path_is_under("c:\\apps\\foo", "C:\\Apps\\Foo\\App.EXE"));
        assert!(!path_is_under(
            "C:\\Apps\\Foo",
            "C:\\Apps\\Foobar\\other.exe"
        ));
        assert!(!path_is_under("C:\\Apps\\Foo", "C:\\Apps\\Foo2"));
        assert!(!path_is_under("C:\\Apps\\Foo", "C:\\Apps\\Fo"));
        assert!(!path_is_under("C:\\Apps\\Foo", "D:\\Tools\\Foo\\a.exe"));
        assert!(!path_is_under("C:\\Apps\\Foo", ""));
    }

    #[test]
    fn a_service_command_line_resolves_to_its_executable() {
        // Issue #4: matching the whole command line let an argument that names
        // the folder pull an unrelated service into the delete plan.
        assert_eq!(
            command_executable("\"C:\\Apps\\Foo\\foo.exe\" --service"),
            "C:\\Apps\\Foo\\foo.exe"
        );
        assert_eq!(
            command_executable("C:\\Apps\\Foo\\foo.exe -k netsvcs"),
            "C:\\Apps\\Foo\\foo.exe"
        );
        assert_eq!(
            command_executable("\"C:\\Program Files\\Foo Bar\\a b.exe\" /run"),
            "C:\\Program Files\\Foo Bar\\a b.exe"
        );
        assert_eq!(
            command_executable("C:\\Apps\\Foobar\\other.exe -c \"C:\\Apps\\Foo\""),
            "C:\\Apps\\Foobar\\other.exe"
        );
    }
}
