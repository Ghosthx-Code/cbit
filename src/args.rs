use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Instant;

use crate::lua::ProjectConfig;

fn find_clang_format() -> Option<String> {
    for name in &[
        "clang-format-19",
        "clang-format-18",
        "clang-format-17",
        "clang-format-16",
        "clang-format-15",
        "clang-format-14",
        "clang-format",
    ] {
        if Command::new(name).arg("--version").output().is_ok() {
            return Some(name.to_string());
        }
    }
    None
}

fn async_compile(src_files: &Vec<String>, config: &ProjectConfig) {
    let start_time = Instant::now();
    cleanup_orphan_objects(src_files);
    if src_files.is_empty() {
        println!(
            "\x1b[1;31m{:>12}\x1b[0m no source files found in `./src`",
            "Error"
        );
        std::process::exit(0);
    }
    let pch_header = "./.build/pch/common.h";
    let pch_output = "./.build/pch/common.h.gch";
    if !std::path::Path::new(&pch_header).exists() {
        let default_headers = r#"
            #ifndef COMMON_H
            #define COMMON_H
            #include <stdio.h>
            #include <stdlib.h>
            #include <string.h>
            #include <stdbool.h>
            #include <stdint.h>
            #include <stddef.h>
            #include <math.h>
            #include <time.h>
            #endif
            "#;
        fs::write(pch_header, default_headers).ok();
    }
    if needs_recompile(&pch_header, std::path::Path::new(&pch_output)) {
        println!(
            "\x1b[1;32m{:>12}\x1b[0m optimized standard libary header...",
            "Procession"
        );
        thread::scope(|s| {
            s.spawn(move || {
                let _ = Command::new(&config.linker)
                    .arg("-x")
                    .arg("c-header")
                    .arg(pch_header)
                    .arg("-o")
                    .arg(pch_output)
                    .status();
            });
        });
    }
    let redundant_headers = vec![
        "#include <stdio.h>",
        "#include <stdlib.h>",
        "#include <string.h>",
        "#include <stdbool.h>",
        "#include <stdint.h>",
        "#include <stddef.h>",
        "#include <math.h>",
        "#include <time.h>",
    ];
    thread::scope(|s| {
        for x in src_files {
            if !x.ends_with(".c") {
                continue;
            }
            let re_header = redundant_headers.clone();
            let linker = config.linker.clone();
            let custom_flags = config.flags.clone();
            let custom_defines = config.defines.clone();
            let custom_std = config.std.clone();
            let config_include_dirs = config.include_dirs.clone();
            s.spawn(move || {
                let path = std::path::Path::new(x);
                let clean_path = path.strip_prefix("./src/").unwrap_or(path);
                let out_path = std::path::Path::new("./.build/out")
                    .join(clean_path)
                    .with_extension("o");
                if let Some(par) = out_path.parent() {
                    fs::create_dir_all(par).ok();
                }
                if !needs_recompile(x, &out_path) {
                    return;
                }
                let mut code = fs::read_to_string(x).unwrap_or_default();
                for header in re_header {
                    code = code.replace(header, "");
                }
                let size_kb = fs::metadata(x)
                    .map(|m| m.len() as f64 / 1024.0)
                    .unwrap_or(0.0);
                println!(
                    "\x1b[1;32m{:>12}\x1b[0m {} \x1b[1;30m[{:.1} KB]\x1b[0m",
                    "Compiling", x, size_kb
                );
                let mut status = Command::new(&linker);
                status
                    .arg("-c")
                    .arg(format!("-std={}", custom_std))
                    .arg("-I./.build/pch/");
                for inc_dir in &config_include_dirs {
                    status.arg(format!("-I{}", inc_dir));
                }
                for def in &custom_defines {
                    status.arg(format!("-D{}", def));
                }
                status.args(custom_flags);
                status.arg(x).arg("-o").arg(&out_path);
                match status.status() {
                    Ok(s) if s.success() => {}
                    _ => eprintln!("Failed To Compile {}", x),
                }
            });
        }
    });
    let duration = start_time.elapsed();
    let duration_secs = duration.as_secs_f32();
    println!(
        "\x1b[1;32m{:>12}\x1b[0m release target(s) in \x1b[1;36m{:.2}s\x1b[0m",
        "Finished", duration_secs
    );
}

