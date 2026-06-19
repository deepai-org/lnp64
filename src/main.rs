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
            if !options.toy_bootstrap {
                return Err(
                    "lnp64 cc is the deprecated Rust bootstrap C compiler; use the real Clang/lld gates, or pass --toy-bootstrap for legacy smoke generation"
                        .to_string(),
                );
            }
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
        "  lnp64 cc --toy-bootstrap [--dump-macros|--dump-preprocessed] <program.c> [more.c ...] [-o program.s]"
    );
}

struct AsmFlatExecOptions {
    input: PathBuf,
    output: Option<PathBuf>,
}

fn encode_flat_exec_hex(program: &Program) -> Result<String, String> {
    let word_pcs = flat_exec_word_pcs(program);
    let mut out = String::new();
    for (pc, instr) in program.instructions.iter().enumerate() {
        for word in encode_flat_exec_instr(program, &word_pcs, pc, instr)? {
            out.push_str(&format!("{word:08x}\n"));
        }
    }
    if out.is_empty() {
        return Err("asm-flat-exec input has no text instructions".to_string());
    }
    Ok(out)
}

fn flat_exec_word_pcs(program: &Program) -> Vec<usize> {
    let mut pcs = Vec::with_capacity(program.instructions.len());
    let mut pc = 0usize;
    for instr in &program.instructions {
        pcs.push(pc);
        pc += flat_exec_instr_word_len(instr);
    }
    pcs
}

fn flat_exec_instr_word_len(instr: &Instr) -> usize {
    match instr {
        Instr::Li(_, Value::Imm(imm)) if imm16(*imm, "LI immediate").is_err() => 2,
        _ => 1,
    }
}

