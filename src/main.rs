use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use structopt::StructOpt;
use rustyline::history::History;
use lazy_static::lazy_static;
use std::collections::HashMap;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Context, Editor};
use rustyline_derive::Helper;
use std::fs;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, StructOpt)]
#[structopt(name = "vmpl-run", about = "Run programs with VMPL support")]
struct Opt {
    #[structopt(short = "r", long = "run", help = "Run the specified program")]
    run: bool,

    #[structopt(short = "s", long = "shell", help = "Run a shell")]
    shell: bool,

    #[structopt(short = "c", long = "preload", help = "Use preload library")]
    preload: bool,

    // libzhook
    #[structopt(short = "z", long = "libzhook", help = "Use libzhook")]
    libzhook: bool,

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
            command.env("VMPL_LOG_LEVEL", "error");
        }

        // 设置 LIBZPHOOK
        if opt.libzhook {
            command.env("LIBZPHOOK", "libzphook_basic.so");
        }

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
            if opt.libzhook {
                // 使用 libdunify.so 和 libzpoline.so
                command.env("LD_PRELOAD", "libdunify.so:libzpoline.so");
            } else {
                // 仅使用libdunify.so
                command.env("LD_PRELOAD", "libdunify.so");
            }
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
    println!("  env <var> <value>        - Set environment variable");
}

fn print_history(history: &History) {
    for (i, h) in history.iter().enumerate() {
        println!("{}: {}", i, h);
    }
}

#[derive(Helper)]
struct VmplHelper {
    commands: Vec<String>,
    env_vars: Vec<String>,
}

impl VmplHelper {
    fn new() -> Self {
        VmplHelper {
            commands: vec![
                "env".to_string(),
                "exit".to_string(),
                "quit".to_string(),
                "help".to_string(),
                "history".to_string(),
            ],
            env_vars: vec![
                "VMPL_INSTALL_PATH".to_string(),
                "VMPL_LOG_LEVEL".to_string(),
                "VMPL_LOG_SHOW_TIME".to_string(),
                "VMPL_LOG_FILE".to_string(),
                "LIBZPHOOK".to_string(),
                "HOTCALLS_CONFIG_FILE".to_string(),
                "VMPL_ENABLED".to_string(),
                "RUN_IN_VMPL".to_string(),
                "RUN_IN_VMPL_PROCESS".to_string(),
                "RUN_IN_VMPL_THREAD".to_string(),
                "RUN_IN_USER_MODE".to_string(),
                "LD_PRELOAD".to_string(),
                "VMPL_DEBUG_ENABLED".to_string(),
            ],
        }
    }

    fn get_env_value(&self, var: &str) -> String {
        env::var(var).unwrap_or_default()
    }