fn build_project(config: &ProjectConfig) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("./.build/out")?;
    fs::create_dir_all("./.build")?;
    fs::create_dir_all("./.build/pch")?;
    let src_files: Vec<String> = fs::read_dir("./src/")?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.path().into_os_string().into_string().ok())
        .collect();
    async_compile(&src_files, config);
    let build_files: Vec<String> = fs::read_dir("./.build/out/")?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.path().into_os_string().into_string().ok())
        .collect();
    let mut link_cmd = Command::new(&config.linker);
    link_cmd
        .args(&build_files)
        .arg("-o")
        .arg(format!(".build/{}", config.name));
    for lib_dir in &config.libs {
        link_cmd.arg("-L").arg(lib_dir);
    }
    link_cmd.args(&config.flags);
    link_cmd.status()?;
    Ok(())
}

fn needs_recompile(src: &str, obj: &std::path::Path) -> bool {
    let src_meta = match fs::metadata(src) {
        Ok(m) => m,
        Err(_) => return true,
    };
    let obj_meta = match fs::metadata(obj) {
        Ok(m) => m,
        Err(_) => return true,
    };
    let src_time = src_meta.modified().unwrap();
    let obj_time = obj_meta.modified().unwrap();
    if src_time > obj_time {
        return true;
    }
    if let Ok(con) = fs::read_to_string(src) {
        for line in con.lines() {
            let line = line.trim();
            if line.starts_with("#include") && line.contains('"') {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line.rfind('"') {
                        if start != end {
                            let header_name = &line[start + 1..end];
                            let header_path = format!("./src/{}", header_name);
                            if let Ok(header_meta) = fs::metadata(&header_path) {
                                if header_meta.modified().unwrap() > obj_time {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn cleanup_orphan_objects(_src_files: &Vec<String>) {
    let out_dir = "./.build/out";
    if let Ok(entries) = fs::read_dir(out_dir) {
        for entry in entries {
            let path = entry.expect("").path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("o") {
                let clean_path = path.strip_prefix(out_dir).unwrap_or(&path);
                let original_c_path = std::path::Path::new("./src/")
                    .join(clean_path)
                    .with_extension("c");
                if !original_c_path.exists() {
                    println!(
                        "\x1b[1;30m{:>12}\x1b[0m old artifacts: {:?}",
                        "Removing",
                        path.file_name().unwrap()
                    );
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
}

fn run_c_formatter(src_files: &Vec<String>, config: &ProjectConfig) {
    let formatter = match find_clang_format() {
        Some(f) => f,
        None => {
            eprintln!(
                "\x1b[1;31m{:>12}\x1b[0m no clang-format found. Please install clang-format",
                "Error"
            );
            return;
        }
    };
    let style = if config.format == "LLVM" || config.format == "opcode" {
        "LLVM"
    } else {
        &config.format
    };
    println!(
        "\x1b[1;32m{:>12}\x1b[0m project files (using {})",
        "Formatting", formatter
    );
    thread::scope(|_| {
        for file in src_files {
            let status = Command::new(&formatter)
                .arg("-i")
                .arg(format!("--style={}", style))
                .arg(file)
                .status();
            match status {
                Ok(exit) if exit.success() => {
                    println!("\x1b[1;30m{:>12}\x1b[0m {}", "Formatted", file)
                }
                _ => {
                    eprintln!("\x1b[1;31m{:>12}\x1b[0m failed to format {}", "Error", file);
                }
            }
        }
    });
}

fn generate_cbit_lua(dir: &std::path::PathBuf, project_name: &str) {
    let content = format!(
        r#"-- cbit.lua - Project configuration for {}
cbit.config({{
    name = "{}",
    version = "0.1.0",
    linker = "gcc",
    std = "c11",
    format = "opcode",
    include_dirs = {{ "lib" }},
    flags = {{ "-Wall", "-Wextra" }},
    defines = {{ "NDEBUG" }},
    libs = {{ "./lib" }},
    profile = function()
        local profile = os.getenv("CBIT_PROFILE") or "debug"
        if profile == "release" then
            return {{ "-O3", "-march=native" }}
        elseif profile == "debug" then
            return {{ "-O0", "-g" }}
        else
            return {{ "-O2" }}
        end
    end,
}})
"#,
        project_name, project_name
    );
    fs::write(format!("{}/cbit.lua", dir.display()), content).ok();
}

pub fn parse_with_config(config: ProjectConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config = config;
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        println!(
            "\x1b[1;33m{:>12}\x1b[0m no command given. Use `cbit --help` for usage.",
            "Hint"
        );
        return Ok(());
    }
    match args[0].as_bytes() {
        b"new" => {
            if args.len() < 2 {
                eprintln!(
                    "\x1b[1;31m{:>12}\x1b[0m usage: cbit new <project-name>",
                    "Error"
                );
                std::process::exit(1);
            }
            let name = &args[1];
            fs::create_dir_all(name)?;
            fs::create_dir_all(format!("{}/src", name))?;
            fs::write(
                format!("{}/src/main.c", name),
                "#include <stdio.h>\n\nint main() {\n    printf(\"hello world\");\n    return 0;\n}",
            )?;
            fs::create_dir_all(format!("{}/lib", name))?;
            // Generate cbit.lua inside the new project
            let project_dir = env::current_dir()?.join(name);
            generate_cbit_lua(&project_dir, name);
            println!(
                "\x1b[1;32m{:>12}\x1b[0m created project '{}' with cbit.lua",
                "Success", name
            );
        }
        b"build" => {
            if args.len() > 1 && args[1] == "--target" {
                let target = if args.len() > 2 {
                    args[2].clone()
                } else {
                    eprintln!(
                        "\x1b[1;31m{:>12}\x1b[0m usage: cbit build --target <arch>",
                        "Error"
                    );
                    std::process::exit(1);
                };
                build_project(&config)?;
                let build_files: Vec<String> = fs::read_dir("./.build/out/")?
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.path().is_file())
                    .filter_map(|entry| entry.path().into_os_string().into_string().ok())
                    .collect();
                Command::new(&config.linker)
                    .args(&build_files)
                    .arg("-o")
                    .arg(format!(".build/{}", config.name))
                    .arg("-L./lib")
                    .arg(format!("-march={}", target))
                    .args(config.flags)
                    .status()?;
            } else {
                build_project(&config)?;
            }
        }
        b"clean" => {
            thread::yield_now();
            let build_dir = Path::new("./.build");
            if build_dir.exists() {
                fs::remove_dir_all("./.build")?;
                println!("\x1b[1;32m{:>12}\x1b[0m Removed `.build` dir", "Removed");
            } else {
                println!("\x1b[1;30m{:>12}\x1b[0m nothing to clean", "Empty");
            }
        }
        b"run" => {
            build_project(&config)?;
            Command::new(format!(".build/{}", config.name)).status()?;
        }
        b"fmt" => {
            let mut src_files = Vec::new();
            if let Ok(entr) = fs::read_dir("./src") {
                for entry in entr.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("c") {
                        src_files.push(path.to_string_lossy().into_owned());
                    }
                }
            }
            if src_files.is_empty() {
                println!("\x1b[1;30m{:>12}\x1b[0m no files to format", "Empty")
            } else {
                run_c_formatter(&src_files, &config);
            }
        }
        b"-h" | b"--help" => {
            println!(
                "\x1b[1;32m{:<10}\x1b[0m A fast, parallel C toolchain",
                "cbit"
            );
            println!("\n\x1b[1;33mUsage:\x1b[0m cbit [COMMAND] [OPTIONS]");

            println!("\n\x1b[1;33mCommands:\x1b[0m");
            println!("  \x1b[1;32m{:<10}\x1b[0m Creates a new C project", "new");
            println!(
                "  \x1b[1;32m{:<10}\x1b[0m Compiles the current project workspace matching active changes",
                "build"
            );
            println!("  \x1b[1;32m{:<10}\x1b[0m Compiles and runs the exe", "run");
            println!(
                "  \x1b[1;32m{:<10}\x1b[0m Wipes out all `.build` dir",
                "clean"
            );
            println!(
                "  \x1b[1;32m{:<10}\x1b[0m Formats code with clang-format",
                "fmt"
            );

            println!("\n\x1b[1;33mOptions:\x1b[0m");
            println!("  \x1b[1;32m-h | --help\x1b[0m Prints help doc");
        }
        _ => {
            eprintln!(
                "\x1b[1;31m{:>12}\x1b[0m unknown command '{}'. Use `cbit --help`",
                "Error", args[0]
            );
            std::process::exit(1);
        }
    }
    Ok(())
}
