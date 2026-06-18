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
use emulator::{Machine, PreparedExecVma};
use loader::{ExecPlanDescriptorOptions, ExecutableProvenance, LoaderOptions, VmaProtection};

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
        "elf-plan" => {
            let options = take_elf_plan_options(&mut args)?;
            let probe = build_elf_exec_probe(&options)?;
            println!(
                "exec-plan version={} entry=0x{:x} initial_sp=0x{:x} tls_base=0x{:x} startup_metadata=0x{:x}",
                probe.plan.version,
                probe.plan.entry.entry_pc,
                probe.plan.entry.initial_sp,
                probe.plan.entry.tls_base,
                probe.plan.entry.startup_metadata_ptr
            );
            println!(
                "descriptor_length={} descriptor_words={} descriptor_validated=true memory_commit_validated=true vmas={} phdr={} tls={} startup_note={} fdr_grants={} measurements={}",
                probe.descriptor.header.total_length,
                probe.descriptor_words.len(),
                probe.prepared.len(),
                probe.plan.phdr.is_some(),
                probe.plan.tls.is_some(),
                probe.plan.startup.is_some(),
                probe.plan.fdr_grants.len(),
                probe.descriptor.measurements.len()
            );
            for (idx, vma) in probe.plan.vmas.iter().enumerate() {
                let prepared_len = probe
                    .prepared
                    .get(idx)
                    .map(|prepared_vma| prepared_vma.bytes.len())
                    .unwrap_or_default();
                println!(
                    "vma[{idx}] addr=0x{:x} len=0x{:x} prot={} provenance={} source=0x{:x}+0x{:x} zero=0x{:x} materialized=0x{:x}",
                    vma.virtual_address,
                    vma.length,
                    format_protection(vma.protection),
                    format_provenance(vma.executable_provenance),
                    vma.source_offset,
                    vma.source_length,
                    vma.zero_fill_length,
                    prepared_len
                );
            }
            Ok(())
        }
        "run-elf" => {
            let options = take_elf_plan_options(&mut args)?;
            let probe = build_elf_exec_probe(&options)?;
            Err(format!(
                "run-elf loaded and committed exec plan for {} at entry=0x{:x}, but ELF text fetch/decode is not implemented yet; see toolchain/lnp64_run_elf.manifest",
                options.input.display(),
                probe.plan.entry.entry_pc
            ))
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
    eprintln!("  lnp64 elf-plan [--load-bias <n>] <program.elf>");
    eprintln!("  lnp64 run-elf [--load-bias <n>] <program.elf>");
    eprintln!(
        "  lnp64 cc [--dump-macros|--dump-preprocessed] <program.c> [more.c ...] [-o program.s]"
    );
}

struct ElfExecProbe {
    plan: loader::ExecPlan,
    prepared: Vec<loader::PreparedVma>,
    descriptor: loader::ExecPlanDescriptor,
    descriptor_words: Vec<u64>,
}

fn build_elf_exec_probe(options: &ElfPlanOptions) -> Result<ElfExecProbe, String> {
    let mut image = fs::read(&options.input)
        .map_err(|err| format!("failed to read {}: {err}", options.input.display()))?;
    let plan = loader::load_static_elf(
        &mut image,
        LoaderOptions {
            load_bias: options.load_bias,
            ..LoaderOptions::default()
        },
    )?;
    let prepared = loader::materialize_vmas(&image, &plan)?;
    let descriptor = loader::build_exec_descriptor(
        &plan,
        ExecPlanDescriptorOptions {
            image_source_cap: 1,
            image_source_generation: 1,
            image_lineage_epoch: 1,
            ..ExecPlanDescriptorOptions::default()
        },
    )?;
    let descriptor_words = loader::encode_exec_descriptor(&descriptor);
    Machine::validate_exec_descriptor_words(&descriptor_words)?;
    let commit_vmas = prepared
        .iter()
        .zip(descriptor.vmas.iter())
        .map(|(prepared_vma, descriptor_vma)| PreparedExecVma {
            virtual_address: prepared_vma.virtual_address,
            protection: descriptor_vma.protection,
            bytes: prepared_vma.bytes.clone(),
        })
        .collect::<Vec<_>>();
    let mut commit_probe = Machine::new(Program::parse(".text\n  NOP\n")?);
    commit_probe.commit_exec_descriptor_memory_image(&descriptor_words, &commit_vmas)?;
    Ok(ElfExecProbe {
        plan,
        prepared,
        descriptor,
        descriptor_words,
    })
}

struct ElfPlanOptions {
    input: PathBuf,
    load_bias: u64,
}

fn take_elf_plan_options(args: &mut Vec<String>) -> Result<ElfPlanOptions, String> {
    let mut load_bias = 0;
    loop {
        let Some(arg) = args.first() else {
            break;
        };
        if arg != "--load-bias" {
            break;
        }
        args.remove(0);
        if args.is_empty() {
            return Err("--load-bias requires a value".to_string());
        }
        load_bias = parse_u64_arg(&args.remove(0), "--load-bias")?;
    }
    let input = take_input(args)?;
    if !args.is_empty() {
        return Err(format!("unexpected elf-plan arguments: {}", args.join(" ")));
    }
    Ok(ElfPlanOptions { input, load_bias })
}

fn parse_u64_arg(value: &str, name: &str) -> Result<u64, String> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).map_err(|err| format!("{name} value {value:?}: {err}"))
    } else {
        value
            .parse::<u64>()
            .map_err(|err| format!("{name} value {value:?}: {err}"))
    }
}

fn format_protection(protection: VmaProtection) -> String {
    let mut text = String::with_capacity(3);
    text.push(if protection.read { 'r' } else { '-' });
    text.push(if protection.write { 'w' } else { '-' });
    text.push(if protection.execute { 'x' } else { '-' });
    text
}

fn format_provenance(provenance: ExecutableProvenance) -> &'static str {
    match provenance {
        ExecutableProvenance::ImageText => "image_text",
        ExecutableProvenance::NonExecutable => "non_executable",
    }
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
