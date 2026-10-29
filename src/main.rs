#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod icons;
mod model;
mod native;
mod scan;
mod settings;
mod theme;
mod ui;
mod userdata;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--plan") {
        let path = args.get(pos + 1);
        let sid = args
            .iter()
            .position(|arg| arg == "--plan-user-sid")
            .and_then(|sid_pos| args.get(sid_pos + 1));
        match (path, sid) {
            (Some(path), Some(sid)) => {
                std::process::exit(actions::run_plan_file(path, sid));
            }
            _ => {
                eprintln!("Rejected elevated plan: missing path or caller SID");
                std::process::exit(4);
            }
        }
    }
    if args.iter().any(|a| a == "--version") {
        println!("AppSweep 1.0.0");
        return;
    }
    if args.iter().any(|a| a == "--selftest") {
        std::process::exit(actions::selftest());
    }
    if args.iter().any(|a| a == "--tasks") {
        // Diagnostic: list scheduled tasks referencing a directory.
        let path = args
            .iter()
            .position(|a| a == "--tasks")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| "C:\\Program Files\\Git".to_string());
        let found = native::tasks_under(&path);
        println!("path: {}", path);
        match &found {
            Some(tasks) => {
                println!("count: {}", tasks.len());
                for t in tasks {
                    println!("  {}", t);
                }
            }
            None => {
                eprintln!("count: unknown (schtasks could not be read)");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--dump") {
        let r = scan::scan_all();
        for a in &r.apps {
            println!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                a.name,
                a.publisher,
                a.source.short(),
                a.install_dir
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                a.launch_exe
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                a.shortcuts.len(),
                a.arch.text()
            );
        }
        return;
    }

    // Initialize process-wide COM security before eframe or any UI component
    // initializes COM. System Restore needs callbacks from Windows services.
    let settings = settings::Settings::load();
    // Settings can be changed while the UI is open, so initialize this before
    // eframe initializes COM even when restore points are currently disabled.
    let restore_com_security_ready = native::initialize_restore_point_com_security();
    if settings.create_restore_point && !restore_com_security_ready {
        eprintln!(
            "System Restore COM security initialization failed; restore-point creation will be reported as unavailable."
        );
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([940.0, 600.0])
            .with_title("AppSweep")
            .with_app_id("appsweep"),
        centered: true,
        ..Default::default()
    };

    let _ = eframe::run_native(
        "AppSweep",
        options,
        Box::new(move |cc| {
            theme::load_fonts(&cc.egui_ctx);
            theme::apply(&cc.egui_ctx, settings.theme, settings.accent, settings.density);
            Ok(Box::new(ui::AppSweep::new(cc, settings)))
        }),
    );
}
