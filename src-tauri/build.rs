use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn find_webview2_loader(manifest_dir: &Path) -> Option<PathBuf> {
    // 优先使用上一次构建生成的 loader（已和主程序同架构链接过）
    let release_dll = manifest_dir.join("target/release/WebView2Loader.dll");
    if release_dll.exists() {
        return Some(release_dll);
    }

    // 首次构建时从 webview2-com-sys 构建产物里找 x64 版本
    let build_dir = manifest_dir.join("target/release/build");
    if let Ok(entries) = fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("webview2-com-sys-") {
                let candidate = entry.path().join("out/x64/WebView2Loader.dll");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map(PathBuf::from).unwrap_or_else(|_| {
        env::current_dir().expect("cannot get current dir")
    });
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // 仅 Windows 目标需要把 WebView2Loader.dll 复制到 src-tauri 根目录，供 Tauri bundle resources 引用
    if target_os == "windows" {
        if let Some(src) = find_webview2_loader(&manifest_dir) {
            let dst = manifest_dir.join("WebView2Loader.dll");
            if let Err(e) = fs::copy(&src, &dst) {
                println!("cargo:warning=failed to copy WebView2Loader.dll: {}", e);
            } else {
                println!("cargo:rerun-if-changed={}", src.display());
            }
        } else {
            println!("cargo:warning=WebView2Loader.dll not found; runtime may fail on target machine");
        }
    }

    tauri_build::build();
}
