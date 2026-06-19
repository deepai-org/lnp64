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
            let data_hex = encode_flat_exec_data_hex(&program)?;
            if let Some(output) = options.output {
                fs::write(&output, hex)
                    .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
            } else {
                print!("{hex}");
            }
            if let Some(output) = options.data_output {
                fs::write(&output, data_hex)
                    .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
            }
            Ok(())
        }
        "elf-flat-exec" => {
            let options = take_elf_flat_exec_options(&mut args)?;
            let (hex, data_hex) = encode_elf_flat_exec_images(&options.input)?;
            if let Some(output) = options.output {
                fs::write(&output, hex)
                    .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
            } else {
                print!("{hex}");
            }
            if let Some(output) = options.data_output {
                fs::write(&output, data_hex)
                    .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
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
            if !options.args.is_empty() {
                return Err(format!(
                    "unexpected elf-plan arguments: {}",
                    options.args.join(" ")
                ));
            }
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
            let namespace_root = take_run_namespace_root(&mut args)?;
            let options = take_elf_plan_options(&mut args)?;
            let mut probe = build_elf_exec_probe(&options)?;
            if let Some(root) = namespace_root {
                probe.machine.set_namespace_root(root)?;
            }
            if !options.args.is_empty() {
                probe.machine.set_args(&options.args)?;
            }
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
            let options = take_run_flat_exec_options(&mut args)?;
            let text = fs::read_to_string(&options.input)
                .map_err(|err| format!("failed to read {}: {err}", options.input.display()))?;
            let data = if let Some(input) = options.data_input {
                let data_hex = fs::read_to_string(&input)
                    .map_err(|err| format!("failed to read {}: {err}", input.display()))?;
                flat_data_hex_to_bytes(&data_hex)?
            } else {
                Vec::new()
            };
            let mut machine = build_flat_exec_machine(&text, &data)?;
            let exit = machine.run_committed_exec()?;
            let regs = machine.last_exit_registers().ok_or_else(|| {
                "flat exec finished without an exit register snapshot".to_string()
            })?;
            let r3 = regs.get(3).copied().unwrap_or_default();
            let r4 = regs.get(4).copied().unwrap_or_default();
            let r5 = regs.get(5).copied().unwrap_or_default();
            let env_page = regs.get(6).copied().unwrap_or_default();
            let mem0 = machine.last_exit_mem0().unwrap_or_default();
            let mem_checksum = machine.last_exit_mem_checksum().unwrap_or_default();
            let errno = machine.current_errno()?;
            let trace = machine
                .committed_exec_retire_trace()
                .iter()
                .map(|record| {
                    let pc_word = record.pc.saturating_sub(0x1000) / 4;
                    let opcode = record.opcode;
                    let tile_id = record.tile_id;
                    let pid = record.pid;
                    let tid = record.tid;
                    let domain_id = record.domain_id;
                    let domain_gen = record.domain_gen;
                    let action = record.action;
                    let operand_rd = record.operand_rd;
                    let operand_rs1 = record.operand_rs1;
                    let operand_rs2 = record.operand_rs2;
                    let operand_rs3 = record.operand_rs3;
                    let operand_imm = record.operand_imm;
                    let result_valid = record.result_valid;
                    let result_reg = record.result_reg;
                    let result_value = record.result_value;
                    let errno = record.errno;
                    let status = record.status;
                    let event_id = record.event_id;
                    let fault_id = record.fault_id;
                    format!(
                        "{{\"pc\":{pc_word},\"opcode\":{opcode},\"tile_id\":{tile_id},\"pid\":{pid},\"tid\":{tid},\"domain_id\":{domain_id},\"domain_gen\":{domain_gen},\"action\":{action},\"operand_rd\":{operand_rd},\"operand_rs1\":{operand_rs1},\"operand_rs2\":{operand_rs2},\"operand_rs3\":{operand_rs3},\"operand_imm\":{operand_imm},\"result_valid\":{result_valid},\"result_reg\":{result_reg},\"result_value\":{result_value},\"errno\":{errno},\"status\":{status},\"event_id\":{event_id},\"fault_id\":{fault_id}}}"
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            println!("EMULATOR_RETIRE [{trace}]");
            println!(
                "EMULATOR_FINAL {{\"exit\":{exit},\"r3\":{r3},\"r4\":{r4},\"r5\":{r5},\"env_page\":{env_page},\"mem0\":{mem0},\"mem_checksum\":{mem_checksum},\"errno\":{errno}}}"
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
    eprintln!("  lnp64 asm-flat-exec <program.s> [-o program.hex] [--data-hex data.hex]");
    eprintln!("  lnp64 elf-flat-exec <program.elf> [-o program.hex] [--data-hex data.hex]");
    eprintln!("  lnp64 run-flat-exec <program.hex> [--data-hex data.hex]");
    eprintln!("  lnp64 run [--namespace-root <dir>] <program.s>");
    eprintln!("  lnp64 elf-plan [--load-bias <n>] <program.elf>");
    eprintln!(
        "  lnp64 run-elf [--namespace-root <dir>] [--load-bias <n>] <program.elf> [argv ...]"
    );
    eprintln!("  lnp64 run-flat-exec <program.hex>");
    eprintln!(
        "  lnp64 cc --toy-bootstrap [--dump-macros|--dump-preprocessed] <program.c> [more.c ...] [-o program.s]"
    );
}

struct AsmFlatExecOptions {
    input: PathBuf,
    output: Option<PathBuf>,
    data_output: Option<PathBuf>,
}

struct RunFlatExecOptions {
    input: PathBuf,
    data_input: Option<PathBuf>,
}

struct ElfFlatExecOptions {
    input: PathBuf,
    output: Option<PathBuf>,
    data_output: Option<PathBuf>,
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

fn encode_flat_exec_data_hex(program: &Program) -> Result<String, String> {
    if program.data.len() > 4096 {
        return Err(format!(
            "flat exec data image is too large: {} bytes > 4096",
            program.data.len()
        ));
    }
    Ok(flat_data_bytes_to_hex(&program.data))
}

fn flat_data_bytes_to_hex(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(8) {
        let mut word = 0u64;
        for (idx, byte) in chunk.iter().enumerate() {
            word |= u64::from(*byte) << (idx * 8);
        }
        out.push_str(&format!("{word:016x}\n"));
    }
    out
}

fn flat_text_bytes_to_hex(text: &[u8]) -> Result<String, String> {
    if text.is_empty() {
        return Err("flat exec image is empty".to_string());
    }
    let mut padded = text.to_vec();
    while padded.len() % 4 != 0 {
        padded.push(0);
    }
    let mut out = String::new();
    for chunk in padded.chunks_exact(4) {
        let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        out.push_str(&format!("{word:08x}\n"));
    }
    Ok(out)
}

fn encode_elf_flat_exec_images(input: &PathBuf) -> Result<(String, String), String> {
    const TEXT_BASE: u64 = 0x1000;
    const DATA_BASE: u64 = isa::DATA_BASE;
    const PAGE_SIZE: u64 = 4096;

    let mut image =
        fs::read(input).map_err(|err| format!("failed to read {}: {err}", input.display()))?;
    let plan = loader::load_static_elf(&mut image, LoaderOptions::default())?;
    if plan.entry.entry_pc != TEXT_BASE {
        return Err(format!(
            "elf-flat-exec requires entry 0x{TEXT_BASE:x}, got 0x{:x}",
            plan.entry.entry_pc
        ));
    }
    let prepared = loader::materialize_vmas(&image, &plan)?;
    let mut text_bytes: Option<Vec<u8>> = None;
    let mut data_bytes = vec![0u8; PAGE_SIZE as usize];
    let mut data_high_water = 0usize;

    for vma in prepared {
        if vma.protection.execute {
            if vma.virtual_address != TEXT_BASE {
                return Err(format!(
                    "elf-flat-exec executable VMA must start at 0x{TEXT_BASE:x}, got 0x{:x}",
                    vma.virtual_address
                ));
            }
            if vma.bytes.len() > PAGE_SIZE as usize {
                return Err(format!(
                    "elf-flat-exec executable image is too large: {} bytes > {PAGE_SIZE}",
                    vma.bytes.len()
                ));
            }
            if text_bytes.replace(vma.bytes).is_some() {
                return Err("elf-flat-exec supports exactly one executable VMA".to_string());
            }
            continue;
        }

        let start = vma.virtual_address;
        let end = start
            .checked_add(vma.bytes.len() as u64)
            .ok_or_else(|| "elf-flat-exec data VMA range overflows".to_string())?;
        if start < DATA_BASE || end > DATA_BASE + PAGE_SIZE {
            return Err(format!(
                "elf-flat-exec non-executable VMA 0x{start:x}..0x{end:x} does not fit flat data page 0x{DATA_BASE:x}..0x{:x}",
                DATA_BASE + PAGE_SIZE
            ));
        }
        let offset = (start - DATA_BASE) as usize;
        let end_offset = offset + vma.bytes.len();
        data_bytes[offset..end_offset].copy_from_slice(&vma.bytes);
        data_high_water = data_high_water.max(end_offset);
    }

    let text = text_bytes.ok_or_else(|| "elf-flat-exec found no executable VMA".to_string())?;
    Ok((
        flat_text_bytes_to_hex(&text)?,
        flat_data_bytes_to_hex(&data_bytes[..data_high_water]),
    ))
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
        Instr::Li(_, Value::Label(_)) => 2,
        Instr::Auipc(_, _) => 2,
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
        Instr::Li(rd, value) => encode_flat_exec_li(program, *rd, value),
        Instr::Auipc(rd, value) => encode_flat_exec_auipc(*rd, value),
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
        Instr::Mulh(rd, rs1, rs2) => Ok(vec![enc_rrr(0xaa, *rd, *rs1, *rs2)]),
        Instr::Mulhu(rd, rs1, rs2) => Ok(vec![enc_rrr(0xab, *rd, *rs1, *rs2)]),
        Instr::Mulhsu(rd, rs1, rs2) => Ok(vec![enc_rrr(0xac, *rd, *rs1, *rs2)]),
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
        Instr::Cmpu(lhs, rhs) => Ok(vec![enc_rrr(0x1c, *lhs, *rhs, Reg(0))]),
        Instr::Cset(rd, condition) => Ok(vec![enc_reg(flat_exec_cset_opcode(*condition), *rd)]),
        Instr::Csel(rd, true_src, false_src, condition) => Ok(vec![enc_rrr(
            flat_exec_csel_opcode(*condition)?,
            *rd,
            *true_src,
            *false_src,
        )]),
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
        Instr::CallReg(target) => Ok(vec![enc_reg(0x28, *target)]),
        Instr::LrGet(dst) => Ok(vec![enc_reg(0x29, *dst)]),
        Instr::LrSet(src) => Ok(vec![enc_reg(0x2a, *src)]),
        Instr::Yield => Ok(vec![enc_reg(0x06, Reg(0))]),
        Instr::Ld(rd, MemRef::BaseOffset(base, offset), Width::Double) => Ok(vec![enc_mem(
            0x30,
            *rd,
            *base,
            imm14(*offset, "LD offset")?,
        )]),
        Instr::Ld(rd, MemRef::BaseOffset(base, offset), Width::Word) => Ok(vec![enc_mem(
            0x31,
            *rd,
            *base,
            imm14(*offset, "LD.W offset")?,
        )]),
        Instr::Ld(rd, MemRef::BaseOffset(base, offset), Width::Half) => Ok(vec![enc_mem(
            0x36,
            *rd,
            *base,
            imm14(*offset, "LD.H offset")?,
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
        Instr::St(MemRef::BaseOffset(base, offset), src, Width::Word) => Ok(vec![enc_mem(
            0x34,
            *src,
            *base,
            imm14(*offset, "ST.W offset")?,
        )]),
        Instr::St(MemRef::BaseOffset(base, offset), src, Width::Half) => Ok(vec![enc_mem(
            0x37,
            *src,
            *base,
            imm14(*offset, "ST.H offset")?,
        )]),
        Instr::St(MemRef::BaseOffset(base, offset), src, Width::Byte) => Ok(vec![enc_mem(
            0x35,
            *src,
            *base,
            imm14(*offset, "ST.B offset")?,
        )]),
        Instr::Alloc(rd, bytes) => Ok(vec![enc_rrr(0x47, *rd, *bytes, Reg(0))]),
        Instr::AllocSize(rd, ptr) => Ok(vec![enc_rrr(0x48, *rd, *ptr, Reg(0))]),
        Instr::Free(ptr) => Ok(vec![enc_reg(0x49, *ptr)]),
        Instr::AllocEx(rd, bytes, align) => Ok(vec![enc_rrr(0x4a, *rd, *bytes, *align)]),
        Instr::AmoSwap(dst, addr, value) => Ok(vec![enc_rrr(0xc5, *dst, *addr, *value)]),
        Instr::AmoAdd(dst, addr, value) => Ok(vec![enc_rrr(0xc6, *dst, *addr, *value)]),
        Instr::AmoAnd(dst, addr, value) => Ok(vec![enc_rrr(0xc7, *dst, *addr, *value)]),
        Instr::AmoOr(dst, addr, value) => Ok(vec![enc_rrr(0xc8, *dst, *addr, *value)]),
        Instr::LockCmpxchg(dst, addr, expected, new_value) => {
            Ok(vec![enc_rrrr(0xc9, *dst, *addr, *expected, *new_value)])
        }
        Instr::AmoXor(dst, addr, value) => Ok(vec![enc_rrr(0xca, *dst, *addr, *value)]),
        Instr::ObjectCtl(result, argblock) => Ok(vec![enc_rrr(0x4b, *result, *argblock, Reg(0))]),
        Instr::CapDup(result, argblock) => Ok(vec![enc_rrr(0x50, *result, *argblock, Reg(0))]),
        Instr::CapSend(result, argblock) => Ok(vec![enc_rrr(0x51, *result, *argblock, Reg(0))]),
        Instr::CapRecv(result, argblock) => Ok(vec![enc_rrr(0x52, *result, *argblock, Reg(0))]),
        Instr::CapRevoke(result, argblock) => Ok(vec![enc_rrr(0x53, *result, *argblock, Reg(0))]),
        Instr::ErrnoGet(rd) => Ok(vec![enc_reg(0x38, *rd)]),
        Instr::ErrnoSet(src) => Ok(vec![enc_reg(0x39, *src)]),
        Instr::DmaCtl(result, argblock) => Ok(vec![enc_rrr(0x5b, *result, *argblock, Reg(0))]),
        Instr::EnvGet(rd, key, index_or_buf, len_or_flags) => Ok(vec![enc_rrrr(
            0x56,
            *rd,
            *key,
            *index_or_buf,
            *len_or_flags,
        )]),
        Instr::WriteFd(fd, buf, len) => Ok(vec![enc_rrr(0x57, Reg(fd.0), *buf, *len)]),
        Instr::Fence => Ok(vec![enc_reg(0xcd, Reg(0))]),
        Instr::Isync(result, addr, len) => Ok(vec![enc_rrr(0xce, *result, *addr, *len)]),
        Instr::Exit(src) => Ok(vec![enc_reg(0x3a, *src)]),
        other => Err(format!(
            "asm-flat-exec cannot encode {other:?}; supported subset is NOP, LI, AUIPC, MOV, ADD/ADDI, SUB, MUL/MULH/MULHU/MULHSU, DIV, UDIV/UREM/SREM, AND/ANDI/OR/ORI/XORI/NOT, LSL/LSLI/LSR/LSRI/ASR/ASRI, SEXT/ZEXT, CLZ/CTZ/POPCNT, ROL/ROR, BSWAP, CMP/CMPU, CSET, CSEL, JMP/CALL/CALL_REG/LR_GET/LR_SET/RET, signed conditional branch, LD/ST.D, LD/ST.W, LD/ST.H, LD/ST.B, ALLOC/ALLOC_EX/ALLOC_SIZE/FREE, OBJECT_CTL, CAP_DUP/SEND/RECV/REVOKE, ERRNO_GET/SET, DMA_CTL, ENV_GET, WRITE_FD, FENCE/ISYNC, AMO, LOCK.CMPXCHG, EXIT"
        )),
    }
}

fn encode_flat_exec_li(program: &Program, rd: Reg, value: &Value) -> Result<Vec<u32>, String> {
    let imm = value_imm32(program, value)?;
    if let Ok(small) = imm16(imm, "LI immediate") {
        Ok(vec![enc_ri(0x01, rd, small)])
    } else {
        Ok(vec![enc_reg(0x04, rd), imm as u32])
    }
}

fn encode_flat_exec_auipc(rd: Reg, value: &Value) -> Result<Vec<u32>, String> {
    let imm = value_imm32_without_labels(value)?;
    Ok(vec![enc_reg(0xd0, rd), imm as u32])
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

fn flat_exec_csel_opcode(condition: Condition) -> Result<u8, String> {
    match condition {
        Condition::Eq => Ok(0xbb),
        Condition::Ne => Ok(0xbc),
        Condition::Lt => Ok(0xbd),
        Condition::Gt => Ok(0xbe),
        Condition::Le => Ok(0xbf),
        Condition::Ge => Ok(0xc0),
        Condition::Ult => Ok(0xc1),
        Condition::Ugt => Ok(0xc2),
        Condition::Ule => Ok(0xc3),
        Condition::Uge => Ok(0xc4),
    }
}

fn flat_exec_cset_opcode(condition: Condition) -> u8 {
    match condition {
        Condition::Eq => 0x3d,
        Condition::Ne => 0x3e,
        Condition::Lt => 0x3f,
        Condition::Gt => 0x40,
        Condition::Le => 0x41,
        Condition::Ge => 0x42,
        Condition::Ult => 0x43,
        Condition::Ugt => 0x44,
        Condition::Ule => 0x45,
        Condition::Uge => 0x46,
    }
}

fn value_imm32(program: &Program, value: &Value) -> Result<i64, String> {
    match value {
        Value::Imm(imm) => imm32(*imm, "LI immediate"),
        Value::Label(label) => program
            .data_labels
            .get(label)
            .copied()
            .or_else(|| program.labels.get(label).map(|pc| *pc as u64))
            .ok_or_else(|| format!("unknown label immediate {label:?}"))
            .and_then(|value| imm32(value as i64, "LI label immediate")),
    }
}

fn value_imm32_without_labels(value: &Value) -> Result<i64, String> {
    match value {
        Value::Imm(imm) => imm32(*imm, "immediate"),
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

fn build_flat_exec_machine(hex_words: &str, data: &[u8]) -> Result<Machine, String> {
    const ZERO_BASE: u64 = 0;
    const DATA_BASE: u64 = isa::DATA_BASE;
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
    if data.len() > PAGE_SIZE {
        return Err(format!(
            "flat exec data image is too large: {} bytes > {PAGE_SIZE}",
            data.len()
        ));
    }
    let zero_page = vec![0u8; PAGE_SIZE];
    let mut text_page = vec![0u8; PAGE_SIZE];
    text_page[..text.len()].copy_from_slice(&text);
    let mut data_page = vec![0u8; PAGE_SIZE];
    data_page[..data.len()].copy_from_slice(data);

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
                virtual_address: ZERO_BASE,
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
            virtual_address: ZERO_BASE,
            protection: PROT_READ | PROT_WRITE,
            bytes: zero_page,
        },
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

fn flat_data_hex_to_bytes(hex_words: &str) -> Result<Vec<u8>, String> {
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
        let word = u64::from_str_radix(word_text, 16)
            .map_err(|err| format!("invalid data hex word on line {}: {err}", idx + 1))?;
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    while bytes.last().copied() == Some(0) {
        bytes.pop();
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
    args: Vec<String>,
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
    let runtime_args = std::mem::take(args);
    Ok(ElfPlanOptions {
        input,
        load_bias,
        args: runtime_args,
    })
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
    let mut data_output = None;
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
        } else if arg == "--data-hex" {
            if data_output.is_some() {
                return Err("duplicate --data-hex".to_string());
            }
            if args.is_empty() {
                return Err("--data-hex requires a path".to_string());
            }
            data_output = Some(PathBuf::from(args.remove(0)));
        } else if arg.starts_with('-') {
            return Err(format!("unexpected asm-flat-exec option {arg:?}"));
        } else if input.is_some() {
            return Err(format!("unexpected asm-flat-exec argument {arg:?}"));
        } else {
            input = Some(PathBuf::from(arg));
        }
    }
    let input = input.ok_or_else(|| "missing input path".to_string())?;
    Ok(AsmFlatExecOptions {
        input,
        output,
        data_output,
    })
}

fn take_elf_flat_exec_options(args: &mut Vec<String>) -> Result<ElfFlatExecOptions, String> {
    let mut input = None;
    let mut output = None;
    let mut data_output = None;
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
        } else if arg == "--data-hex" {
            if data_output.is_some() {
                return Err("duplicate --data-hex".to_string());
            }
            if args.is_empty() {
                return Err("--data-hex requires a path".to_string());
            }
            data_output = Some(PathBuf::from(args.remove(0)));
        } else if arg.starts_with('-') {
            return Err(format!("unexpected elf-flat-exec option {arg:?}"));
        } else if input.is_some() {
            return Err(format!("unexpected elf-flat-exec argument {arg:?}"));
        } else {
            input = Some(PathBuf::from(arg));
        }
    }
    let input = input.ok_or_else(|| "missing input path".to_string())?;
    Ok(ElfFlatExecOptions {
        input,
        output,
        data_output,
    })
}

fn take_run_flat_exec_options(args: &mut Vec<String>) -> Result<RunFlatExecOptions, String> {
    let mut input = None;
    let mut data_input = None;
    while !args.is_empty() {
        let arg = args.remove(0);
        if arg == "--data-hex" {
            if data_input.is_some() {
                return Err("duplicate --data-hex".to_string());
            }
            if args.is_empty() {
                return Err("--data-hex requires a path".to_string());
            }
            data_input = Some(PathBuf::from(args.remove(0)));
        } else if arg.starts_with('-') {
            return Err(format!("unexpected run-flat-exec option {arg:?}"));
        } else if input.is_some() {
            return Err(format!("unexpected run-flat-exec argument {arg:?}"));
        } else {
            input = Some(PathBuf::from(arg));
        }
    }
    let input = input.ok_or_else(|| "missing input path".to_string())?;
    Ok(RunFlatExecOptions { input, data_input })
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
            args: Vec::new(),
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
            args: Vec::new(),
        })
        .unwrap();
        let exit = probe.machine.run_committed_exec().unwrap();

        assert_eq!(exit, 0);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn elf_flat_exec_exports_flat_compatible_static_elf() {
        let path = std::env::temp_dir().join(format!(
            "lnp64-elf-flat-exec-probe-{}.elf",
            std::process::id()
        ));
        fs::write(&path, minimal_static_exit_elf_at(0x1000)).unwrap();

        let (hex, data_hex) = encode_elf_flat_exec_images(&path).unwrap();

        assert_eq!(
            hex,
            concat!("3a000000\n", "00000000\n", "00000000\n", "00000000\n",)
        );
        assert_eq!(data_hex, "");

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
    fn asm_flat_exec_encodes_yield() {
        let source = r#"
            .text
              YIELD
              LI r1, 0
              EXIT r1
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, concat!("06000000\n", "01080000\n", "3a080000\n",));
    }

    #[test]
    fn asm_flat_exec_encodes_data_labels_and_data_hex() {
        let source = r#"
            .data
            msg: .string "ok\n"
            buf: .zero 1
            .text
              LI r1, msg
              LI r2, buf
              WRITE_FD fd1, r1, r2
              EXIT r0
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();
        let data_hex = encode_flat_exec_data_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "04080000\n",
                "00010000\n",
                "04100000\n",
                "00010004\n",
                "57084400\n",
                "3a000000\n",
            )
        );
        assert_eq!(data_hex, "00000000000a6b6f\n");
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
    fn asm_flat_exec_encodes_cmpu_csel_subset() {
        let source = r#"
            .text
              LI r1, 5
              LI r2, 9
              LI r3, 1
              LI r4, 2
              LI r5, 4
              LI r6, 8
              CMP r1, r2
              CSEL.LT r7, r3, r4
              CSEL.GT r8, r3, r4
              CSEL.LE r9, r5, r6
              CSEL.GE r10, r5, r6
              LI r11, 16
              LI r12, 32
              CMP r1, r1
              CSEL.EQ r13, r11, r12
              CSEL.NE r14, r11, r12
              LI r15, -1
              LI r16, 1
              CMPU r15, r16
              LI r17, 64
              LI r18, 128
              LI r19, 256
              LI r21, 512
              CSEL.ULT r22, r17, r18
              CSEL.UGT r23, r17, r18
              CSEL.ULE r24, r19, r21
              CSEL.UGE r25, r19, r21
              ADD r26, r7, r8
              ADD r26, r26, r9
              ADD r26, r26, r10
              ADD r26, r26, r13
              ADD r26, r26, r14
              ADD r26, r26, r22
              ADD r26, r26, r23
              ADD r26, r26, r24
              ADD r26, r26, r25
              EXIT r26
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080005\n",
                "01100009\n",
                "01180001\n",
                "01200002\n",
                "01280004\n",
                "01300008\n",
                "1b088000\n",
                "bd38c800\n",
                "be40c800\n",
                "bf494c00\n",
                "c0514c00\n",
                "01580010\n",
                "01600020\n",
                "1b084000\n",
                "bb6ad800\n",
                "bc72d800\n",
                "0178ffff\n",
                "01800001\n",
                "1c7c0000\n",
                "01880040\n",
                "01900080\n",
                "01980100\n",
                "01a80200\n",
                "c1b46400\n",
                "c2bc6400\n",
                "c3c4ea00\n",
                "c4ccea00\n",
                "10d1d000\n",
                "10d69200\n",
                "10d69400\n",
                "10d69a00\n",
                "10d69c00\n",
                "10d6ac00\n",
                "10d6ae00\n",
                "10d6b000\n",
                "10d6b200\n",
                "3ad00000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_cset_subset() {
        let source = r#"
            .text
              LI r1, 5
              LI r2, 9
              CMP r1, r2
              CSET.LT r3
              CSET.GT r4
              CSET.LE r5
              CSET.GE r6
              CMP r1, r1
              CSET.EQ r7
              CSET.NE r8
              LI r15, -1
              LI r16, 1
              CMPU r15, r16
              CSET.ULT r9
              CSET.UGT r10
              CSET.ULE r11
              CSET.UGE r12
              ADD r13, r3, r4
              ADD r13, r13, r5
              ADD r13, r13, r6
              ADD r13, r13, r7
              ADD r13, r13, r8
              ADD r13, r13, r9
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
                "01080005\n",
                "01100009\n",
                "1b088000\n",
                "3f180000\n",
                "40200000\n",
                "41280000\n",
                "42300000\n",
                "1b084000\n",
                "3d380000\n",
                "3e400000\n",
                "0178ffff\n",
                "01800001\n",
                "1c7c0000\n",
                "43480000\n",
                "44500000\n",
                "45580000\n",
                "46600000\n",
                "1068c800\n",
                "106b4a00\n",
                "106b4c00\n",
                "106b4e00\n",
                "106b5000\n",
                "106b5200\n",
                "106b5400\n",
                "106b5600\n",
                "106b5800\n",
                "3a680000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_high_multiply_subset() {
        let source = r#"
            .text
              LI r1, 1
              LSLI r1, r1, 32
              MOV r2, r1
              MULH r3, r1, r2
              MULHU r4, r1, r2
              LI r5, -1
              LSLI r5, r5, 32
              MULH r6, r5, r1
              MULHSU r7, r5, r1
              MULHU r8, r5, r1
              LI r9, 1
              LI r10, -1
              LI r11, 0xffffffff
              LI r12, 0
              CMP r3, r9
              CSEL.EQ r13, r9, r12
              CMP r4, r9
              CSEL.EQ r14, r9, r12
              ADD r13, r13, r14
              CMP r6, r10
              CSEL.EQ r15, r9, r12
              ADD r13, r13, r15
              CMP r7, r10
              CSEL.EQ r16, r9, r12
              ADD r13, r13, r16
              CMP r8, r11
              CSEL.EQ r17, r9, r12
              ADD r13, r13, r17
              EXIT r13
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080001\n",
                "a4084020\n",
                "02104000\n",
                "aa184400\n",
                "ab204400\n",
                "0128ffff\n",
                "a4294020\n",
                "aa314200\n",
                "ac394200\n",
                "ab414200\n",
                "01480001\n",
                "0150ffff\n",
                "04580000\n",
                "ffffffff\n",
                "01600000\n",
                "1b1a4000\n",
                "bb6a5800\n",
                "1b224000\n",
                "bb725800\n",
                "106b5c00\n",
                "1b328000\n",
                "bb7a5800\n",
                "106b5e00\n",
                "1b3a8000\n",
                "bb825800\n",
                "106b6000\n",
                "1b42c000\n",
                "bb8a5800\n",
                "106b6200\n",
                "3a680000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_auipc_fence_subset() {
        let source = r#"
            .text
              AUIPC r3, 0
              FENCE.SC
              AUIPC r4, 8
              LI r1, 1
              LI r2, 0
              LI r5, 4096
              CMP r3, r5
              CSEL.EQ r6, r1, r2
              LI r7, 4116
              CMP r4, r7
              CSEL.EQ r8, r1, r2
              ADD r9, r6, r8
              EXIT r9
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "d0180000\n",
                "00000000\n",
                "cd000000\n",
                "d0200000\n",
                "00000008\n",
                "01080001\n",
                "01100000\n",
                "01281000\n",
                "1b194000\n",
                "bb304400\n",
                "01381014\n",
                "1b21c000\n",
                "bb404400\n",
                "10499000\n",
                "3a480000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_half_word_load_store_subset() {
        let source = r#"
            .text
              LI r1, 0
              LI r2, 0x12345678
              ST.W [r1, 0], r2
              LD.W r3, [r1, 0]
              LI r4, 0xabcd
              ST.H [r1, 4], r4
              LD.H r5, [r1, 4]
              LI r6, 0x55aa
              ST.H [r1, 6], r6
              LD.W r7, [r1, 4]
              LI r8, 0x12345678
              LI r9, 0xabcd
              LI r10, 0x55aaabcd
              LI r11, 1
              LI r12, 0
              CMP r3, r8
              CSEL.EQ r13, r11, r12
              CMP r5, r9
              CSEL.EQ r14, r11, r12
              ADD r13, r13, r14
              CMP r7, r10
              CSEL.EQ r15, r11, r12
              ADD r13, r13, r15
              EXIT r13
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080000\n",
                "04100000\n",
                "12345678\n",
                "34104000\n",
                "31184000\n",
                "04200000\n",
                "0000abcd\n",
                "37204004\n",
                "36284004\n",
                "013055aa\n",
                "37304006\n",
                "31384004\n",
                "04400000\n",
                "12345678\n",
                "04480000\n",
                "0000abcd\n",
                "04500000\n",
                "55aaabcd\n",
                "01580001\n",
                "01600000\n",
                "1b1a0000\n",
                "bb6ad800\n",
                "1b2a4000\n",
                "bb72d800\n",
                "106b5c00\n",
                "1b3a8000\n",
                "bb7ad800\n",
                "106b5e00\n",
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
              LR_GET r3
              LR_SET r3
              CALL_REG r3
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
                "29180000\n",
                "2a180000\n",
                "28180000\n",
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
              ALLOC_SIZE r5, r2
              LI r3, 65
              ST.B [r2, 0], r3
              LD.B r4, [r2, 0]
              FREE r2
              LI r6, 16
              ALLOC_EX r7, r6, r1
              EXIT r4
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080002\n",
                "47104000\n",
                "48288000\n",
                "01180041\n",
                "35188000\n",
                "32208000\n",
                "49100000\n",
                "01300010\n",
                "4a398200\n",
                "3a200000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_memory_order_subset() {
        let source = r#"
            .text
              LI r1, 0
              LI r2, 8
              ISYNC r3, r1, r2
              LI r4, 41
              LI r5, 42
              LOCK.CMPXCHG r6, r1, r4, r5
              EXIT r6
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "01080000\n",
                "01100008\n",
                "ce184400\n",
                "01200029\n",
                "0128002a\n",
                "c9304850\n",
                "3a300000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_amo_subset() {
        let source = r#"
            .text
              AMO.SWAP r4, r1, r2
              AMO.ADD r5, r1, r3
              AMO.AND r6, r1, r4
              AMO.OR r7, r1, r5
              AMO.XOR r8, r1, r6
              EXIT r8
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "c5204400\n",
                "c6284600\n",
                "c7304800\n",
                "c8384a00\n",
                "ca404c00\n",
                "3a400000\n",
            )
        );
    }

    #[test]
    fn asm_flat_exec_encodes_dma_ctl_subset() {
        let source = r#"
            .text
              OBJECT_CTL r4, r10
              CAP_DUP r5, r10
              CAP_SEND r6, r10
              CAP_RECV r7, r10
              CAP_REVOKE r6, r10
              DMA_CTL r7, r10
              EXIT r7
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(
            hex,
            concat!(
                "4b228000\n",
                "502a8000\n",
                "51328000\n",
                "523a8000\n",
                "53328000\n",
                "5b3a8000\n",
                "3a380000\n",
            )
        );
    }

    fn minimal_static_elf() -> Vec<u8> {
        minimal_static_elf_at(0x400000)
    }

    fn minimal_static_elf_at(vaddr: u64) -> Vec<u8> {
        let mut image = vec![0; 0x200];
        image[0..4].copy_from_slice(b"\x7fELF");
        image[4] = ELFCLASS64;
        image[5] = ELFDATA2LSB;
        image[6] = EV_CURRENT;
        put_u16(&mut image, 16, ET_EXEC);
        put_u16(&mut image, 18, EM_LNP64);
        put_u32(&mut image, 20, u32::from(EV_CURRENT));
        put_u64(&mut image, 24, vaddr);
        put_u64(&mut image, 32, ELF64_EHDR_SIZE as u64);
        put_u16(&mut image, 52, ELF64_EHDR_SIZE as u16);
        put_u16(&mut image, 54, ELF64_PHDR_SIZE as u16);
        put_u16(&mut image, 56, 1);

        let phdr = ELF64_EHDR_SIZE;
        put_u32(&mut image, phdr, PT_LOAD);
        put_u32(&mut image, phdr + 4, PF_R | PF_X);
        put_u64(&mut image, phdr + 8, 0x100);
        put_u64(&mut image, phdr + 16, vaddr);
        put_u64(&mut image, phdr + 32, 16);
        put_u64(&mut image, phdr + 40, 16);
        put_u64(&mut image, phdr + 48, 4096);
        image[0x100..0x110].fill(0xcc);
        image
    }

    fn minimal_static_exit_elf() -> Vec<u8> {
        minimal_static_exit_elf_at(0x400000)
    }

    fn minimal_static_exit_elf_at(vaddr: u64) -> Vec<u8> {
        let mut image = minimal_static_elf_at(vaddr);
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
