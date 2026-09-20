use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap_or_default();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=native_libs/litert/headers/wrapper.h");
    println!("cargo:rerun-if-changed=native_libs/litert/headers/c_api.h");
    println!("cargo:rerun-if-changed=native_libs/litert/headers/c_api_types.h");
    println!("cargo:rerun-if-changed=native_libs/litert/headers/common.h");
    println!("cargo:rerun-if-changed=native_libs/litert/headers/gpu_delegate.h");
    println!("cargo:rerun-if-changed=native_libs/litert/headers/coreml_delegate.h");
    println!("cargo:rerun-if-changed=native_libs/litert/headers/xnnpack_delegate.h");

    let headers_dir = manifest_dir.join("native_libs/litert/headers");
    let wrapper_header = headers_dir.join("wrapper.h");

    // Generate Rust FFI bindings via bindgen
    if wrapper_header.exists() {
        let mut builder = bindgen::Builder::default()
            .header(wrapper_header.to_str().unwrap())
            .clang_arg(format!("-I{}", headers_dir.display()))
            .clang_arg("-DTFL_STATIC_LIBRARY_BUILD")
            .allowlist_function("TfLite.*")
            .allowlist_type("TfLite.*")
            .allowlist_var("kTfLite.*")
            .derive_default(true)
            .derive_debug(true)
            .layout_tests(false)
            .generate_comments(false);

        if target.contains("android") {
            if let Some((sysroot, clang_include)) = find_ndk_sysroot_and_clang() {
                builder = builder.clang_arg(format!("--sysroot={}", sysroot.display()));
                let usr_include = sysroot.join("usr/include");
                builder = builder.clang_arg(format!("-isystem{}", usr_include.display()));

                let arch_sub = match target.as_str() {
                    "aarch64-linux-android" => Some("aarch64-linux-android"),
                    "armv7-linux-androideabi" => Some("arm-linux-androideabi"),
                    "i686-linux-android" => Some("i686-linux-android"),
                    "x86_64-linux-android" => Some("x86_64-linux-android"),
                    _ => None,
                };
                if let Some(arch) = arch_sub {
                    builder = builder.clang_arg(format!("-isystem{}", usr_include.join(arch).display()));
                }
                if let Some(clang_inc) = clang_include {
                    builder = builder.clang_arg(format!("-isystem{}", clang_inc.display()));
                }
            }
        } else if let Some(llvm_inc) = find_host_clang_include() {
            builder = builder.clang_arg(format!("-isystem{}", llvm_inc.display()));
        }

        let bindings = builder
            .generate()
            .expect("Unable to generate LiteRT bindings");

        let out_file = out_dir.join("litert_bindings.rs");
        bindings
            .write_to_file(out_file)
            .expect("Couldn't write LiteRT bindings");
    }

    // Configure target-specific static library linkage
    let target_arch_dir = match target.as_str() {
        "aarch64-linux-android" => Some(("android-arm64", "c++_shared", false)),
        "armv7-linux-androideabi" => Some(("android-arm32", "c++_shared", false)),
        "aarch64-apple-ios" => Some(("ios-arm64", "c++", true)),
        "aarch64-apple-ios-sim" | "x86_64-apple-ios" => Some(("ios-arm64-sim", "c++", true)),
        _ => None,
    };

    if let Some((arch_folder, cpp_stdlib, is_apple)) = target_arch_dir {
        let lib_dir = manifest_dir.join("native_libs/litert").join(arch_folder);
        let has_lib = lib_dir.join("libtensorflowlite_c.a").exists()
            || lib_dir.join("libtensorflowlite_c.so").exists()
            || lib_dir.join("tensorflowlite_c.lib").exists();

        if has_lib {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=static=tensorflowlite_c");
            println!("cargo:rustc-link-lib={}", cpp_stdlib);

            if is_apple {
                println!("cargo:rustc-link-lib=framework=Accelerate");
                println!("cargo:rustc-link-lib=framework=CoreML");
            }
        }
    }
}

