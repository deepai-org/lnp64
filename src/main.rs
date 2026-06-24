mod asm;
mod emulator;
mod isa;
mod loader;
mod lowering;
mod native;
mod personality_lowering;

use std::env;
use std::fs;
use std::path::PathBuf;

use asm::Program;
use emulator::{Machine, PreparedExecVma};
use isa::{Instr, MemRef, Pcr, Reg, SelCond, Target, Value, Width};
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
            machine.set_record_retire_trace(true);
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
            let regs_json = regs
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",");
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
                "EMULATOR_FINAL {{\"exit\":{exit},\"regs\":[{regs_json}],\"r3\":{r3},\"r4\":{r4},\"r5\":{r5},\"env_page\":{env_page},\"mem0\":{mem0},\"mem_checksum\":{mem_checksum},\"errno\":{errno}}}"
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
            out.push_str(&format!("{word:016x}\n"));
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
    while padded.len() % 8 != 0 {
        padded.push(0);
    }
    let mut out = String::new();
    for chunk in padded.chunks_exact(8) {
        let word = u64::from_le_bytes(chunk.try_into().unwrap());
        out.push_str(&format!("{word:016x}\n"));
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
        pc += flat_exec_instr_word_len(program, instr);
    }
    pcs
}

fn flat_exec_instr_word_len(_program: &Program, instr: &Instr) -> usize {
    match instr {
        // `ld rd, label` is an assembler pseudo: `li rd, label` then `ld rd, (rd)`.
        Instr::Ld(_, MemRef::Label(_), _) | Instr::LdS(_, MemRef::Label(_), _) => 2,
        _ => 1,
    }
}