    fn get_executables_from_path(&self, prefix: &str) -> Vec<Pair> {
        let mut matches = Vec::new();
        let path_var = env::var("PATH").unwrap_or_default();
        
        // 遍历 PATH 中的每个目录
        for path in path_var.split(':') {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.filter_map(Result::ok) {
                    let file_path = entry.path();
                    
                    // 检查是否是文件且有执行权限
                    if file_path.is_file() {
                        if let Ok(metadata) = entry.metadata() {
                            let mode = metadata.permissions().mode();
                            // 检查是否有执行权限 (0o111 = --x--x--x)
                            if mode & 0o111 != 0 {
                                if let Some(file_name) = file_path.file_name() {
                                    let name = file_name.to_string_lossy();
                                    if name.starts_with(prefix) {
                                        matches.push(Pair {
                                            display: name.to_string(),
                                            replacement: name.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 去重
        matches.sort_by(|a, b| a.display.cmp(&b.display));
        matches.dedup_by(|a, b| a.display == b.display);
        
        matches
    }

    fn get_path_completions(&self, input: &str) -> Vec<Pair> {
        let mut matches = Vec::new();
        
        // 如果输入不包含路径分隔符，则同时搜索当前目录和 PATH
        if !input.contains('/') {
            // 先搜索 PATH 中的可执行文件
            matches.extend(self.get_executables_from_path(input));
            
            // 再搜索当前目录
            if let Ok(entries) = fs::read_dir(".") {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if let Some(file_name) = path.file_name() {
                        let name = file_name.to_string_lossy();
                        if name.starts_with(input) {
                            matches.push(Pair {
                                display: format!("{}{}", name, if path.is_dir() { "/" } else { "" }),
                                replacement: name.to_string(),
                            });
                        }
                    }
                }
            }
            
            return matches;
        }
        
        // 原有的路径补全逻辑
        let input_path = Path::new(input);
        
        // 获取要搜索的目录路径和前缀
        let (search_dir, prefix) = if input.ends_with('/') {
            (input.to_string(), "".to_string())
        } else {
            match input_path.parent() {
                Some(parent) => (
                    parent.to_string_lossy().to_string(),
                    input_path.file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default()
                ),
                None => ("./".to_string(), input.to_string())
            }
        };

        // 如果搜索目录为空，使用当前目录
        let search_dir = if search_dir.is_empty() { ".".to_string() } else { search_dir };

        // 读取目录内容
        if let Ok(entries) = fs::read_dir(&search_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                let file_name = path.file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();

                // 检查是否匹配前缀
                if file_name.starts_with(&prefix) {
                    let mut full_path = if search_dir == "." {
                        file_name.clone()
                    } else {
                        format!("{}/{}", search_dir, file_name)
                    };

                    // 如果是目录，添加斜杠
                    if path.is_dir() {
                        // 如果没有斜杠，则添加斜杠
                        if !full_path.ends_with('/') {
                            full_path.push('/');
                        }
                    }

                    // 添加到匹配列表
                    matches.push(Pair {
                        display: format!("{}{}", file_name, if path.is_dir() { "/" } else { "" }),
                        replacement: full_path,
                    });
                }
            }
        }

        matches
    }
}

impl Completer for VmplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let mut matches: Vec<Pair> = Vec::new();
        
        // 获取当前正在输入的单词
        let split: Vec<&str> = line[..pos].split_whitespace().collect();
        let current_word = if line[..pos].ends_with(' ') {
            ""
        } else {
            split.last().unwrap_or(&"")
        };
        
        match split.get(0) {
            None | Some(&"") => {
                // 命令补全
                for cmd in &self.commands {
                    if cmd.starts_with(current_word) {
                        matches.push(Pair {
                            display: cmd.clone(),
                            replacement: cmd.clone(),
                        });
                    }
                }
            },
            Some(&"env") | Some(&"e") if split.len() <= 2 => {
                // 环境变量补全
                for var in &self.env_vars {
                    if var.starts_with(current_word) {
                        let current_value = self.get_env_value(var);
                        let display = format!("{} (current: {})", var, current_value);
                        matches.push(Pair {
                            display,
                            replacement: var.clone(),
                        });
                    }
                }
            },
            Some(&"env") | Some(&"e") if split.len() == 3 => {
                // 环境变量值补全
                if let Some(var_name) = split.get(1) {
                    let current_value = self.get_env_value(var_name);
                    if !current_value.is_empty() {
                        matches.push(Pair {
                            display: format!("current: {}", current_value),
                            replacement: current_value,
                        });
                    }
                }
            },
            _ => {
                // 文件路径补全
                if !current_word.is_empty() {
                    matches.extend(self.get_path_completions(current_word));
                }
            }
        }
        
        // 计算补全开始位置
        let start = if current_word.is_empty() {
            pos
        } else {
            pos - current_word.len()
        };
        
        Ok((start, matches))
    }
}

impl Hinter for VmplHelper {
    type Hint = String;

    fn hint(&self, line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        let split: Vec<&str> = line.split_whitespace().collect();
        match split.get(0) {
            Some(&"env") | Some(&"e") => {
                match split.len() {
                    1 => Some(String::from(" <variable> <value>")),
                    2 => {
                        let var = split[1];
                        if let Some(found_var) = self.env_vars.iter().find(|v| v.starts_with(var)) {
                            let current = self.get_env_value(found_var);
                            if !current.is_empty() {
                                Some(format!(" current value: {}", current))
                            } else {
                                Some(String::from(" <value>"))
                            }
                        } else {
                            Some(String::from(" <value>"))
                        }
                    }
                    _ => None
                }
            }
            Some(&"help") | Some(&"h") => Some(String::from(" - Show available commands")),
            Some(&"exit") | Some(&"quit") | Some(&"q") => Some(String::from(" - Exit the shell")),
            Some(&"history") => Some(String::from(" - Show command history")),
            _ => None
        }
    }
}

impl Highlighter for VmplHelper {}
impl Validator for VmplHelper {}

// A readline shell
fn run_shell(opt: &mut Opt) -> std::io::Result<()> {
    // 配置 rustyline
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();
    
    let helper = VmplHelper::new();
    let mut rl = Editor::with_config(config);
    rl.set_helper(Some(helper));
    
    // 尝试加载历史记录
    let history_file = format!("{}/.vmpl_history", env::var("HOME").unwrap_or_default());
    if rl.load_history(&history_file).is_err() {
        println!("No previous history.");
    }
    
    lazy_static::initialize(&ENV_VARS);
    println!("VMPL Shell - Type 'help' for commands, 'exit' to quit");
    
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
                        if parts.len() < 3 {
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
                        print_history(rl.history());
                    }
                    _ => {
                        if let Err(e) = run_command(opt, parts) {
                            eprintln!("Error running program: {}", e);
                        }
                    }
                }
                
                rl.add_history_entry(line.as_str());
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }
    
    // 保存历史记录
    rl.save_history(&history_file).unwrap_or_else(|e| {
        eprintln!("Error saving history: {}", e);
    });
    
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