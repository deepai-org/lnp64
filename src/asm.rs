use std::collections::HashMap;

use crate::isa::*;

#[derive(Debug, Clone)]
pub struct Program {
    pub instructions: Vec<Instr>,
    pub labels: HashMap<String, usize>,
    pub data_labels: HashMap<String, u64>,
    pub data: Vec<u8>,
}

impl Program {
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut parser = Parser::default();
        for (idx, raw) in source.lines().enumerate() {
            parser.line_no = idx + 1;
            parser.parse_line(raw)?;
        }
        parser.finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Section {
    #[default]
    Text,
    Data,
}

#[derive(Default)]
struct Parser {
    section: Section,
    line_no: usize,
    instructions: Vec<Instr>,
    labels: HashMap<String, usize>,
    data_labels: HashMap<String, u64>,
    data: Vec<u8>,
    data_relocs: Vec<(usize, String)>,
}

impl Parser {
    fn finish(mut self) -> Result<Program, String> {
        for (offset, label) in &self.data_relocs {
            let Some(value) = self
                .data_labels
                .get(label)
                .copied()
                .or_else(|| self.labels.get(label).map(|addr| *addr as u64))
            else {
                return Err(format!("unknown data label {label:?}"));
            };
            self.data[*offset..*offset + 8].copy_from_slice(&value.to_le_bytes());
        }
        Ok(Program {
            instructions: self.instructions,
            labels: self.labels,
            data_labels: self.data_labels,
            data: self.data,
        })
    }

    fn parse_line(&mut self, raw: &str) -> Result<(), String> {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            return Ok(());
        }
        if line == ".text" {
            self.section = Section::Text;
            return Ok(());
        }
        if line == ".data" {
            self.section = Section::Data;
            return Ok(());
        }

        let mut rest = line;
        if let Some((label, after)) = split_label(rest) {
            self.define_label(label)?;
            rest = after.trim();
            if rest.is_empty() {
                return Ok(());
            }
        }

        match self.section {
            Section::Text => {
                let instr = self.parse_instr(rest)?;
                self.instructions.push(instr);
            }
            Section::Data => self.parse_data(rest)?,
        }
        Ok(())
    }

    fn define_label(&mut self, label: &str) -> Result<(), String> {
        validate_label(label).map_err(|err| self.err(err))?;
        if self.labels.contains_key(label) || self.data_labels.contains_key(label) {
            return Err(self.err(format!("duplicate label {label:?}")));
        }
        match self.section {
            Section::Text => {
                self.labels
                    .insert(label.to_string(), self.instructions.len());
            }
            Section::Data => {
                self.data_labels
                    .insert(label.to_string(), DATA_BASE + self.data.len() as u64);
            }
        }
        Ok(())
    }

    fn parse_data(&mut self, line: &str) -> Result<(), String> {
        let (directive, rest) = split_once_ws(line);
        match directive {
            ".string" => {
                let value = parse_string(rest.trim()).map_err(|err| self.err(err))?;
                self.data.extend_from_slice(value.as_bytes());
                self.data.push(0);
            }
            ".bytes" => {
                for part in split_operands(rest) {
                    let byte = parse_i64(&part).map_err(|err| self.err(err))?;
                    if !(0..=255).contains(&byte) {
                        return Err(self.err(format!("byte out of range: {byte}")));
                    }
                    self.data.push(byte as u8);
                }
            }
            ".quad" => {
                let text = rest.trim();
                let value = if let Ok(value) = parse_i64(text) {
                    value as u64
                } else {
                    let offset = self.data.len();
                    self.data_relocs.push((offset, text.to_string()));
                    0
                };
                self.data.extend_from_slice(&value.to_le_bytes());
            }
            ".zero" => {
                let count = parse_i64(rest.trim()).map_err(|err| self.err(err))?;
                if count < 0 {
                    return Err(self.err(".zero requires a non-negative count".to_string()));
                }
                self.data.resize(self.data.len() + count as usize, 0);
            }
            other => return Err(self.err(format!("unknown data directive {other:?}"))),
        }
        Ok(())
    }