fn encode_flat_exec_instr(
    program: &Program,
    word_pcs: &[usize],
    pc: usize,
    instr: &Instr,
) -> Result<Vec<u64>, String> {
    match instr {
        Instr::Nop => Ok(vec![enc_reg(0x00, Reg(0))]),
        Instr::Li(rd, value) => encode_flat_exec_li(program, word_pcs, *rd, value),
        Instr::Liu(rd, rs1, imm) => Ok(vec![enc_i(0x04, *rd, *rs1, imm32(*imm, "LIU immediate")?)]),
        Instr::Auipc(rd, value) => encode_flat_exec_auipc(*rd, value),
        Instr::Mov(rd, rs1) => Ok(vec![enc_rrr(0x02, *rd, *rs1, Reg(0))]),
        Instr::Add(rd, rs1, rs2) => Ok(vec![enc_rrr(0x10, *rd, *rs1, *rs2)]),
        Instr::Addi(rd, rs1, imm) => {
            Ok(vec![enc_i(0xa0, *rd, *rs1, imm32(*imm, "ADDI immediate")?)])
        }
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
        Instr::Andi(rd, rs1, imm) => {
            Ok(vec![enc_i(0xa1, *rd, *rs1, imm32(*imm, "ANDI immediate")?)])
        }
        Instr::Or(rd, rs1, rs2) => Ok(vec![enc_rrr(0x15, *rd, *rs1, *rs2)]),
        Instr::Ori(rd, rs1, imm) => {
            Ok(vec![enc_i(0xa2, *rd, *rs1, imm32(*imm, "ORI immediate")?)])
        }
        Instr::Xor(rd, rs1, rs2) => Ok(vec![enc_rrr(0x16, *rd, *rs1, *rs2)]),
        Instr::Xori(rd, rs1, imm) => {
            Ok(vec![enc_i(0xa3, *rd, *rs1, imm32(*imm, "XORI immediate")?)])
        }
        Instr::Not(rd, rs1) => Ok(vec![enc_rrr(0x17, *rd, *rs1, Reg(0))]),
        Instr::Lsl(rd, rs1, rs2) => Ok(vec![enc_rrr(0x18, *rd, *rs1, *rs2)]),
        Instr::Lsli(rd, rs1, imm) => {
            Ok(vec![enc_i(0xa4, *rd, *rs1, imm32(*imm, "SLLI immediate")?)])
        }
        Instr::Lsr(rd, rs1, rs2) => Ok(vec![enc_rrr(0x19, *rd, *rs1, *rs2)]),
        Instr::Lsri(rd, rs1, imm) => {
            Ok(vec![enc_i(0xa5, *rd, *rs1, imm32(*imm, "SRLI immediate")?)])
        }
        Instr::Asr(rd, rs1, rs2) => Ok(vec![enc_rrr(0x1a, *rd, *rs1, *rs2)]),
        Instr::Asri(rd, rs1, imm) => {
            Ok(vec![enc_i(0xa6, *rd, *rs1, imm32(*imm, "SRAI immediate")?)])
        }
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
        Instr::Slt(rd, rs1, rs2) => Ok(vec![enc_rrr(0x1b, *rd, *rs1, *rs2)]),
        Instr::Sltu(rd, rs1, rs2) => Ok(vec![enc_rrr(0x1c, *rd, *rs1, *rs2)]),
        Instr::Slti(rd, rs1, imm) => {
            Ok(vec![enc_i(0x1d, *rd, *rs1, imm32(*imm, "SLTI immediate")?)])
        }
        Instr::Sltiu(rd, rs1, imm) => {
            Ok(vec![enc_i(0x1e, *rd, *rs1, imm32(*imm, "SLTIU immediate")?)])
        }
        Instr::Jmp(target) => Ok(vec![enc_j(0x20, Reg(0), branch_delta(program, word_pcs, pc, target)?)]),
        Instr::Jal(rd, target) => {
            Ok(vec![enc_j(0x27, *rd, branch_delta(program, word_pcs, pc, target)?)])
        }
        Instr::Jalr(rd, rs1, imm) => Ok(vec![enc_i(0x28, *rd, *rs1, imm32(*imm, "JALR offset")?)]),
        Instr::Beq(rs1, rs2, target) => {
            Ok(vec![enc_b(0x21, *rs1, *rs2, branch_delta(program, word_pcs, pc, target)?)])
        }
        Instr::Bne(rs1, rs2, target) => {
            Ok(vec![enc_b(0x22, *rs1, *rs2, branch_delta(program, word_pcs, pc, target)?)])
        }
        Instr::Blt(rs1, rs2, target) => {
            Ok(vec![enc_b(0x23, *rs1, *rs2, branch_delta(program, word_pcs, pc, target)?)])
        }
        Instr::Bge(rs1, rs2, target) => {
            Ok(vec![enc_b(0x24, *rs1, *rs2, branch_delta(program, word_pcs, pc, target)?)])
        }
        Instr::Bltu(rs1, rs2, target) => {
            Ok(vec![enc_b(0x25, *rs1, *rs2, branch_delta(program, word_pcs, pc, target)?)])
        }
        Instr::Bgeu(rs1, rs2, target) => {
            Ok(vec![enc_b(0x26, *rs1, *rs2, branch_delta(program, word_pcs, pc, target)?)])
        }
        Instr::Sel(cc, rd, ra, rb, rt, rf) => {
            // rd[55:51] ra=rs1[50:46] rb=rs2[45:41] rt=rs3[40:36] rf=rs4[35:31];
            // one opcode per condition (0x40-0x45), mirroring the branch family.
            let op = match cc {
                SelCond::Eq => 0x40,
                SelCond::Ne => 0x41,
                SelCond::Lt => 0x42,
                SelCond::Ge => 0x43,
                SelCond::Ltu => 0x44,
                SelCond::Geu => 0x45,
            };
            Ok(vec![enc_rrrrr(op, *rd, *ra, *rb, *rt, *rf)])
        }
        Instr::LrD(rd, rs1) => Ok(vec![enc_rrr(0xc5, *rd, *rs1, Reg(0))]),
        Instr::ScD(rd, rs2, rs1) => Ok(vec![enc_rrr(0xc6, *rd, *rs1, *rs2)]),
        Instr::Yield => Ok(vec![enc_reg(0x06, Reg(0))]),
        Instr::Sleep(ticks) => Ok(vec![enc_reg(0x07, *ticks)]),
        Instr::Ld(rd, MemRef::BaseOffset(base, offset), Width::Double) => {
            Ok(vec![enc_i(0x30, *rd, *base, imm32(*offset, "LD offset")?)])
        }
        Instr::Ld(rd, MemRef::BaseOffset(base, offset), Width::Word) => {
            Ok(vec![enc_i(0x31, *rd, *base, imm32(*offset, "LWU offset")?)])
        }
        Instr::Ld(rd, MemRef::BaseOffset(base, offset), Width::Half) => {
            Ok(vec![enc_i(0x36, *rd, *base, imm32(*offset, "LHU offset")?)])
        }
        Instr::Ld(rd, MemRef::BaseOffset(base, offset), Width::Byte) => {
            Ok(vec![enc_i(0x32, *rd, *base, imm32(*offset, "LBU offset")?)])
        }
        Instr::LdS(rd, MemRef::BaseOffset(base, offset), Width::Word) => {
            Ok(vec![enc_i(0x05, *rd, *base, imm32(*offset, "LW offset")?)])
        }
        Instr::LdS(rd, MemRef::BaseOffset(base, offset), Width::Byte) => {
            Ok(vec![enc_i(0x08, *rd, *base, imm32(*offset, "LB offset")?)])
        }
        Instr::LdS(rd, MemRef::BaseOffset(base, offset), Width::Half) => {
            Ok(vec![enc_i(0x09, *rd, *base, imm32(*offset, "LH offset")?)])
        }
        Instr::LdS(_, _, Width::Double) => {
            Err("LD.D has no sign-extending form".to_string())
        }
        Instr::Ld(rd, MemRef::Label(label), width) | Instr::LdS(rd, MemRef::Label(label), width) => {
            let mut words =
                encode_flat_exec_li(program, word_pcs, *rd, &Value::Label(label.clone()))?;
            let signed = matches!(instr, Instr::LdS(..));
            let opcode = match (width, signed) {
                (Width::Double, _) => 0x30,
                (Width::Word, false) => 0x31,
                (Width::Half, false) => 0x36,
                (Width::Byte, false) => 0x32,
                (Width::Word, true) => 0x05,
                (Width::Byte, true) => 0x08,
                (Width::Half, true) => 0x09,
            };
            words.push(enc_i(opcode, *rd, *rd, 0));
            Ok(words)
        }
        Instr::St(MemRef::BaseOffset(base, offset), src, Width::Double) => {
            Ok(vec![enc_s(0x33, *base, *src, imm32(*offset, "SD offset")?)])
        }
        Instr::St(MemRef::BaseOffset(base, offset), src, Width::Word) => {
            Ok(vec![enc_s(0x34, *base, *src, imm32(*offset, "SW offset")?)])
        }
        Instr::St(MemRef::BaseOffset(base, offset), src, Width::Half) => {
            Ok(vec![enc_s(0x37, *base, *src, imm32(*offset, "SH offset")?)])
        }
        Instr::St(MemRef::BaseOffset(base, offset), src, Width::Byte) => {
            Ok(vec![enc_s(0x35, *base, *src, imm32(*offset, "SB offset")?)])
        }
        Instr::Alloc(rd, bytes) => Ok(vec![enc_rrr(0x47, *rd, *bytes, Reg(0))]),
        Instr::AllocSize(rd, ptr) => Ok(vec![enc_rrr(0x48, *rd, *ptr, Reg(0))]),
        Instr::Free(ptr) => Ok(vec![enc_reg(0x49, *ptr)]),
        Instr::AllocEx(rd, bytes, align) => Ok(vec![enc_rrr(0x4a, *rd, *bytes, *align)]),
        Instr::ObjectCtl(result, argblock) => Ok(vec![enc_rrr(0x4b, *result, *argblock, Reg(0))]),
        Instr::CapDup(result, argblock) => Ok(vec![enc_rrr(0x50, *result, *argblock, Reg(0))]),
        Instr::CapSend(result, argblock) => Ok(vec![enc_rrr(0x51, *result, *argblock, Reg(0))]),
        Instr::CapRecv(result, argblock) => Ok(vec![enc_rrr(0x52, *result, *argblock, Reg(0))]),
        Instr::CapRevoke(result, argblock) => Ok(vec![enc_rrr(0x53, *result, *argblock, Reg(0))]),
        Instr::DomainCtl(result, argblock) => Ok(vec![enc_rrr(0x4c, *result, *argblock, Reg(0))]),
        Instr::ErrnoGet(rd) => Ok(vec![enc_reg(0x38, *rd)]),
        Instr::ErrnoSet(src) => Ok(vec![enc_reg(0x39, *src)]),
        Instr::GetPcr(dst, pcr) => Ok(vec![enc_rrr(0x54, *dst, Reg(pcr_selector(*pcr)?), Reg(0))]),
        Instr::SetPcr(result, pcr, src) => {
            Ok(vec![enc_rrr(0x55, *result, Reg(pcr_selector(*pcr)?), *src)])
        }
        Instr::DmaCtl(result, argblock) => Ok(vec![enc_rrr(0x5b, *result, *argblock, Reg(0))]),
        Instr::EnvGet(rd, key, index_or_buf, len_or_flags) => Ok(vec![enc_rrrr(
            0x56,
            *rd,
            *key,
            *index_or_buf,
            *len_or_flags,
        )]),
        Instr::Mmap(dst, hint, len, prot, fd, offset) => {
            // v2: single 64-bit word; fd→rs4, offset→rs5.
            Ok(vec![slots(0x6a, *dst, *hint, *len, *prot, Reg(fd.0), *offset)])
        }
        Instr::Mprotect(addr, len, prot) => Ok(vec![enc_rrr(0x6c, *addr, *len, *prot)]),
        Instr::Sigaction(signum, handler) => Ok(vec![enc_rrr(0x62, *signum, *handler, Reg(0))]),
        Instr::Kill(pid, signum) => Ok(vec![enc_rrr(0x64, *pid, *signum, Reg(0))]),
        Instr::Sigret => Ok(vec![enc_reg(0x65, Reg(0))]),
        Instr::Inb(dst, port) => Ok(vec![enc_rrr(0x80, *dst, *port, Reg(0))]),
        Instr::Outb(port, src) => Ok(vec![enc_rrr(0x81, *port, *src, Reg(0))]),
        Instr::LoadUcode(buf, len) => Ok(vec![enc_rrr(0x82, *buf, *len, Reg(0))]),
        Instr::WriteFd(fd, buf, len) => Ok(vec![enc_rrr(0x57, Reg(fd.0), *buf, *len)]),
        Instr::ReadFd(fd, buf, len) => Ok(vec![enc_rrr(0x2d, Reg(fd.0), *buf, *len)]),
        // Unified endpoint IPC verbs (Phase 3): result=rd, ep handle=rs1, msg
        // descriptor pointer=rs2. send/recv route over byte-fds in the RTL via
        // the WRITE_FD/READ_FD datapath (EP-I-lite); wait is emulator/libc-only
        // until the M16 endpoint engine lands (EP-I-full).
        Instr::Send(result, ep, desc) => Ok(vec![enc_rrr(0x83, *result, *ep, *desc)]),
        Instr::Recv(result, ep, desc) => Ok(vec![enc_rrr(0x84, *result, *ep, *desc)]),
        Instr::Wait(result, waitset, timeout) => {
            Ok(vec![enc_rrr(0x86, *result, *waitset, *timeout)])
        }
        Instr::Await(result, fd, mask) => Ok(vec![enc_rrr(0x2e, *result, Reg(fd.0), *mask)]),
        Instr::AwaitDyn(result, fd_reg, mask, timeout) => Ok(vec![enc_rrrr(0x4d, *result, *fd_reg, *mask, *timeout)]),
        Instr::AwaitEx(result, fd, argblock) => {
            Ok(vec![enc_rrr(0x71, *result, Reg(fd.0), *argblock)])
        }
        Instr::CallCap(result, fd, arg0, arg1) => {
            Ok(vec![enc_rrrr(0x2f, *result, Reg(fd.0), *arg0, *arg1)])
        }
        Instr::RetCap(result, value0, value1) => Ok(vec![enc_rrr(0x4f, *result, *value0, *value1)]),
        Instr::Pull(result, fd, buf, len) => {
            Ok(vec![enc_rrrr(0x2b, *result, Reg(fd.0), *buf, *len)])
        }
        Instr::Push(result, fd, buf, len) => {
            Ok(vec![enc_rrrr(0x2c, *result, Reg(fd.0), *buf, *len)])
        }
        Instr::OpenFdDyn(dst, path, flags) => Ok(vec![enc_rrr(0x6d, *dst, *path, *flags)]),
        Instr::OpenDirDyn(dst, path, flags) => Ok(vec![enc_rrr(0x73, *dst, *path, *flags)]),
        Instr::MkdirPathAt(dir, path, mode) => Ok(vec![enc_rrr(0x74, *dir, *path, *mode)]),
        Instr::RenamePathAt(old_dir, old_path, new_dir, new_path) => Ok(vec![enc_rrrr(
            0x75, *old_dir, *old_path, *new_dir, *new_path,
        )]),
        Instr::LinkPathAt(old_dir, old_path, new_dir, new_path, flags) => Ok(vec![enc_rrrrr(
            0x76, *old_dir, *old_path, *new_dir, *new_path, *flags,
        )]),
        Instr::SymlinkPathAt(target, dir, link_path) => {
            Ok(vec![enc_rrr(0x77, *target, *dir, *link_path)])
        }
        Instr::ReadlinkPathAt(dir, path, buf, len) => {
            Ok(vec![enc_rrrr(0x78, *dir, *path, *buf, *len)])
        }
        Instr::ChdirPath(path) => Ok(vec![enc_reg(0x79, *path)]),
        Instr::GetcwdPath(buf, len) => Ok(vec![enc_rrr(0x7a, *buf, *len, Reg(0))]),
        Instr::ChmodPathAt(dir, path, mode, flags) => {
            Ok(vec![enc_rrrr(0x7b, *dir, *path, *mode, *flags)])
        }
        Instr::ChownPathAt(dir, path, uid, gid, flags) => Ok(vec![enc_rrrrr(
            0x7c, *dir, *path, *uid, *gid, *flags,
        )]),
        Instr::ReaddirFdDyn(fd, dirent_buf) => Ok(vec![enc_rrr(0xcf, *fd, *dirent_buf, Reg(0))]),
        Instr::FdCloseDyn(fd) => Ok(vec![enc_reg(0x6e, *fd)]),
        Instr::CloneSpawn(dst, entry, arg) => Ok(vec![enc_rrr(0x59, *dst, *entry, *arg)]),
        Instr::ThreadJoin(result, tid, retval) => Ok(vec![enc_rrr(0x5a, *result, *tid, *retval)]),
        Instr::FutexWait(addr, expected) => Ok(vec![enc_rrr(0xcb, *addr, *expected, Reg(0))]),
        Instr::FutexWake(addr, count) => Ok(vec![enc_rrr(0xcc, *addr, *count, Reg(0))]),
        Instr::Fork(dst) => Ok(vec![enc_reg(0x7d, *dst)]),
        Instr::Exec(path, argv, envp) => Ok(vec![enc_rrr(0x7f, *path, *argv, *envp)]),
        Instr::Fence => Ok(vec![enc_reg(0xcd, Reg(0))]),
        Instr::Isync(result, addr, len) => Ok(vec![enc_rrr(0xce, *result, *addr, *len)]),
        Instr::Exit(src) => Ok(vec![enc_reg(0x3a, *src)]),
        other => Err(format!(
            "asm-flat-exec cannot encode {other:?} into a v2 64-bit word"
        )),
    }
}