fn encode_flat_exec_instr(
    program: &Program,
    word_pcs: &[usize],
    pc: usize,
    instr: &Instr,
) -> Result<Vec<u32>, String> {
    match instr {
        Instr::Nop => Ok(vec![enc_reg(0x00, Reg(0))]),
        Instr::Li(rd, value) => encode_flat_exec_li(*rd, value),
        Instr::Mov(rd, rs1) => Ok(vec![enc_rrr(0x02, *rd, *rs1, Reg(0))]),
        Instr::Add(rd, rs1, rs2) => Ok(vec![enc_rrr(0x10, *rd, *rs1, *rs2)]),
        Instr::Addi(rd, rs1, imm) => Ok(vec![enc_mem(
            0xa0,
            *rd,
            *rs1,
            imm14(*imm, "ADDI immediate")?,
        )]),
        Instr::Sub(rd, rs1, rs2) => Ok(vec![enc_rrr(0x11, *rd, *rs1, *rs2)]),
        Instr::Mul(rd, rs1, rs2) => Ok(vec![enc_rrr(0x12, *rd, *rs1, *rs2)]),
        Instr::Div(rd, rs1, rs2) => Ok(vec![enc_rrr(0x13, *rd, *rs1, *rs2)]),
        Instr::Udiv(rd, rs1, rs2) => Ok(vec![enc_rrr(0xa7, *rd, *rs1, *rs2)]),
        Instr::Srem(rd, rs1, rs2) => Ok(vec![enc_rrr(0xa8, *rd, *rs1, *rs2)]),
        Instr::Urem(rd, rs1, rs2) => Ok(vec![enc_rrr(0xa9, *rd, *rs1, *rs2)]),
        Instr::And(rd, rs1, rs2) => Ok(vec![enc_rrr(0x14, *rd, *rs1, *rs2)]),
        Instr::Andi(rd, rs1, imm) => Ok(vec![enc_mem(
            0xa1,
            *rd,
            *rs1,
            imm14(*imm, "ANDI immediate")?,
        )]),
        Instr::Or(rd, rs1, rs2) => Ok(vec![enc_rrr(0x15, *rd, *rs1, *rs2)]),
        Instr::Ori(rd, rs1, imm) => Ok(vec![enc_mem(
            0xa2,
            *rd,
            *rs1,
            imm14(*imm, "ORI immediate")?,
        )]),
        Instr::Xor(rd, rs1, rs2) => Ok(vec![enc_rrr(0x16, *rd, *rs1, *rs2)]),
        Instr::Xori(rd, rs1, imm) => Ok(vec![enc_mem(
            0xa3,
            *rd,
            *rs1,
            imm14(*imm, "XORI immediate")?,
        )]),
        Instr::Not(rd, rs1) => Ok(vec![enc_rrr(0x17, *rd, *rs1, Reg(0))]),
        Instr::Lsl(rd, rs1, rs2) => Ok(vec![enc_rrr(0x18, *rd, *rs1, *rs2)]),
        Instr::Lsli(rd, rs1, imm) => Ok(vec![enc_mem(
            0xa4,
            *rd,
            *rs1,
            imm14(*imm, "LSLI immediate")?,
        )]),
        Instr::Lsr(rd, rs1, rs2) => Ok(vec![enc_rrr(0x19, *rd, *rs1, *rs2)]),
        Instr::Lsri(rd, rs1, imm) => Ok(vec![enc_mem(
            0xa5,
            *rd,
            *rs1,
            imm14(*imm, "LSRI immediate")?,
        )]),
        Instr::Asr(rd, rs1, rs2) => Ok(vec![enc_rrr(0x1a, *rd, *rs1, *rs2)]),
        Instr::Asri(rd, rs1, imm) => Ok(vec![enc_mem(
            0xa6,
            *rd,
            *rs1,
            imm14(*imm, "ASRI immediate")?,
        )]),
        Instr::SextB(rd, rs1) => Ok(vec![enc_rrr(0xad, *rd, *rs1, Reg(0))]),
        Instr::SextH(rd, rs1) => Ok(vec![enc_rrr(0xae, *rd, *rs1, Reg(0))]),
        Instr::SextW(rd, rs1) => Ok(vec![enc_rrr(0xaf, *rd, *rs1, Reg(0))]),
        Instr::ZextB(rd, rs1) => Ok(vec![enc_rrr(0xb0, *rd, *rs1, Reg(0))]),
        Instr::ZextH(rd, rs1) => Ok(vec![enc_rrr(0xb1, *rd, *rs1, Reg(0))]),
        Instr::ZextW(rd, rs1) => Ok(vec![enc_rrr(0xb2, *rd, *rs1, Reg(0))]),
        Instr::Clz(rd, rs1) => Ok(vec![enc_rrr(0xb3, *rd, *rs1, Reg(0))]),
        Instr::Ctz(rd, rs1) => Ok(vec![enc_rrr(0xb4, *rd, *rs1, Reg(0))]),
        Instr::Popcnt(rd, rs1) => Ok(vec![enc_rrr(0xb5, *rd, *rs1, Reg(0))]),
        Instr::Rol(rd, rs1, rs2) => Ok(vec![enc_rrr(0xb6, *rd, *rs1, *rs2)]),
        Instr::Ror(rd, rs1, rs2) => Ok(vec![enc_rrr(0xb7, *rd, *rs1, *rs2)]),
        Instr::Bswap16(rd, rs1) => Ok(vec![enc_rrr(0xb8, *rd, *rs1, Reg(0))]),
        Instr::Bswap32(rd, rs1) => Ok(vec![enc_rrr(0xb9, *rd, *rs1, Reg(0))]),
        Instr::Bswap64(rd, rs1) => Ok(vec![enc_rrr(0xba, *rd, *rs1, Reg(0))]),
        Instr::Cmp(lhs, rhs) => Ok(vec![enc_rrr(0x1b, *lhs, *rhs, Reg(0))]),
        Instr::Ret => Ok(vec![enc_reg(0x1f, Reg(0))]),
        Instr::Jmp(target) => Ok(vec![enc_branch(
            0x20,
            branch_delta(program, word_pcs, pc, target)?,
        )]),
        Instr::Branch(condition, target) => Ok(vec![enc_branch(
            flat_exec_branch_opcode(*condition)?,
            branch_delta(program, word_pcs, pc, target)?,
        )]),
        Instr::Call(target) => Ok(vec![enc_branch(
            0x27,
            branch_delta(program, word_pcs, pc, target)?,
        )]),
        Instr::Ld(rd, MemRef::BaseOffset(base, offset), Width::Double) => Ok(vec![enc_mem(
            0x30,
            *rd,
            *base,
            imm14(*offset, "LD offset")?,
        )]),
        Instr::Ld(rd, MemRef::BaseOffset(base, offset), Width::Byte) => Ok(vec![enc_mem(
            0x32,
            *rd,
            *base,
            imm14(*offset, "LD.B offset")?,
        )]),
        Instr::St(MemRef::BaseOffset(base, offset), src, Width::Double) => Ok(vec![enc_mem(
            0x33,
            *src,
            *base,
            imm14(*offset, "ST offset")?,
        )]),
        Instr::St(MemRef::BaseOffset(base, offset), src, Width::Byte) => Ok(vec![enc_mem(
            0x35,
            *src,
            *base,
            imm14(*offset, "ST.B offset")?,
        )]),
        Instr::Alloc(rd, bytes) => Ok(vec![enc_rrr(0x47, *rd, *bytes, Reg(0))]),
        Instr::ErrnoGet(rd) => Ok(vec![enc_reg(0x38, *rd)]),
        Instr::ErrnoSet(src) => Ok(vec![enc_reg(0x39, *src)]),
        Instr::EnvGet(rd, key, index_or_buf, len_or_flags) => Ok(vec![enc_rrrr(
            0x56,
            *rd,
            *key,
            *index_or_buf,
            *len_or_flags,
        )]),
        Instr::Exit(src) => Ok(vec![enc_reg(0x3a, *src)]),
        other => Err(format!(
            "asm-flat-exec cannot encode {other:?}; supported subset is NOP, LI, MOV, ADD/ADDI, SUB, MUL, DIV, UDIV/UREM/SREM, AND/ANDI/OR/ORI/XOR/XORI/NOT, LSL/LSLI/LSR/LSRI/ASR/ASRI, SEXT/ZEXT, CLZ/CTZ/POPCNT, ROL/ROR, BSWAP, CMP, JMP/CALL/RET, signed conditional branch, LD/ST.D, LD/ST.B, ALLOC, ERRNO_GET/SET, ENV_GET, EXIT"
        )),
    }
}

