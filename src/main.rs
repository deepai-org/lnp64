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
use isa::{Condition, Instr, MemRef, Reg, Target, Value, Width};
use loader::{
    ExecEntry, ExecPlan, ExecPlanDescriptorOptions, ExecutableProvenance, LoaderOptions,
    MemoryType, VmaProtection, VmaRecord,
};

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
        "asm-flat-exec" => {
            let options = take_asm_flat_exec_options(&mut args)?;
            let source = fs::read_to_string(&options.input)
                .map_err(|err| format!("failed to read {}: {err}", options.input.display()))?;
            let program = Program::parse(&source)?;
            let hex = encode_flat_exec_hex(&program)?;
            if let Some(output) = options.output {
                fs::write(&output, hex)
                    .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
            } else {
                print!("{hex}");
            }
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
            let mut probe = build_elf_exec_probe(&options)?;
            let exit = probe.machine.run_committed_exec()?;
            if exit == 0 {
                println!(
                    "run-elf executed {} entry=0x{:x} exit=0",
                    options.input.display(),
                    probe.plan.entry.entry_pc
                );
                Ok(())
            } else {
                Err(format!(
                    "run-elf executed {} entry=0x{:x} exit={exit}",
                    options.input.display(),
                    probe.plan.entry.entry_pc
                ))
            }
        }
        "run-flat-exec" => {
            let input = take_input(&mut args)?;
            if !args.is_empty() {
                return Err(format!(
                    "unexpected run-flat-exec arguments: {}",
                    args.join(" ")
                ));
            }
            let text = fs::read_to_string(&input)
                .map_err(|err| format!("failed to read {}: {err}", input.display()))?;
            let mut machine = build_flat_exec_machine(&text)?;
            let exit = machine.run_committed_exec()?;
            let regs = machine.last_exit_registers().ok_or_else(|| {
                "flat exec finished without an exit register snapshot".to_string()
            })?;
            let r3 = regs.get(3).copied().unwrap_or_default();
            let r4 = regs.get(4).copied().unwrap_or_default();
            let r5 = regs.get(5).copied().unwrap_or_default();
            let env_page = regs.get(6).copied().unwrap_or_default();
            let mem0 = machine.last_exit_mem0().unwrap_or_default();
            let trace = machine
                .committed_exec_retire_trace()
                .iter()
                .map(|(pc, opcode)| {
                    let pc_word = pc.saturating_sub(0x1000) / 4;
                    format!("{{\"pc\":{pc_word},\"opcode\":{opcode}}}")
                })
                .collect::<Vec<_>>()
                .join(",");
            println!("EMULATOR_RETIRE [{trace}]");
            println!(
                "EMULATOR_FINAL {{\"exit\":{exit},\"r3\":{r3},\"r4\":{r4},\"r5\":{r5},\"env_page\":{env_page},\"mem0\":{mem0}}}"
            );
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
    eprintln!("  lnp64 asm-flat-exec <program.s> [-o program.hex]");
    eprintln!("  lnp64 run [--namespace-root <dir>] <program.s>");
    eprintln!("  lnp64 elf-plan [--load-bias <n>] <program.elf>");
    eprintln!("  lnp64 run-elf [--load-bias <n>] <program.elf>");
    eprintln!("  lnp64 run-flat-exec <program.hex>");
    eprintln!(
        "  lnp64 cc [--dump-macros|--dump-preprocessed] <program.c> [more.c ...] [-o program.s]"
    );
}

struct AsmFlatExecOptions {
    input: PathBuf,
    output: Option<PathBuf>,
}

fn encode_flat_exec_hex(program: &Program) -> Result<String, String> {
    let mut out = String::new();
    for (pc, instr) in program.instructions.iter().enumerate() {
        let word = encode_flat_exec_instr(program, pc, instr)?;
        out.push_str(&format!("{word:08x}\n"));
    }
    if out.is_empty() {
        return Err("asm-flat-exec input has no text instructions".to_string());
    }
    Ok(out)
}

