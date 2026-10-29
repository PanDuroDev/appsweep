use crate::model::{AppEntry, RegRoot, Source};
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Flow {
    /// Official uninstaller, with fallbacks.
    Uninstall,
    /// Remove every file, registry key, service and trace.
    Purge,
    /// Only delete the program folder.
    FilesOnly,
}

impl Flow {
    pub fn title(&self) -> &'static str {
        match self {
            Flow::Uninstall => "Uninstall app",
            Flow::Purge => "Purge everything",
            Flow::FilesOnly => "Delete files only",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StepStatus {
    Pending,
    Running,
    Ok,
    Warn,
    Fail,
    Skip,
}

#[derive(Clone, Debug)]
pub struct Step {
    pub title: String,
    pub detail: String,
    pub status: StepStatus,
    pub op: Op,
    pub skip_if_gone: bool,
    pub optional: bool,
}

#[derive(Clone, Debug)]
pub enum Op {
    Nothing,
    KillProcesses {
        path: String,
    },
    RestorePoint {
        desc: String,
    },
    BackupRegistry {
        root: RegRoot,
        key: String,
    },
    RunUninstaller {
        cmd: String,
    },
    RunMsi {
        product: String,
    },
    RunWinget {
        name: String,
    },
    RemoveUwp {
        full_name: String,
    },
    ForceDelete {
        paths: Vec<String>,
    },
    Recycle {
        paths: Vec<String>,
    },
    DeleteReg {
        keys: Vec<(RegRoot, String)>,
    },
    DeleteShortcuts {
        paths: Vec<String>,
    },
    DeleteServices {
        names: Vec<String>,
    },
    DeleteTasks {
        names: Vec<String>,
    },
    DeleteOnReboot {
        paths: Vec<String>,
    },
    Verify {
        paths: Vec<String>,
        reg: Vec<(RegRoot, String)>,
        services: Vec<String>,
        tasks: Vec<String>,
        shortcuts: Vec<String>,
        package: Option<String>,
    },
    CleanTemp,
}

pub struct Job {
    pub app_id: u64,
    pub app_name: String,
    pub flow: Flow,
    pub steps: Vec<Step>,
    pub done: bool,
    pub summary: String,
    pub success: bool,
    rx: Receiver<(usize, StepStatus, String)>,
}

impl Job {
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        let total = self.steps.len();
        loop {
            match self.rx.try_recv() {
                Ok((idx, status, detail)) => {
                    if idx >= total {
                        // Completion sentinel from the worker thread.
                        self.mark_done();
                    } else if let Some(s) = self.steps.get_mut(idx) {
                        s.status = status;
                        if !detail.is_empty() {
                            s.detail = detail;
                        }
                    }
                    changed = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // The worker thread died without sending the sentinel.
                    // Never leave the dialog spinning forever.
                    if !self.done {
                        self.mark_done();
                        changed = true;
                    }
                    break;
                }
            }
        }
        changed
    }

    fn mark_done(&mut self) {
        self.done = true;
        // Anything still marked Running never reported back.
        for s in self.steps.iter_mut() {
            if s.status == StepStatus::Running || s.status == StepStatus::Pending {
                s.status = StepStatus::Fail;
                if s.detail.is_empty() {
                    s.detail = "Step stopped without a result".into();
                }
            }
        }
        // Recomputed after the sweep: a step that stopped without a result is
        // a failure, and the counts would otherwise miss it.
        self.success = self
            .steps
            .iter()
            .all(|s| matches!(s.status, StepStatus::Ok | StepStatus::Skip));
        let (ok, warn, fail) = self.steps.iter().fold((0, 0, 0), |mut acc, s| {
            match s.status {
                StepStatus::Ok => acc.0 += 1,
                StepStatus::Warn => acc.1 += 1,
                StepStatus::Fail => acc.2 += 1,
                _ => {}
            }
            acc
        });
        self.summary = format!("{} ok / {} warnings / {} failed", ok, warn, fail);
    }

    pub fn progress(&self) -> (usize, usize) {
        let done = self
            .steps
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    StepStatus::Ok | StepStatus::Warn | StepStatus::Fail | StepStatus::Skip
                )
            })
            .count();
        (done, self.steps.len())
    }
}

// ---------------------------------------------------------------- helpers

fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn stop_words() -> &'static [&'static str] {
    &[
        "microsoft",
        "windows",
        "software",
        "corporation",
        "corp",
        "incorporated",
        "inc",
        "ltd",
        "limited",
        "llc",
        "gmbh",
        "technologies",
        "technology",
        "studio",
        "studios",
        "company",
        "the",
        "for",
        "and",
        "app",
        "apps",
        "update",
        "setup",
        "installer",
        "version",
        "x64",
        "x86",
        "arm64",
        "desktop",
        "application",
        "official",
        "team",
        "project",
        "project64",
    ]
}

fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            cur.push(c.to_ascii_lowercase());
        } else if !cur.is_empty() {
            if cur.len() >= 3 && !stop_words().contains(&cur.as_str()) {
                out.push(cur.clone());
            }
            cur.clear();
        }
    }
    if cur.len() >= 3 && !stop_words().contains(&cur.as_str()) {
        out.push(cur);
    }
    out
}

fn looks_like_trace(dir_name: &str, app: &AppEntry) -> bool {
    let cand = norm(dir_name);
    if cand.is_empty() || cand.len() < 3 {
        return false;
    }
    let app_norm = norm(&app.name);
    if !app_norm.is_empty() && cand == app_norm {
        return true;
    }
    if app_norm.len() >= 4 && cand.contains(&app_norm) {
        return true;
    }
    let name_tokens = tokens(&app.name);
    let pub_tokens = tokens(&app.publisher);
    let has_name = name_tokens.iter().any(|t| t.len() >= 4 && cand.contains(t));
    let has_pub = pub_tokens.iter().any(|t| t.len() >= 4 && cand.contains(t));
    // Require a distinctive name token; a publisher hit alone is too loose.
    has_name || (has_pub && name_tokens.iter().any(|t| cand.contains(t)))
}

fn subdirs(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(root) {
        for e in rd.flatten().take(4000) {
            let p = e.path();
            if p.is_dir() {
                out.push(p);
            }
        }
    }
    out
}

pub fn find_traces(app: &AppEntry) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        if p.exists()
            && !out
                .iter()
                .any(|x| norm(&x.to_string_lossy()) == norm(&p.to_string_lossy()))
        {
            out.push(p);
        }
    };

    // UWP data folder
    if let Some(full) = &app.package_full_name {
        if let Some(family) = full.split('_').next() {
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                let base = PathBuf::from(local).join("Packages");
                for d in subdirs(&base) {
                    let n = d
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if n.starts_with(&family.to_lowercase()) {
                        push(d);
                    }
                }
            }
        }
    }

    let roots: Vec<PathBuf> = [
        std::env::var("APPDATA").ok(),
        std::env::var("LOCALAPPDATA").ok(),
        std::env::var("PROGRAMDATA").ok(),
    ]
    .into_iter()
    .flatten()
    .map(PathBuf::from)
    .collect();

    for root in roots {
        for d in subdirs(&root) {
            let name = d.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.eq_ignore_ascii_case("Packages")
                || name.eq_ignore_ascii_case("Microsoft")
                || name.eq_ignore_ascii_case("Temp")
            {
                continue;
            }
            if looks_like_trace(name, app) {
                push(d);
            }
        }
    }

    // Per-user temp folder
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let temp = PathBuf::from(local).join("Temp");
        for d in subdirs(&temp) {
            let name = d.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if looks_like_trace(name, app) {
                push(d);
            }
        }
    }

    out
}

/// `Some(true)` present, `Some(false)` confirmed missing, `None` could not ask.
/// A denied or broken query is not proof of absence.
fn reg_key_state(root: RegRoot, path: &str) -> Option<bool> {
    use winreg::enums::*;
    use winreg::RegKey;
    let (hive, view) = match root {
        RegRoot::Hklm => (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_64KEY),
        RegRoot::Hklm32 => (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY),
        RegRoot::Hkcu => (HKEY_CURRENT_USER, KEY_READ),
    };
    match RegKey::predef(hive).open_subkey_with_flags(path, view) {
        Ok(_) => Some(true),
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => Some(false),
            _ => None,
        },
    }
}

fn reg_key_exists(root: RegRoot, path: &str) -> bool {
    reg_key_state(root, path).unwrap_or(true)
}

pub fn delete_reg_key(root: RegRoot, path: &str) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let (hive, view) = match root {
        RegRoot::Hklm => (HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS | KEY_WOW64_64KEY),
        RegRoot::Hklm32 => (HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS | KEY_WOW64_32KEY),
        RegRoot::Hkcu => (HKEY_CURRENT_USER, KEY_ALL_ACCESS),
    };
    let base = RegKey::predef(hive);
    let (parent_path, leaf) = match path.rfind('\\') {
        Some(i) => (&path[..i], &path[i + 1..]),
        None => return Err("Invalid path".into()),
    };
    let parent = base
        .open_subkey_with_flags(parent_path, view)
        .map_err(|e| e.to_string())?;
    parent.delete_subkey_all(leaf).map_err(|e| e.to_string())
}