fn pcr_selector(pcr: Pcr) -> Result<usize, String> {
    match pcr {
        Pcr::Pid => Ok(0),
        Pcr::Ppid => Ok(1),
        Pcr::Tid => Ok(2),
        Pcr::Tp => Ok(3),
        Pcr::Uid => Ok(4),
        Pcr::Gid => Ok(5),
        Pcr::Sigmask => Ok(6),
        Pcr::Sigpending => Ok(7),
        Pcr::RealtimeSec => Ok(8),
        Pcr::RealtimeNsec => Ok(9),
        Pcr::CredProfile => Ok(10),
        Pcr::CredHandle => Ok(11),
    }
}

// v2: `li rd, imm32` is the assembler alias for `addi rd, r0, imm32` (one word).
fn encode_flat_exec_li(
    program: &Program,
    word_pcs: &[usize],
    rd: Reg,
    value: &Value,
) -> Result<Vec<u64>, String> {
    let imm = value_imm32(program, word_pcs, value)?;
    Ok(vec![enc_i(0xa0, rd, Reg(0), imm)])
}

// v2: `auipc rd, imm32` is one U-type word: rd = pc + sext32(imm).
fn encode_flat_exec_auipc(rd: Reg, value: &Value) -> Result<Vec<u64>, String> {
    let imm = value_imm32_without_labels(value)?;
    Ok(vec![enc_u(0xd0, rd, imm)])
}


