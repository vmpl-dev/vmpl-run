use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use structopt::StructOpt;
use rustyline::history::History;
use rustyline::Editor;
use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Debug, StructOpt)]
#[structopt(name = "vmpl-run", about = "Run programs with VMPL support")]
struct Opt {
    #[structopt(short = "r", long = "run", help = "Run the specified program")]
    run: bool,

    #[structopt(short = "s", long = "shell", help = "Run a shell")]
    shell: bool,

    #[structopt(short = "c", long = "preload", help = "Use preload library")]
    preload: bool,

    #[structopt(short = "h", long = "hotcalls", help = "Enable hotcalls")]
    hotcalls: Option<String>,

    #[structopt(short = "e", long = "enable-vmpl", help = "Enable VMPL")]
    enable_vmpl: bool,

    #[structopt(short = "v", long = "vmpl", help = "Run in VMPL mode")]
    vmpl: bool,

    #[structopt(short = "p", long = "vmpl-process", help = "Run in VMPL for new processes")]
    vmpl_process: bool,

    #[structopt(short = "t", long = "vmpl-thread", help = "Run in VMPL for new threads")]
    vmpl_thread: bool,

    #[structopt(short = "u", long = "user-mode", help = "Run in user mode")]
    user_mode: bool,

    #[structopt(short = "d", long = "debug", help = "Enable debug mode")]
    debug: bool,

    #[structopt(short = "l", long = "log-level", help = "Set log level")]
    log_level: Option<String>,

    #[structopt(short = "T", long = "log-show-time", help = "Show log time")]
    log_show_time: Option<bool>,

    #[structopt(short = "L", long = "log-file", help = "Set log file")]
    log_file: Option<String>,

    #[structopt(short = "S", long = "silent", help = "Silent mode")]
    silent: bool,

    #[structopt(name = "program", help = "Program to run")]
    program: Option<String>,

    #[structopt(name = "program_args", help = "Arguments for the program")]
    program_args: Vec<String>,
}

#[allow(dead_code)]
fn get_install_path() -> PathBuf {
    if let Ok(path) = env::var("VMPL_INSTALL_PATH") {
        PathBuf::from(path)
    } else {
        PathBuf::from("/usr/local")
    }
}

fn get_mmap_min_addr() -> u64 {
    let value = std::fs::read_to_string("/proc/sys/vm/mmap_min_addr").unwrap();
    value.trim().parse::<u64>().unwrap()
}

fn run_program(opt: &Opt) -> std::io::Result<()> {
    if let Some(program) = &opt.program {
        let mut command = Command::new(program);
        command.args(&opt.program_args);

        // 设置 log 级别
        if let Some(log_level) = &opt.log_level {
            command.env("VMPL_LOG_LEVEL", log_level);
        }

        // 设置 log 是否显示时间
        if let Some(log_show_time) = &opt.log_show_time {
            command.env("VMPL_LOG_SHOW_TIME", log_show_time.to_string());
        }

        // 设置 log 文件
        if let Some(log_file) = &opt.log_file {
            command.env("VMPL_LOG_FILE", log_file);
        }

        // 设置 silent 模式
        if opt.silent {
            command.env("VMPL_SILENT", "1");
        }

        // 设置 LIBZPHOOK
        command.env("LIBZPHOOK", "libzphook_basic.so");

        // 设置 HOTCALLS_CONFIG_FILE
        if let Some(hotcalls) = &opt.hotcalls {
            command.env("HOTCALLS_CONFIG_FILE", hotcalls);
        }

        // 设置 dunify.c 中定义的环境变量
        if opt.enable_vmpl {
            command.env("VMPL_ENABLED", "1");
        }

        if opt.vmpl {
            command.env("RUN_IN_VMPL", "1");
        }

        if opt.vmpl_process {
            command.env("RUN_IN_VMPL_PROCESS", "1");
        }

        if opt.vmpl_thread {
            command.env("RUN_IN_VMPL_THREAD", "1");
        }

        if opt.user_mode {
            command.env("RUN_IN_USER_MODE", "1");
        }

        // 设置 LD_PRELOAD
        if opt.preload {
            // 使用 libdunify.so 和 libzpoline.so
            command.env("LD_PRELOAD", "libdunify.so:libzpoline.so");
        } else {
            // 仅使用libzpoline.so
            command.env("LD_PRELOAD", "libzpoline.so");
        }
        
        // 设置其他可选环境变量

        if opt.debug {
            command.env("VMPL_DEBUG_ENABLED", "1");
        }

        // 如果 mmap_min_addr 不为 0，则需要设置为 0
        if get_mmap_min_addr() != 0 {
            Command::new("sudo")
            .args(&["sh", "-c", "echo 0 > /proc/sys/vm/mmap_min_addr"])
            .status()?;
        }

        // 设置自定义环境变量，可覆盖命令行设置的环境变量
        for (var, value) in ENV_VARS.lock().unwrap().iter() {
            command.env(var, value);
        }

        // 运行程序
        let status = command.status()?;
        if !status.success() {
            eprintln!("Program exited with status: {}", status);
        }
    } else {
        eprintln!("No program specified");
    }
    Ok(())
}

fn run_command(opt: &mut Opt, args: Vec<&str>) -> std::io::Result<()> {
    opt.program = Some(args[0].to_string());
    opt.program_args = args[1..].iter().map(|&s| s.to_string()).collect();
    run_program(opt)
}

lazy_static! {
    static ref ENV_VARS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

fn with_env(var: &str, value: &str) -> std::io::Result<()> {
    ENV_VARS.lock().unwrap().insert(var.to_string(), value.to_string());
    Ok(())
}

fn print_help() {
    println!("VMPL Shell - Type 'exit' to quit");
    println!("Available commands:");
    println!("  help                     - Show this help message");
    println!("  history                  - Show command history");
    println!("  exit                     - Exit the shell");
}

fn print_history(history: &History) {
    for (i, h) in history.iter().enumerate() {
        println!("{}: {}", i, h);
    }
}

// A readline shell
fn run_shell(opt: &mut Opt) -> std::io::Result<()> {
    let mut rl = Editor::<()>::new();
    lazy_static::initialize(&ENV_VARS);
    println!("VMPL Shell - Type 'exit' to quit");
    loop {
        let readline = rl.readline("vmpl> ");
        match readline {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                
                let parts: Vec<&str> = line.split_whitespace().collect();
                match parts[0] {
                    "e" | "env" => {
                        // 设置环境变量
                        if parts.len() < 2 {
                            println!("Usage: env <var> <value>");
                            continue;
                        }
                        with_env(parts[1], parts[2])?;
                    }
                    "q" | "exit" | "quit" => break,
                    "h" | "help" => {
                        print_help();
                    }
                    "history" => {
                        print_history(&rl.history());
                    }
                    _ => {
                        // Treat as a direct command (implicit run)
                        if let Err(e) = run_command(opt, parts) {
                            eprintln!("Error running program: {}", e);
                        }
                    }
                }
                
                rl.add_history_entry(line.as_str());
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut opt = Opt::from_args();
    
    if opt.shell {
        run_shell(&mut opt)?;
    } else if opt.run {
        run_program(&opt)?;
    } else {
        eprintln!("Please specify --run to execute a program or --shell for interactive mode");
    }
    
    Ok(())
}