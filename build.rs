extern crate cc;

use std::env;
use std::process::Command;

fn llvm_config(args: &[&str]) -> String {
    let llvm_config_path = match env::var("LLVM_CONFIG") {
        Ok(v) => v,
        Err(_e) => "llvm-config".to_string(),
    };

    Command::new(llvm_config_path)
        .args(args)
        .arg("--link-static")
        .output()
        .map(|output| String::from_utf8(output.stdout).expect("llvm-config output is not UTF-8"))
        .expect("Error while running llvm-config")
}

fn get_llvm_includedir() -> Vec<String> {
    llvm_config(&["--cflags"])
        .split(&[' ', '\n'][..])
        .filter(|word| word.starts_with("-I"))
        .map(|word| &word[2..])
        .map(str::to_owned)
        .collect::<Vec<_>>()
}

fn get_llvm_libs() -> Vec<String> {
    llvm_config(&["--libnames", "core", "profiledata"])
        .split(&[' ', '\n'][..])
        .filter(|s| !s.is_empty())
        .map(|lib| {
            if !cfg!(target_env = "msvc") {
                assert!(lib.starts_with("lib"));
                assert!(lib.ends_with(".a"));
                &lib[3..lib.len() - 2]
            } else {
                assert!(lib.ends_with(".lib"));
                &lib[..lib.len() - 4]
            }
        }).map(str::to_owned)
        .collect::<Vec<_>>()
}

fn get_llvm_system_libs() -> Vec<String> {
    llvm_config(&["--system-libs"])
        .split(&[' ', '\n'][..])
        .filter(|s| !s.is_empty())
        .map(|lib| {
            if !cfg!(target_env = "msvc") {
                assert!(lib.starts_with("-l"));
                &lib[2..]
            } else {
                assert!(lib.ends_with(".lib"));
                &lib[..lib.len() - 4]
            }
        }).map(str::to_owned)
        .collect::<Vec<_>>()
}

fn get_llvm_libdir() -> String {
    llvm_config(&["--libdir"])
}

fn main() {
    let target = env::var("TARGET").expect("TARGET was not set");

    let mut build = cc::Build::new();

    if !cfg!(target_env = "msvc") {
        build.flag("-Wno-unused-parameter");
    }

    build.file("src/c/llvmgcov.cpp");

    for include in get_llvm_includedir() {
        build.include(include);
    }

    println!("cargo:rustc-link-search=native={}", get_llvm_libdir());
    for lib in get_llvm_libs() {
        println!("cargo:rustc-link-lib=static={}", lib);
    }
    for lib in get_llvm_system_libs() {
        println!("cargo:rustc-link-lib=dylib={}", lib);
    }

    build.cpp(true);

    if !cfg!(target_env = "msvc") {
        build.flag("-fno-builtin");
        build.flag("-fno-exceptions");
        build.flag("-std=c++11");
    }

    if target.contains("darwin") {
        build.flag("-stdlib=libc++");
        let cross_sysroot = env::var("CROSS_SYSROOT");
        if let Ok(cross_sysroot) = cross_sysroot {
          build.flag(&format!("-isysroot={}", cross_sysroot));
          build.flag(&format!("-I{}/usr/include", cross_sysroot));
          build.flag(&format!("-iframework={}/System/Library/Frameworks", cross_sysroot));
        }
    }

    build.compile("libllvmgcov.a");
}