fn encode_flat_exec_instr(program: &Program, pc: usize, instr: &Instr) -> Result<u32, String> {
    match instr {
        Instr::Nop => Ok(enc_reg(0x00, Reg(0))),
        Instr::Li(rd, value) => Ok(enc_ri(0x01, *rd, value_imm16(value)?)),
        Instr::Add(rd, rs1, rs2) => Ok(enc_rrr(0x10, *rd, *rs1, *rs2)),
        Instr::Sub(rd, rs1, rs2) => Ok(enc_rrr(0x11, *rd, *rs1, *rs2)),
        Instr::Mul(rd, rs1, rs2) => Ok(enc_rrr(0x12, *rd, *rs1, *rs2)),
        Instr::Cmp(lhs, rhs) => Ok(enc_rrr(0x1b, *lhs, *rhs, Reg(0))),
        Instr::Jmp(target) => Ok(enc_branch(0x20, branch_delta(program, pc, target)?)),
        Instr::Branch(condition, target) => Ok(enc_branch(
            flat_exec_branch_opcode(*condition)?,
            branch_delta(program, pc, target)?,
        )),
        Instr::Ld(rd, MemRef::BaseOffset(base, offset), Width::Double) => {
            Ok(enc_mem(0x30, *rd, *base, imm14(*offset, "LD offset")?))
        }
        Instr::St(MemRef::BaseOffset(base, offset), src, Width::Double) => {
            Ok(enc_mem(0x33, *src, *base, imm14(*offset, "ST offset")?))
        }
        Instr::ErrnoGet(rd) => Ok(enc_reg(0x38, *rd)),
        Instr::ErrnoSet(src) => Ok(enc_reg(0x39, *src)),
        Instr::EnvGet(rd, key, index_or_buf, len_or_flags) => {
            Ok(enc_rrrr(0x56, *rd, *key, *index_or_buf, *len_or_flags))
        }
        Instr::Exit(src) => Ok(enc_reg(0x3a, *src)),
        other => Err(format!(
            "asm-flat-exec cannot encode {other:?}; supported subset is NOP, LI, ADD, SUB, MUL, CMP, JMP, signed conditional branch, LD/ST.D base+offset, ERRNO_GET/SET, ENV_GET, EXIT"
        )),
    }
}

fn flat_exec_branch_opcode(condition: Condition) -> Result<u8, String> {
    match condition {
        Condition::Eq => Ok(0x21),
        Condition::Ne => Ok(0x22),
        Condition::Lt => Ok(0x23),
        Condition::Gt => Ok(0x24),
        Condition::Le => Ok(0x25),
        Condition::Ge => Ok(0x26),
        other => Err(format!(
            "asm-flat-exec does not yet encode unsigned branch condition {other:?}"
        )),
    }
}

fn value_imm16(value: &Value) -> Result<i64, String> {
    match value {
        Value::Imm(imm) => imm16(*imm, "LI immediate"),
        Value::Label(label) => Err(format!(
            "asm-flat-exec does not yet materialize label immediate {label:?}"
        )),
    }
}

fn branch_delta(program: &Program, pc: usize, target: &Target) -> Result<i64, String> {
    let target_pc = match target {
        Target::Address(address) => *address,
        Target::Label(label) => program
            .labels
            .get(label)
            .copied()
            .ok_or_else(|| format!("unknown branch label {label:?}"))?,
    };
    imm24(target_pc as i64 - pc as i64, "branch delta")
}

fn imm16(value: i64, name: &str) -> Result<i64, String> {
    if !(-32768..=32767).contains(&value) {
        return Err(format!("{name} out of signed 16-bit range: {value}"));
    }
    Ok(value)
}

fn imm14(value: i64, name: &str) -> Result<i64, String> {
    if !(-8192..=8191).contains(&value) {
        return Err(format!("{name} out of signed 14-bit range: {value}"));
    }
    Ok(value)
}

fn imm24(value: i64, name: &str) -> Result<i64, String> {
    if !(-8_388_608..=8_388_607).contains(&value) {
        return Err(format!("{name} out of signed 24-bit range: {value}"));
    }
    Ok(value)
}

fn enc_ri(opcode: u8, rd: Reg, imm: i64) -> u32 {
    (u32::from(opcode) << 24) | (((rd.0 as u32) & 0x1f) << 19) | ((imm as u32) & 0xffff)
}

fn enc_rrr(opcode: u8, rd: Reg, rs1: Reg, rs2: Reg) -> u32 {
    (u32::from(opcode) << 24)
        | (((rd.0 as u32) & 0x1f) << 19)
        | (((rs1.0 as u32) & 0x1f) << 14)
        | (((rs2.0 as u32) & 0x1f) << 9)
}

fn enc_rrrr(opcode: u8, rd: Reg, rs1: Reg, rs2: Reg, rs3: Reg) -> u32 {
    enc_rrr(opcode, rd, rs1, rs2) | (((rs3.0 as u32) & 0x1f) << 4)
}

fn enc_mem(opcode: u8, reg_a: Reg, base: Reg, imm: i64) -> u32 {
    (u32::from(opcode) << 24)
        | (((reg_a.0 as u32) & 0x1f) << 19)
        | (((base.0 as u32) & 0x1f) << 14)
        | ((imm as u32) & 0x3fff)
}

fn enc_reg(opcode: u8, reg: Reg) -> u32 {
    (u32::from(opcode) << 24) | (((reg.0 as u32) & 0x1f) << 19)
}

