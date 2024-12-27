use std::env;
use std::path::PathBuf;
use std::process::Command;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "vmpl-run", about = "Run programs with VMPL support")]
struct Opt {
    #[structopt(short = "r", long = "run", help = "Run the specified program")]
    run: bool,

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
        let mut preload = String::new();
        preload.push_str("libdunify.so");
        preload.push(':');
        preload.push_str("libzpoline.so");
        
        command.env("LD_PRELOAD", preload);
        
        // 设置其他可选环境变量
        if opt.hotcalls {
            command.env("HOTCALLS_ENABLED", "1");
        }

        if opt.debug {
            command.env("VMPL_DEBUG_ENABLED", "1");
        }

        // 如果 mmap_min_addr 不为 0，则需要设置为 0
        if get_mmap_min_addr() != 0 {
            Command::new("sudo")
            .args(&["sh", "-c", "echo 0 > /proc/sys/vm/mmap_min_addr"])
            .status()?;
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

fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();
    
    if opt.run {
        run_program(&opt)?;
    } else {
        eprintln!("Please specify --run to execute a program");
    }
    
    Ok(())
}