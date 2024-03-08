use std::env;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short = "r", long = "run")]
    run: bool,

    #[structopt(short = "h", long = "hotcalls")]
    hotcalls: bool,

    #[structopt(short = "v", long = "vmpl-mm")]
    vmpl_mm: bool,

    #[structopt(short = "d", long = "debug")]
    debug: bool,

    #[structopt(name = "program")]
    program: Option<String>,

    #[structopt(name = "program_args", multiple = true)]
    program_args: Vec<String>,
}

fn main() {
    let opt = Opt::from_args();

    if opt.run {
        if let Some(program) = opt.program {
            let mut command = std::process::Command::new(program);
            command.args(opt.program_args);

            // Set environment variables
            for (key, value) in env::vars() {
                command.env(key, value);
            }

            if opt.hotcalls {
                command.env("HOTCALLS_ENABLED", "1");
            }

            if opt.vmpl_mm {
                command.env("VMPL_MM_ENABLED", "1");
            }

            if opt.debug {
                command.env("VMPL_DEBUG_ENABLED", "1");
            }

            command.spawn().expect("Failed to run the program");
        } else {
            println!("No program specified");
        }
    }
}