fn enc_branch(opcode: u8, delta_words: i64) -> u32 {
    (u32::from(opcode) << 24) | ((delta_words as u32) & 0x00ff_ffff)
}

fn build_flat_exec_machine(hex_words: &str) -> Result<Machine, String> {
    const DATA_BASE: u64 = 0;
    const TEXT_BASE: u64 = 0x1000;
    const PAGE_SIZE: usize = 4096;
    const PROT_READ: u64 = 1 << 0;
    const PROT_WRITE: u64 = 1 << 1;
    const PROT_EXECUTE: u64 = 1 << 2;

    let text = flat_hex_words_to_bytes(hex_words)?;
    if text.len() > PAGE_SIZE {
        return Err(format!(
            "flat exec image is too large: {} bytes > {PAGE_SIZE}",
            text.len()
        ));
    }
    let mut text_page = vec![0u8; PAGE_SIZE];
    text_page[..text.len()].copy_from_slice(&text);
    let data_page = vec![0u8; PAGE_SIZE];

    let plan = ExecPlan {
        version: 1,
        entry: ExecEntry {
            entry_pc: TEXT_BASE,
            initial_sp: 0,
            tls_base: 0,
            startup_metadata_ptr: 0,
        },
        vmas: vec![
            VmaRecord {
                virtual_address: DATA_BASE,
                length: PAGE_SIZE as u64,
                protection: VmaProtection {
                    read: true,
                    write: true,
                    execute: false,
                },
                memory_type: MemoryType::Image,
                executable_provenance: ExecutableProvenance::NonExecutable,
                source_offset: 0,
                source_length: 0,
                zero_fill_length: PAGE_SIZE as u64,
                mapping_flags: 0,
            },
            VmaRecord {
                virtual_address: TEXT_BASE,
                length: PAGE_SIZE as u64,
                protection: VmaProtection {
                    read: true,
                    write: false,
                    execute: true,
                },
                memory_type: MemoryType::Image,
                executable_provenance: ExecutableProvenance::ImageText,
                source_offset: 0,
                source_length: text.len() as u64,
                zero_fill_length: (PAGE_SIZE - text.len()) as u64,
                mapping_flags: 0,
            },
        ],
        phdr: None,
        tls: None,
        startup: None,
        fdr_grants: Vec::new(),
    };
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
    let prepared = vec![
        PreparedExecVma {
            virtual_address: DATA_BASE,
            protection: PROT_READ | PROT_WRITE,
            bytes: data_page,
        },
        PreparedExecVma {
            virtual_address: TEXT_BASE,
            protection: PROT_READ | PROT_EXECUTE,
            bytes: text_page,
        },
    ];
    let mut machine = Machine::new(Program::parse(".text\n  NOP\n")?);
    machine.commit_exec_descriptor_memory_image(&descriptor_words, &prepared)?;
    Ok(machine)
}

fn flat_hex_words_to_bytes(hex_words: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    for (idx, raw_line) in hex_words.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let word_text = line
            .strip_prefix("0x")
            .or_else(|| line.strip_prefix("0X"))
            .unwrap_or(line);
        let word = u32::from_str_radix(word_text, 16)
            .map_err(|err| format!("invalid hex word on line {}: {err}", idx + 1))?;
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    if bytes.is_empty() {
        return Err("flat exec image is empty".to_string());
    }
    Ok(bytes)
}

struct ElfExecProbe {
    plan: loader::ExecPlan,
    prepared: Vec<loader::PreparedVma>,
    descriptor: loader::ExecPlanDescriptor,
    descriptor_words: Vec<u64>,
    machine: Machine,
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
        machine: commit_probe,
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

fn take_asm_flat_exec_options(args: &mut Vec<String>) -> Result<AsmFlatExecOptions, String> {
    let mut input = None;
    let mut output = None;
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
        } else if arg.starts_with('-') {
            return Err(format!("unexpected asm-flat-exec option {arg:?}"));
        } else if input.is_some() {
            return Err(format!("unexpected asm-flat-exec argument {arg:?}"));
        } else {
            input = Some(PathBuf::from(arg));
        }
    }
    let input = input.ok_or_else(|| "missing input path".to_string())?;
    Ok(AsmFlatExecOptions { input, output })
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

#[cfg(test)]
mod tests {
    use super::*;

    const ELFCLASS64: u8 = 2;
    const ELFDATA2LSB: u8 = 1;
    const EV_CURRENT: u8 = 1;
    const ET_EXEC: u16 = 2;
    const EM_LNP64: u16 = 0x6c64;
    const PT_LOAD: u32 = 1;
    const PF_X: u32 = 1;
    const PF_R: u32 = 4;
    const ELF64_EHDR_SIZE: usize = 64;
    const ELF64_PHDR_SIZE: usize = 56;

