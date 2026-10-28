fn main() {
    println!("cargo:rerun-if-changed=native/helper.cpp");
    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-changed=assets/app.manifest");

    cc::Build::new()
        .cpp(true)
        .file("native/helper.cpp")
        .flag_if_supported("/std:c++17")
        .flag_if_supported("/EHsc")
        .flag_if_supported("/utf-8")
        .compile("appsweep_native");

    println!("cargo:rustc-link-lib=shell32");
    println!("cargo:rustc-link-lib=ole32");
    println!("cargo:rustc-link-lib=uuid");
    println!("cargo:rustc-link-lib=shlwapi");
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=gdi32");
    println!("cargo:rustc-link-lib=advapi32");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.set_manifest_file("assets/app.manifest");
        res.set("ProductName", "AppSweep");
        res.set("FileDescription", "AppSweep - Windows app manager");
        res.set("CompanyName", "AppSweep");
        res.set("LegalCopyright", "AppSweep");
        res.set("FileVersion", "1.0.0.0");
        res.set("ProductVersion", "1.0.0.0");
        if let Err(err) = res.compile() {
            println!("cargo:warning=resource compile failed: {err}");
        }
    }
}