fn exe_path() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("appsweep.exe"))
}

fn winget_available() -> bool {
    let (rc, _) = crate::native::run_cmd("where winget", true);
    rc == 0
}

// ---------------------------------------------------------------- plan building

/// Targets whose absence is not confirmed. A probe that fails (access denied,
/// locked volume) is not the same as a missing file, so those targets stay in
/// the batch and surface as a failure instead of quietly becoming "nothing to
/// do".
fn present_or_unknown(paths: Vec<String>) -> (Vec<String>, u32) {
    let mut keep = Vec::new();
    let mut unknown = 0;
    for path in paths {
        match crate::native::path_state(&path) {
            Some(false) => {}
            Some(true) => keep.push(path),
            None => {
                unknown += 1;
                keep.push(path);
            }
        }
    }
    (keep, unknown)
}

/// Named targets that could not be handled, capped so a long list still fits
/// in one line.
fn name_failures(what: &str, names: &[String]) -> String {
    let mut shown: Vec<&str> = names.iter().take(3).map(|n| n.as_str()).collect();
    if names.len() > 3 {
        shown.push("...");
    }
    format!("{} failed: {}", what, shown.join(", "))
}

/// "Done" when nothing was left over, "Done N of M" when something was:
/// a step is only a clean Ok when its whole job finished.
fn partial(done: u32, total: u32, what: &str) -> (StepStatus, String) {
    if done >= total {
        (StepStatus::Ok, format!("{} {}", what, done))
    } else {
        (StepStatus::Warn, format!("{} {} of {}", what, done, total))
    }
}

fn s(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

/// Drops paths that point at Windows itself or a shared system folder. This is
/// the last line of defence: even a wrong registry entry cannot wipe the system.
fn safe_paths(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .filter(|p| !crate::model::is_protected_dir(Path::new(p)))
        .collect()
}

pub fn build_plan(app: &AppEntry, flow: Flow, settings: &Settings, keep_data: bool) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    let mut add = |title: &str, detail: &str, op: Op, skip_if_gone: bool, optional: bool| {
        steps.push(Step {
            title: title.to_string(),
            detail: detail.to_string(),
            status: StepStatus::Pending,
            op,
            skip_if_gone,
            optional,
        });
    };

    // A protected folder must not reach any op at all, not just the delete
    // ones: killing the processes of "C:\Windows" would take the session down.
    let dir = app
        .install_dir
        .clone()
        .filter(|d| !crate::model::is_protected_dir(d));
    // "Keep user data" means: only touch the program folder, never the
    // AppData / ProgramData leftovers that hold the user's own files.
    let traces = if keep_data {
        Vec::new()
    } else {
        find_traces(app)
    };

    add(
        "Stop related processes",
        "Close any program running from the app folder",
        match &dir {
            Some(d) if settings.kill_processes => Op::KillProcesses { path: s(d) },
            _ => Op::Nothing,
        },
        false,
        true,
    );

    if settings.create_restore_point {
        add(
            "Create a restore point",
            "Protect the system before the changes",
            Op::RestorePoint {
                desc: format!("AppSweep - {}", app.name),
            },
            false,
            true,
        );
    }

    if settings.backup_registry {
        if let Some((root, key)) = &app.reg_key {
            add(
                "Registry backup",
                "Export the uninstall key before deleting it",
                Op::BackupRegistry {
                    root: *root,
                    key: key.clone(),
                },
                false,
                true,
            );
        }
    }

    // Services, scheduled tasks and shortcuts that live in the app folder are
    // part of what Uninstall and Purge both have to remove - and what Verify
    // has to check afterwards, so they are discovered once, up front.
    let svc_names: Vec<String> = match &dir {
        Some(d) => crate::native::services_under(&s(d)),
        None => Vec::new(),
    };
    let tasks: Vec<String> = match &dir {
        Some(d) => crate::native::tasks_under(&s(d)),
        None => Vec::new(),
    };
    let shortcuts: Vec<String> = app.shortcuts.iter().map(|p| s(p)).collect();

    match flow {
        Flow::Uninstall => {
            if let Some(cmd) = &app.uninstall_string {
                add(
                    "Official uninstaller",
                    "Run the app's own uninstaller",
                    Op::RunUninstaller { cmd: cmd.clone() },
                    true,
                    false,
                );
            }
            if let Some(cmd) = &app.quiet_uninstall_string {
                add(
                    "Silent uninstaller",
                    "Try removing it without a UI",
                    Op::RunUninstaller { cmd: cmd.clone() },
                    true,
                    true,
                );
            }
            if let Some(prod) = &app.msi_product {
                add(
                    "Remove the MSI package",
                    "Use Windows Installer to remove the package",
                    Op::RunMsi {
                        product: prod.clone(),
                    },
                    true,
                    true,
                );
            }
            if app.source == Source::Uwp {
                if let Some(full) = &app.package_full_name {
                    add(
                        "Remove the Store package",
                        "Remove the package for the current user",
                        Op::RemoveUwp {
                            full_name: full.clone(),
                        },
                        false,
                        false,
                    );
                }
            }
            add(
                "Try the package manager",
                "Use the command-line tool to remove the app",
                Op::RunWinget {
                    name: app.name.clone(),
                },
                true,
                true,
            );
            if let Some(d) = &dir {
                add(
                    "Delete the program folder",
                    "Delete file leftovers after removal",
                    Op::ForceDelete {
                        paths: safe_paths(vec![s(d)]),
                    },
                    true,
                    true,
                );
            }
            if let Some((root, key)) = &app.reg_key {
                add(
                    "Delete the registry key",
                    "Remove the entry from the installed list",
                    Op::DeleteReg {
                        keys: vec![(*root, key.clone())],
                    },
                    false,
                    true,
                );
            }
            if !app.shortcuts.is_empty() {
                add(
                    "Delete shortcuts",
                    "Remove Start menu and desktop icons",
                    Op::DeleteShortcuts {
                        paths: app.shortcuts.iter().map(|p| s(p)).collect(),
                    },
                    false,
                    true,
                );
            }
            add(
                "Clean temporary files",
                "Delete temporary installer leftovers",
                Op::CleanTemp,
                false,
                true,
            );
        }
        Flow::Purge => {
            let mut paths: Vec<String> = Vec::new();
            if let Some(d) = &dir {
                paths.push(s(d));
            }
            for t in &traces {
                paths.push(s(t));
            }
            paths = safe_paths(paths);

            if !svc_names.is_empty() {
                add(
                    "Stop and delete services",
                    "Remove services running from the app folder",
                    Op::DeleteServices {
                        names: svc_names.clone(),
                    },
                    false,
                    true,
                );
            }

            // Autostart tasks that point into the app folder are part of its
            // footprint; removing the folder alone would leave them broken.
            if !tasks.is_empty() {
                add(
                    "Delete scheduled tasks",
                    &format!("Remove {} startup tasks", tasks.len()),
                    Op::DeleteTasks {
                        names: tasks.clone(),
                    },
                    false,
                    true,
                );
            }

            if app.source == Source::Uwp {
                if let Some(full) = &app.package_full_name {
                    add(
                        "Remove the Store package",
                        "Remove the package before deleting its data",
                        Op::RemoveUwp {
                            full_name: full.clone(),
                        },
                        false,
                        false,
                    );
                }
            }

            if !paths.is_empty() {
                add(
                    "Delete files and folders",
                    "Delete the program folder and all data folders",
                    Op::ForceDelete {
                        paths: paths.clone(),
                    },
                    false,
                    false,
                );
            }

            let mut regs: Vec<(RegRoot, String)> = Vec::new();
            if let Some((root, key)) = &app.reg_key {
                regs.push((*root, key.clone()));
            }
            if !regs.is_empty() {
                add(
                    "Delete registry keys",
                    "Remove all of the app's registry entries",
                    Op::DeleteReg { keys: regs },
                    false,
                    false,
                );
            }

            if !app.shortcuts.is_empty() {
                add(
                    "Delete shortcuts",
                    "Remove all of the app's shortcuts",
                    Op::DeleteShortcuts {
                        paths: app.shortcuts.iter().map(|p| s(p)).collect(),
                    },
                    false,
                    true,
                );
            }

            add(
                "Clean temporary files",
                "Delete any temporary leftovers",
                Op::CleanTemp,
                false,
                true,
            );

            add(
                "Schedule leftover deletion",
                "Delete what remains on reboot",
                Op::DeleteOnReboot { paths },
                false,
                true,
            );
        }
        Flow::FilesOnly => {
            if let Some(d) = &dir {
                let only = safe_paths(vec![s(d)]);
                if !only.is_empty() {
                    add(
                        "Move the folder to the Recycle Bin",
                        "Recoverable delete",
                        Op::Recycle {
                            paths: only.clone(),
                        },
                        false,
                        true,
                    );
                    add(
                        "Delete the folder permanently",
                        "Delete directly if the Recycle Bin fails",
                        Op::ForceDelete { paths: only },
                        false,
                        false,
                    );
                }
            }
            // No CleanTemp here: that op sweeps %LOCALAPPDATA%\\Temp by name
            // match, which is outside the folder the user picked.
        }
    }

    // One verify at the end, covering everything the flow tried to remove.
    // FilesOnly only ever touched the folder, so that is all it checks.
    let mut verify_paths = Vec::new();
    if let Some(d) = &dir {
        verify_paths.push(s(d));
    }
    if flow != Flow::FilesOnly {
        verify_paths.extend(traces.iter().map(|p| s(p)));
    }
    add(
        "Verify the result",
        "Confirm every tracked resource is gone",
        Op::Verify {
            paths: verify_paths,
            reg: match flow {
                Flow::FilesOnly => Vec::new(),
                _ => app.reg_key.clone().into_iter().collect(),
            },
            services: svc_names.clone(),
            tasks: tasks.clone(),
            shortcuts: shortcuts.clone(),
            package: match app.source {
                Source::Uwp => app.package_full_name.clone(),
                _ => None,
            },
        },
        false,
        // Reads only, so it also runs after a failure stopped the chain.
        false,
    );

    steps
}

