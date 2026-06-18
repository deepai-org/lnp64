const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const EI_VERSION: usize = 6;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;
const ET_EXEC: u16 = 2;
const ET_DYN: u16 = 3;
const EM_LNP64: u16 = 0x6c64;
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;
const PAGE_SIZE: u64 = 4096;
const ELF64_EHDR_SIZE: usize = 64;
const ELF64_PHDR_SIZE: usize = 56;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecPlan {
    pub version: u64,
    pub entry: ExecEntry,
    pub vmas: Vec<VmaRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecEntry {
    pub entry_pc: u64,
    pub initial_sp: u64,
    pub tls_base: u64,
    pub startup_metadata_ptr: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmaRecord {
    pub virtual_address: u64,
    pub length: u64,
    pub protection: VmaProtection,
    pub memory_type: MemoryType,
    pub executable_provenance: ExecutableProvenance,
    pub source_offset: u64,
    pub source_length: u64,
    pub zero_fill_length: u64,
    pub mapping_flags: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmaProtection {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryType {
    Image,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutableProvenance {
    ImageText,
    NonExecutable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoaderOptions {
    pub initial_sp: u64,
    pub tls_base: u64,
    pub startup_metadata_ptr: u64,
    pub allow_wx: bool,
}

impl Default for LoaderOptions {
    fn default() -> Self {
        Self {
            initial_sp: 0,
            tls_base: 0,
            startup_metadata_ptr: 0,
            allow_wx: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ProgramHeader {
    typ: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
    align: u64,
}

pub fn build_static_exec_plan(image: &[u8], options: LoaderOptions) -> Result<ExecPlan, String> {
    if image.len() < ELF64_EHDR_SIZE {
        return Err("ELF header is truncated".to_string());
    }
    if image.get(0..4) != Some(b"\x7fELF") {
        return Err("ELF magic is invalid".to_string());
    }
    if image[EI_CLASS] != ELFCLASS64 {
        return Err("ELF class is not ELF64".to_string());
    }
    if image[EI_DATA] != ELFDATA2LSB {
        return Err("ELF data encoding is not little-endian".to_string());
    }
    if image[EI_VERSION] != EV_CURRENT {
        return Err("ELF version is unsupported".to_string());
    }

    let e_type = read_u16(image, 16)?;
    if e_type != ET_EXEC && e_type != ET_DYN {
        return Err("ELF type is not a static executable profile".to_string());
    }
    if read_u16(image, 18)? != EM_LNP64 {
        return Err("ELF machine is not EM_LNP64".to_string());
    }
    if read_u32(image, 20)? != u32::from(EV_CURRENT) {
        return Err("ELF header version is unsupported".to_string());
    }

    let entry_pc = read_u64(image, 24)?;
    let phoff = checked_usize(read_u64(image, 32)?, "program-header offset")?;
    let phentsize = usize::from(read_u16(image, 54)?);
    let phnum = usize::from(read_u16(image, 56)?);
    if phentsize != ELF64_PHDR_SIZE {
        return Err("ELF program-header entry size is unsupported".to_string());
    }
    let ph_table_len = phentsize
        .checked_mul(phnum)
        .ok_or_else(|| "ELF program-header table length overflows".to_string())?;
    let ph_table_end = phoff
        .checked_add(ph_table_len)
        .ok_or_else(|| "ELF program-header table end overflows".to_string())?;
    if ph_table_end > image.len() {
        return Err("ELF program-header table is truncated".to_string());
    }

    let mut vmas = Vec::new();
    for idx in 0..phnum {
        let base = phoff + idx * phentsize;
        let ph = read_program_header(image, base)?;
        match ph.typ {
            PT_LOAD => vmas.push(vma_from_load(image, ph, options.allow_wx)?),
            PT_DYNAMIC => return Err("PT_DYNAMIC is unsupported by the static loader".to_string()),
            _ => {}
        }
    }
    if vmas.is_empty() {
        return Err("ELF image has no PT_LOAD segments".to_string());
    }
    reject_overlapping_vmas(&vmas)?;
    if !vmas.iter().any(|vma| {
        vma.protection.execute
            && entry_pc >= vma.virtual_address
            && entry_pc < vma.virtual_address.saturating_add(vma.length)
    }) {
        return Err("ELF entry point is not inside an executable PT_LOAD segment".to_string());
    }

    Ok(ExecPlan {
        version: 1,
        entry: ExecEntry {
            entry_pc,
            initial_sp: options.initial_sp,
            tls_base: options.tls_base,
            startup_metadata_ptr: options.startup_metadata_ptr,
        },
        vmas,
    })
}

fn vma_from_load(image: &[u8], ph: ProgramHeader, allow_wx: bool) -> Result<VmaRecord, String> {
    if ph.flags & !(PF_R | PF_W | PF_X) != 0 {
        return Err("PT_LOAD has unsupported permission flags".to_string());
    }
    if ph.memsz == 0 {
        return Err("PT_LOAD memory size is zero".to_string());
    }
    if ph.filesz > ph.memsz {
        return Err("PT_LOAD file size exceeds memory size".to_string());
    }
    if ph.align != 0 && (ph.align < PAGE_SIZE || !ph.align.is_power_of_two()) {
        return Err("PT_LOAD alignment is not page-sized power-of-two".to_string());
    }
    let file_end = ph
        .offset
        .checked_add(ph.filesz)
        .ok_or_else(|| "PT_LOAD file range overflows".to_string())?;
    if checked_usize(file_end, "PT_LOAD file end")? > image.len() {
        return Err("PT_LOAD file range is truncated".to_string());
    }
    ph.vaddr
        .checked_add(ph.memsz)
        .ok_or_else(|| "PT_LOAD virtual range overflows".to_string())?;

    let write = ph.flags & PF_W != 0;
    let execute = ph.flags & PF_X != 0;
    if write && execute && !allow_wx {
        return Err("PT_LOAD requests writable executable mapping".to_string());
    }

    Ok(VmaRecord {
        virtual_address: ph.vaddr,
        length: ph.memsz,
        protection: VmaProtection {
            read: ph.flags & (PF_R | PF_W | PF_X) != 0,
            write,
            execute,
        },
        memory_type: MemoryType::Image,
        executable_provenance: if execute {
            ExecutableProvenance::ImageText
        } else {
            ExecutableProvenance::NonExecutable
        },
        source_offset: ph.offset,
        source_length: ph.filesz,
        zero_fill_length: ph.memsz - ph.filesz,
        mapping_flags: 0,
    })
}

fn reject_overlapping_vmas(vmas: &[VmaRecord]) -> Result<(), String> {
    for (idx, left) in vmas.iter().enumerate() {
        let left_end = left
            .virtual_address
            .checked_add(left.length)
            .ok_or_else(|| "VMA range overflows".to_string())?;
        for right in &vmas[idx + 1..] {
            let right_end = right
                .virtual_address
                .checked_add(right.length)
                .ok_or_else(|| "VMA range overflows".to_string())?;
            if left.virtual_address < right_end && right.virtual_address < left_end {
                return Err("PT_LOAD virtual ranges overlap".to_string());
            }
        }
    }
    Ok(())
}

fn read_program_header(image: &[u8], base: usize) -> Result<ProgramHeader, String> {
    Ok(ProgramHeader {
        typ: read_u32(image, base)?,
        flags: read_u32(image, base + 4)?,
        offset: read_u64(image, base + 8)?,
        vaddr: read_u64(image, base + 16)?,
        filesz: read_u64(image, base + 32)?,
        memsz: read_u64(image, base + 40)?,
        align: read_u64(image, base + 48)?,
    })
}

fn checked_usize(value: u64, field: &str) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| format!("{field} exceeds host usize"))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let field = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "ELF field is truncated".to_string())?;
    Ok(u16::from_le_bytes([field[0], field[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let field = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "ELF field is truncated".to_string())?;
    Ok(u32::from_le_bytes([field[0], field[1], field[2], field[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let field = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| "ELF field is truncated".to_string())?;
    Ok(u64::from_le_bytes([
        field[0], field[1], field[2], field[3], field[4], field[5], field[6], field[7],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct TestPhdr {
        typ: u32,
        flags: u32,
        offset: u64,
        vaddr: u64,
        filesz: u64,
        memsz: u64,
        align: u64,
    }

    #[test]
    fn static_elf_loader_builds_bounded_exec_plan() {
        let image = test_elf(&[
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_X,
                offset: 0x100,
                vaddr: 0x400000,
                filesz: 16,
                memsz: 16,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x402000,
                filesz: 8,
                memsz: 24,
                align: PAGE_SIZE,
            },
        ]);
        let plan = build_static_exec_plan(
            &image,
            LoaderOptions {
                initial_sp: 0x700000,
                tls_base: 0x710000,
                startup_metadata_ptr: 0x720000,
                allow_wx: false,
            },
        )
        .unwrap();

        assert_eq!(plan.version, 1);
        assert_eq!(plan.entry.entry_pc, 0x400000);
        assert_eq!(plan.entry.initial_sp, 0x700000);
        assert_eq!(plan.vmas.len(), 2);
        assert_eq!(
            plan.vmas[0].protection,
            VmaProtection {
                read: true,
                write: false,
                execute: true
            }
        );
        assert_eq!(
            plan.vmas[0].executable_provenance,
            ExecutableProvenance::ImageText
        );
        assert_eq!(
            plan.vmas[1].protection,
            VmaProtection {
                read: true,
                write: true,
                execute: false
            }
        );
        assert_eq!(plan.vmas[1].zero_fill_length, 16);
    }

    #[test]
    fn static_elf_loader_rejects_wrong_machine() {
        let mut image = test_elf(&[text_phdr()]);
        put_u16(&mut image, 18, 0x3e);
        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();
        assert!(err.contains("EM_LNP64"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_dynamic_segment() {
        let image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_DYNAMIC,
                flags: PF_R,
                offset: 0x200,
                vaddr: 0x402000,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
        ]);
        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();
        assert!(err.contains("PT_DYNAMIC"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_writable_executable_loads() {
        let image = test_elf(&[TestPhdr {
            flags: PF_R | PF_W | PF_X,
            ..text_phdr()
        }]);
        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();
        assert!(err.contains("writable executable"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_overlapping_loads() {
        let image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R,
                offset: 0x200,
                vaddr: 0x400008,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
        ]);
        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();
        assert!(err.contains("overlap"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_entry_outside_executable_segment() {
        let mut image = test_elf(&[text_phdr()]);
        put_u64(&mut image, 24, 0x402000);
        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();
        assert!(err.contains("entry point"), "{err}");
    }

    fn text_phdr() -> TestPhdr {
        TestPhdr {
            typ: PT_LOAD,
            flags: PF_R | PF_X,
            offset: 0x100,
            vaddr: 0x400000,
            filesz: 16,
            memsz: 16,
            align: PAGE_SIZE,
        }
    }

    fn test_elf(phdrs: &[TestPhdr]) -> Vec<u8> {
        let phoff = ELF64_EHDR_SIZE;
        let mut image = vec![0; 0x300];
        image[0..4].copy_from_slice(b"\x7fELF");
        image[EI_CLASS] = ELFCLASS64;
        image[EI_DATA] = ELFDATA2LSB;
        image[EI_VERSION] = EV_CURRENT;
        put_u16(&mut image, 16, ET_EXEC);
        put_u16(&mut image, 18, EM_LNP64);
        put_u32(&mut image, 20, u32::from(EV_CURRENT));
        put_u64(&mut image, 24, 0x400000);
        put_u64(&mut image, 32, phoff as u64);
        put_u16(&mut image, 52, ELF64_EHDR_SIZE as u16);
        put_u16(&mut image, 54, ELF64_PHDR_SIZE as u16);
        put_u16(&mut image, 56, phdrs.len() as u16);

        for (idx, phdr) in phdrs.iter().enumerate() {
            let base = phoff + idx * ELF64_PHDR_SIZE;
            put_u32(&mut image, base, phdr.typ);
            put_u32(&mut image, base + 4, phdr.flags);
            put_u64(&mut image, base + 8, phdr.offset);
            put_u64(&mut image, base + 16, phdr.vaddr);
            put_u64(&mut image, base + 32, phdr.filesz);
            put_u64(&mut image, base + 40, phdr.memsz);
            put_u64(&mut image, base + 48, phdr.align);
            let start = phdr.offset as usize;
            let end = start + phdr.filesz as usize;
            for byte in &mut image[start..end] {
                *byte = 0xcc;
            }
        }
        image
    }

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