    #[test]
    fn run_elf_probe_loads_and_commits_minimal_static_elf() {
        let path =
            std::env::temp_dir().join(format!("lnp64-run-elf-probe-{}.elf", std::process::id()));
        fs::write(&path, minimal_static_elf()).unwrap();

        let probe = build_elf_exec_probe(&ElfPlanOptions {
            input: path.clone(),
            load_bias: 0,
        })
        .unwrap();

        assert_eq!(probe.plan.entry.entry_pc, 0x400000);
        assert_eq!(probe.prepared.len(), 1);
        assert_eq!(probe.prepared[0].virtual_address, 0x400000);
        assert_eq!(probe.prepared[0].bytes, vec![0xcc; 16]);
        assert_eq!(probe.descriptor.vmas.len(), 1);
        assert!(!probe.descriptor_words.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn run_elf_executes_minimal_exit_static_elf() {
        let path = std::env::temp_dir().join(format!(
            "lnp64-run-elf-exec-probe-{}.elf",
            std::process::id()
        ));
        fs::write(&path, minimal_static_exit_elf()).unwrap();

        let mut probe = build_elf_exec_probe(&ElfPlanOptions {
            input: path.clone(),
            load_bias: 0,
        })
        .unwrap();
        let exit = probe.machine.run_committed_exec().unwrap();

        assert_eq!(exit, 0);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn asm_flat_exec_encodes_top_level_smoke_subset() {
        let source = r#"
            .text
              LI r1, 7
              LI r2, 5
              ADD r3, r1, r2
              ST [r0, 0], r3
              LD r4, [r0, 0]
              JMP after_skip
              LI r5, 99
            after_skip:
              LI r10, 2
              ENV_GET r6, r10, r0, r0
              EXIT r4
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080007\n",
                "01100005\n",
                "10184400\n",
                "33180000\n",
                "30200000\n",
                "20000002\n",
                "01280063\n",
                "01500002\n",
                "56328000\n",
                "3a200000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_cmp_and_signed_branch_subset() {
        let source = r#"
            .text
              LI r1, 3
              LI r2, 3
              CMP r1, r2
              BEQ equal
              LI r3, 4
              JMP done
            equal:
              LI r3, 17
            done:
              EXIT r3
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080003\n",
                "01100003\n",
                "1b088000\n",
                "21000003\n",
                "01180004\n",
                "20000002\n",
                "01180011\n",
                "3a180000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_mul_subset() {
        let source = r#"
            .text
              LI r1, 6
              LI r2, 7
              MUL r3, r1, r2
              EXIT r3
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!("01080006\n", "01100007\n", "12184400\n", "3a180000\n",)
        );
    }

    #[test]
    fn asm_flat_exec_encodes_sub_subset() {
        let source = r#"
            .text
              LI r1, 9
              LI r2, 4
              SUB r3, r1, r2
              EXIT r3
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!("01080009\n", "01100004\n", "11184400\n", "3a180000\n",)
        );
    }

    fn minimal_static_elf() -> Vec<u8> {
        let mut image = vec![0; 0x200];
        image[0..4].copy_from_slice(b"\x7fELF");
        image[4] = ELFCLASS64;
        image[5] = ELFDATA2LSB;
        image[6] = EV_CURRENT;
        put_u16(&mut image, 16, ET_EXEC);
        put_u16(&mut image, 18, EM_LNP64);
        put_u32(&mut image, 20, u32::from(EV_CURRENT));
        put_u64(&mut image, 24, 0x400000);
        put_u64(&mut image, 32, ELF64_EHDR_SIZE as u64);
        put_u16(&mut image, 52, ELF64_EHDR_SIZE as u16);
        put_u16(&mut image, 54, ELF64_PHDR_SIZE as u16);
        put_u16(&mut image, 56, 1);

        let phdr = ELF64_EHDR_SIZE;
        put_u32(&mut image, phdr, PT_LOAD);
        put_u32(&mut image, phdr + 4, PF_R | PF_X);
        put_u64(&mut image, phdr + 8, 0x100);
        put_u64(&mut image, phdr + 16, 0x400000);
        put_u64(&mut image, phdr + 32, 16);
        put_u64(&mut image, phdr + 40, 16);
        put_u64(&mut image, phdr + 48, 4096);
        image[0x100..0x110].fill(0xcc);
        image
    }

    fn minimal_static_exit_elf() -> Vec<u8> {
        let mut image = minimal_static_elf();
        put_u32(&mut image, 0x100, 0x3a00_0000);
        image[0x104..0x110].fill(0);
        image
    }

    fn put_u16(image: &mut [u8], offset: usize, value: u16) {
        image[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(image: &mut [u8], offset: usize, value: u32) {
        image[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(image: &mut [u8], offset: usize, value: u64) {
        image[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