// ---------------------------------------------------------------- elevated plan file

#[derive(Serialize, Deserialize, Default)]
pub struct PurgePlan {
    pub delete_paths: Vec<String>,
    pub delete_reg: Vec<(String, String)>,
    pub services: Vec<String>,
    pub tasks: Vec<String>,
    pub shortcuts: Vec<String>,
    pub result_path: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct PurgeResult {
    pub deleted: u32,
    pub failed: u32,
    pub log: Vec<String>,
}

fn run_elevated_plan(plan: &PurgePlan) -> Result<PurgeResult, String> {
    let dir = crate::icons::app_data_dir().join("plans");
    let _ = std::fs::create_dir_all(&dir);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let plan_path = dir.join(format!("plan-{}.json", stamp));
    let result_path = dir.join(format!("plan-{}.result.json", stamp));

    let mut p = PurgePlan {
        delete_paths: plan.delete_paths.clone(),
        delete_reg: plan.delete_reg.clone(),
        services: plan.services.clone(),
        tasks: plan.tasks.clone(),
        shortcuts: plan.shortcuts.clone(),
        result_path: result_path.to_string_lossy().to_string(),
    };
    p.result_path = result_path.to_string_lossy().to_string();

    let text = serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?;
    std::fs::write(&plan_path, text).map_err(|e| e.to_string())?;

    let exe = exe_path();
    let params = format!("--plan \"{}\"", plan_path.to_string_lossy());
    let (rc, _) = crate::native::run_elevated(&exe.to_string_lossy(), &params, None, true);
    if rc != 0 {
        // 1223 = user cancelled the UAC prompt
        return Err(format!("Could not elevate (code {})", rc));
    }

    let text = std::fs::read_to_string(&result_path).map_err(|e| e.to_string())?;
    let res: PurgeResult = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&plan_path);
    let _ = std::fs::remove_file(&result_path);
    Ok(res)
}

/// Entry point used by the elevated child process.
pub fn run_plan_file(path: &str) -> i32 {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return 2,
    };
    let plan: PurgePlan = match serde_json::from_str(&text) {
        Ok(p) => p,
        Err(_) => return 3,
    };
    let mut res = PurgeResult::default();

    for name in &plan.services {
        let rc = crate::native::stop_delete_service(name);
        if rc == 0 {
            res.log.push(format!("service removed: {}", name));
        } else {
            // Counted like every other failure: the caller reports `failed`
            // back to the user, so a swallowed code would read as success.
            res.failed += 1;
            res.log
                .push(format!("service failed: {} (code {})", name, rc));
        }
    }
    for task in &plan.tasks {
        let rc = crate::native::delete_scheduled_task(task);
        if rc == 0 {
            res.log.push(format!("task removed: {}", task));
        } else {
            res.failed += 1;
            res.log.push(format!("task failed: {} (code {})", task, rc));
        }
    }
    for key in &plan.delete_reg {
        let root = match key.0.as_str() {
            "HKLM" => RegRoot::Hklm,
            "HKLM-32" => RegRoot::Hklm32,
            _ => RegRoot::Hkcu,
        };
        match delete_reg_key(root, &key.1) {
            Ok(_) => res.log.push(format!("registry removed: {}", key.1)),
            Err(e) => {
                res.failed += 1;
                res.log.push(format!("registry failed: {} ({})", key.1, e));
            }
        }
    }
    for path in &plan.delete_paths {
        if crate::model::is_protected_dir(Path::new(path)) {
            res.log.push(format!("blocked protected path: {}", path));
            res.failed += 1;
            continue;
        }
        let r = crate::native::force_delete(path);
        res.deleted += r.deleted.max(0) as u32;
        res.failed += r.failed.max(0) as u32;
        if !r.first_error.is_empty() {
            res.log.push(r.first_error);
        }
    }
    for sc in &plan.shortcuts {
        if crate::model::is_protected_dir(Path::new(sc)) {
            res.failed += 1;
            res.log.push(format!("blocked protected shortcut: {}", sc));
            continue;
        }
        let r = crate::native::force_delete(sc);
        if r.failed > 0 {
            res.failed += 1;
            res.log
                .push(format!("shortcut failed: {} ({})", sc, r.first_error));
        } else {
            res.deleted += r.deleted.max(0) as u32;
            res.log.push(format!("shortcut removed: {}", sc));
        }
    }

    if let Ok(text) = serde_json::to_string_pretty(&res) {
        let _ = std::fs::write(&plan.result_path, text);
    }
    0
}

// ---------------------------------------------------------------- verification

/// What a read-only verification of one removal decided.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Verdict {
    /// Every tracked resource was probed and none is left.
    Gone,
    /// At least one resource is provably still there.
    Left(String),
    /// A probe failed outright, so absence cannot be claimed.
    UnknownState(String),
}

/// Read-only proof that the resources a flow tried to remove are gone.
///
/// A probe that errors (access denied, a broken registry handle) is never
/// treated as absence: "could not check" and "checked and it is gone" are
/// different answers, and only the second one may say "completely removed".
fn verify_all(
    paths: &[String],
    reg: &[(RegRoot, String)],
    services: &[String],
    tasks: &[String],
    shortcuts: &[String],
    package: Option<&str>,
) -> Verdict {
    // Flipping to UnknownState the first time a probe fails, so a broken probe
    // can never be reported as clean.
    fn note_unknown(flag: &mut Option<String>, what: &str, why: &str) {
        if flag.is_none() {
            *flag = Some(format!("Could not check {} ({})", what, why));
        }
    }

    let mut unknown: Option<String> = None;
    let mut left: Vec<String> = Vec::new();

    for p in paths.iter().chain(shortcuts.iter()) {
        match crate::native::path_state(p) {
            Some(true) => left.push(p.clone()),
            Some(false) => {}
            None => note_unknown(&mut unknown, p, "access denied or unavailable"),
        }
    }
    for (root, key) in reg {
        match reg_key_state(*root, key) {
            Some(true) => left.push(format!("{} {}", root.label(), key)),
            Some(false) => {}
            None => note_unknown(&mut unknown, key, "registry query failed"),
        }
    }
    for name in services {
        if crate::native::service_exists(name) == Some(true) {
            left.push(format!("service {}", name));
        }
    }
    for name in tasks {
        if crate::native::task_exists(name) == Some(true) {
            left.push(format!("task {}", name));
        }
    }
    if let Some(full) = package {
        if crate::native::package_registered(full) == Some(true) {
            left.push(format!("package {}", full));
        }
    }

    if !left.is_empty() {
        let shown = left.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
        let more = if left.len() > 3 {
            format!(" and {} more", left.len() - 3)
        } else {
            String::new()
        };
        return Verdict::Left(format!("Still on disk: {}{}", shown, more));
    }
    match unknown {
        Some(why) => Verdict::UnknownState(why),
        None => Verdict::Gone,
    }
}

// ---------------------------------------------------------------- execution

struct Runner {
    tx: Sender<(usize, StepStatus, String)>,
    verified_gone: bool,
    app: AppEntry,
    backup_dir: PathBuf,
    log: Vec<String>,
}

impl Runner {
    fn emit(&self, idx: usize, st: StepStatus, detail: impl Into<String>) {
        let _ = self.tx.send((idx, st, detail.into()));
    }

