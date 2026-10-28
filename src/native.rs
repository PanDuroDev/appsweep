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
    take_string(
        |out, cap| unsafe { as_current_user_sid(out, cap) },
        256,
    )
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
    let ok = unsafe { as_extract_icon(wpath.as_ptr(), size as i32, &mut ptr, &mut w, &mut h, &mut stride) };
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
            if cwd.is_some() { c.as_ptr() } else { std::ptr::null() },
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
            if cwd.is_some() { c.as_ptr() } else { std::ptr::null() },
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
        as_force_delete(w.as_ptr(), &mut deleted, &mut failed, log.as_mut_ptr(), 2048);
    }
    let end = log.iter().position(|&c| c == 0).unwrap_or(log.len());
    DeleteReport {
        deleted,
        failed,
        first_error: String::from_utf16_lossy(&log[..end]),
    }
}

pub fn path_exists(path: &str) -> bool {
    let w = wide(path);
    unsafe { as_path_exists(w.as_ptr()) != 0 }
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
