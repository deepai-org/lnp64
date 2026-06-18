mod asm;
mod c_compiler;
mod c_constants;
mod c_escapes;
mod c_layouts;
mod c_macro_rewrites;
mod c_queue_rewrites;
mod c_static_rewrites;
mod c_support_sources;
mod c_type_rewrites;
mod emulator;
mod isa;
mod loader;
mod lowering;
mod native;

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
            let namespace_root = take_run_namespace_root(&mut args)?;
            let input = take_input(&mut args)?;
            if args.first().is_some_and(|arg| arg == "--") {
                args.remove(0);
            }
            let source = fs::read_to_string(&input)
                .map_err(|err| format!("failed to read {}: {err}", input.display()))?;
            let program = Program::parse(&source)?;
            let mut machine = Machine::new(program);
            if let Some(root) = namespace_root {
                machine.set_namespace_root(root)?;
            }
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
            let options = take_cc_options(&mut args)?;
            let text = if options.dump_macros {
                c_compiler::macro_expand_files(&options.inputs)?
            } else if options.dump_preprocessed {
                c_compiler::preprocess_files(&options.inputs)?
            } else {
                c_compiler::compile_files(&options.inputs)?
            };
            if let Some(output) = options.output {
                fs::write(&output, text)
                    .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
            } else {
                print!("{text}");
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
    eprintln!("  lnp64 run [--namespace-root <dir>] <program.s>");
    eprintln!(
        "  lnp64 cc [--dump-macros|--dump-preprocessed] <program.c> [more.c ...] [-o program.s]"
    );
}

fn take_run_namespace_root(args: &mut Vec<String>) -> Result<Option<PathBuf>, String> {
    let mut root = None;
    loop {
        let Some(arg) = args.first() else {
            break;
        };
        if arg != "--namespace-root" {
            break;
        }
        args.remove(0);
        if root.is_some() {
            return Err("duplicate --namespace-root".to_string());
        }
        if args.is_empty() {
            return Err("--namespace-root requires a directory".to_string());
        }
        root = Some(PathBuf::from(args.remove(0)));
    }
    Ok(root)
}

fn take_input(args: &mut Vec<String>) -> Result<PathBuf, String> {
    if args.is_empty() {
        return Err("missing input path".to_string());
    }
    Ok(PathBuf::from(args.remove(0)))
}

struct CcOptions {
    inputs: Vec<PathBuf>,
    output: Option<PathBuf>,
    dump_macros: bool,
    dump_preprocessed: bool,
}

fn take_cc_options(args: &mut Vec<String>) -> Result<CcOptions, String> {
    let mut inputs = Vec::new();
    let mut output = None;
    let mut dump_macros = false;
    let mut dump_preprocessed = false;
    while !args.is_empty() {
        let arg = args.remove(0);
        if arg == "-o" {
            if output.is_some() {
                return Err("duplicate -o".to_string());
            }
            if args.is_empty() {
                return Err("-o requires a path".to_string());
            }
            output = Some(PathBuf::from(args.remove(0)));
        } else if arg == "--dump-preprocessed" {
            dump_preprocessed = true;
        } else if arg == "--dump-macros" {
            dump_macros = true;
        } else if arg.starts_with('-') {
            return Err(format!("unexpected cc option {arg:?}"));
        } else {
            inputs.push(PathBuf::from(arg));
        }
    }
    if inputs.is_empty() {
        return Err("missing input path".to_string());
    }
    Ok(CcOptions {
        inputs,
        output,
        dump_macros,
        dump_preprocessed,
    })
}