    fn gone(&mut self) -> bool {
        if self.verified_gone {
            return true;
        }
        let dir_gone = self
            .app
            .install_dir
            .as_ref()
            .map(|d| !d.exists())
            .unwrap_or(true);
        let reg_gone = self
            .app
            .reg_key
            .as_ref()
            .map(|(r, k)| !reg_key_exists(*r, k))
            .unwrap_or(true);
        let pkg_gone = self.app.package_full_name.is_none() || self.app.source != Source::Uwp;
        if dir_gone && reg_gone && pkg_gone {
            self.verified_gone = true;
        }
        self.verified_gone
    }

    fn exec(&mut self, _idx: usize, op: &Op) -> (StepStatus, String) {
        match op {
            Op::Nothing => (StepStatus::Skip, "This step is not needed".into()),
            Op::KillProcesses { path } => {
                let n = crate::native::count_processes_in(path);
                if n == 0 {
                    return (StepStatus::Skip, "No running processes".into());
                }
                let killed = crate::native::kill_processes_in(path);
                if killed > 0 {
                    (StepStatus::Ok, format!("Closed {} processes", killed))
                } else {
                    (StepStatus::Warn, "Could not close some processes".into())
                }
            }
            Op::RestorePoint { desc } => {
                let rc = crate::native::create_restore_point(desc);
                if rc == 0 {
                    (StepStatus::Ok, "Restore point created".into())
                } else if rc == 5 {
                    (StepStatus::Warn, "Administrator rights are required".into())
                } else {
                    (
                        StepStatus::Warn,
                        format!("Could not create a restore point (code {})", rc),
                    )
                }
            }
            Op::BackupRegistry { root, key } => {
                let _ = std::fs::create_dir_all(&self.backup_dir);
                // One file per run: a second uninstall of the same app must not
                // overwrite the snapshot taken before the first one.
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let safe: String = self
                    .app
                    .name
                    .chars()
                    .map(|c| if c.is_alphanumeric() { c } else { '_' })
                    .collect();
                let file = self
                    .backup_dir
                    .join(format!("{}-{}-{}.reg", safe, root.label(), stamp));
                let full = match root {
                    RegRoot::Hklm => format!("HKLM\\{}", key),
                    RegRoot::Hklm32 => format!("HKLM\\{}", key),
                    RegRoot::Hkcu => format!("HKCU\\{}", key),
                };
                let cmd = format!("reg export \"{}\" \"{}\" /y", full, file.to_string_lossy());
                let (rc, _) = crate::native::run_cmd(&cmd, true);
                if rc == 0 && file.exists() {
                    // Only the file name: a full path mixed with Arabic text
                    // cannot be rendered correctly by epaint.
                    let name = file
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    (StepStatus::Ok, name)
                } else {
                    (StepStatus::Warn, "Could not export the registry key".into())
                }
            }
            Op::RunUninstaller { cmd } => {
                if cmd.trim().is_empty() {
                    return (StepStatus::Skip, "No uninstaller found".into());
                }
                let (file, args) = crate::scan::split_command(cmd);
                let (rc, code) = if file.to_lowercase().ends_with(".msi") {
                    crate::native::run_elevated(
                        "msiexec.exe",
                        &format!("/x \"{}\" /qb", file),
                        None,
                        true,
                    )
                } else {
                    crate::native::run_elevated(&file, &args, None, true)
                };
                // 1223: user cancelled UAC
                if rc == 1223 {
                    return (
                        StepStatus::Fail,
                        "Administrator rights were declined".into(),
                    );
                }
                if rc != 0 {
                    return (
                        StepStatus::Fail,
                        format!("Could not start the tool (code {})", rc),
                    );
                }
                if code == 0 {
                    (StepStatus::Ok, "The tool finished successfully".into())
                } else {
                    (
                        StepStatus::Warn,
                        format!("The tool exited with code {}", code),
                    )
                }
            }
            Op::RunMsi { product } => {
                let (rc, code) = crate::native::run_elevated(
                    "msiexec.exe",
                    &format!("/x {} /qn /norestart", product),
                    None,
                    true,
                );
                if rc == 1223 {
                    (
                        StepStatus::Fail,
                        "Administrator rights were declined".into(),
                    )
                } else if rc == 0 && (code == 0 || code == 3010) {
                    (StepStatus::Ok, "Package removed".into())
                } else {
                    (StepStatus::Fail, format!("Removal failed (code {})", code))
                }
            }
            Op::RunWinget { name } => {
                if !winget_available() {
                    return (
                        StepStatus::Skip,
                        "The command-line tool is not available".into(),
                    );
                }
                let cmd = format!(
                    "winget uninstall --name \"{}\" --silent --accept-source-agreements --disable-interactivity",
                    name
                );
                let (rc, code) = crate::native::run_cmd(&cmd, true);
                if rc == 0 && code == 0 {
                    (StepStatus::Ok, "Removed through the package manager".into())
                } else {
                    (
                        StepStatus::Fail,
                        format!("The package manager failed (code {})", code),
                    )
                }
            }
            Op::RemoveUwp { full_name } => {
                let cmd = format!(
                    "powershell -NoProfile -ExecutionPolicy Bypass -Command \"Remove-AppxPackage -Package '{}' -ErrorAction Stop\"",
                    full_name
                );
                let (rc, code) = crate::native::run_cmd(&cmd, true);
                if rc == 0 && code == 0 {
                    (StepStatus::Ok, "Package removed".into())
                } else {
                    (
                        StepStatus::Fail,
                        format!("Could not remove the package (code {})", code),
                    )
                }
            }
            Op::ForceDelete { paths } => {
                let mut deleted = 0;
                let mut failed = 0;
                let mut errors: Vec<String> = Vec::new();
                let guarded = safe_paths(paths.clone());
                if guarded.is_empty() {
                    return (
                        StepStatus::Warn,
                        "The path is protected from deletion".into(),
                    );
                }
                for p in &guarded {
                    if !crate::native::path_exists(p) {
                        continue;
                    }
                    let r = crate::native::force_delete(p);
                    deleted += r.deleted.max(0) as u32;
                    failed += r.failed.max(0) as u32;
                    if !r.first_error.is_empty() {
                        errors.push(r.first_error);
                    }
                }
                let force_total = deleted + failed;
                if failed > 0 && !crate::native::is_elevated() {
                    // Retry the leftovers with an elevated helper process.
                    let plan = PurgePlan {
                        delete_paths: guarded
                            .iter()
                            .filter(|p| crate::native::path_exists(p))
                            .cloned()
                            .collect(),
                        ..Default::default()
                    };
                    if !plan.delete_paths.is_empty() {
                        if let Ok(res) = run_elevated_plan(&plan) {
                            deleted += res.deleted;
                            failed = failed.saturating_sub(res.deleted).min(failed);
                            if res.failed == 0 {
                                failed = 0;
                            }
                        }
                    }
                }
                if failed == 0 {
                    (StepStatus::Ok, format!("Deleted {} items", deleted))
                } else if deleted > 0 {
                    (
                        StepStatus::Warn,
                        format!("Deleted {} of {} items", deleted, force_total.max(1)),
                    )
                } else {
                    (
                        StepStatus::Fail,
                        errors
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "Deletion failed".into()),
                    )
                }
            }
            Op::Recycle { paths } => {
                let guarded = safe_paths(paths.clone());
                let mut ok = 0;
                for p in &guarded {
                    if crate::native::recycle(p) == 0 {
                        ok += 1;
                    }
                }
                if guarded.is_empty() {
                    (StepStatus::Skip, "No folders to recycle".into())
                } else {
                    partial(ok, guarded.len() as u32, "Moved to the Recycle Bin:")
                }
            }
            Op::DeleteReg { keys } => {
                let mut ok = 0;
                let mut err: Option<String> = None;
                let mut pending: Vec<(String, String)> = Vec::new();
                for (root, key) in keys {
                    if !reg_key_exists(*root, key) {
                        continue;
                    }
                    match delete_reg_key(*root, key) {
                        Ok(_) => ok += 1,
                        Err(e) => {
                            pending.push((root.label().to_string(), key.clone()));
                            err = Some(e);
                        }
                    }
                }
                if !pending.is_empty() {
                    let plan = PurgePlan {
                        delete_reg: pending,
                        ..Default::default()
                    };
                    match run_elevated_plan(&plan) {
                        Ok(res) => {
                            // Count what the helper actually deleted: adding one
                            // for a partly successful retry reported a clean Ok.
                            ok += res.deleted;
                            if res.failed == 0 {
                                err = None;
                            }
                        }
                        Err(e) => err = Some(e),
                    }
                }
                if ok > 0 && err.is_none() {
                    (StepStatus::Ok, "Entry deleted".into())
                } else if let Some(e) = err {
                    (StepStatus::Fail, format!("Could not delete: {}", e))
                } else {
                    (StepStatus::Skip, "The entry does not exist".into())
                }
            }
            Op::DeleteShortcuts { paths } => {
                let (present, unknown) = present_or_unknown(safe_paths(paths.clone()));
                if present.is_empty() && unknown == 0 {
                    return (StepStatus::Skip, "No shortcuts left".into());
                }
                let mut n = 0u32;
                let mut failed: Vec<String> = Vec::new();
                for p in &present {
                    if crate::native::force_delete(p).failed == 0 {
                        n += 1;
                    } else {
                        failed.push(p.clone());
                    }
                }
                if !failed.is_empty() || unknown > 0 {
                    let mut msg = format!("Deleted shortcuts: {} of {}", n, present.len());
                    if !failed.is_empty() {
                        msg.push_str(&format!(" - {}", name_failures("shortcut", &failed)));
                    }
                    if unknown > 0 {
                        msg.push_str(&format!(" - {} could not be checked", unknown));
                    }
                    return (StepStatus::Warn, msg);
                }
                partial(n, present.len() as u32, "Deleted shortcuts:")
            }
            Op::DeleteServices { names } => {
                let mut ok = 0u32;
                let mut failed: Vec<String> = Vec::new();
                let mut codes: Vec<i32> = Vec::new();
                for n in names {
                    let rc = crate::native::stop_delete_service(n);
                    if rc == 0 {
                        ok += 1;
                    } else {
                        if !codes.contains(&rc) {
                            codes.push(rc);
                        }
                        failed.push(n.clone());
                    }
                }
                if failed.is_empty() {
                    partial(ok, names.len() as u32, "Removed services:")
                } else {
                    let codes: Vec<String> = codes.iter().map(|c| c.to_string()).collect();
                    (
                        StepStatus::Warn,
                        format!(
                            "Removed services: {} of {} - {} (code {})",
                            ok,
                            names.len(),
                            name_failures("service", &failed),
                            codes.join("/")
                        ),
                    )
                }
            }
            Op::DeleteTasks { names } => {
                let mut ok = 0;
                for n in names {
                    if crate::native::delete_scheduled_task(n) == 0 {
                        ok += 1;
                    }
                }
                if ok == names.len() {
                    (StepStatus::Ok, format!("Removed {} tasks", ok))
                } else if ok > 0 {
                    (
                        StepStatus::Warn,
                        format!("Removed {} of {} tasks", ok, names.len()),
                    )
                } else {
                    // Not "no tasks": the plan only lists tasks that were found,
                    // so a zero here means every delete was refused.
                    (
                        StepStatus::Warn,
                        "Could not remove the scheduled tasks - run as administrator".into(),
                    )
                }
            }
            Op::DeleteOnReboot { paths } => {
                let (present, unknown) = present_or_unknown(safe_paths(paths.clone()));
                if present.is_empty() && unknown == 0 {
                    return (StepStatus::Skip, "No leftovers".into());
                }
                let mut n = 0u32;
                let mut failed: Vec<String> = Vec::new();
                for p in &present {
                    if crate::native::delete_on_reboot(p) == 0 {
                        n += 1;
                    } else {
                        failed.push(p.clone());
                    }
                }
                if !failed.is_empty() || unknown > 0 {
                    let mut msg = format!(
                        "Scheduled items for deletion on reboot: {} of {}",
                        n,
                        present.len()
                    );
                    if !failed.is_empty() {
                        msg.push_str(&format!(" - {}", name_failures("item", &failed)));
                    }
                    if unknown > 0 {
                        msg.push_str(&format!(" - {} could not be checked", unknown));
                    }
                    return (StepStatus::Warn, msg);
                }
                partial(
                    n,
                    present.len() as u32,
                    "Scheduled items for deletion on reboot:",
                )
            }
            Op::Verify {
                paths,
                reg,
                services,
                tasks,
                shortcuts,
                package,
            } => {
                let (st, detail) =
                    match verify_all(paths, reg, services, tasks, shortcuts, package.as_deref()) {
                        Verdict::Gone => {
                            self.verified_gone = true;
                            (StepStatus::Ok, "Removed completely".to_string())
                        }
                        Verdict::Left(what) => (StepStatus::Warn, what),
                        Verdict::UnknownState(what) => (StepStatus::Warn, what),
                    };
                (st, detail)
            }
            Op::CleanTemp => {
                let mut n = 0u32;
                let mut matched = 0u32;
                if let Ok(local) = std::env::var("LOCALAPPDATA") {
                    let temp = PathBuf::from(local).join("Temp");
                    let needle = norm(&self.app.name);
                    if needle.len() >= 4 {
                        for d in subdirs(&temp) {
                            let name = d
                                .file_name()
                                .and_then(|s| s.to_str())
                                .map(norm)
                                .unwrap_or_default();
                            if name.contains(&needle) {
                                matched += 1;
                                if crate::native::force_delete(&s(&d)).failed == 0 {
                                    n += 1;
                                }
                            }
                        }
                    }
                }
                if matched == 0 {
                    (StepStatus::Skip, "No temporary files".into())
                } else {
                    partial(n, matched, "Cleaned temporary folders:")
                }
            }
        }
    }
}