fn encode_flat_exec_li(rd: Reg, value: &Value) -> Result<Vec<u32>, String> {
    let imm = value_imm32(value)?;
    if let Ok(small) = imm16(imm, "LI immediate") {
        Ok(vec![enc_ri(0x01, rd, small)])
    } else {
        Ok(vec![enc_reg(0x04, rd), imm as u32])
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

fn value_imm32(value: &Value) -> Result<i64, String> {
    match value {
        Value::Imm(imm) => imm32(*imm, "LI immediate"),
        Value::Label(label) => Err(format!(
            "asm-flat-exec does not yet materialize label immediate {label:?}"
        )),
    }
}

fn branch_delta(
    program: &Program,
    word_pcs: &[usize],
    pc: usize,
    target: &Target,
) -> Result<i64, String> {
    let target_pc = match target {
        Target::Address(address) => *address,
        Target::Label(label) => program
            .labels
            .get(label)
            .copied()
            .ok_or_else(|| format!("unknown branch label {label:?}"))?,
    };
    let target_word_pc = word_pcs
        .get(target_pc)
        .copied()
        .ok_or_else(|| format!("branch target out of range: {target_pc}"))?;
    let current_word_pc = word_pcs
        .get(pc)
        .copied()
        .ok_or_else(|| format!("branch source out of range: {pc}"))?;
    imm24(
        target_word_pc as i64 - current_word_pc as i64,
        "branch delta",
    )
}

fn imm16(value: i64, name: &str) -> Result<i64, String> {
    if !(-32768..=32767).contains(&value) {
        return Err(format!("{name} out of signed 16-bit range: {value}"));
    }
    Ok(value)
}

fn imm32(value: i64, name: &str) -> Result<i64, String> {
    if !(i64::from(i32::MIN)..=i64::from(u32::MAX)).contains(&value) {
        return Err(format!("{name} out of 32-bit literal range: {value}"));
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
    toy_bootstrap: bool,
}

fn take_cc_options(args: &mut Vec<String>) -> Result<CcOptions, String> {
    let mut inputs = Vec::new();
    let mut output = None;
    let mut dump_macros = false;
    let mut dump_preprocessed = false;
    let mut toy_bootstrap = false;
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
        } else if arg == "--toy-bootstrap" {
            toy_bootstrap = true;
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
        toy_bootstrap,
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

    #[test]
    fn asm_flat_exec_encodes_bitwise_subset() {
        let source = r#"
            .text
              LI r1, 10
              LI r2, 12
              AND r3, r1, r2
              XOR r4, r1, r2
              OR r5, r3, r4
              EXIT r5
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "0108000a\n",
                "0110000c\n",
                "14184400\n",
                "16204400\n",
                "1528c800\n",
                "3a280000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_shift_subset() {
        let source = r#"
            .text
              LI r1, 3
              LI r2, 1
              LSL r3, r1, r2
              LSR r4, r1, r2
              ADD r5, r3, r4
              EXIT r5
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080003\n",
                "01100001\n",
                "18184400\n",
                "19204400\n",
                "1028c800\n",
                "3a280000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_immediate_alu_subset() {
        let source = r#"
            .text
              LI r1, 8
              ADDI r2, r1, 5
              ANDI r3, r2, 15
              ORI r4, r3, 32
              XORI r5, r4, 7
              LSLI r6, r5, 1
              LSRI r7, r6, 2
              LI r8, -8
              ASRI r9, r8, 1
              ADDI r10, r9, 11
              ADD r11, r7, r10
              EXIT r11
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080008\n",
                "a0104005\n",
                "a118800f\n",
                "a220c020\n",
                "a3290007\n",
                "a4314001\n",
                "a5398002\n",
                "0140fff8\n",
                "a64a0001\n",
                "a052400b\n",
                "1059d400\n",
                "3a580000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_extend_subset() {
        let source = r#"
            .text
              LI r1, 255
              SEXT.B r2, r1
              ADDI r2, r2, 2
              ZEXT.B r3, r1
              LI r4, 65535
              SEXT.H r5, r4
              ADDI r5, r5, 3
              ZEXT.H r6, r4
              LI r7, 4294967295
              SEXT.W r8, r7
              ADDI r8, r8, 4
              ZEXT.W r9, r7
              ADD r10, r2, r5
              ADD r10, r10, r8
              ADDI r10, r10, 6
              EXIT r10
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "010800ff\n",
                "ad104000\n",
                "a0108002\n",
                "b0184000\n",
                "04200000\n",
                "0000ffff\n",
                "ae290000\n",
                "a0294003\n",
                "b1310000\n",
                "04380000\n",
                "ffffffff\n",
                "af41c000\n",
                "a0420004\n",
                "b249c000\n",
                "10508a00\n",
                "10529000\n",
                "a0528006\n",
                "3a500000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_count_rotate_bswap_subset() {
        let source = r#"
            .text
              LI r1, 16
              CLZ r2, r1
              ADDI r2, r2, -55
              CTZ r3, r1
              LI r4, 3855
              POPCNT r4, r4
              ADDI r4, r4, -5
              LI r5, 1
              LI r6, 8
              ROL r7, r5, r6
              ROR r8, r7, r6
              LI r9, 4660
              BSWAP16 r10, r9
              ANDI r10, r10, 15
              LI r11, 305419896
              BSWAP32 r11, r11
              LSRI r11, r11, 24
              ANDI r11, r11, 15
              LI r12, 255
              BSWAP64 r12, r12
              LSRI r12, r12, 56
              ANDI r12, r12, 15
              ADD r13, r2, r3
              ADD r13, r13, r4
              ADD r13, r13, r8
              ADD r13, r13, r10
              ADD r13, r13, r11
              ADD r13, r13, r12
              EXIT r13
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080010\n",
                "b3104000\n",
                "a010bfc9\n",
                "b4184000\n",
                "01200f0f\n",
                "b5210000\n",
                "a0213ffb\n",
                "01280001\n",
                "01300008\n",
                "b6394c00\n",
                "b741cc00\n",
                "01481234\n",
                "b8524000\n",
                "a152800f\n",
                "04580000\n",
                "12345678\n",
                "b95ac000\n",
                "a55ac018\n",
                "a15ac00f\n",
                "016000ff\n",
                "ba630000\n",
                "a5630038\n",
                "a163000f\n",
                "10688600\n",
                "106b4800\n",
                "106b5000\n",
                "106b5400\n",
                "106b5600\n",
                "106b5800\n",
                "3a680000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_unsigned_division_subset() {
        let source = r#"
            .text
              LI r1, 17
              LI r2, 5
              UDIV r3, r1, r2
              UREM r4, r1, r2
              ADD r5, r3, r4
              EXIT r5
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080011\n",
                "01100005\n",
                "a7184400\n",
                "a9204400\n",
                "1028c800\n",
                "3a280000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_signed_division_subset() {
        let source = r#"
            .text
              LI r1, 17
              LI r2, 5
              DIV r3, r1, r2
              SREM r4, r1, r2
              ADD r5, r3, r4
              EXIT r5
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080011\n",
                "01100005\n",
                "13184400\n",
                "a8204400\n",
                "1028c800\n",
                "3a280000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_not_subset() {
        let source = r#"
            .text
              LI r1, 7
              NOT r2, r1
              EXIT r2
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, concat!("01080007\n", "17104000\n", "3a100000\n",));
    }

    #[test]
    fn asm_flat_exec_encodes_wide_li_and_word_branch_subset() {
        let source = r#"
            .text
              LI r1, 4294967295
              JMP done
              LI r2, 1
            done:
              EXIT r1
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "04080000\n",
                "ffffffff\n",
                "20000002\n",
                "01100001\n",
                "3a080000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_call_return_subset() {
        let source = r#"
            .text
              LI r1, 5
              CALL add2
              EXIT r1
            add2:
              LI r2, 2
              ADD r1, r1, r2
              RET
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080005\n",
                "27000002\n",
                "3a080000\n",
                "01100002\n",
                "10084400\n",
                "1f000000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_alloc_and_byte_load_store_subset() {
        let source = r#"
            .text
              LI r1, 2
              ALLOC r2, r1
              LI r3, 65
              ST.B [r2, 0], r3
              LD.B r4, [r2, 0]
              EXIT r4
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080002\n",
                "47104000\n",
                "01180041\n",
                "35188000\n",
                "32208000\n",
                "3a200000\n",
            )
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