fn find_ndk_sysroot_and_clang() -> Option<(PathBuf, Option<PathBuf>)> {
    for var in &["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "NDK_HOME", "NDK_ROOT", "ANDROID_NDK_LATEST_HOME"] {
        if let Ok(val) = env::var(var) {
            let p = PathBuf::from(val);
            if p.exists() {
                if let Some(res) = find_sysroot_in_ndk(&p) {
                    return Some(res);
                }
            }
        }
    }

    for var in &["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(val) = env::var(var) {
            let ndk_dir = PathBuf::from(val).join("ndk");
            if ndk_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(ndk_dir) {
                    let mut versions: Vec<PathBuf> = entries
                        .filter_map(|e| e.ok().map(|e| e.path()))
                        .filter(|p| p.is_dir())
                        .collect();
                    versions.sort();
                    if let Some(latest) = versions.last() {
                        if let Some(res) = find_sysroot_in_ndk(latest) {
                            return Some(res);
                        }
                    }
                }
            }
        }
    }

    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        let ndk_dir = PathBuf::from(local_app_data).join("Android").join("Sdk").join("ndk");
        if ndk_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(ndk_dir) {
                let mut versions: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.is_dir())
                    .collect();
                versions.sort();
                if let Some(latest) = versions.last() {
                    if let Some(res) = find_sysroot_in_ndk(latest) {
                        return Some(res);
                    }
                }
            }
        }
    }

    None
}

fn find_sysroot_in_ndk(ndk_path: &std::path::Path) -> Option<(PathBuf, Option<PathBuf>)> {
    let prebuilt_dir = ndk_path.join("toolchains/llvm/prebuilt");
    if prebuilt_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(prebuilt_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let sysroot = entry.path().join("sysroot");
                if sysroot.is_dir() {
                    let mut clang_inc = None;
                    let clang_dir = entry.path().join("lib/clang");
                    if clang_dir.is_dir() {
                        if let Ok(c_entries) = std::fs::read_dir(clang_dir) {
                            for c_entry in c_entries.filter_map(|e| e.ok()) {
                                let inc = c_entry.path().join("include");
                                if inc.is_dir() {
                                    clang_inc = Some(inc);
                                    break;
                                }
                            }
                        }
                    }
                    return Some((sysroot, clang_inc));
                }
            }
        }
    }
    None
}

fn find_host_clang_include() -> Option<PathBuf> {
    // 1. Try querying clang directly via `clang -print-resource-dir`
    if let Ok(output) = std::process::Command::new("clang")
        .arg("-print-resource-dir")
        .output()
    {
        if output.status.success() {
            let res_dir_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let inc_dir = PathBuf::from(res_dir_str).join("include");
            if inc_dir.is_dir() {
                return Some(inc_dir);
            }
        }
    }

    // 2. Try LIBCLANG_PATH / LLVM_HOME / LLVM_PATH environment variables
    for var in &["LIBCLANG_PATH", "LLVM_HOME", "LLVM_PATH"] {
        if let Ok(val) = env::var(var) {
            let p = PathBuf::from(val);
            // If pointing to bin or lib, check parent or sibling clang dir
            let candidates = [
                p.join("lib/clang"),
                p.join("clang"),
                p.parent().map(|parent| parent.join("lib/clang")).unwrap_or_default(),
            ];
            for candidate in &candidates {
                if let Some(inc) = probe_clang_version_dirs(candidate) {
                    return Some(inc);
                }
            }
        }
    }

    // 3. Known platform-specific default installations
    let candidate_roots = [
        // Windows
        PathBuf::from("C:\\Program Files\\LLVM\\lib\\clang"),
        PathBuf::from("C:\\Program Files (x86)\\LLVM\\lib\\clang"),
        // Linux
        PathBuf::from("/usr/lib/clang"),
        PathBuf::from("/usr/lib64/clang"),
        PathBuf::from("/usr/local/lib/clang"),
        // macOS (Homebrew / Xcode Command Line Tools / Xcode toolchains)
        PathBuf::from("/Library/Developer/CommandLineTools/usr/lib/clang"),
        PathBuf::from("/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/clang"),
        PathBuf::from("/opt/homebrew/opt/llvm/lib/clang"),
        PathBuf::from("/usr/local/opt/llvm/lib/clang"),
    ];

    for root in &candidate_roots {
        if let Some(inc) = probe_clang_version_dirs(root) {
            return Some(inc);
        }
    }

    None
}

fn probe_clang_version_dirs(base: &std::path::Path) -> Option<PathBuf> {
    if base.is_dir() {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.filter_map(|e| e.ok()) {
                let inc = entry.path().join("include");
                if inc.is_dir() {
                    return Some(inc);
                }
            }
        }
    }
    None
}