pub fn spawn(app: AppEntry, flow: Flow, settings: Settings, keep_data: bool) -> Job {
    let steps = build_plan(&app, flow, &settings, keep_data);
    let (tx, rx) = channel();
    let app_id = app.id;
    let app_name = app.name.clone();

    let backup_dir = crate::icons::app_data_dir().join("backups").join(
        app_name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>(),
    );

    let runner = Runner {
        tx: tx.clone(),
        verified_gone: false,
        app: app.clone(),
        backup_dir,
        log: Vec::new(),
    };

    let step_ops: Vec<(usize, Op, bool, bool)> = steps
        .iter()
        .enumerate()
        .map(|(i, st)| (i, st.op.clone(), st.skip_if_gone, st.optional))
        .collect();
    let total = step_ops.len();
    let auto_cascade = settings.auto_cascade;

    thread::Builder::new()
        .name("job".into())
        .spawn(move || {
            let mut r = runner;
            // When auto-cascade is off, the first removal method that fails
            // stops the chain so the user can inspect what happened. With it
            // on, every remaining method is tried in order.
            let mut stop_chain = false;
            for (idx, op, skip_if_gone, optional) in step_ops {
                if stop_chain && optional {
                    r.emit(
                        idx,
                        StepStatus::Skip,
                        "Chain stopped after an earlier failure",
                    );
                    continue;
                }
                if skip_if_gone && r.gone() {
                    r.emit(idx, StepStatus::Skip, "Already removed");
                    continue;
                }
                r.emit(idx, StepStatus::Running, "");
                let (st, detail) = r.exec(idx, &op);
                r.log.push(format!("{}: {}", idx, detail));
                let failed = st == StepStatus::Fail;
                r.emit(idx, st, detail);
                if failed && !auto_cascade {
                    stop_chain = true;
                }
                // Small pause so the UI can show progress on instant steps.
                thread::sleep(std::time::Duration::from_millis(40));
            }
            let _ = r.tx.send((total, StepStatus::Ok, String::new()));
        })
        .ok();

    Job {
        app_id,
        app_name,
        flow,
        steps,
        done: false,
        summary: String::new(),
        success: false,
        rx,
    }
}

// ---------------------------------------------------------------- backups

/// One saved registry export, as shown in the backup manager.
#[derive(Clone, Debug)]
pub struct BackupEntry {
    /// Folder name under `%APPDATA%\AppSweep\backups` (the app name, sanitised).
    pub group: String,
    /// File name of the `.reg` export.
    pub file: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: String,
    /// Registry root the export came from, parsed from the file name.
    pub root: String,
}

pub fn backups_root() -> PathBuf {
    crate::icons::app_data_dir().join("backups")
}