fn value_imm32(program: &Program, word_pcs: &[usize], value: &Value) -> Result<i64, String> {
    const TEXT_BASE: u64 = 0x1000;
    match value {
        Value::Imm(imm) => imm32(*imm, "LI immediate"),
        Value::Label(label) => {
            let value = if let Some(data_addr) = program.data_labels.get(label).copied() {
                data_addr
            } else if let Some(text_pc) = program.labels.get(label).copied() {
                let word_pc = word_pcs
                    .get(text_pc)
                    .copied()
                    .ok_or_else(|| format!("label {label:?} has out-of-range text pc {text_pc}"))?;
                TEXT_BASE + (word_pc as u64 * 8)
            } else {
                return Err(format!("unknown label immediate {label:?}"));
            };
            imm32(value as i64, "LI label immediate")
        }
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
    // v2: control-transfer offsets are instruction counts; each instruction is
    // one 64-bit word, so the word-pc delta is the instruction count directly.
    imm32(
        target_word_pc as i64 - current_word_pc as i64,
        "branch delta (instruction count)",
    )
}

fn imm32(value: i64, name: &str) -> Result<i64, String> {
    if !(i64::from(i32::MIN)..=i64::from(u32::MAX)).contains(&value) {
        return Err(format!("{name} out of 32-bit literal range: {value}"));
    }
    Ok(value)
}

// --- v2 64-bit encoders. Slots: rd[55:51] rs1[50:46] rs2[45:41] rs3[40:36]
// rs4[35:31] rs5[30:26]. imm32 sits below the lowest used register slot.
fn slots(opcode: u8, rd: Reg, rs1: Reg, rs2: Reg, rs3: Reg, rs4: Reg, rs5: Reg) -> u64 {
    ((opcode as u64) << 56)
        | (((rd.0 as u64) & 0x1f) << 51)
        | (((rs1.0 as u64) & 0x1f) << 46)
        | (((rs2.0 as u64) & 0x1f) << 41)
        | (((rs3.0 as u64) & 0x1f) << 36)
        | (((rs4.0 as u64) & 0x1f) << 31)
        | (((rs5.0 as u64) & 0x1f) << 26)
}

fn enc_reg(opcode: u8, rd: Reg) -> u64 {
    slots(opcode, rd, Reg(0), Reg(0), Reg(0), Reg(0), Reg(0))
}

fn enc_rrr(opcode: u8, rd: Reg, rs1: Reg, rs2: Reg) -> u64 {
    slots(opcode, rd, rs1, rs2, Reg(0), Reg(0), Reg(0))
}

fn enc_rrrr(opcode: u8, rd: Reg, rs1: Reg, rs2: Reg, rs3: Reg) -> u64 {
    slots(opcode, rd, rs1, rs2, rs3, Reg(0), Reg(0))
}

fn enc_rrrrr(opcode: u8, rd: Reg, rs1: Reg, rs2: Reg, rs3: Reg, rs4: Reg) -> u64 {
    slots(opcode, rd, rs1, rs2, rs3, rs4, Reg(0))
}

// I-type: rd, rs1, imm32 at [45:14].
fn enc_i(opcode: u8, rd: Reg, rs1: Reg, imm: i64) -> u64 {
    ((opcode as u64) << 56)
        | (((rd.0 as u64) & 0x1f) << 51)
        | (((rs1.0 as u64) & 0x1f) << 46)
        | (((imm as u32 as u64) & 0xffff_ffff) << 14)
}

// S-type: rd-slot=0, rs1(base), rs2(src), imm32 at [40:9].
fn enc_s(opcode: u8, base: Reg, src: Reg, imm: i64) -> u64 {
    ((opcode as u64) << 56)
        | (((base.0 as u64) & 0x1f) << 46)
        | (((src.0 as u64) & 0x1f) << 41)
        | (((imm as u32 as u64) & 0xffff_ffff) << 9)
}

// B-type: rd-slot=0, rs1, rs2, imm32 (instr-count) at [40:9].
fn enc_b(opcode: u8, rs1: Reg, rs2: Reg, delta_words: i64) -> u64 {
    ((opcode as u64) << 56)
        | (((rs1.0 as u64) & 0x1f) << 46)
        | (((rs2.0 as u64) & 0x1f) << 41)
        | (((delta_words as u32 as u64) & 0xffff_ffff) << 9)
}

// U-type: rd, imm32 at [50:19].
fn enc_u(opcode: u8, rd: Reg, imm: i64) -> u64 {
    ((opcode as u64) << 56)
        | (((rd.0 as u64) & 0x1f) << 51)
        | (((imm as u32 as u64) & 0xffff_ffff) << 19)
}

// J-type: rd, imm32 (instr-count) at [50:19].
fn enc_j(opcode: u8, rd: Reg, delta_words: i64) -> u64 {
    ((opcode as u64) << 56)
        | (((rd.0 as u64) & 0x1f) << 51)
        | (((delta_words as u32 as u64) & 0xffff_ffff) << 19)
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
            source_cap: 1,
            source_generation: 1,
            lineage_epoch: 1,
            bytes: zero_page,
        },
        PreparedExecVma {
            virtual_address: DATA_BASE,
            protection: PROT_READ | PROT_WRITE,
            source_cap: 1,
            source_generation: 1,
            lineage_epoch: 1,
            bytes: data_page,
        },
        PreparedExecVma {
            virtual_address: TEXT_BASE,
            protection: PROT_READ | PROT_EXECUTE,
            source_cap: 1,
            source_generation: 1,
            lineage_epoch: 1,
            bytes: text_page,
        },
    ];
    let mut machine = Machine::new(Program::parse(".text\n  NOP\n")?);
    machine.commit_exec_descriptor_memory_image(&descriptor_words, &prepared)?;
    // Mirror the RTL top-program fixture's fixed heap/mmap windows so the
    // per-program manifest cosim is byte-exact (the image-derived heap placement
    // is correct for real ELF exec but does not match the RTL SRAM fixture).
    machine.set_flat_exec_allocation_bases(isa::FLAT_EXEC_HEAP_BASE, isa::FLAT_EXEC_MMAP_BASE)?;
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
        let word = u64::from_str_radix(word_text, 16)
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
            source_cap: descriptor_vma.source_cap,
            source_generation: descriptor_vma.source_generation,
            lineage_epoch: descriptor_vma.lineage_epoch,
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

        assert_eq!(hex, concat!("3a00000000000000\n", "0000000000000000\n",));
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

        assert_eq!(hex, "a00800000001c000\na010000000014000\n1018440000000000\n3300060000000000\n3020000000000000\n2000000000100000\na02800000018c000\na050000000008000\n5632800000000000\n3a20000000000000\n");
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

        assert_eq!(hex, "0600000000000000\na008000000000000\n3a08000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_get_pcr_subset() {
        let source = r#"
            .text
              GET_PCR r1, PID
              GET_PCR r2, TID
              GET_PCR r3, TLS_BASE
              LI r4, 4096
              SET_PCR r6, TP, r4
              GET_PCR r5, TP
              EXIT r0
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "5408000000000000\n5410800000000000\n5418c00000000000\na020000004000000\n5530c80000000000\n5428c00000000000\n3a00000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_namespace_compat_subset() {
        let source = r#"
            .text
              OPEN_DIR_DYN r1, r2, r3
              MKDIR_PATH_AT r4, r5, r6
              RENAME_PATH_AT r7, r8, r9, r10
              LINK_PATH_AT r11, r12, r13, r14, r15
              SYMLINK_PATH_AT r16, r17, r18
              READLINK_PATH_AT r19, r20, r21, r22
              CHDIR_PATH r23
              GETCWD_PATH r24, r25
              CHMOD_PATH_AT r26, r27, r28, r29
              CHOWN_PATH_AT r1, r2, r3, r4, r5
              EXIT r0
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "7308860000000000\n74214c0000000000\n753a12a000000000\n765b1ae780000000\n7784640000000000\n789d2b6000000000\n79b8000000000000\n7ac6400000000000\n7bd6f9d000000000\n7c08864280000000\n3a00000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_endpoint_verbs() {
        // F1-step-2: byte-fd transfer is the send/recv verbs (0x83/0x84); the
        // WRITE_FD_DYN/READ_FD_DYN forms (0x3b/0x3c) are retired. result=rd,
        // ep handle=rs1, msg descriptor pointer=rs2.
        let source = r#"
            .text
              SEND r2, r5, r20
              RECV r2, r6, r21
              EXIT r1
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "8311680000000000\n8411aa0000000000\n3a08000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_static_fd_push_pull() {
        let source = r#"
            .text
              LI r12, 80
              LI r13, 1
              PUSH r14, fd4, r12, r13
              PULL r16, fd3, r15, r13
              EXIT r16
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a060000000140000\na068000000004000\n2c7118d000000000\n2b80ded000000000\n3a80000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_static_fd_read_write() {
        let source = r#"
            .text
              LI r12, 80
              LI r13, 1
              WRITE_FD fd5, r12, r13
              READ_FD fd6, r14, r13
              EXIT r1
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a060000000140000\na068000000004000\n572b1a0000000000\n2d339a0000000000\n3a08000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_await_static_and_dynamic() {
        let source = r#"
            .text
              LI r16, 4
              LI r20, 1
              AWAIT r14, fd3, r20
              AWAIT_DYN r17, r16, r20
              EXIT r17
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a080000000010000\na0a0000000004000\n2e70e80000000000\n4d8c280000000000\n3a88000000000000\n");
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

        assert_eq!(hex, "a008000040000000\na010000040010000\n5708440000000000\n3a00000000000000\n");
        assert_eq!(data_hex, "00000000000a6b6f\n");
    }

    #[test]
    fn asm_flat_exec_encodes_single_word_mmap_with_later_label() {
        let source = r#"
            .text
              LI r1, handler
              LI r2, 16
              LI r3, 3
              MMAP r4, r0, r2, r3, fd0, r0
            handler:
              EXIT r0
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a008000004080000\na010000000040000\na01800000000c000\n6a20043000000000\n3a00000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_reg_compare_branch_subset() {
        let source = r#"
            .text
              LI r1, 3
              LI r2, 3
              BEQ r1, r2, equal
              LI r3, 4
              JMP done
            equal:
              LI r3, 17
            done:
              EXIT r3
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a00800000000c000\na01000000000c000\n2100440000000600\na018000000010000\n2000000000100000\na018000000044000\n3a18000000000000\n");
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

        assert_eq!(hex, "a008000000018000\na01000000001c000\n1218440000000000\n3a18000000000000\n");
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

        assert_eq!(hex, "a008000000024000\na010000000010000\n1118440000000000\n3a18000000000000\n");
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

        assert_eq!(hex, "a008000000028000\na010000000030000\n1418440000000000\n1620440000000000\n1528c80000000000\n3a28000000000000\n");
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

        assert_eq!(hex, "a00800000000c000\na010000000004000\n1818440000000000\n1920440000000000\n1028c80000000000\n3a28000000000000\n");
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

        assert_eq!(hex, "a008000000020000\na010400000014000\na11880000003c000\na220c00000080000\na32900000001c000\na431400000004000\na539800000008000\na0403ffffffe0000\na64a000000004000\na05240000002c000\n1059d40000000000\n3a58000000000000\n");
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

        assert_eq!(hex, "a0080000003fc000\nad10400000000000\na010800000008000\nb018400000000000\na02000003fffc000\nae29000000000000\na02940000000c000\nb131000000000000\na0383fffffffc000\naf41c00000000000\na042000000010000\nb249c00000000000\n10508a0000000000\n1052900000000000\na052800000018000\n3a50000000000000\n");
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

        assert_eq!(hex, "a008000000040000\nb310400000000000\na010bffffff24000\nb418400000000000\na020000003c3c000\nb521000000000000\na0213ffffffec000\na028000000004000\na030000000020000\nb6394c0000000000\nb741cc0000000000\na0480000048d0000\nb852400000000000\na15280000003c000\na058048d159e0000\nb95ac00000000000\na55ac00000060000\na15ac0000003c000\na0600000003fc000\nba63000000000000\na5630000000e0000\na16300000003c000\n1068860000000000\n106b480000000000\n106b500000000000\n106b540000000000\n106b560000000000\n106b580000000000\n3a68000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_slt_subset() {
        let source = r#"
            .text
              LI r1, 5
              LI r2, 9
              SLT r3, r1, r2
              SLTU r4, r1, r2
              SLTI r5, r1, 7
              SLTIU r6, r1, 7
              EXIT r3
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a008000000014000\na010000000024000\n1b18440000000000\n1c20440000000000\n1d2840000001c000\n1e3040000001c000\n3a18000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_liu_subset() {
        let source = r#"
            .text
              LI r1, 1
              LIU r1, r1, 2
              EXIT r1
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a008000000004000\n0408400000008000\n3a08000000000000\n");
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
              ADD r13, r3, r4
              ADD r13, r13, r6
              ADD r13, r13, r7
              ADD r13, r13, r8
              EXIT r13
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a008000000004000\na408400000080000\n0210400000000000\naa18440000000000\nab20440000000000\na0283fffffffc000\na429400000080000\naa31420000000000\nac39420000000000\nab41420000000000\n1068c80000000000\n106b4c0000000000\n106b4e0000000000\n106b500000000000\n3a68000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_auipc_fence_subset() {
        let source = r#"
            .text
              AUIPC r3, 0
              FENCE.SC
              AUIPC r4, 8
              EXIT r3
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "d018000000000000\ncd00000000000000\nd020000000400000\n3a18000000000000\n");
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
              ADD r13, r3, r5
              ADD r13, r13, r7
              EXIT r13
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a008000000000000\na010048d159e0000\n3400440000000000\n3118400000000000\na02000002af34000\n3700480000000800\n3628400000010000\na0300000156a8000\n37004c0000000c00\n3138400000010000\n1068ca0000000000\n106b4e0000000000\n3a68000000000000\n");
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

        assert_eq!(hex, "a008000000044000\na010000000014000\na718440000000000\na920440000000000\n1028c80000000000\n3a28000000000000\n");
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

        assert_eq!(hex, "a008000000044000\na010000000014000\n1318440000000000\na820440000000000\n1028c80000000000\n3a28000000000000\n");
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

        assert_eq!(hex, "a00800000001c000\n1710400000000000\n3a10000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_li_imm32_and_jmp_subset() {
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

        assert_eq!(hex, "a0083fffffffc000\n2000000000100000\na010000000004000\n3a08000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_call_return_subset() {
        let source = r#"
            .text
              LI r1, 5
              CALL add2
              EXIT r1
            add2:
              JALR r4, r3, 0
              LI r2, 2
              ADD r1, r1, r2
              RET
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a008000000014000\n2708000000100000\n3a08000000000000\n2820c00000000000\na010000000008000\n1008440000000000\n2800400000000000\n");
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

        assert_eq!(hex, "a008000000008000\n4710400000000000\n4828800000000000\na018000000104000\n3500860000000000\n3220800000000000\n4910000000000000\na030000000040000\n4a39820000000000\n3a20000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_memory_order_subset() {
        let source = r#"
            .text
              LI r1, 0
              LI r2, 8
              ISYNC r3, r1, r2
              LI r4, 41
              EXIT r4
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a008000000000000\na010000000020000\nce18440000000000\na0200000000a4000\n3a20000000000000\n");
    }

    #[test]
    fn asm_flat_exec_encodes_lr_sc_subset() {
        let source = r#"
            .text
              LI r1, 0
              LR.D r4, r1
              SC.D r5, r2, r1
              EXIT r5
        "#;
        let program = Program::parse(source).unwrap();
        let hex = encode_flat_exec_hex(&program).unwrap();

        assert_eq!(hex, "a008000000000000\nc520400000000000\nc628440000000000\n3a28000000000000\n");
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

        assert_eq!(hex, "4b22800000000000\n502a800000000000\n5132800000000000\n523a800000000000\n5332800000000000\n5b3a800000000000\n3a38000000000000\n");
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
        // v2: EXIT r0 is one 64-bit word, opcode 0x3a in the high byte.
        put_u64(&mut image, 0x100, 0x3a00_0000_0000_0000);
        image[0x108..0x110].fill(0);
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