    fn parse_instr(&self, line: &str) -> Result<Instr, String> {
        let (op_raw, rest) = split_once_ws(line);
        let op = op_raw.to_ascii_uppercase();
        let args = split_operands(rest);
        let arity = |n: usize| -> Result<(), String> {
            if args.len() == n {
                Ok(())
            } else {
                Err(self.err(format!("{op} expects {n} operands, got {}", args.len())))
            }
        };

        let instr = match op.as_str() {
            "NOP" => {
                arity(0)?;
                Instr::Nop
            }
            "LI" => {
                arity(2)?;
                Instr::Li(reg(&args[0])?, value(&args[1]))
            }
            "AUIPC" => {
                arity(2)?;
                Instr::Auipc(reg(&args[0])?, value(&args[1]))
            }
            "MOV" => {
                arity(2)?;
                Instr::Mov(reg(&args[0])?, reg(&args[1])?)
            }
            "ADD" => alu3(&args, Instr::Add)?,
            "ADDI" => alu_imm(&args, Instr::Addi)?,
            "SUB" => alu3(&args, Instr::Sub)?,
            "MUL" => alu3(&args, Instr::Mul)?,
            "MULH" => alu3(&args, Instr::Mulh)?,
            "MULHU" => alu3(&args, Instr::Mulhu)?,
            "MULHSU" => alu3(&args, Instr::Mulhsu)?,
            "DIV" => alu3(&args, Instr::Div)?,
            "UDIV" => alu3(&args, Instr::Udiv)?,
            "SREM" => alu3(&args, Instr::Srem)?,
            "UREM" => alu3(&args, Instr::Urem)?,
            "AND" => alu3(&args, Instr::And)?,
            "ANDI" => alu_imm(&args, Instr::Andi)?,
            "OR" => alu3(&args, Instr::Or)?,
            "ORI" => alu_imm(&args, Instr::Ori)?,
            "XOR" => alu3(&args, Instr::Xor)?,
            "XORI" => alu_imm(&args, Instr::Xori)?,
            "NOT" => {
                arity(2)?;
                Instr::Not(reg(&args[0])?, reg(&args[1])?)
            }
            "LSL" => alu3(&args, Instr::Lsl)?,
            "LSLI" => alu_imm(&args, Instr::Lsli)?,
            "LSR" => alu3(&args, Instr::Lsr)?,
            "LSRI" => alu_imm(&args, Instr::Lsri)?,
            "ASR" => alu3(&args, Instr::Asr)?,
            "ASRI" => alu_imm(&args, Instr::Asri)?,
            "SEXT.B" => alu2(&args, Instr::SextB)?,
            "SEXT.H" => alu2(&args, Instr::SextH)?,
            "SEXT.W" => alu2(&args, Instr::SextW)?,
            "ZEXT.B" => alu2(&args, Instr::ZextB)?,
            "ZEXT.H" => alu2(&args, Instr::ZextH)?,
            "ZEXT.W" => alu2(&args, Instr::ZextW)?,
            "CLZ" => alu2(&args, Instr::Clz)?,
            "CTZ" => alu2(&args, Instr::Ctz)?,
            "POPCNT" => alu2(&args, Instr::Popcnt)?,
            "ROL" => alu3(&args, Instr::Rol)?,
            "ROR" => alu3(&args, Instr::Ror)?,
            "BSWAP16" => alu2(&args, Instr::Bswap16)?,
            "BSWAP32" => alu2(&args, Instr::Bswap32)?,
            "BSWAP64" => alu2(&args, Instr::Bswap64)?,
            "SLT" => alu3(&args, Instr::Slt)?,
            "SLTU" => alu3(&args, Instr::Sltu)?,
            "SLTI" => alu_imm(&args, Instr::Slti)?,
            "SLTIU" => alu_imm(&args, Instr::Sltiu)?,
            "SEL.EQ" => sel5(&args, SelCond::Eq)?,
            "SEL.NE" => sel5(&args, SelCond::Ne)?,
            "SEL.LT" => sel5(&args, SelCond::Lt)?,
            "SEL.GE" => sel5(&args, SelCond::Ge)?,
            "SEL.LTU" => sel5(&args, SelCond::Ltu)?,
            "SEL.GEU" => sel5(&args, SelCond::Geu)?,
            "LIU" => alu_imm(&args, Instr::Liu)?,
            "JMP" => {
                arity(1)?;
                Instr::Jmp(target(&args[0]))
            }
            "BEQ" => branch(&args, Instr::Beq)?,
            "BNE" => branch(&args, Instr::Bne)?,
            "BLT" => branch(&args, Instr::Blt)?,
            "BGE" => branch(&args, Instr::Bge)?,
            "BLTU" => branch(&args, Instr::Bltu)?,
            "BGEU" => branch(&args, Instr::Bgeu)?,
            // assembler aliases: bgt/ble swap operands.
            "BGT" => branch(&args, Instr::Blt).map(swap_branch_operands)?,
            "BLE" => branch(&args, Instr::Bge).map(swap_branch_operands)?,
            "BGTU" => branch(&args, Instr::Bltu).map(swap_branch_operands)?,
            "BLEU" => branch(&args, Instr::Bgeu).map(swap_branch_operands)?,
            "JAL" => {
                arity(2)?;
                Instr::Jal(reg(&args[0])?, target(&args[1]))
            }
            "JALR" => {
                arity(3)?;
                Instr::Jalr(reg(&args[0])?, reg(&args[1])?, parse_i64(&args[2])?)
            }
            // `call sym` = `jal r1, sym`; `ret` = `jalr r0, r1, 0`.
            "CALL" => {
                arity(1)?;
                Instr::Jal(Reg(1), target(&args[0]))
            }
            "RET" => {
                arity(0)?;
                Instr::Jalr(Reg(0), Reg(1), 0)
            }
            "LR.D" => {
                arity(2)?;
                Instr::LrD(reg(&args[0])?, reg(&args[1])?)
            }
            "SC.D" => {
                arity(3)?;
                Instr::ScD(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "LD" | "LD.D" => load(&args, Width::Double)?,
            "LWU" | "LD.W" => load(&args, Width::Word)?,
            "LHU" | "LD.H" => load(&args, Width::Half)?,
            "LBU" | "LD.B" => load(&args, Width::Byte)?,
            "LW" => load_signed(&args, Width::Word)?,
            "LH" => load_signed(&args, Width::Half)?,
            "LB" => load_signed(&args, Width::Byte)?,
            "SD" | "ST" | "ST.D" => store(&args, Width::Double)?,
            "SW" | "ST.W" => store(&args, Width::Word)?,
            "SH" | "ST.H" => store(&args, Width::Half)?,
            "SB" | "ST.B" => store(&args, Width::Byte)?,
            "FENCE" | "FENCE.ACQ" | "FENCE.REL" | "FENCE.ACQ_REL" | "FENCE.SC" => {
                arity(0)?;
                Instr::Fence
            }
            "ISYNC" => {
                if args.len() == 2 {
                    Instr::Isync(Reg(1), reg(&args[0])?, reg(&args[1])?)
                } else {
                    arity(3)?;
                    Instr::Isync(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
                }
            }
            "PULL" => {
                arity(4)?;
                Instr::Pull(
                    reg(&args[0])?,
                    fd(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "PUSH" => {
                arity(4)?;
                Instr::Push(
                    reg(&args[0])?,
                    fd(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "AWAIT" => {
                arity(3)?;
                Instr::Await(reg(&args[0])?, fd(&args[1])?, reg(&args[2])?)
            }
            "AWAIT_DYN" => {
                if args.len() == 4 {
                    Instr::AwaitDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?, reg(&args[3])?)
                } else {
                    arity(3)?;
                    Instr::AwaitDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?, Reg(0))
                }
            }
            "ALLOC" => {
                arity(2)?;
                Instr::Alloc(reg(&args[0])?, reg(&args[1])?)
            }
            "ALLOC_EX" => {
                arity(3)?;
                Instr::AllocEx(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "ALLOC_SIZE" => {
                arity(2)?;
                Instr::AllocSize(reg(&args[0])?, reg(&args[1])?)
            }
            "ENV_GET" => {
                arity(4)?;
                Instr::EnvGet(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "RANDOM" => {
                arity(3)?;
                Instr::Random(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "FREE" => {
                arity(1)?;
                Instr::Free(reg(&args[0])?)
            }
            "OPEN_AT" | "OPEN_FD" => {
                arity(3)?;
                Instr::OpenFd(fd(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "OPEN_AT_DYN" | "OPEN_FD_DYN" => {
                if args.len() == 3 {
                    Instr::OpenFdDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
                } else {
                    arity(4)?;
                    Instr::OpenAtDyn(
                        reg(&args[0])?,
                        reg(&args[1])?,
                        reg(&args[2])?,
                        reg(&args[3])?,
                    )
                }
            }
            "OPEN_DIR" | "OPEN_DIR_AT" => {
                arity(3)?;
                Instr::OpenDir(fd(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "OPEN_DIR_DYN" | "OPEN_DIR_AT_DYN" => {
                arity(3)?;
                Instr::OpenDirDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "PREAD_FD" => {
                arity(4)?;
                Instr::PreadFd(
                    fd(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "PREAD_FD_DYN" => {
                arity(4)?;
                Instr::PreadFdDyn(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "READDIR_FD" => {
                arity(2)?;
                Instr::ReaddirFd(fd(&args[0])?, reg(&args[1])?)
            }
            "READDIR_FD_DYN" => {
                arity(2)?;
                Instr::ReaddirFdDyn(reg(&args[0])?, reg(&args[1])?)
            }
            "REWINDDIR_FD" => {
                arity(1)?;
                Instr::RewinddirFd(fd(&args[0])?)
            }
            "REWINDDIR_FD_DYN" => {
                arity(1)?;
                Instr::RewinddirFdDyn(reg(&args[0])?)
            }
            "PWRITE_FD" => {
                arity(4)?;
                Instr::PwriteFd(
                    fd(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "PWRITE_FD_DYN" => {
                arity(4)?;
                Instr::PwriteFdDyn(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "MKDIR_PATH" => {
                arity(2)?;
                Instr::MkdirPath(reg(&args[0])?, reg(&args[1])?)
            }
            "MKDIR_PATH_AT" => {
                arity(3)?;
                Instr::MkdirPathAt(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "UNLINK_PATH" => {
                arity(1)?;
                Instr::UnlinkPath(reg(&args[0])?)
            }
            "UNLINK_PATH_AT" => {
                arity(3)?;
                Instr::UnlinkPathAt(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "RENAME_PATH" => {
                arity(2)?;
                Instr::RenamePath(reg(&args[0])?, reg(&args[1])?)
            }
            "RENAME_PATH_AT" => {
                arity(4)?;
                Instr::RenamePathAt(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "LINK_PATH" => {
                arity(3)?;
                Instr::LinkPath(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "LINK_PATH_AT" => {
                arity(5)?;
                Instr::LinkPathAt(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                    reg(&args[4])?,
                )
            }
            "SYMLINK_PATH" => {
                arity(2)?;
                Instr::SymlinkPath(reg(&args[0])?, reg(&args[1])?)
            }
            "SYMLINK_PATH_AT" => {
                arity(3)?;
                Instr::SymlinkPathAt(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "READLINK_PATH" => {
                arity(3)?;
                Instr::ReadlinkPath(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "READLINK_PATH_AT" => {
                arity(4)?;
                Instr::ReadlinkPathAt(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "CHDIR_PATH" => {
                arity(1)?;
                Instr::ChdirPath(reg(&args[0])?)
            }
            "GETCWD_PATH" => {
                arity(2)?;
                Instr::GetcwdPath(reg(&args[0])?, reg(&args[1])?)
            }
            "CHMOD_PATH" => {
                arity(3)?;
                Instr::ChmodPath(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "CHMOD_PATH_AT" => {
                arity(4)?;
                Instr::ChmodPathAt(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "CHOWN_PATH" => {
                arity(4)?;
                Instr::ChownPath(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "CHOWN_PATH_AT" => {
                arity(5)?;
                Instr::ChownPathAt(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                    reg(&args[4])?,
                )
            }
            "UTIME_PATH" => {
                arity(3)?;
                Instr::UtimePath(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "UTIME_PATH_AT" => {
                arity(4)?;
                Instr::UtimePathAt(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "UTIME_FD" => {
                arity(2)?;
                Instr::UtimeFd(fd(&args[0])?, reg(&args[1])?)
            }
            "UTIME_FD_DYN" => {
                arity(2)?;
                Instr::UtimeFdDyn(reg(&args[0])?, reg(&args[1])?)
            }
            "STAT_PATH" => {
                arity(3)?;
                Instr::StatPath(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "STAT_PATH_AT" => {
                arity(4)?;
                Instr::StatPathAt(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "STAT_FD" => {
                arity(2)?;
                Instr::StatFd(reg(&args[0])?, fd(&args[1])?)
            }
            "STAT_FD_DYN" => {
                arity(2)?;
                Instr::StatFdDyn(reg(&args[0])?, reg(&args[1])?)
            }
            "FCNTL_FD_DYN" => {
                arity(3)?;
                Instr::FcntlFdDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "FD_CLOSE" => {
                arity(1)?;
                Instr::FdClose(fd(&args[0])?)
            }
            "FD_CLOSE_DYN" => {
                arity(1)?;
                Instr::FdCloseDyn(reg(&args[0])?)
            }
            "FD_SEEK" => {
                arity(3)?;
                Instr::FdSeek(fd(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "FD_SEEK_DYN" => {
                arity(3)?;
                Instr::FdSeekDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "WAIT_ON_FD" => {
                arity(2)?;
                Instr::WaitOnFd(fd(&args[0])?, reg(&args[1])?)
            }
            "FD_DUP" => {
                arity(2)?;
                Instr::FdDup(fd(&args[0])?, fd(&args[1])?)
            }
            "FD_DUP2" => {
                arity(2)?;
                Instr::FdDup2(fd(&args[0])?, fd(&args[1])?)
            }
            "ERRNO_GET" => {
                arity(1)?;
                Instr::ErrnoGet(reg(&args[0])?)
            }
            "ERRNO_SET" => {
                arity(1)?;
                Instr::ErrnoSet(reg(&args[0])?)
            }
            "WAIT_PID" => {
                arity(2)?;
                Instr::WaitPid(reg(&args[0])?, reg(&args[1])?)
            }
            "GET_PCR" => {
                arity(2)?;
                Instr::GetPcr(
                    reg(&args[0])?,
                    parse_pcr(&args[1]).map_err(|err| self.err(err))?,
                )
            }
            "SET_PCR" => {
                arity(3)?;
                Instr::SetPcr(
                    reg(&args[0])?,
                    parse_pcr(&args[1]).map_err(|err| self.err(err))?,
                    reg(&args[2])?,
                )
            }
            "FORK" => {
                arity(1)?;
                Instr::Fork(reg(&args[0])?)
            }
            "EXEC" => {
                if args.len() != 2 && args.len() != 3 {
                    return Err(self.err("EXEC expects 2 or 3 arguments".to_string()));
                }
                let envp = if args.len() == 3 {
                    reg(&args[2])?
                } else {
                    Reg(0)
                };
                Instr::Exec(reg(&args[0])?, reg(&args[1])?, envp)
            }
            "SPAWN" => {
                arity(2)?;
                Instr::Spawn(reg(&args[0])?, reg(&args[1])?)
            }
            "CLONE.SPAWN" | "CLONE_SPAWN" => {
                arity(3)?;
                Instr::CloneSpawn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "THREAD_JOIN" => {
                arity(3)?;
                Instr::ThreadJoin(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "THREAD_DETACH" => {
                arity(2)?;
                Instr::ThreadDetach(reg(&args[0])?, reg(&args[1])?)
            }
            "YIELD" => {
                arity(0)?;
                Instr::Yield
            }
            "SLEEP" => {
                arity(1)?;
                Instr::Sleep(reg(&args[0])?)
            }
            "EXIT" => {
                arity(1)?;
                Instr::Exit(reg(&args[0])?)
            }
            "MMAP" => {
                arity(6)?;
                Instr::Mmap(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                    fd(&args[4])?,
                    reg(&args[5])?,
                )
            }
            "MUNMAP" => {
                arity(2)?;
                Instr::Munmap(reg(&args[0])?, reg(&args[1])?)
            }
            "MPROTECT" => {
                arity(3)?;
                Instr::Mprotect(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "SIGACTION" => {
                arity(2)?;
                Instr::Sigaction(reg(&args[0])?, reg(&args[1])?)
            }
            "SIGMASK_SET" => {
                arity(1)?;
                Instr::SigmaskSet(reg(&args[0])?)
            }
            "ALARM" => {
                arity(2)?;
                Instr::Alarm(reg(&args[0])?, reg(&args[1])?)
            }
            "KILL" => {
                arity(2)?;
                Instr::Kill(reg(&args[0])?, reg(&args[1])?)
            }
            "SIGRET" => {
                arity(0)?;
                Instr::Sigret
            }
            "FUTEX_WAIT" => {
                arity(2)?;
                Instr::FutexWait(reg(&args[0])?, reg(&args[1])?)
            }
            "FUTEX_WAKE" => {
                arity(2)?;
                Instr::FutexWake(reg(&args[0])?, reg(&args[1])?)
            }
            "INB" => {
                arity(2)?;
                Instr::Inb(reg(&args[0])?, reg(&args[1])?)
            }
            "OUTB" => {
                arity(2)?;
                Instr::Outb(reg(&args[0])?, reg(&args[1])?)
            }
            "LOAD_UCODE" => {
                arity(2)?;
                Instr::LoadUcode(reg(&args[0])?, reg(&args[1])?)
            }
            "MSG_SEND" => {
                arity(3)?;
                Instr::MsgSend(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            // Unified endpoint IPC verbs (unified_object_model.md §3).
            "ENDPOINT_CREATE" => {
                arity(2)?;
                Instr::EndpointCreate(reg(&args[0])?, reg(&args[1])?)
            }
            "SEND" => {
                arity(3)?;
                Instr::Send(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "RECV" => {
                arity(3)?;
                Instr::Recv(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "WAIT" => {
                arity(3)?;
                Instr::Wait(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "OBJECT_CTL" | "EVENT_CTL" | "TIMER_CTL" => {
                arity(2)?;
                Instr::ObjectCtl(reg(&args[0])?, reg(&args[1])?)
            }
            "DMA_CTL" => {
                arity(2)?;
                Instr::DmaCtl(reg(&args[0])?, reg(&args[1])?)
            }
            "CAP_SEND" => {
                arity(2)?;
                Instr::CapSend(reg(&args[0])?, reg(&args[1])?)
            }
            "CAP_RECV" => {
                arity(2)?;
                Instr::CapRecv(reg(&args[0])?, reg(&args[1])?)
            }
            "CAP_DUP" => {
                arity(2)?;
                Instr::CapDup(reg(&args[0])?, reg(&args[1])?)
            }
            "CAP_REVOKE" => {
                arity(2)?;
                Instr::CapRevoke(reg(&args[0])?, reg(&args[1])?)
            }
            "DOMAIN_CTL" => {
                arity(2)?;
                Instr::DomainCtl(reg(&args[0])?, reg(&args[1])?)
            }
            "NS_CTL" => {
                arity(2)?;
                Instr::NsCtl(reg(&args[0])?, reg(&args[1])?)
            }
            "GATE_CALL" | "CALL_CAP" => {
                arity(4)?;
                Instr::CallCap(
                    reg(&args[0])?,
                    fd(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "GATE_RETURN" | "RET_CAP" => {
                arity(3)?;
                Instr::RetCap(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "FADD" => fpu3(&args, Instr::FAdd)?,
            "FSUB" => fpu3(&args, Instr::FSub)?,
            "FMUL" => fpu3(&args, Instr::FMul)?,
            "FDIV" => fpu3(&args, Instr::FDiv)?,
            "VADD.32" => vec3(&args, Instr::VAdd32)?,
            _ => return Err(self.err(format!("unknown instruction {op_raw:?}"))),
        };
        Ok(instr)
    }

    fn err(&self, message: String) -> String {
        format!("line {}: {message}", self.line_no)
    }
}

fn alu3(args: &[String], ctor: fn(Reg, Reg, Reg) -> Instr) -> Result<Instr, String> {
    if args.len() != 3 {
        return Err(format!(
            "ALU instruction expects 3 operands, got {}",
            args.len()
        ));
    }
    Ok(ctor(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?))
}

// sel.<cc> rd, ra, rb, rt, rf -- fused compare-and-select.
fn sel5(args: &[String], cc: SelCond) -> Result<Instr, String> {
    if args.len() != 5 {
        return Err(format!("sel expects 5 operands, got {}", args.len()));
    }
    Ok(Instr::Sel(
        cc,
        reg(&args[0])?,
        reg(&args[1])?,
        reg(&args[2])?,
        reg(&args[3])?,
        reg(&args[4])?,
    ))
}

fn alu2(args: &[String], ctor: fn(Reg, Reg) -> Instr) -> Result<Instr, String> {
    if args.len() != 2 {
        return Err(format!(
            "ALU instruction expects 2 operands, got {}",
            args.len()
        ));
    }
    Ok(ctor(reg(&args[0])?, reg(&args[1])?))
}

fn alu_imm(args: &[String], ctor: fn(Reg, Reg, i64) -> Instr) -> Result<Instr, String> {
    if args.len() != 3 {
        return Err(format!(
            "immediate ALU instruction expects 3 operands, got {}",
            args.len()
        ));
    }
    Ok(ctor(reg(&args[0])?, reg(&args[1])?, parse_i64(&args[2])?))
}

fn fpu3(args: &[String], ctor: fn(FReg, FReg, FReg) -> Instr) -> Result<Instr, String> {
    if args.len() != 3 {
        return Err(format!(
            "FPU instruction expects 3 operands, got {}",
            args.len()
        ));
    }
    Ok(ctor(freg(&args[0])?, freg(&args[1])?, freg(&args[2])?))
}

fn vec3(args: &[String], ctor: fn(VReg, VReg, VReg) -> Instr) -> Result<Instr, String> {
    if args.len() != 3 {
        return Err(format!(
            "vector instruction expects 3 operands, got {}",
            args.len()
        ));
    }
    Ok(ctor(vreg(&args[0])?, vreg(&args[1])?, vreg(&args[2])?))
}

// v2 reg-compare branch: `b** rs1, rs2, target`.
fn branch(args: &[String], ctor: fn(Reg, Reg, Target) -> Instr) -> Result<Instr, String> {
    if args.len() != 3 {
        return Err(format!("branch expects 3 operands, got {}", args.len()));
    }
    Ok(ctor(reg(&args[0])?, reg(&args[1])?, target(&args[2])))
}

// bgt/ble (and unsigned) are operand-swapped spellings of blt/bge.
fn swap_branch_operands(instr: Instr) -> Instr {
    match instr {
        Instr::Beq(a, b, t) => Instr::Beq(b, a, t),
        Instr::Bne(a, b, t) => Instr::Bne(b, a, t),
        Instr::Blt(a, b, t) => Instr::Blt(b, a, t),
        Instr::Bge(a, b, t) => Instr::Bge(b, a, t),
        Instr::Bltu(a, b, t) => Instr::Bltu(b, a, t),
        Instr::Bgeu(a, b, t) => Instr::Bgeu(b, a, t),
        other => other,
    }
}

fn load(args: &[String], width: Width) -> Result<Instr, String> {
    if args.len() != 2 {
        return Err(format!("load expects 2 operands, got {}", args.len()));
    }
    Ok(Instr::Ld(reg(&args[0])?, mem(&args[1])?, width))
}

fn load_signed(args: &[String], width: Width) -> Result<Instr, String> {
    if args.len() != 2 {
        return Err(format!("load expects 2 operands, got {}", args.len()));
    }
    Ok(Instr::LdS(reg(&args[0])?, mem(&args[1])?, width))
}

fn store(args: &[String], width: Width) -> Result<Instr, String> {
    if args.len() != 2 {
        return Err(format!("store expects 2 operands, got {}", args.len()));
    }
    Ok(Instr::St(mem(&args[0])?, reg(&args[1])?, width))
}

fn reg(text: &str) -> Result<Reg, String> {
    parse_reg(text)
}

fn fd(text: &str) -> Result<FdReg, String> {
    parse_fd(text)
}

fn freg(text: &str) -> Result<FReg, String> {
    parse_freg(text)
}

fn vreg(text: &str) -> Result<VReg, String> {
    parse_vreg(text)
}

fn value(text: &str) -> Value {
    parse_i64(text)
        .map(Value::Imm)
        .unwrap_or_else(|_| Value::Label(text.to_string()))
}

fn target(text: &str) -> Target {
    if let Ok(addr) = text.parse::<usize>() {
        Target::Address(addr)
    } else {
        Target::Label(text.to_string())
    }
}

fn mem(text: &str) -> Result<MemRef, String> {
    let trimmed = text.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Ok(MemRef::Label(trimmed.to_string()));
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let parts = split_operands(inner);
    match parts.as_slice() {
        [base] => Ok(MemRef::BaseOffset(reg(base)?, 0)),
        [base, offset] => Ok(MemRef::BaseOffset(reg(base)?, parse_i64(offset)?)),
        _ => Err(format!("invalid memory reference {text:?}")),
    }
}

fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            ';' | '#' if !in_string => return &line[..idx],
            _ => {}
        }
    }
    line
}

fn split_label(line: &str) -> Option<(&str, &str)> {
    let idx = line.find(':')?;
    let label = line[..idx].trim();
    if label.is_empty() || label.contains(char::is_whitespace) {
        return None;
    }
    Some((label, &line[idx + 1..]))
}

fn split_once_ws(line: &str) -> (&str, &str) {
    let trimmed = line.trim();
    if let Some(idx) = trimmed.find(char::is_whitespace) {
        (&trimmed[..idx], trimmed[idx..].trim())
    } else {
        (trimmed, "")
    }
}

pub fn split_operands(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => {
                current.push(ch);
                escaped = true;
            }
            '"' => {
                in_string = !in_string;
                current.push(ch);
            }
            '[' if !in_string => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' if !in_string => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if !in_string && bracket_depth == 0 => {
                let item = current.trim();
                if !item.is_empty() {
                    out.push(item.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let item = current.trim();
    if !item.is_empty() {
        out.push(item.to_string());
    }
    out
}

fn validate_label(label: &str) -> Result<(), String> {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return Err("empty label".to_string());
    };
    if !(first == '_' || first == '.' || first.is_ascii_alphabetic()) {
        return Err(format!("invalid label {label:?}"));
    }
    if chars.any(|ch| !(ch == '_' || ch == '.' || ch.is_ascii_alphanumeric())) {
        return Err(format!("invalid label {label:?}"));
    }
    Ok(())
}

pub fn parse_i64(text: &str) -> Result<i64, String> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).map_err(|_| format!("invalid integer {text:?}"))
    } else if let Some(hex) = text.strip_prefix("-0x") {
        i64::from_str_radix(hex, 16)
            .map(|v| -v)
            .map_err(|_| format!("invalid integer {text:?}"))
    } else {
        text.parse::<i64>()
            .map_err(|_| format!("invalid integer {text:?}"))
    }
}

pub fn parse_string(text: &str) -> Result<String, String> {
    let text = text.trim();
    if !text.starts_with('"') || !text.ends_with('"') {
        return Err(format!("expected string literal, got {text:?}"));
    }
    let inner = &text[1..text.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(esc) = chars.next() else {
            return Err("unterminated escape sequence".to_string());
        };
        match esc {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            other => return Err(format!("unsupported escape \\{other}")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_data_and_text_labels() {
        let program = Program::parse(
            r#"
            .data
            msg: .string "ok\n"
            num: .quad 7
            .text
            start:
              LI r1, msg
              LD r2, num
              EXIT r0
            "#,
        )
        .unwrap();
        assert_eq!(program.labels["start"], 0);
        assert_eq!(program.data_labels["msg"], DATA_BASE);
        assert_eq!(program.instructions.len(), 3);
    }

    #[test]
    fn parses_object_ctl_profile_aliases() {
        let program = Program::parse(
            r#"
            .text
              OBJECT_CTL r1, r2
              EVENT_CTL r3, r4
              TIMER_CTL r5, r6
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::ObjectCtl(Reg(1), Reg(2))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::ObjectCtl(Reg(3), Reg(4))
        ));
        assert!(matches!(
            program.instructions[2],
            Instr::ObjectCtl(Reg(5), Reg(6))
        ));
    }

    #[test]
    fn parses_namespace_control() {
        let program = Program::parse(
            r#"
            .text
              NS_CTL r7, r8
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::NsCtl(Reg(7), Reg(8))
        ));
    }

    #[test]
    fn parses_open_at_and_legacy_open_fd_aliases() {
        let program = Program::parse(
            r#"
            .text
              OPEN_AT fd3, r1, r2
              OPEN_AT_DYN r4, r5, r6
              OPEN_AT_DYN r13, r14, r15, r16
              OPEN_FD fd7, r8, r9
              OPEN_FD_DYN r10, r11, r12
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::OpenFd(FdReg(3), Reg(1), Reg(2))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::OpenFdDyn(Reg(4), Reg(5), Reg(6))
        ));
        assert!(matches!(
            program.instructions[2],
            Instr::OpenAtDyn(Reg(13), Reg(14), Reg(15), Reg(16))
        ));
        assert!(matches!(
            program.instructions[3],
            Instr::OpenFd(FdReg(7), Reg(8), Reg(9))
        ));
        assert!(matches!(
            program.instructions[4],
            Instr::OpenFdDyn(Reg(10), Reg(11), Reg(12))
        ));
    }

    #[test]
    fn parses_endpoint_verbs() {
        // F1-step-2 retired READ_FD_DYN/WRITE_FD_DYN (and the PULL_DYN/PUSH_DYN
        // twins): byte-fd recv/send go through the unified send/recv/wait verbs.
        let program = Program::parse(
            r#"
            .text
              SEND r2, r5, r20
              RECV r2, r6, r21
              WAIT r7, r5, r6
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::Send(Reg(2), Reg(5), Reg(20))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::Recv(Reg(2), Reg(6), Reg(21))
        ));
        assert!(matches!(
            program.instructions[2],
            Instr::Wait(Reg(7), Reg(5), Reg(6))
        ));
    }

    #[test]
    fn parses_unified_endpoint_verbs() {
        let program = Program::parse(
            r#"
            .text
              ENDPOINT_CREATE r2, r0
              SEND r3, r2, r4
              RECV r5, r2, r6
              WAIT r7, r8, r9
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::EndpointCreate(Reg(2), Reg(0))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::Send(Reg(3), Reg(2), Reg(4))
        ));
        assert!(matches!(
            program.instructions[2],
            Instr::Recv(Reg(5), Reg(2), Reg(6))
        ));
        assert!(matches!(
            program.instructions[3],
            Instr::Wait(Reg(7), Reg(8), Reg(9))
        ));
    }


    #[test]
    fn parses_get_pcr_tls_base_alias() {
        let program = Program::parse(
            r#"
              .text
              GET_PCR r1, TLS_BASE
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::GetPcr(Reg(1), Pcr::Tp)
        ));
    }

    #[test]
    fn set_pcr_requires_result_selector_and_source() {
        let err = Program::parse(
            r#"
              .text
              SET_PCR TP, r1
            "#,
        )
        .unwrap_err();

        assert!(
            err.contains("SET_PCR expects 3 operands"),
            "unexpected error: {err}"
        );

        let program = Program::parse(
            r#"
              .text
              SET_PCR r2, TP, r1
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::SetPcr(Reg(2), Pcr::Tp, Reg(1))
        ));
    }

    #[test]
    fn parses_gate_and_legacy_call_cap_aliases() {
        let program = Program::parse(
            r#"
            .text
              GATE_CALL r1, fd2, r3, r4
              GATE_RETURN r5, r6, r7
              CALL_CAP r8, fd9, r10, r11
              RET_CAP r12, r13, r14
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::CallCap(Reg(1), FdReg(2), Reg(3), Reg(4))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::RetCap(Reg(5), Reg(6), Reg(7))
        ));
        assert!(matches!(
            program.instructions[2],
            Instr::CallCap(Reg(8), FdReg(9), Reg(10), Reg(11))
        ));
        assert!(matches!(
            program.instructions[3],
            Instr::RetCap(Reg(12), Reg(13), Reg(14))
        ));
    }

    #[test]
    fn parses_isync_with_implicit_or_explicit_result() {
        let program = Program::parse(
            r#"
            .text
              ISYNC r2, r3
              ISYNC r4, r5, r6
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::Isync(Reg(1), Reg(2), Reg(3))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::Isync(Reg(4), Reg(5), Reg(6))
        ));
    }

    #[test]
    fn parses_compiler_baseline_integer_ops() {
        let program = Program::parse(
            r#"
            .text
              ADDI r1, r2, -7
              AUIPC r2, 4096
              ANDI r3, r4, 255
              MULHU r5, r6, r7
              UDIV r8, r9, r10
              SREM r11, r12, r13
              SEXT.W r14, r15
              ZEXT.B r16, r17
              CLZ r18, r19
              POPCNT r20, r21
              ROL r22, r23, r24
              BSWAP64 r25, r26
              SLT r27, r28, r29
              LR.D r30, r1
              SC.D r4, r5, r6
              FENCE.SC
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::Addi(Reg(1), Reg(2), -7)
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::Auipc(Reg(2), Value::Imm(4096))
        ));
        assert!(matches!(
            program.instructions[2],
            Instr::Andi(Reg(3), Reg(4), 255)
        ));
        assert!(matches!(
            program.instructions[3],
            Instr::Mulhu(Reg(5), Reg(6), Reg(7))
        ));
        assert!(matches!(
            program.instructions[4],
            Instr::Udiv(Reg(8), Reg(9), Reg(10))
        ));
        assert!(matches!(
            program.instructions[5],
            Instr::Srem(Reg(11), Reg(12), Reg(13))
        ));
        assert!(matches!(
            program.instructions[6],
            Instr::SextW(Reg(14), Reg(15))
        ));
        assert!(matches!(
            program.instructions[7],
            Instr::ZextB(Reg(16), Reg(17))
        ));
        assert!(matches!(
            program.instructions[8],
            Instr::Clz(Reg(18), Reg(19))
        ));
        assert!(matches!(
            program.instructions[9],
            Instr::Popcnt(Reg(20), Reg(21))
        ));
        assert!(matches!(
            program.instructions[10],
            Instr::Rol(Reg(22), Reg(23), Reg(24))
        ));
        assert!(matches!(
            program.instructions[11],
            Instr::Bswap64(Reg(25), Reg(26))
        ));
        assert!(matches!(
            program.instructions[12],
            Instr::Slt(Reg(27), Reg(28), Reg(29))
        ));
        assert!(matches!(
            program.instructions[13],
            Instr::LrD(Reg(30), Reg(1))
        ));
        assert!(matches!(
            program.instructions[14],
            Instr::ScD(Reg(4), Reg(5), Reg(6))
        ));
        assert!(matches!(program.instructions[15], Instr::Fence));
    }

    #[test]
    fn parses_dma_ctl_instruction() {
        let program = Program::parse(
            r#"
            .text
              DMA_CTL r1, r2
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::DmaCtl(Reg(1), Reg(2))
        ));
    }

    #[test]
    fn rejects_raw_hardware_and_syscall_escape_opcodes() {
        for opcode in [
            "INT",
            "IRQ",
            "RAW_IRQ",
            "INTERRUPT",
            "MMIO",
            "RAW_MMIO",
            "MMIO_LOAD",
            "MMIO_STORE",
            "PHYS_LOAD",
            "PHYS_STORE",
            "RAW_DMA",
            "ECALL",
            "SVC",
            "RAW_SYSCALL",
            "SYSCALL",
            "syscall",
        ] {
            let source = format!(".text\n  {opcode} r1, r2\n");
            let err = Program::parse(&source).unwrap_err();
            assert!(
                err.contains("unknown instruction"),
                "unexpected error for {opcode}: {err}"
            );
        }
    }

    #[test]
    fn parses_capability_control_instructions() {
        let program = Program::parse(
            r#"
            .text
              CAP_DUP r1, r2
              CAP_REVOKE r3, r4
              CAP_SEND r5, r6
              CAP_RECV r7, r8
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::CapDup(Reg(1), Reg(2))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::CapRevoke(Reg(3), Reg(4))
        ));
        assert!(matches!(
            program.instructions[2],
            Instr::CapSend(Reg(5), Reg(6))
        ));
        assert!(matches!(
            program.instructions[3],
            Instr::CapRecv(Reg(7), Reg(8))
        ));
    }

    #[test]
    fn parses_allocator_metadata_instructions() {
        let program = Program::parse(
            r#"
            .text
              ALLOC_EX r1, r2, r3
              ALLOC_SIZE r4, r5
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::AllocEx(Reg(1), Reg(2), Reg(3))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::AllocSize(Reg(4), Reg(5))
        ));
    }

    #[test]
    fn parses_random_instruction() {
        let program = Program::parse(
            r#"
            .text
              ENV_GET r4, r5, r6, r7
              RANDOM r1, r2, r3
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::EnvGet(Reg(4), Reg(5), Reg(6), Reg(7))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::Random(Reg(1), Reg(2), Reg(3))
        ));
    }

    #[test]
    fn parses_thread_join_instruction() {
        let program = Program::parse(
            r#"
            .text
              EXEC r9, r10
              EXEC r11, r12, r13
              THREAD_JOIN r1, r2, r3
              THREAD_DETACH r4, r5
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::Exec(Reg(9), Reg(10), Reg(0))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::Exec(Reg(11), Reg(12), Reg(13))
        ));
        assert!(matches!(
            program.instructions[2],
            Instr::ThreadJoin(Reg(1), Reg(2), Reg(3))
        ));
        assert!(matches!(
            program.instructions[3],
            Instr::ThreadDetach(Reg(4), Reg(5))
        ));
    }

    #[test]
    fn parses_alarm_instruction() {
        let program = Program::parse(
            r#"
            .text
              ALARM r1, r2
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::Alarm(Reg(1), Reg(2))
        ));
    }
}