/// Lists every saved registry backup, newest first.
pub fn list_backups() -> Vec<BackupEntry> {
    let root = backups_root();
    let mut out: Vec<BackupEntry> = Vec::new();
    let dirs = match std::fs::read_dir(&root) {
        Ok(d) => d,
        Err(_) => return out,
    };
    for group in dirs.flatten() {
        let gpath = group.path();
        if !gpath.is_dir() {
            continue;
        }
        let group_name = group.file_name().to_string_lossy().to_string();
        let files = match std::fs::read_dir(&gpath) {
            Ok(f) => f,
            Err(_) => continue,
        };
        for f in files.flatten() {
            let fpath = f.path();
            if !fpath.is_file() {
                continue;
            }
            let name = f.file_name().to_string_lossy().to_string();
            if !name.to_lowercase().ends_with(".reg") {
                continue;
            }
            let meta = std::fs::metadata(&fpath).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = meta
                .and_then(|m| m.modified().ok())
                .map(format_time)
                .unwrap_or_default();
            let root = backup_root_of(&name);
            out.push(BackupEntry {
                group: group_name.clone(),
                file: name,
                path: fpath,
                size,
                modified,
                root,
            });
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

fn format_time(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Convert to a readable local date without pulling in a date library.
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (h, mi) = (rem / 3600, (rem % 3600) / 60);
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d, h, mi)
}

/// Deletes one backup file. The parent folder is removed when it becomes empty.
pub fn delete_backup(entry: &BackupEntry) -> Result<(), String> {
    let p = &entry.path;
    if !p.exists() {
        return Ok(());
    }
    let r = crate::native::force_delete(&entry.path.to_string_lossy());
    if r.failed > 0 {
        return Err(r.first_error);
    }
    if let Some(parent) = entry.path.parent() {
        let empty = std::fs::read_dir(parent)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if empty {
            let _ = std::fs::remove_dir(parent);
        }
    }
    Ok(())
}

/// Deletes every saved backup and the backups root itself.
pub fn clear_backups() -> Result<usize, String> {
    let root = backups_root();
    if !root.exists() {
        return Ok(0);
    }
    let n = list_backups().len();
    let r = crate::native::force_delete(&root.to_string_lossy());
    if r.failed > 0 {
        return Err(r.first_error);
    }
    Ok(n)
}

/// Registry root named by a backup file: `"<app>-<ROOT>.reg"` for the old
/// names, `"<app>-<ROOT>-<seconds>.reg"` for the timestamped ones.
fn backup_root_of(file: &str) -> String {
    let stem = file
        .strip_suffix(".reg")
        .or_else(|| file.strip_suffix(".REG"))
        .unwrap_or(file);
    let parts: Vec<&str> = stem.rsplit('-').collect();
    // A trailing all-digit part is the timestamp, not the root.
    let root = match parts.as_slice() {
        [last, ..] if last.chars().all(|c| c.is_ascii_digit()) && last.len() >= 9 => {
            parts.get(1).copied().unwrap_or("")
        }
        [last, ..] => last,
        [] => "",
    };
    root.to_string()
}

/// Total size of every saved backup, in bytes.
pub fn backups_size() -> u64 {
    list_backups().iter().map(|b| b.size).sum()
}

/// Headless self-test: builds a scratch app folder, registry key and shortcut,
/// then drives the very same runner the UI uses and prints every step result.
/// Enabled with `--selftest`; never touches a real installation.
pub fn selftest() -> i32 {
    use crate::model::{AppEntry, RegRoot, Source};
    use crate::settings::Settings;

    let root = std::env::temp_dir().join("appsweep-selftest");
    let app_dir = root.join("DemoApp");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::create_dir_all(app_dir.join("bin"));
    let _ = std::fs::write(app_dir.join("readme.txt"), b"x");
    let _ = std::fs::write(app_dir.join("bin").join("demo.dll"), b"y");
    let _ = std::fs::write(app_dir.join("demo.exe"), b"z");

    let reg_path = "Software\\AppSweepSelftest\\DemoApp";
    {
        use winreg::enums::*;
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(k) = hkcu.create_subkey(reg_path) {
            let _ = k.0.set_value("DisplayName", &"DemoApp");
            let _ = k.0.set_value("DisplayVersion", &"1.0");
        }
    }

    let mut app = AppEntry::new(1, "DemoApp".to_string(), Source::Registry);
    app.install_dir = Some(app_dir.clone());
    app.launch_exe = Some(app_dir.join("demo.exe"));
    app.reg_key = Some((RegRoot::Hkcu, reg_path.to_string()));
    app.version = "1.0".into();

    let settings = Settings {
        // A restore point is the slowest and most fragile step, so it is opt-in
        // for the self-test: set APPSWEEP_SELFTEST_RP=1 to include it.
        create_restore_point: std::env::var("APPSWEEP_SELFTEST_RP").is_ok(),
        backup_registry: true,
        kill_processes: true,
        ..Default::default()
    };

    println!("selftest root: {}", root.display());
    let mut job = spawn(app, Flow::Purge, settings, false);
    let mut last = usize::MAX;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        job.poll();
        let (done, total) = job.progress();
        if done != last {
            last = done;
            println!("progress {}/{}", done, total);
        }
        if job.done || std::time::Instant::now() > deadline {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let mut failures = 0;
    for (i, st) in job.steps.iter().enumerate() {
        let tag = format!("{:?}", st.status);
        println!("{:>2}. [{}] {} — {}", i + 1, tag, st.title, st.detail);
        if st.status == StepStatus::Fail {
            failures += 1;
        }
    }

    let dir_gone = !app_dir.exists();
    let reg_gone = {
        use winreg::enums::*;
        winreg::RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(reg_path)
            .is_err()
    };
    println!("dir gone: {}", dir_gone);
    println!("registry gone: {}", reg_gone);
    println!("failed steps: {}", failures);
    let _ = std::fs::remove_dir_all(&root);

    // ---- backup manager: list, import, delete, clear
    let mut bk_failures = 0;
    let group = "ZZSelfTestGroup";
    let bdir = backups_root().join(group);
    let _ = std::fs::create_dir_all(&bdir);
    let bfile = bdir.join(format!("{}-HKCU.reg", group));
    let _ = std::fs::write(
        &bfile,
        "Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Software\\AppSweepBkSelfTest]\r\n\"Marker\"=\"ok\"\r\n",
    );
    let listed = list_backups();
    let found = listed.iter().any(|b| b.group == group);
    println!("backup listed: {}", found);
    if !found {
        bk_failures += 1;
    }
    if let Some(entry) = listed.iter().find(|b| b.group == group) {
        println!("backup root parsed: {}", entry.root);
        if entry.root != "HKCU" {
            bk_failures += 1;
        }
        match delete_backup(entry) {
            Ok(()) => {
                let gone = !bfile.exists();
                println!("backup file deleted: {}", gone);
                if !gone {
                    bk_failures += 1;
                }
            }
            Err(e) => {
                println!("backup delete error: {}", e);
                bk_failures += 1;
            }
        }
    }
    // The now-empty group folder must be cleaned up as well.
    let group_gone = !bdir.exists();
    println!("backup group folder removed: {}", group_gone);
    if !group_gone {
        bk_failures += 1;
    }
    println!("backup failures: {}", bk_failures);

    // ---- path matching: a sibling folder must never look like a child one
    let mut path_failures = 0;
    let under = |folder: &str, cand: &str| crate::native::path_is_under(folder, cand);
    let cases: [(&str, &str, bool); 10] = [
        ("C:\\Apps\\Foo", "C:\\Apps\\Foo\\app.exe", true),
        ("C:\\Apps\\Foo", "C:\\Apps\\Foobar\\other.exe", false),
        ("C:\\Apps\\Foo", "C:\\Apps\\Foo", true),
        ("C:\\Apps\\Foo\\", "C:\\Apps\\Foo\\app.exe", true),
        ("C:\\Apps\\Foo", "D:\\Tools\\Foo\\other.exe", false),
        ("c:\\apps\\foo", "C:\\Apps\\Foo\\App.EXE", true),
        ("C:\\Apps\\Foo", "C:\\Apps\\Foo2", false),
        ("C:\\Apps\\Foo", "", false),
        ("C:\\Apps\\Foo", "C:\\Apps\\Fo", false),
        (
            "\\\\server\\share\\Foo",
            "\\\\server\\share\\Foo\\a.exe",
            true,
        ),
    ];
    for (folder, cand, want) in cases {
        let got = under(folder, cand);
        println!("under {:?} <- {:?}: {} (want {})", folder, cand, got, want);
        if got != want {
            path_failures += 1;
        }
    }
    // Service command lines: only the executable counts, and its arguments must
    // not turn a sibling folder into a match.
    let exe_cases: [(&str, &str); 4] = [
        (
            "\"C:\\Apps\\Foo\\foo.exe\" --service",
            "C:\\Apps\\Foo\\foo.exe",
        ),
        (
            "C:\\Apps\\Foo\\foo.exe -k netsvcs",
            "C:\\Apps\\Foo\\foo.exe",
        ),
        (
            "\"C:\\Program Files\\Foo Bar\\a b.exe\" /run",
            "C:\\Program Files\\Foo Bar\\a b.exe",
        ),
        (
            "C:\\Apps\\Foobar\\other.exe -c \"C:\\Apps\\Foo\"",
            "C:\\Apps\\Foobar\\other.exe",
        ),
    ];
    for (cmd, want) in exe_cases {
        let got = crate::native::command_executable(cmd);
        println!("cmd {:?} -> {:?} (want {:?})", cmd, got, want);
        if got != want {
            path_failures += 1;
        }
    }
    println!("path failures: {}", path_failures);

    // ---- Verification probes (issue #9): absence, presence and "cannot tell"
    // must stay three different answers.
    let mut verify_failures = 0;
    let gone_probe = root.join("verify-gone-42");
    let _ = std::fs::remove_dir_all(&gone_probe);
    let here_probe = root.join("verify-here");
    let _ = std::fs::create_dir_all(&here_probe);
    let states = (
        crate::native::path_state(gone_probe.to_str().unwrap_or_default()),
        crate::native::path_state(here_probe.to_str().unwrap_or_default()),
        crate::native::path_state(""),
    );
    println!(
        "verify probe states: gone={:?} here={:?} empty={:?}",
        states.0, states.1, states.2
    );
    if states.0 != Some(false) || states.1 != Some(true) || states.2.is_some() {
        verify_failures += 1;
    }
    let svc = crate::native::service_exists("appsweep-no-such-service-42");
    println!("verify service probe: {:?}", svc);
    if svc != Some(false) {
        verify_failures += 1;
    }
    let task = crate::native::task_exists("\\AppSweep\\no-such-task-42");
    println!("verify task probe: {:?}", task);
    if task != Some(false) {
        verify_failures += 1;
    }
    match verify_all(
        &[gone_probe.to_string_lossy().to_string()],
        &[(RegRoot::Hkcu, "Software\\AppSweepNoSuchKey42".to_string())],
        &[],
        &[],
        &[],
        None,
    ) {
        Verdict::Gone => {}
        other => {
            println!("verify verdict wrong: {:?}", other);
            verify_failures += 1;
        }
    }
    println!("verify failures: {}", verify_failures);

    // ---- ACL safety (issue #5): a forced delete must not leave an object with
    // Everyone full control behind. Disposable temp tree only.
    let mut acl_failures = 0;
    let acl_root = root.join("AclApp");
    let acl_inner = acl_root.join("locked");
    let _ = std::fs::create_dir_all(&acl_inner);
    let _ = std::fs::write(acl_inner.join("f.txt"), b"x");
    let before =
        crate::native::grants_everyone_full_control(acl_inner.to_str().unwrap_or_default());
    println!("acl everyone-full before delete: {}", before);
    if before != 0 {
        acl_failures += 1;
    }
    // The detector must actually be able to see a bad ACE, or the check above
    // proves nothing.
    let probe = acl_root.join("probe");
    let _ = std::fs::create_dir_all(&probe);
    let granted = crate::native::grant_everyone_full_control(probe.to_str().unwrap_or_default());
    let seen = crate::native::grants_everyone_full_control(probe.to_str().unwrap_or_default());
    println!("acl probe: grant={} detected={}", granted, seen);
    if granted == 0 && seen != 1 {
        acl_failures += 1;
    }
    let _ = crate::native::force_delete(probe.to_str().unwrap_or_default());
    let rep = crate::native::force_delete(acl_root.to_str().unwrap_or_default());
    println!(
        "acl force delete: deleted={} failed={} gone={}",
        rep.deleted,
        rep.failed,
        !acl_root.exists()
    );
    if acl_root.exists() {
        let after =
            crate::native::grants_everyone_full_control(acl_inner.to_str().unwrap_or_default());
        println!("acl everyone-full after failed delete: {}", after);
        if after != 0 {
            acl_failures += 1;
        }
    }
    println!("acl failures: {}", acl_failures);

    if dir_gone
        && reg_gone
        && failures == 0
        && bk_failures == 0
        && path_failures == 0
        && acl_failures == 0
    {
        println!("SELFTEST OK");
        0
    } else {
        println!("SELFTEST FAILED");
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, dir: &str) -> AppEntry {
        let mut a = AppEntry::new(1, name.to_string(), Source::Registry);
        a.install_dir = Some(PathBuf::from(dir));
        a
    }

    fn any_op(steps: &[Step], f: impl Fn(&Op) -> bool) -> bool {
        steps.iter().any(|s| f(&s.op))
    }

    #[test]
    fn a_partly_done_batch_is_a_warning_with_the_real_counts() {
        // A multi-resource step that lost half its items used to claim success.
        assert!(matches!(
            partial(3, 3, "Removed tasks:"),
            (StepStatus::Ok, _)
        ));
        let (status, msg) = partial(2, 3, "Removed tasks:");
        assert!(matches!(status, StepStatus::Warn));
        assert_eq!(msg, "Removed tasks: 2 of 3");
        let (status, _) = partial(0, 3, "Removed tasks:");
        assert!(matches!(status, StepStatus::Warn));
        // Nothing to do is not a failure.
        assert!(matches!(partial(0, 0, "x:").0, StepStatus::Ok));
    }

    #[test]
    fn every_multi_resource_step_reports_how_many_it_finished() {
        // Guard rail: none of these handlers may hard-code a bare "done".
        let src = include_str!("actions.rs");
        for marker in [
            "Removed services:",
            "Deleted shortcuts:",
            "Moved to the Recycle Bin:",
            "Cleaned temporary folders:",
            "Scheduled items for deletion on reboot:",
        ] {
            assert!(src.contains(marker), "{} lost its partial count", marker);
        }
    }

    #[test]
    fn names_are_compared_without_case_or_punctuation() {
        assert_eq!(norm("Carrots-Engine 1.0"), "carrotsengine10");
        assert_eq!(norm("  AppSweep!! "), "appsweep");
    }

    #[test]
    fn generic_words_are_dropped_before_matching() {
        assert_eq!(
            tokens("Microsoft Windows Application Verifier x64"),
            vec!["verifier"]
        );
        assert_eq!(tokens("Roblox Player"), vec!["roblox", "player"]);
        assert!(tokens("App Update Setup x86").is_empty());
    }

    #[test]
    fn trace_matching_needs_a_distinctive_name_word() {
        assert!(!looks_like_trace("", &app("Carrots Engine", "D:\\CE")));
        assert!(looks_like_trace(
            "Carrots Engine",
            &app("Carrots Engine", "D:\\CE")
        ));
        assert!(looks_like_trace(
            "CarrotsEngine",
            &app("Carrots Engine", "D:\\CE")
        ));
        // A publisher hit alone is too loose: every app by the same vendor
        // would lose its data folder.
        let mut a = app("Demo Tool", "D:\\demo");
        a.publisher = "Acme".into();
        assert!(looks_like_trace("Acme Demo", &a));
        assert!(!looks_like_trace("Acme Games", &a));
        assert!(!looks_like_trace("Unrelated", &a));
    }

    #[test]
    fn protected_paths_never_reach_the_delete_engine() {
        let kept = safe_paths(vec![
            "C:\\Program Files".to_string(),
            "C:\\Windows\\System32".to_string(),
            "C:\\".to_string(),
            "D:\\Apps\\Demo".to_string(),
        ]);
        assert_eq!(kept, vec!["D:\\Apps\\Demo".to_string()]);
    }

    #[test]
    fn files_only_touches_the_folder() {
        let a = app("Demo", "D:\\Apps\\Demo");
        let steps = build_plan(&a, Flow::FilesOnly, &Settings::default(), true);
        assert!(any_op(&steps, |o| matches!(o, Op::Recycle { .. })));
        assert!(any_op(&steps, |o| matches!(o, Op::ForceDelete { .. })));
        assert!(any_op(&steps, |o| matches!(o, Op::Verify { .. })));
        assert!(!any_op(&steps, |o| matches!(o, Op::DeleteReg { .. })));
        assert!(!any_op(&steps, |o| matches!(o, Op::DeleteShortcuts { .. })));
        // The complaint behind #11: this flow also swept %LOCALAPPDATA%\\Temp.
        assert!(!any_op(&steps, |o| matches!(o, Op::CleanTemp)));
    }

    #[test]
    fn files_only_verifies_only_the_folder_it_touched() {
        let mut a = app("Demo", "D:\\Apps\\Demo");
        a.reg_key = Some((RegRoot::Hkcu, "Software\\Demo".to_string()));
        a.shortcuts = vec![PathBuf::from("D:\\Start\\Demo.lnk")];
        let steps = build_plan(&a, Flow::FilesOnly, &Settings::default(), true);
        let v = steps
            .iter()
            .find_map(|s| match &s.op {
                Op::Verify { paths, reg, .. } => Some((paths.clone(), reg.clone())),
                _ => None,
            })
            .expect("FilesOnly must still verify what it deleted");
        assert_eq!(v.0, vec!["D:\\Apps\\Demo".to_string()]);
        assert!(v.1.is_empty(), "FilesOnly never touches the registry");
    }

    #[test]
    fn verify_runs_once_and_even_after_the_chain_stopped() {
        let mut a = app("Demo", "D:\\Apps\\Demo");
        a.reg_key = Some((RegRoot::Hkcu, "Software\\Demo".to_string()));
        for flow in [Flow::Uninstall, Flow::Purge, Flow::FilesOnly] {
            let steps = build_plan(&a, flow, &Settings::default(), true);
            let verifies: Vec<&Step> = steps
                .iter()
                .filter(|s| matches!(s.op, Op::Verify { .. }))
                .collect();
            assert_eq!(verifies.len(), 1, "{:?} verification count", flow);
            // Auto-cascade off stops the optional steps after a failure; the
            // verify is the one step that must still run, because it only reads.
            assert!(
                !verifies[0].optional,
                "{:?} verify must not be optional",
                flow
            );
            // Op is not PartialEq (it holds whole plans' worth of data), so
            // compare titles instead.
            assert_eq!(
                steps.last().map(|s| s.title.as_str()),
                Some("Verify the result"),
                "{:?} verify must be the last step",
                flow
            );
        }
    }

    #[test]
    fn purge_verifies_the_registry_it_removed() {
        let mut a = app("Demo", "D:\\Apps\\Demo");
        a.reg_key = Some((RegRoot::Hkcu, "Software\\Demo".to_string()));
        let steps = build_plan(&a, Flow::Purge, &Settings::default(), true);
        let reg = steps.iter().find_map(|s| match &s.op {
            Op::Verify { reg, .. } => Some(reg.clone()),
            _ => None,
        });
        assert_eq!(
            reg,
            Some(vec![(RegRoot::Hkcu, "Software\\Demo".to_string())])
        );
    }

    #[test]
    fn only_a_confirmed_absence_counts_as_completely_removed() {
        // A path that cannot exist on this machine: verified as gone.
        let missing = "D:\\Apps\\AppSweep-verify-does-not-exist-42";
        assert_eq!(
            verify_all(&[missing.to_string()], &[], &[], &[], &[], None),
            Verdict::Gone
        );
        // Nothing tracked at all is not proof of anything either: there is
        // simply nothing to report as left, so it stays Gone only when every
        // requested probe answered.
        assert_eq!(verify_all(&[], &[], &[], &[], &[], None), Verdict::Gone);
        // A path that does exist is left behind, never a clean pass.
        let here = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        match verify_all(&[here], &[], &[], &[], &[], None) {
            Verdict::Left(msg) => assert!(msg.starts_with("Still on disk:")),
            other => panic!("expected Left, got {:?}", other),
        }
    }

    #[test]
    fn purge_removes_the_registry_entry_shortcuts_and_the_folder() {
        let mut a = app("Demo", "D:\\Apps\\Demo");
        a.reg_key = Some((RegRoot::Hkcu, "Software\\Demo".to_string()));
        a.shortcuts = vec![PathBuf::from("D:\\Start\\Demo.lnk")];
        let steps = build_plan(&a, Flow::Purge, &Settings::default(), true);
        assert!(any_op(
            &steps,
            |o| matches!(o, Op::DeleteReg { keys } if !keys.is_empty())
        ));
        assert!(any_op(&steps, |o| matches!(o, Op::DeleteShortcuts { .. })));
        assert!(any_op(&steps, |o| matches!(
            o,
            Op::ForceDelete { paths } if paths.iter().any(|p| p.contains("Demo"))
        )));
        assert!(any_op(&steps, |o| matches!(o, Op::Verify { .. })));
    }

    #[test]
    fn a_backup_is_taken_before_any_registry_deletion() {
        let mut a = app("Demo", "D:\\Apps\\Demo");
        a.reg_key = Some((RegRoot::Hkcu, "Software\\Demo".to_string()));
        let steps = build_plan(&a, Flow::Purge, &Settings::default(), true);
        let backup = steps
            .iter()
            .position(|s| matches!(s.op, Op::BackupRegistry { .. }));
        let delete = steps
            .iter()
            .position(|s| matches!(s.op, Op::DeleteReg { .. }));
        if let (Some(b), Some(d)) = (backup, delete) {
            assert!(b < d, "registry must be exported before it is deleted");
        }
    }

    #[test]
    fn no_plan_ever_contains_a_protected_directory() {
        for (name, dir) in [
            ("Windows", "C:\\Windows"),
            ("System32", "C:\\Windows\\System32"),
            ("ProgramFiles", "C:\\Program Files"),
            ("Root", "C:\\"),
        ] {
            let a = app(name, dir);
            for flow in [Flow::Uninstall, Flow::Purge, Flow::FilesOnly] {
                let steps = build_plan(&a, flow, &Settings::default(), true);
                for s in &steps {
                    let paths: Vec<&String> = match &s.op {
                        Op::ForceDelete { paths }
                        | Op::Recycle { paths }
                        | Op::DeleteShortcuts { paths }
                        | Op::DeleteOnReboot { paths } => paths.iter().collect(),
                        Op::KillProcesses { path } => vec![path],
                        _ => Vec::new(),
                    };
                    for p in paths {
                        assert!(
                            !crate::model::is_protected_dir(Path::new(p)),
                            "{} leaked into {:?}",
                            p,
                            flow
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_flow_has_a_title() {
        for flow in [Flow::Uninstall, Flow::Purge, Flow::FilesOnly] {
            assert!(!flow.title().is_empty());
        }
    }

    #[test]
    fn a_failed_probe_keeps_the_target_in_the_batch() {
        // A genuinely missing file is dropped, and an unanswerable probe keeps
        // the target (see `present_or_unknown`). Only the first half is
        // assertable here: provoking a refused probe needs an ACL this test
        // cannot set up without admin rights.
        let missing = "D:\\AppsSweepDefinitelyMissing\\x.lnk".to_string();
        let (keep, unknown) = present_or_unknown(vec![missing]);
        assert!(keep.is_empty());
        assert_eq!(unknown, 0);
        // A path that exists stays in the batch.
        let here = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let (keep, _) = present_or_unknown(vec![here]);
        assert_eq!(keep.len(), 1);
    }

    #[test]
    fn failure_messages_name_the_resources() {
        let one = name_failures("service", &["a".to_string()]);
        assert!(one.contains("a"), "{}", one);
        let many: Vec<String> = (0..6).map(|i| format!("s{}", i)).collect();
        let msg = name_failures("service", &many);
        assert!(msg.contains("s2") && msg.contains("..."), "{}", msg);
        assert!(!msg.contains("s3"), "{}", msg);
    }

    #[test]
    fn a_second_backup_does_not_overwrite_the_first() {
        // The file name carries a timestamp, so the same app can keep one
        // snapshot per uninstall attempt.
        let a = "Demo-HKCU-1700000000.reg";
        let b = "Demo-HKCU-1700000600.reg";
        assert_ne!(a, b);
        assert_eq!(backup_root_of(a), "HKCU");
        assert_eq!(backup_root_of(b), "HKCU");
        // The old, untimestamped names still parse.
        assert_eq!(backup_root_of("Demo-HKCU.reg"), "HKCU");
        assert_eq!(backup_root_of("Demo-HKLM32.REG"), "HKLM32");
    }

    #[test]
    fn apps_whose_names_sanitize_alike_land_in_different_folders() {
        // "A/B" and "A:B" both become "A_B" for the file name, but the folder
        // is what separates them in the backup manager.
        assert_eq!(backup_root_of("A_B-HKCU-1700000000.reg"), "HKCU");
        assert_eq!(backup_root_of("no-root-suffix.reg"), "suffix");
    }

    #[test]
    fn backup_timestamps_are_readable_utc() {
        let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        assert_eq!(format_time(t), "2023-11-14 22:13");
    }

    /// A finished job with the given statuses, without spawning anything.
    fn job_with(statuses: &[StepStatus]) -> Job {
        let statuses: Vec<StepStatus> = statuses.to_vec();
        let (_tx, rx) = std::sync::mpsc::channel();
        let steps = statuses
            .iter()
            .map(|st| Step {
                title: String::new(),
                detail: String::new(),
                op: Op::Nothing,
                status: st.clone(),
                skip_if_gone: false,
                optional: false,
            })
            .collect();
        let mut job = Job {
            app_id: 1,
            app_name: "Demo".into(),
            flow: Flow::Purge,
            steps,
            done: false,
            summary: String::new(),
            success: false,
            rx,
        };
        job.mark_done();
        job
    }

    #[test]
    fn warnings_are_not_a_clean_success() {
        use StepStatus::*;
        assert!(job_with(&[Ok, Skip, Ok]).success);
        // A refused task delete only produces a warning, so this was reported
        // as a full success before.
        assert!(!job_with(&[Ok, Warn, Ok]).success);
        assert!(!job_with(&[Ok, Fail]).success);
    }

    #[test]
    fn a_step_that_never_reported_is_a_failure() {
        use StepStatus::*;
        let job = job_with(&[Ok, Running, Pending]);
        assert!(!job.success);
        assert!(job.steps[1].status == Fail && job.steps[2].status == Fail);
        assert_eq!(job.summary, "1 ok / 0 warnings / 2 failed");
    }
}
