mod asm;
mod c_compiler;
mod emulator;
mod isa;

use std::env;
use std::fs;
use std::path::PathBuf;

use asm::Program;
use emulator::Machine;

fn main() {
    if let Err(err) = run() {
        eprintln!("lnp64: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        usage();
        return Err("missing command".to_string());
    }

    match args.remove(0).as_str() {
        "asm" => {
            let input = take_input(&mut args)?;
            let source = fs::read_to_string(&input)
                .map_err(|err| format!("failed to read {}: {err}", input.display()))?;
            let program = Program::parse(&source)?;
            println!(
                "assembled: {} instructions, {} data bytes",
                program.instructions.len(),
                program.data.len()
            );
            Ok(())
        }
        "run" => {
            let input = take_input(&mut args)?;
            if args.first().is_some_and(|arg| arg == "--") {
                args.remove(0);
            }
            let source = fs::read_to_string(&input)
                .map_err(|err| format!("failed to read {}: {err}", input.display()))?;
            let program = Program::parse(&source)?;
            let mut machine = Machine::new(program);
            let run_args = if args.is_empty() {
                vec![
                    input
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("lnp64-program")
                        .to_string(),
                ]
            } else {
                args
            };
            machine.set_args(&run_args)?;
            let code = machine.run()?;
            if code != 0 {
                std::process::exit(code.clamp(1, 255));
            }
            Ok(())
        }
        "cc" => {
            let input = take_input(&mut args)?;
            let output = take_output(&mut args)?;
            if !args.is_empty() {
                return Err(format!("unexpected arguments: {}", args.join(" ")));
            }
            let source = fs::read_to_string(&input)
                .map_err(|err| format!("failed to read {}: {err}", input.display()))?;
            let asm = c_compiler::compile(&source)?;
            if let Some(output) = output {
                fs::write(&output, asm)
                    .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
            } else {
                print!("{asm}");
            }
            Ok(())
        }
        "help" | "--help" | "-h" => {
            usage();
            Ok(())
        }
        other => Err(format!("unknown command {other:?}")),
    }
}

fn usage() {
    eprintln!("usage:");
    eprintln!("  lnp64 asm <program.s>");
    eprintln!("  lnp64 run <program.s>");
    eprintln!("  lnp64 cc <program.c> [-o program.s]");
}

fn take_input(args: &mut Vec<String>) -> Result<PathBuf, String> {
    if args.is_empty() {
        return Err("missing input path".to_string());
    }
    Ok(PathBuf::from(args.remove(0)))
}

fn take_output(args: &mut Vec<String>) -> Result<Option<PathBuf>, String> {
    if args.is_empty() {
        return Ok(None);
    }
    if args[0] != "-o" {
        return Ok(None);
    }
    args.remove(0);
    if args.is_empty() {
        return Err("-o requires a path".to_string());
    }
    Ok(Some(PathBuf::from(args.remove(0))))
}
