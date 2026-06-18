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
const PT_INTERP: u32 = 3;
const PT_NOTE: u32 = 4;
const PT_PHDR: u32 = 6;
const PT_TLS: u32 = 7;
const SHT_RELA: u32 = 4;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;
const R_LNP64_NONE: u32 = 0;
const R_LNP64_ABS64: u32 = 1;
const R_LNP64_ABS32: u32 = 2;
const R_LNP64_GLOB_DAT: u32 = 6;
const R_LNP64_RELATIVE: u32 = 7;
const R_LNP64_TLS_TPREL64: u32 = 8;
const R_LNP64_TLS_DTPREL64: u32 = 9;
const R_LNP64_FDR_DESC64: u32 = 10;
const PAGE_SIZE: u64 = 4096;
const ELF64_EHDR_SIZE: usize = 64;
const ELF64_PHDR_SIZE: usize = 56;
const ELF64_SHDR_SIZE: usize = 64;
const ELF64_RELA_SIZE: usize = 24;
const LNP64_STARTUP_NOTE_MAGIC: &[u8; 8] = b"LNP64ST\0";
const STARTUP_NOTE_HEADER_SIZE: usize = 64;
const STARTUP_FDR_RECORD_SIZE: usize = 64;
const MAX_STARTUP_FDRS: usize = 256;
const MAX_EXEC_PLAN_VMAS: usize = 256;
const MAX_EXEC_PLAN_MEASUREMENTS: usize = 64;
const EXEC_PLAN_HEADER_RECORD_SIZE: u64 = 72;
const EXEC_PLAN_ENTRY_RECORD_SIZE: u64 = 32;
const EXEC_PLAN_VMA_RECORD_SIZE: u64 = 88;
const EXEC_PLAN_FDR_GRANT_RECORD_SIZE: u64 = 64;
const EXEC_PLAN_MEASUREMENT_RECORD_SIZE: u64 = 32;
const VMA_PROT_READ: u64 = 1 << 0;
const VMA_PROT_WRITE: u64 = 1 << 1;
const VMA_PROT_EXECUTE: u64 = 1 << 2;
const MEMORY_TYPE_IMAGE: u64 = 1;
const EXECUTABLE_PROVENANCE_IMAGE_TEXT: u64 = 1;
const EXECUTABLE_PROVENANCE_NON_EXECUTABLE: u64 = 2;
const STARTUP_FDR_FLAG_CLOSE_ON_EXEC: u64 = 1 << 0;
const STARTUP_FDR_FLAG_PRESERVE: u64 = 1 << 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecPlan {
    pub version: u64,
    pub entry: ExecEntry,
    pub vmas: Vec<VmaRecord>,
    pub phdr: Option<PhdrDescriptor>,
    pub tls: Option<TlsDescriptor>,
    pub startup: Option<StartupDescriptor>,
    pub fdr_grants: Vec<StartupFdrDescriptor>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedVma {
    pub virtual_address: u64,
    pub protection: VmaProtection,
    pub executable_provenance: ExecutableProvenance,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhdrDescriptor {
    pub virtual_address: u64,
    pub source_offset: u64,
    pub byte_len: u64,
    pub entry_size: u64,
    pub entry_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlsDescriptor {
    pub virtual_address: u64,
    pub source_offset: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub alignment: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecPlanDescriptor {
    pub header: ExecPlanHeader,
    pub entry: ExecEntry,
    pub vmas: Vec<ExecPlanVmaDescriptor>,
    pub fdr_grants: Vec<ExecPlanFdrGrantDescriptor>,
    pub measurements: Vec<ExecPlanMeasurementDescriptor>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecPlanHeader {
    pub version: u64,
    pub total_length: u64,
    pub flags: u64,
    pub vma_count: u64,
    pub fdr_count: u64,
    pub measurement_count: u64,
    pub expected_domain_generation: u64,
    pub expected_process_generation: u64,
    pub expected_lineage_epoch: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecPlanVmaDescriptor {
    pub virtual_address: u64,
    pub length: u64,
    pub protection: u64,
    pub memory_type: u64,
    pub executable_provenance: u64,
    pub source_cap: u64,
    pub source_offset: u64,
    pub source_generation: u64,
    pub lineage_epoch: u64,
    pub zero_fill_length: u64,
    pub mapping_flags: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecPlanFdrGrantDescriptor {
    pub slot: u64,
    pub kind: u64,
    pub rights: u64,
    pub flags: u64,
    pub source_cap: u64,
    pub source_generation: u64,
    pub close_on_exec: u64,
    pub preserve: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecPlanMeasurementDescriptor {
    pub algorithm: u64,
    pub measurement_ref: u64,
    pub manifest_ref: u64,
    pub attestation_ref: u64,
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
pub struct StartupDescriptor {
    pub flags: u64,
    pub argc_addr: u64,
    pub argv_addr: u64,
    pub envp_addr: u64,
    pub auxv_addr: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StartupFdrDescriptor {
    pub slot: u64,
    pub kind: u64,
    pub rights: u64,
    pub flags: u64,
    pub object_id: u64,
    pub generation: u64,
    pub name_offset: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoaderOptions {
    pub initial_sp: u64,
    pub tls_base: u64,
    pub startup_metadata_ptr: u64,
    pub allow_wx: bool,
    pub load_bias: u64,
}

impl Default for LoaderOptions {
    fn default() -> Self {
        Self {
            initial_sp: 0,
            tls_base: 0,
            startup_metadata_ptr: 0,
            allow_wx: false,
            load_bias: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecPlanDescriptorOptions {
    pub flags: u64,
    pub expected_domain_generation: u64,
    pub expected_process_generation: u64,
    pub expected_lineage_epoch: u64,
    pub image_source_cap: u64,
    pub image_source_generation: u64,
    pub image_lineage_epoch: u64,
    pub measurements: Vec<ExecPlanMeasurementDescriptor>,
}

impl Default for ExecPlanDescriptorOptions {
    fn default() -> Self {
        Self {
            flags: 0,
            expected_domain_generation: 0,
            expected_process_generation: 0,
            expected_lineage_epoch: 0,
            image_source_cap: 0,
            image_source_generation: 0,
            image_lineage_epoch: 0,
            measurements: Vec::new(),
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

pub fn load_static_elf(image: &mut [u8], options: LoaderOptions) -> Result<ExecPlan, String> {
    let plan = build_static_exec_plan(image, options)?;
    apply_rela_sections(image, &plan, options.load_bias)?;
    Ok(plan)
}

pub fn materialize_vmas(image: &[u8], plan: &ExecPlan) -> Result<Vec<PreparedVma>, String> {
    let mut prepared = Vec::with_capacity(plan.vmas.len());
    for vma in &plan.vmas {
        let length = checked_usize(vma.length, "VMA length")?;
        let source_offset = checked_usize(vma.source_offset, "VMA source offset")?;
        let source_length = checked_usize(vma.source_length, "VMA source length")?;
        if vma.source_length > vma.length {
            return Err("VMA source length exceeds VMA length".to_string());
        }
        let source_end = source_offset
            .checked_add(source_length)
            .ok_or_else(|| "VMA source range overflows".to_string())?;
        if source_end > image.len() {
            return Err("VMA source range is truncated".to_string());
        }
        let mut bytes = vec![0; length];
        bytes[..source_length].copy_from_slice(&image[source_offset..source_end]);
        prepared.push(PreparedVma {
            virtual_address: vma.virtual_address,
            protection: vma.protection,
            executable_provenance: vma.executable_provenance,
            bytes,
        });
    }
    Ok(prepared)
}

pub fn build_exec_descriptor(
    plan: &ExecPlan,
    options: ExecPlanDescriptorOptions,
) -> Result<ExecPlanDescriptor, String> {
    if plan.vmas.len() > MAX_EXEC_PLAN_VMAS {
        return Err("exec-plan VMA count exceeds architectural limit".to_string());
    }
    if plan.fdr_grants.len() > MAX_STARTUP_FDRS {
        return Err("exec-plan FDR grant count exceeds architectural limit".to_string());
    }
    if options.measurements.len() > MAX_EXEC_PLAN_MEASUREMENTS {
        return Err("exec-plan measurement count exceeds architectural limit".to_string());
    }
    let total_length = exec_descriptor_total_length(
        plan.vmas.len(),
        plan.fdr_grants.len(),
        options.measurements.len(),
    )?;
    let vmas = plan
        .vmas
        .iter()
        .map(|vma| ExecPlanVmaDescriptor {
            virtual_address: vma.virtual_address,
            length: vma.length,
            protection: protection_bits(vma.protection),
            memory_type: memory_type_id(vma.memory_type),
            executable_provenance: executable_provenance_id(vma.executable_provenance),
            source_cap: options.image_source_cap,
            source_offset: vma.source_offset,
            source_generation: options.image_source_generation,
            lineage_epoch: options.image_lineage_epoch,
            zero_fill_length: vma.zero_fill_length,
            mapping_flags: vma.mapping_flags,
        })
        .collect();
    let fdr_grants = plan
        .fdr_grants
        .iter()
        .map(|grant| ExecPlanFdrGrantDescriptor {
            slot: grant.slot,
            kind: grant.kind,
            rights: grant.rights,
            flags: grant.flags,
            source_cap: grant.object_id,
            source_generation: grant.generation,
            close_on_exec: u64::from(grant.flags & STARTUP_FDR_FLAG_CLOSE_ON_EXEC != 0),
            preserve: u64::from(grant.flags & STARTUP_FDR_FLAG_PRESERVE != 0),
        })
        .collect();

    Ok(ExecPlanDescriptor {
        header: ExecPlanHeader {
            version: plan.version,
            total_length,
            flags: options.flags,
            vma_count: plan.vmas.len() as u64,
            fdr_count: plan.fdr_grants.len() as u64,
            measurement_count: options.measurements.len() as u64,
            expected_domain_generation: options.expected_domain_generation,
            expected_process_generation: options.expected_process_generation,
            expected_lineage_epoch: options.expected_lineage_epoch,
        },
        entry: plan.entry,
        vmas,
        fdr_grants,
        measurements: options.measurements,
    })
}

pub fn encode_exec_descriptor(descriptor: &ExecPlanDescriptor) -> Vec<u64> {
    let mut words = Vec::with_capacity(
        13 + descriptor.vmas.len() * 11
            + descriptor.fdr_grants.len() * 8
            + descriptor.measurements.len() * 4,
    );
    words.extend_from_slice(&[
        descriptor.header.version,
        descriptor.header.total_length,
        descriptor.header.flags,
        descriptor.header.vma_count,
        descriptor.header.fdr_count,
        descriptor.header.measurement_count,
        descriptor.header.expected_domain_generation,
        descriptor.header.expected_process_generation,
        descriptor.header.expected_lineage_epoch,
        descriptor.entry.entry_pc,
        descriptor.entry.initial_sp,
        descriptor.entry.tls_base,
        descriptor.entry.startup_metadata_ptr,
    ]);
    for vma in &descriptor.vmas {
        words.extend_from_slice(&[
            vma.virtual_address,
            vma.length,
            vma.protection,
            vma.memory_type,
            vma.executable_provenance,
            vma.source_cap,
            vma.source_offset,
            vma.source_generation,
            vma.lineage_epoch,
            vma.zero_fill_length,
            vma.mapping_flags,
        ]);
    }
    for grant in &descriptor.fdr_grants {
        words.extend_from_slice(&[
            grant.slot,
            grant.kind,
            grant.rights,
            grant.flags,
            grant.source_cap,
            grant.source_generation,
            grant.close_on_exec,
            grant.preserve,
        ]);
    }
    for measurement in &descriptor.measurements {
        words.extend_from_slice(&[
            measurement.algorithm,
            measurement.measurement_ref,
            measurement.manifest_ref,
            measurement.attestation_ref,
        ]);
    }
    words
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
    if e_type == ET_EXEC && options.load_bias != 0 {
        return Err("ET_EXEC images cannot be loaded with a nonzero load bias".to_string());
    }
    if read_u16(image, 18)? != EM_LNP64 {
        return Err("ELF machine is not EM_LNP64".to_string());
    }
    if read_u32(image, 20)? != u32::from(EV_CURRENT) {
        return Err("ELF header version is unsupported".to_string());
    }

    let entry_pc = read_u64(image, 24)?
        .checked_add(options.load_bias)
        .ok_or_else(|| "ELF entry point plus load bias overflows".to_string())?;
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
    let mut phdr = None;
    let mut tls = None;
    let mut startup = None;
    let mut fdr_grants = Vec::new();
    for idx in 0..phnum {
        let base = phoff + idx * phentsize;
        let ph = read_program_header(image, base)?;
        match ph.typ {
            PT_LOAD => vmas.push(vma_from_load(
                image,
                ph,
                options.allow_wx,
                options.load_bias,
            )?),
            PT_DYNAMIC => return Err("PT_DYNAMIC is unsupported by the static loader".to_string()),
            PT_INTERP => return Err("PT_INTERP is unsupported by the static loader".to_string()),
            PT_NOTE => parse_startup_note_segment(image, ph, &mut startup, &mut fdr_grants)?,
            PT_PHDR => parse_phdr_segment(
                image,
                ph,
                phoff,
                phentsize,
                phnum,
                options.load_bias,
                &mut phdr,
            )?,
            PT_TLS => parse_tls_segment(image, ph, options.load_bias, &mut tls)?,
            _ => {}
        }
    }
    if vmas.is_empty() {
        return Err("ELF image has no PT_LOAD segments".to_string());
    }
    reject_overlapping_vmas(&vmas)?;
    validate_metadata_ranges(&vmas, phdr, tls)?;
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
        phdr,
        tls,
        startup,
        fdr_grants,
    })
}

fn vma_from_load(
    image: &[u8],
    ph: ProgramHeader,
    allow_wx: bool,
    load_bias: u64,
) -> Result<VmaRecord, String> {
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
    if ph.align != 0 && load_bias % ph.align != 0 {
        return Err("PT_LOAD load bias does not preserve segment alignment".to_string());
    }
    let file_end = ph
        .offset
        .checked_add(ph.filesz)
        .ok_or_else(|| "PT_LOAD file range overflows".to_string())?;
    if checked_usize(file_end, "PT_LOAD file end")? > image.len() {
        return Err("PT_LOAD file range is truncated".to_string());
    }
    let virtual_address = ph
        .vaddr
        .checked_add(load_bias)
        .ok_or_else(|| "PT_LOAD virtual address plus load bias overflows".to_string())?;
    virtual_address
        .checked_add(ph.memsz)
        .ok_or_else(|| "PT_LOAD virtual range overflows".to_string())?;

    let write = ph.flags & PF_W != 0;
    let execute = ph.flags & PF_X != 0;
    if write && execute && !allow_wx {
        return Err("PT_LOAD requests writable executable mapping".to_string());
    }

    Ok(VmaRecord {
        virtual_address,
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

fn apply_rela_sections(image: &mut [u8], plan: &ExecPlan, load_bias: u64) -> Result<(), String> {
    let shoff = read_u64(image, 40)?;
    let shentsize = usize::from(read_u16(image, 58)?);
    let shnum = usize::from(read_u16(image, 60)?);
    if shoff == 0 || shnum == 0 {
        return Ok(());
    }
    if shentsize != ELF64_SHDR_SIZE {
        return Err("ELF section-header entry size is unsupported".to_string());
    }
    let shoff = checked_usize(shoff, "section-header offset")?;
    let sh_table_len = shentsize
        .checked_mul(shnum)
        .ok_or_else(|| "ELF section-header table length overflows".to_string())?;
    let sh_table_end = shoff
        .checked_add(sh_table_len)
        .ok_or_else(|| "ELF section-header table end overflows".to_string())?;
    if sh_table_end > image.len() {
        return Err("ELF section-header table is truncated".to_string());
    }

    for idx in 0..shnum {
        let base = shoff + idx * shentsize;
        if read_u32(image, base + 4)? != SHT_RELA {
            continue;
        }
        let rela_offset = checked_usize(read_u64(image, base + 24)?, "RELA section offset")?;
        let rela_size = checked_usize(read_u64(image, base + 32)?, "RELA section size")?;
        let rela_entsize = checked_usize(read_u64(image, base + 56)?, "RELA entry size")?;
        if rela_entsize != ELF64_RELA_SIZE {
            return Err("RELA entry size is unsupported".to_string());
        }
        if rela_size % ELF64_RELA_SIZE != 0 {
            return Err("RELA section size is not entry-aligned".to_string());
        }
        let rela_end = rela_offset
            .checked_add(rela_size)
            .ok_or_else(|| "RELA section range overflows".to_string())?;
        if rela_end > image.len() {
            return Err("RELA section is truncated".to_string());
        }

        for entry in (rela_offset..rela_end).step_by(ELF64_RELA_SIZE) {
            let r_offset = read_u64(image, entry)?;
            let r_info = read_u64(image, entry + 8)?;
            let r_addend = read_i64(image, entry + 16)?;
            let reloc_type = (r_info & 0xffff_ffff) as u32;
            let symbol_index = r_info >> 32;
            match reloc_type {
                R_LNP64_NONE => {}
                R_LNP64_ABS64 | R_LNP64_GLOB_DAT => {
                    let name = if reloc_type == R_LNP64_ABS64 {
                        "ABS64"
                    } else {
                        "GLOB_DAT"
                    };
                    if symbol_index != 0 {
                        return Err(format!(
                            "RELA {name} with symbol index is unsupported by static loader"
                        ));
                    }
                    let target = r_offset
                        .checked_add(load_bias)
                        .ok_or_else(|| "RELA target plus load bias overflows".to_string())?;
                    let value = u64::try_from(i128::from(r_addend))
                        .map_err(|_| format!("RELA {name} value is out of range"))?;
                    let file_offset = relocation_file_offset(plan, target, 8)?;
                    image[file_offset..file_offset + 8].copy_from_slice(&value.to_le_bytes());
                }
                R_LNP64_ABS32 => {
                    if symbol_index != 0 {
                        return Err(
                            "RELA ABS32 with symbol index is unsupported by static loader"
                                .to_string(),
                        );
                    }
                    let target = r_offset
                        .checked_add(load_bias)
                        .ok_or_else(|| "RELA target plus load bias overflows".to_string())?;
                    let value = u32::try_from(i128::from(r_addend))
                        .map_err(|_| "RELA ABS32 value is out of range".to_string())?;
                    let file_offset = relocation_file_offset(plan, target, 4)?;
                    image[file_offset..file_offset + 4].copy_from_slice(&value.to_le_bytes());
                }
                R_LNP64_TLS_TPREL64 | R_LNP64_TLS_DTPREL64 => {
                    let name = if reloc_type == R_LNP64_TLS_TPREL64 {
                        "TLS_TPREL64"
                    } else {
                        "TLS_DTPREL64"
                    };
                    if symbol_index != 0 {
                        return Err(format!(
                            "RELA {name} with symbol index is unsupported by static loader"
                        ));
                    }
                    let tls = plan
                        .tls
                        .ok_or_else(|| format!("RELA {name} requires PT_TLS segment"))?;
                    let value = u64::try_from(i128::from(r_addend))
                        .map_err(|_| format!("RELA {name} value is out of range"))?;
                    if value > tls.memory_size {
                        return Err(format!("RELA {name} offset exceeds PT_TLS memory size"));
                    }
                    let target = r_offset
                        .checked_add(load_bias)
                        .ok_or_else(|| "RELA target plus load bias overflows".to_string())?;
                    let file_offset = relocation_file_offset(plan, target, 8)?;
                    image[file_offset..file_offset + 8].copy_from_slice(&value.to_le_bytes());
                }
                R_LNP64_FDR_DESC64 => {
                    if symbol_index != 0 {
                        return Err(
                            "RELA FDR_DESC64 with symbol index is unsupported by static loader"
                                .to_string(),
                        );
                    }
                    let index = u64::try_from(i128::from(r_addend))
                        .map_err(|_| "RELA FDR_DESC64 value is out of range".to_string())?;
                    if usize::try_from(index)
                        .ok()
                        .is_none_or(|idx| idx >= plan.fdr_grants.len())
                    {
                        return Err("RELA FDR_DESC64 index exceeds startup FDR table".to_string());
                    }
                    let target = r_offset
                        .checked_add(load_bias)
                        .ok_or_else(|| "RELA target plus load bias overflows".to_string())?;
                    let file_offset = relocation_file_offset(plan, target, 8)?;
                    image[file_offset..file_offset + 8].copy_from_slice(&index.to_le_bytes());
                }
                R_LNP64_RELATIVE => {
                    if symbol_index != 0 {
                        return Err("RELA relative relocation must not name a symbol".to_string());
                    }
                    let target = r_offset
                        .checked_add(load_bias)
                        .ok_or_else(|| "RELA target plus load bias overflows".to_string())?;
                    let value = i128::from(load_bias) + i128::from(r_addend);
                    let value = u64::try_from(value)
                        .map_err(|_| "RELA relative value is out of range".to_string())?;
                    let file_offset = relocation_file_offset(plan, target, 8)?;
                    image[file_offset..file_offset + 8].copy_from_slice(&value.to_le_bytes());
                }
                other => {
                    return Err(format!("unsupported LNP64 relocation type {other}"));
                }
            }
        }
    }
    Ok(())
}

fn relocation_file_offset(plan: &ExecPlan, target: u64, width: u64) -> Result<usize, String> {
    let target_end = target
        .checked_add(width)
        .ok_or_else(|| "RELA target range overflows".to_string())?;
    for vma in &plan.vmas {
        let file_start = vma.virtual_address;
        let file_end = vma
            .virtual_address
            .checked_add(vma.source_length)
            .ok_or_else(|| "VMA file-backed range overflows".to_string())?;
        if target >= file_start && target_end <= file_end {
            let delta = target - vma.virtual_address;
            let file_offset = vma
                .source_offset
                .checked_add(delta)
                .ok_or_else(|| "RELA file offset overflows".to_string())?;
            return checked_usize(file_offset, "RELA file offset");
        }
    }
    Err("RELA target is outside file-backed PT_LOAD data".to_string())
}

fn parse_phdr_segment(
    image: &[u8],
    ph: ProgramHeader,
    phoff: usize,
    phentsize: usize,
    phnum: usize,
    load_bias: u64,
    phdr: &mut Option<PhdrDescriptor>,
) -> Result<(), String> {
    if phdr.is_some() {
        return Err("duplicate PT_PHDR segment".to_string());
    }
    if ph.offset != phoff as u64 {
        return Err("PT_PHDR offset does not match program-header table".to_string());
    }
    let expected_len = u64::try_from(phentsize)
        .ok()
        .and_then(|entry_size| entry_size.checked_mul(phnum as u64))
        .ok_or_else(|| "PT_PHDR table length overflows".to_string())?;
    if ph.filesz != expected_len || ph.memsz != expected_len {
        return Err("PT_PHDR size does not match program-header table".to_string());
    }
    let file_end = ph
        .offset
        .checked_add(ph.filesz)
        .ok_or_else(|| "PT_PHDR file range overflows".to_string())?;
    if checked_usize(file_end, "PT_PHDR file end")? > image.len() {
        return Err("PT_PHDR file range is truncated".to_string());
    }
    let virtual_address = ph
        .vaddr
        .checked_add(load_bias)
        .ok_or_else(|| "PT_PHDR virtual address plus load bias overflows".to_string())?;
    virtual_address
        .checked_add(ph.memsz)
        .ok_or_else(|| "PT_PHDR virtual range overflows".to_string())?;

    *phdr = Some(PhdrDescriptor {
        virtual_address,
        source_offset: ph.offset,
        byte_len: ph.filesz,
        entry_size: phentsize as u64,
        entry_count: phnum as u64,
    });
    Ok(())
}

fn parse_tls_segment(
    image: &[u8],
    ph: ProgramHeader,
    load_bias: u64,
    tls: &mut Option<TlsDescriptor>,
) -> Result<(), String> {
    if tls.is_some() {
        return Err("duplicate PT_TLS segment".to_string());
    }
    if ph.memsz == 0 {
        return Err("PT_TLS memory size is zero".to_string());
    }
    if ph.filesz > ph.memsz {
        return Err("PT_TLS file size exceeds memory size".to_string());
    }
    if ph.align != 0 && !ph.align.is_power_of_two() {
        return Err("PT_TLS alignment is not a power of two".to_string());
    }
    let file_end = ph
        .offset
        .checked_add(ph.filesz)
        .ok_or_else(|| "PT_TLS file range overflows".to_string())?;
    if checked_usize(file_end, "PT_TLS file end")? > image.len() {
        return Err("PT_TLS file range is truncated".to_string());
    }
    let virtual_address = ph
        .vaddr
        .checked_add(load_bias)
        .ok_or_else(|| "PT_TLS virtual address plus load bias overflows".to_string())?;
    virtual_address
        .checked_add(ph.memsz)
        .ok_or_else(|| "PT_TLS virtual range overflows".to_string())?;

    *tls = Some(TlsDescriptor {
        virtual_address,
        source_offset: ph.offset,
        file_size: ph.filesz,
        memory_size: ph.memsz,
        alignment: ph.align,
    });
    Ok(())
}

fn exec_descriptor_total_length(
    vmas: usize,
    fdrs: usize,
    measurements: usize,
) -> Result<u64, String> {
    let vma_bytes = checked_mul_u64(vmas, EXEC_PLAN_VMA_RECORD_SIZE, "exec-plan VMA records")?;
    let fdr_bytes = checked_mul_u64(
        fdrs,
        EXEC_PLAN_FDR_GRANT_RECORD_SIZE,
        "exec-plan FDR grant records",
    )?;
    let measurement_bytes = checked_mul_u64(
        measurements,
        EXEC_PLAN_MEASUREMENT_RECORD_SIZE,
        "exec-plan measurement records",
    )?;
    EXEC_PLAN_HEADER_RECORD_SIZE
        .checked_add(EXEC_PLAN_ENTRY_RECORD_SIZE)
        .and_then(|len| len.checked_add(vma_bytes))
        .and_then(|len| len.checked_add(fdr_bytes))
        .and_then(|len| len.checked_add(measurement_bytes))
        .ok_or_else(|| "exec-plan descriptor length overflows".to_string())
}

fn checked_mul_u64(count: usize, record_size: u64, field: &str) -> Result<u64, String> {
    let count = u64::try_from(count).map_err(|_| format!("{field} count exceeds u64"))?;
    count
        .checked_mul(record_size)
        .ok_or_else(|| format!("{field} length overflows"))
}

fn protection_bits(protection: VmaProtection) -> u64 {
    let mut bits = 0;
    if protection.read {
        bits |= VMA_PROT_READ;
    }
    if protection.write {
        bits |= VMA_PROT_WRITE;
    }
    if protection.execute {
        bits |= VMA_PROT_EXECUTE;
    }
    bits
}

fn memory_type_id(memory_type: MemoryType) -> u64 {
    match memory_type {
        MemoryType::Image => MEMORY_TYPE_IMAGE,
    }
}

fn executable_provenance_id(provenance: ExecutableProvenance) -> u64 {
    match provenance {
        ExecutableProvenance::ImageText => EXECUTABLE_PROVENANCE_IMAGE_TEXT,
        ExecutableProvenance::NonExecutable => EXECUTABLE_PROVENANCE_NON_EXECUTABLE,
    }
}

fn parse_startup_note_segment(
    image: &[u8],
    ph: ProgramHeader,
    startup: &mut Option<StartupDescriptor>,
    fdr_grants: &mut Vec<StartupFdrDescriptor>,
) -> Result<(), String> {
    let note_end = ph
        .offset
        .checked_add(ph.filesz)
        .ok_or_else(|| "PT_NOTE range overflows".to_string())?;
    let note_start = checked_usize(ph.offset, "PT_NOTE offset")?;
    let note_end = checked_usize(note_end, "PT_NOTE end")?;
    if note_end > image.len() {
        return Err("PT_NOTE range is truncated".to_string());
    }
    let note = &image[note_start..note_end];
    if note.len() < LNP64_STARTUP_NOTE_MAGIC.len()
        || &note[..LNP64_STARTUP_NOTE_MAGIC.len()] != LNP64_STARTUP_NOTE_MAGIC
    {
        return Ok(());
    }
    if startup.is_some() {
        return Err("duplicate LNP64 startup note".to_string());
    }
    if note.len() < STARTUP_NOTE_HEADER_SIZE {
        return Err("LNP64 startup note is truncated".to_string());
    }
    let version = read_u64(note, 8)?;
    if version != 1 {
        return Err("LNP64 startup note version is unsupported".to_string());
    }
    let fdr_count = read_u64(note, 56)?;
    let fdr_count =
        usize::try_from(fdr_count).map_err(|_| "startup FDR count exceeds host usize")?;
    if fdr_count > MAX_STARTUP_FDRS {
        return Err("startup FDR count exceeds architectural limit".to_string());
    }
    let needed = STARTUP_NOTE_HEADER_SIZE
        .checked_add(
            fdr_count
                .checked_mul(STARTUP_FDR_RECORD_SIZE)
                .ok_or_else(|| "startup FDR table length overflows".to_string())?,
        )
        .ok_or_else(|| "startup note length overflows".to_string())?;
    if needed > note.len() {
        return Err("startup FDR table is truncated".to_string());
    }

    *startup = Some(StartupDescriptor {
        flags: read_u64(note, 16)?,
        argc_addr: read_u64(note, 24)?,
        argv_addr: read_u64(note, 32)?,
        envp_addr: read_u64(note, 40)?,
        auxv_addr: read_u64(note, 48)?,
    });

    for idx in 0..fdr_count {
        let base = STARTUP_NOTE_HEADER_SIZE + idx * STARTUP_FDR_RECORD_SIZE;
        let reserved = read_u64(note, base + 56)?;
        if reserved != 0 {
            return Err("startup FDR record reserved field is nonzero".to_string());
        }
        fdr_grants.push(StartupFdrDescriptor {
            slot: read_u64(note, base)?,
            kind: read_u64(note, base + 8)?,
            rights: read_u64(note, base + 16)?,
            flags: read_u64(note, base + 24)?,
            object_id: read_u64(note, base + 32)?,
            generation: read_u64(note, base + 40)?,
            name_offset: read_u64(note, base + 48)?,
        });
    }
    Ok(())
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

fn validate_metadata_ranges(
    vmas: &[VmaRecord],
    phdr: Option<PhdrDescriptor>,
    tls: Option<TlsDescriptor>,
) -> Result<(), String> {
    if let Some(phdr) = phdr {
        ensure_range_covered_by_vma(vmas, phdr.virtual_address, phdr.byte_len, "PT_PHDR")?;
    }
    if let Some(tls) = tls {
        ensure_range_covered_by_vma(vmas, tls.virtual_address, tls.memory_size, "PT_TLS")?;
    }
    Ok(())
}

fn ensure_range_covered_by_vma(
    vmas: &[VmaRecord],
    virtual_address: u64,
    length: u64,
    label: &str,
) -> Result<(), String> {
    let end = virtual_address
        .checked_add(length)
        .ok_or_else(|| format!("{label} virtual range overflows"))?;
    if vmas.iter().any(|vma| {
        let Some(vma_end) = vma.virtual_address.checked_add(vma.length) else {
            return false;
        };
        virtual_address >= vma.virtual_address && end <= vma_end
    }) {
        Ok(())
    } else {
        Err(format!("{label} virtual range is outside PT_LOAD segments"))
    }
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

fn read_i64(bytes: &[u8], offset: usize) -> Result<i64, String> {
    let field = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| "ELF field is truncated".to_string())?;
    Ok(i64::from_le_bytes([
        field[0], field[1], field[2], field[3], field[4], field[5], field[6], field[7],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const R_LNP64_PC32: u32 = 3;

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
                load_bias: 0,
            },
        )
        .unwrap();

        assert_eq!(plan.version, 1);
        assert_eq!(plan.entry.entry_pc, 0x400000);
        assert_eq!(plan.entry.initial_sp, 0x700000);
        assert!(plan.phdr.is_none());
        assert!(plan.tls.is_none());
        assert!(plan.startup.is_none());
        assert!(plan.fdr_grants.is_empty());
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
    fn static_elf_loader_materializes_vma_bytes_and_zero_fill() {
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
        let plan = build_static_exec_plan(&image, LoaderOptions::default()).unwrap();

        let prepared = materialize_vmas(&image, &plan).unwrap();

        assert_eq!(prepared.len(), 2);
        assert_eq!(prepared[0].virtual_address, 0x400000);
        assert_eq!(prepared[0].bytes, vec![0xcc; 16]);
        assert_eq!(
            prepared[0].executable_provenance,
            ExecutableProvenance::ImageText
        );
        assert_eq!(prepared[1].virtual_address, 0x402000);
        assert_eq!(prepared[1].bytes.len(), 24);
        assert_eq!(&prepared[1].bytes[..8], &[0xcc; 8]);
        assert_eq!(&prepared[1].bytes[8..], &[0; 16]);
        assert_eq!(
            prepared[1].protection,
            VmaProtection {
                read: true,
                write: true,
                execute: false,
            }
        );
    }

    #[test]
    fn static_elf_loader_rejects_truncated_materialization_source() {
        let image = test_elf(&[TestPhdr {
            typ: PT_LOAD,
            flags: PF_R | PF_X,
            offset: 0x100,
            vaddr: 0x400000,
            filesz: 16,
            memsz: 16,
            align: PAGE_SIZE,
        }]);
        let plan = build_static_exec_plan(&image, LoaderOptions::default()).unwrap();

        let err = materialize_vmas(&image[..0x108], &plan).unwrap_err();

        assert!(err.contains("VMA source range is truncated"), "{err}");
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
    fn static_elf_loader_rejects_interpreter_segment() {
        let image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_INTERP,
                flags: PF_R,
                offset: 0x280,
                vaddr: 0,
                filesz: 16,
                memsz: 16,
                align: 1,
            },
        ]);

        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();

        assert!(err.contains("PT_INTERP"), "{err}");
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

    #[test]
    fn static_elf_loader_rejects_rebased_exec_image() {
        let image = test_elf(&[text_phdr()]);

        let err = build_static_exec_plan(
            &image,
            LoaderOptions {
                load_bias: 0x1000,
                ..LoaderOptions::default()
            },
        )
        .unwrap_err();

        assert!(err.contains("ET_EXEC"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_misaligned_pie_load_bias() {
        let mut image = test_elf(&[text_phdr()]);
        put_u16(&mut image, 16, ET_DYN);

        let err = build_static_exec_plan(
            &image,
            LoaderOptions {
                load_bias: 0x123,
                ..LoaderOptions::default()
            },
        )
        .unwrap_err();

        assert!(err.contains("load bias"), "{err}");
    }

    #[test]
    fn static_elf_loader_parses_phdr_segment() {
        let phdrs = [
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_X,
                offset: 0x100,
                vaddr: 0x400000,
                filesz: 16,
                memsz: 0x1000,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_PHDR,
                flags: PF_R,
                offset: ELF64_EHDR_SIZE as u64,
                vaddr: 0x400100,
                filesz: (2 * ELF64_PHDR_SIZE) as u64,
                memsz: (2 * ELF64_PHDR_SIZE) as u64,
                align: 8,
            },
        ];
        let mut image = test_elf(&phdrs);
        put_u16(&mut image, 16, ET_DYN);

        let plan = build_static_exec_plan(
            &image,
            LoaderOptions {
                load_bias: 0x1000,
                ..LoaderOptions::default()
            },
        )
        .unwrap();

        assert_eq!(
            plan.phdr,
            Some(PhdrDescriptor {
                virtual_address: 0x401100,
                source_offset: ELF64_EHDR_SIZE as u64,
                byte_len: (2 * ELF64_PHDR_SIZE) as u64,
                entry_size: ELF64_PHDR_SIZE as u64,
                entry_count: 2,
            })
        );
    }

    #[test]
    fn static_elf_loader_rejects_duplicate_phdr_segments() {
        let phdrs = [
            text_phdr(),
            TestPhdr {
                typ: PT_PHDR,
                flags: PF_R,
                offset: ELF64_EHDR_SIZE as u64,
                vaddr: 0x3ff000,
                filesz: (3 * ELF64_PHDR_SIZE) as u64,
                memsz: (3 * ELF64_PHDR_SIZE) as u64,
                align: 8,
            },
            TestPhdr {
                typ: PT_PHDR,
                flags: PF_R,
                offset: ELF64_EHDR_SIZE as u64,
                vaddr: 0x3ff000,
                filesz: (3 * ELF64_PHDR_SIZE) as u64,
                memsz: (3 * ELF64_PHDR_SIZE) as u64,
                align: 8,
            },
        ];
        let image = test_elf(&phdrs);

        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();

        assert!(err.contains("duplicate PT_PHDR"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_malformed_phdr_segment() {
        let phdrs = [
            text_phdr(),
            TestPhdr {
                typ: PT_PHDR,
                flags: PF_R,
                offset: 0x280,
                vaddr: 0x3ff000,
                filesz: (2 * ELF64_PHDR_SIZE) as u64,
                memsz: (2 * ELF64_PHDR_SIZE) as u64,
                align: 8,
            },
        ];
        let image = test_elf(&phdrs);

        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();

        assert!(err.contains("PT_PHDR offset"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_phdr_outside_load_segments() {
        let phdrs = [
            text_phdr(),
            TestPhdr {
                typ: PT_PHDR,
                flags: PF_R,
                offset: ELF64_EHDR_SIZE as u64,
                vaddr: 0x500000,
                filesz: (2 * ELF64_PHDR_SIZE) as u64,
                memsz: (2 * ELF64_PHDR_SIZE) as u64,
                align: 8,
            },
        ];
        let image = test_elf(&phdrs);

        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();

        assert!(err.contains("PT_PHDR virtual range"), "{err}");
    }

    #[test]
    fn static_elf_loader_parses_tls_segment() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x280,
                vaddr: 0x500000,
                filesz: 32,
                memsz: 0x1000,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_TLS,
                flags: PF_R,
                offset: 0x280,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 24,
                align: 16,
            },
        ]);
        put_u16(&mut image, 16, ET_DYN);

        let plan = build_static_exec_plan(
            &image,
            LoaderOptions {
                load_bias: 0x1000,
                ..LoaderOptions::default()
            },
        )
        .unwrap();

        assert_eq!(
            plan.tls,
            Some(TlsDescriptor {
                virtual_address: 0x501000,
                source_offset: 0x280,
                file_size: 8,
                memory_size: 24,
                alignment: 16,
            })
        );
    }

    #[test]
    fn static_elf_loader_rejects_tls_outside_load_segments() {
        let image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_TLS,
                flags: PF_R,
                offset: 0x280,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 24,
                align: 16,
            },
        ]);

        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();

        assert!(err.contains("PT_TLS virtual range"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_duplicate_tls_segments() {
        let image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_TLS,
                flags: PF_R,
                offset: 0x280,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 24,
                align: 16,
            },
            TestPhdr {
                typ: PT_TLS,
                flags: PF_R,
                offset: 0x300,
                vaddr: 0x501000,
                filesz: 8,
                memsz: 24,
                align: 16,
            },
        ]);

        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();

        assert!(err.contains("duplicate PT_TLS"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_malformed_tls_segment() {
        let image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_TLS,
                flags: PF_R,
                offset: 0x280,
                vaddr: 0x500000,
                filesz: 24,
                memsz: 8,
                align: 16,
            },
        ]);

        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();

        assert!(
            err.contains("PT_TLS file size exceeds memory size"),
            "{err}"
        );
    }

    #[test]
    fn static_elf_loader_applies_relative_relocations_with_load_bias() {
        let mut image = test_elf(&[
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_X,
                offset: 0x100,
                vaddr: 0x1000,
                filesz: 16,
                memsz: 16,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x2000,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
        ]);
        put_u16(&mut image, 16, ET_DYN);
        put_u64(&mut image, 24, 0x1000);
        install_rela_section(&mut image, 0x2000, R_LNP64_RELATIVE, 0x55);

        let plan = load_static_elf(
            &mut image,
            LoaderOptions {
                load_bias: 0x100000,
                ..LoaderOptions::default()
            },
        )
        .unwrap();

        assert_eq!(plan.entry.entry_pc, 0x101000);
        assert_eq!(plan.vmas[1].virtual_address, 0x102000);
        assert_eq!(read_u64(&image, 0x200).unwrap(), 0x100055);
    }

    #[test]
    fn static_elf_loader_applies_symbolless_abs64_relocations() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
        ]);
        install_rela_section(&mut image, 0x500000, R_LNP64_ABS64, 0x1234);

        load_static_elf(&mut image, LoaderOptions::default()).unwrap();

        assert_eq!(read_u64(&image, 0x200).unwrap(), 0x1234);
    }

    #[test]
    fn static_elf_loader_applies_symbolless_abs32_relocations() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x500000,
                filesz: 4,
                memsz: 4,
                align: PAGE_SIZE,
            },
        ]);
        install_rela_section(&mut image, 0x500000, R_LNP64_ABS32, 0x1234);

        load_static_elf(&mut image, LoaderOptions::default()).unwrap();

        assert_eq!(read_u32(&image, 0x200).unwrap(), 0x1234);
    }

    #[test]
    fn static_elf_loader_rejects_abs32_overflow() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x500000,
                filesz: 4,
                memsz: 4,
                align: PAGE_SIZE,
            },
        ]);
        install_rela_section(&mut image, 0x500000, R_LNP64_ABS32, 0x1_0000_0000);

        let err = load_static_elf(&mut image, LoaderOptions::default()).unwrap_err();

        assert!(err.contains("ABS32 value is out of range"), "{err}");
    }

    #[test]
    fn static_elf_loader_applies_symbolless_glob_dat_relocations() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
        ]);
        install_rela_section(&mut image, 0x500000, R_LNP64_GLOB_DAT, 0x1234);

        load_static_elf(&mut image, LoaderOptions::default()).unwrap();

        assert_eq!(read_u64(&image, 0x200).unwrap(), 0x1234);
    }

    #[test]
    fn static_elf_loader_applies_symbolless_tls_tprel64_relocations() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x280,
                vaddr: 0x600000,
                filesz: 32,
                memsz: 0x1000,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_TLS,
                flags: PF_R,
                offset: 0x280,
                vaddr: 0x600000,
                filesz: 8,
                memsz: 24,
                align: 16,
            },
        ]);
        install_rela_section(&mut image, 0x500000, R_LNP64_TLS_TPREL64, 16);

        load_static_elf(&mut image, LoaderOptions::default()).unwrap();

        assert_eq!(read_u64(&image, 0x200).unwrap(), 16);
    }

    #[test]
    fn static_elf_loader_applies_symbolless_tls_dtprel64_relocations() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x280,
                vaddr: 0x600000,
                filesz: 32,
                memsz: 0x1000,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_TLS,
                flags: PF_R,
                offset: 0x280,
                vaddr: 0x600000,
                filesz: 8,
                memsz: 24,
                align: 16,
            },
        ]);
        install_rela_section(&mut image, 0x500000, R_LNP64_TLS_DTPREL64, 8);

        load_static_elf(&mut image, LoaderOptions::default()).unwrap();

        assert_eq!(read_u64(&image, 0x200).unwrap(), 8);
    }

    #[test]
    fn static_elf_loader_rejects_tls_relocation_without_tls_segment() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
        ]);
        install_rela_section(&mut image, 0x500000, R_LNP64_TLS_TPREL64, 0);

        let err = load_static_elf(&mut image, LoaderOptions::default()).unwrap_err();

        assert!(err.contains("requires PT_TLS"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_tls_relocation_outside_tls_image() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x280,
                vaddr: 0x600000,
                filesz: 32,
                memsz: 0x1000,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_TLS,
                flags: PF_R,
                offset: 0x280,
                vaddr: 0x600000,
                filesz: 8,
                memsz: 24,
                align: 16,
            },
        ]);
        install_rela_section(&mut image, 0x500000, R_LNP64_TLS_TPREL64, 25);

        let err = load_static_elf(&mut image, LoaderOptions::default()).unwrap_err();

        assert!(err.contains("offset exceeds PT_TLS"), "{err}");
    }

    #[test]
    fn static_elf_loader_applies_symbolless_fdr_desc64_relocations() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
            TestPhdr {
                typ: PT_NOTE,
                flags: PF_R,
                offset: 0x300,
                vaddr: 0,
                filesz: 192,
                memsz: 192,
                align: 8,
            },
        ]);
        install_startup_note(&mut image, 1);
        install_startup_fdr(
            &mut image,
            0,
            3,
            1,
            0xff,
            STARTUP_FDR_FLAG_CLOSE_ON_EXEC,
            0xabc,
            0xdef,
            0x44,
            0,
        );
        install_rela_section_at(&mut image, 0x3c0, 0x400, 0x500000, R_LNP64_FDR_DESC64, 0, 0);

        let plan = load_static_elf(&mut image, LoaderOptions::default()).unwrap();

        assert_eq!(plan.fdr_grants.len(), 1);
        assert_eq!(read_u64(&image, 0x200).unwrap(), 0);
    }

    #[test]
    fn static_elf_loader_rejects_fdr_desc64_outside_startup_table() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x500000,
                filesz: 8,
                memsz: 8,
                align: PAGE_SIZE,
            },
        ]);
        install_rela_section(&mut image, 0x500000, R_LNP64_FDR_DESC64, 0);

        let err = load_static_elf(&mut image, LoaderOptions::default()).unwrap_err();

        assert!(
            err.contains("FDR_DESC64 index exceeds startup FDR"),
            "{err}"
        );
    }

    #[test]
    fn static_elf_loader_rejects_symbolful_abs64_without_symbol_resolution() {
        let mut image = test_elf(&[text_phdr()]);
        install_rela_section_with_symbol(&mut image, 0x400000, R_LNP64_ABS64, 1, 0x1234);

        let err = load_static_elf(&mut image, LoaderOptions::default()).unwrap_err();

        assert!(err.contains("ABS64 with symbol index"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_unsupported_relocations() {
        let mut image = test_elf(&[text_phdr()]);
        install_rela_section(&mut image, 0x400000, R_LNP64_PC32, 0);
        let err = load_static_elf(&mut image, LoaderOptions::default()).unwrap_err();
        assert!(err.contains("unsupported LNP64 relocation type"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_relocations_outside_file_backed_loads() {
        let mut image = test_elf(&[text_phdr()]);
        install_rela_section(&mut image, 0x401000, R_LNP64_RELATIVE, 0);
        let err = load_static_elf(&mut image, LoaderOptions::default()).unwrap_err();
        assert!(err.contains("outside file-backed PT_LOAD"), "{err}");
    }

    #[test]
    fn static_elf_loader_parses_startup_note_descriptors() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_NOTE,
                flags: PF_R,
                offset: 0x300,
                vaddr: 0,
                filesz: 192,
                memsz: 192,
                align: 8,
            },
        ]);
        install_startup_note(&mut image, 2);
        install_startup_fdr(&mut image, 0, 0, 1, 0xf, 0x10, 0x100, 7, 0x40, 0);
        install_startup_fdr(&mut image, 1, 1, 2, 0x3, 0x20, 0x200, 8, 0x48, 0);

        let plan = build_static_exec_plan(&image, LoaderOptions::default()).unwrap();

        assert_eq!(
            plan.startup,
            Some(StartupDescriptor {
                flags: 0xabc,
                argc_addr: 0x700000,
                argv_addr: 0x700008,
                envp_addr: 0x700080,
                auxv_addr: 0x700100,
            })
        );
        assert_eq!(plan.fdr_grants.len(), 2);
        assert_eq!(plan.fdr_grants[0].slot, 0);
        assert_eq!(plan.fdr_grants[0].kind, 1);
        assert_eq!(plan.fdr_grants[0].rights, 0xf);
        assert_eq!(plan.fdr_grants[1].slot, 1);
        assert_eq!(plan.fdr_grants[1].generation, 8);
    }

    #[test]
    fn static_elf_loader_builds_manifest_shaped_exec_descriptor() {
        let mut image = test_elf(&[
            text_phdr(),
            TestPhdr {
                typ: PT_LOAD,
                flags: PF_R | PF_W,
                offset: 0x200,
                vaddr: 0x402000,
                filesz: 8,
                memsz: 24,
                align: PAGE_SIZE,
            },
            startup_note_phdr(128),
        ]);
        install_startup_note(&mut image, 1);
        install_startup_fdr(
            &mut image,
            0,
            3,
            9,
            0xf0,
            STARTUP_FDR_FLAG_CLOSE_ON_EXEC | STARTUP_FDR_FLAG_PRESERVE,
            0xabc,
            0xdef,
            0,
            0,
        );
        let plan = build_static_exec_plan(
            &image,
            LoaderOptions {
                initial_sp: 0x700000,
                tls_base: 0x710000,
                startup_metadata_ptr: 0x720000,
                ..LoaderOptions::default()
            },
        )
        .unwrap();

        let descriptor = build_exec_descriptor(
            &plan,
            ExecPlanDescriptorOptions {
                flags: 0x55,
                expected_domain_generation: 10,
                expected_process_generation: 11,
                expected_lineage_epoch: 12,
                image_source_cap: 0x1000,
                image_source_generation: 0x2000,
                image_lineage_epoch: 0x3000,
                measurements: vec![ExecPlanMeasurementDescriptor {
                    algorithm: 1,
                    measurement_ref: 2,
                    manifest_ref: 3,
                    attestation_ref: 4,
                }],
            },
        )
        .unwrap();

        assert_eq!(descriptor.header.version, 1);
        assert_eq!(descriptor.header.flags, 0x55);
        assert_eq!(descriptor.header.vma_count, 2);
        assert_eq!(descriptor.header.fdr_count, 1);
        assert_eq!(descriptor.header.measurement_count, 1);
        assert_eq!(descriptor.header.expected_domain_generation, 10);
        assert_eq!(descriptor.header.expected_process_generation, 11);
        assert_eq!(descriptor.header.expected_lineage_epoch, 12);
        assert_eq!(
            descriptor.header.total_length,
            EXEC_PLAN_HEADER_RECORD_SIZE
                + EXEC_PLAN_ENTRY_RECORD_SIZE
                + 2 * EXEC_PLAN_VMA_RECORD_SIZE
                + EXEC_PLAN_FDR_GRANT_RECORD_SIZE
                + EXEC_PLAN_MEASUREMENT_RECORD_SIZE
        );
        assert_eq!(descriptor.entry.entry_pc, 0x400000);
        assert_eq!(descriptor.entry.initial_sp, 0x700000);
        assert_eq!(descriptor.entry.tls_base, 0x710000);
        assert_eq!(descriptor.entry.startup_metadata_ptr, 0x720000);
        assert_eq!(descriptor.vmas[0].source_cap, 0x1000);
        assert_eq!(descriptor.vmas[0].source_generation, 0x2000);
        assert_eq!(descriptor.vmas[0].lineage_epoch, 0x3000);
        assert_eq!(
            descriptor.vmas[0].protection,
            VMA_PROT_READ | VMA_PROT_EXECUTE
        );
        assert_eq!(
            descriptor.vmas[0].executable_provenance,
            EXECUTABLE_PROVENANCE_IMAGE_TEXT
        );
        assert_eq!(
            descriptor.vmas[1].protection,
            VMA_PROT_READ | VMA_PROT_WRITE
        );
        assert_eq!(
            descriptor.vmas[1].executable_provenance,
            EXECUTABLE_PROVENANCE_NON_EXECUTABLE
        );
        assert_eq!(descriptor.vmas[1].zero_fill_length, 16);
        assert_eq!(descriptor.fdr_grants[0].slot, 3);
        assert_eq!(descriptor.fdr_grants[0].source_cap, 0xabc);
        assert_eq!(descriptor.fdr_grants[0].source_generation, 0xdef);
        assert_eq!(descriptor.fdr_grants[0].close_on_exec, 1);
        assert_eq!(descriptor.fdr_grants[0].preserve, 1);
        assert_eq!(descriptor.measurements[0].measurement_ref, 2);

        let words = encode_exec_descriptor(&descriptor);
        assert_eq!(words.len(), 47);
        assert_eq!(
            &words[0..9],
            &[1, descriptor.header.total_length, 0x55, 2, 1, 1, 10, 11, 12]
        );
        assert_eq!(&words[9..13], &[0x400000, 0x700000, 0x710000, 0x720000]);
        assert_eq!(
            &words[13..24],
            &[
                0x400000,
                16,
                VMA_PROT_READ | VMA_PROT_EXECUTE,
                MEMORY_TYPE_IMAGE,
                EXECUTABLE_PROVENANCE_IMAGE_TEXT,
                0x1000,
                0x100,
                0x2000,
                0x3000,
                0,
                0,
            ]
        );
        assert_eq!(
            &words[35..43],
            &[
                3,
                9,
                0xf0,
                STARTUP_FDR_FLAG_CLOSE_ON_EXEC | STARTUP_FDR_FLAG_PRESERVE,
                0xabc,
                0xdef,
                1,
                1,
            ]
        );
        assert_eq!(&words[43..47], &[1, 2, 3, 4]);
    }

    #[test]
    fn exec_descriptor_rejects_unbounded_record_counts() {
        let image = test_elf(&[text_phdr()]);
        let mut plan = build_static_exec_plan(&image, LoaderOptions::default()).unwrap();
        plan.vmas = vec![plan.vmas[0]; MAX_EXEC_PLAN_VMAS + 1];

        let err = build_exec_descriptor(&plan, ExecPlanDescriptorOptions::default()).unwrap_err();

        assert!(err.contains("VMA count"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_bad_startup_note_version() {
        let mut image = test_elf(&[text_phdr(), startup_note_phdr(64)]);
        install_startup_note(&mut image, 0);
        put_u64(&mut image, 0x308, 2);
        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();
        assert!(err.contains("startup note version"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_truncated_startup_fdr_table() {
        let mut image = test_elf(&[text_phdr(), startup_note_phdr(64)]);
        install_startup_note(&mut image, 1);
        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();
        assert!(err.contains("startup FDR table is truncated"), "{err}");
    }

    #[test]
    fn static_elf_loader_rejects_nonzero_startup_fdr_reserved_field() {
        let mut image = test_elf(&[text_phdr(), startup_note_phdr(128)]);
        install_startup_note(&mut image, 1);
        install_startup_fdr(&mut image, 0, 5, 1, 0xf, 0, 1, 1, 0, 99);
        let err = build_static_exec_plan(&image, LoaderOptions::default()).unwrap_err();
        assert!(err.contains("reserved field"), "{err}");
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

    fn startup_note_phdr(filesz: u64) -> TestPhdr {
        TestPhdr {
            typ: PT_NOTE,
            flags: PF_R,
            offset: 0x300,
            vaddr: 0,
            filesz,
            memsz: filesz,
            align: 8,
        }
    }

    fn test_elf(phdrs: &[TestPhdr]) -> Vec<u8> {
        let phoff = ELF64_EHDR_SIZE;
        let mut image = vec![0; 0x500];
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
            if phdr.typ == PT_PHDR {
                continue;
            }
            let start = phdr.offset as usize;
            let end = start + phdr.filesz as usize;
            for byte in &mut image[start..end] {
                *byte = 0xcc;
            }
        }
        image
    }

    fn install_startup_note(image: &mut [u8], fdr_count: u64) {
        let base = 0x300;
        image[base..base + 8].copy_from_slice(LNP64_STARTUP_NOTE_MAGIC);
        put_u64(image, base + 8, 1);
        put_u64(image, base + 16, 0xabc);
        put_u64(image, base + 24, 0x700000);
        put_u64(image, base + 32, 0x700008);
        put_u64(image, base + 40, 0x700080);
        put_u64(image, base + 48, 0x700100);
        put_u64(image, base + 56, fdr_count);
    }

    #[allow(clippy::too_many_arguments)]
    fn install_startup_fdr(
        image: &mut [u8],
        index: usize,
        slot: u64,
        kind: u64,
        rights: u64,
        flags: u64,
        object_id: u64,
        generation: u64,
        name_offset: u64,
        reserved: u64,
    ) {
        let base = 0x300 + STARTUP_NOTE_HEADER_SIZE + index * STARTUP_FDR_RECORD_SIZE;
        put_u64(image, base, slot);
        put_u64(image, base + 8, kind);
        put_u64(image, base + 16, rights);
        put_u64(image, base + 24, flags);
        put_u64(image, base + 32, object_id);
        put_u64(image, base + 40, generation);
        put_u64(image, base + 48, name_offset);
        put_u64(image, base + 56, reserved);
    }

    fn install_rela_section(image: &mut [u8], target: u64, reloc_type: u32, addend: i64) {
        install_rela_section_with_symbol(image, target, reloc_type, 0, addend);
    }

    fn install_rela_section_with_symbol(
        image: &mut [u8],
        target: u64,
        reloc_type: u32,
        symbol_index: u64,
        addend: i64,
    ) {
        install_rela_section_at(
            image,
            0x300,
            0x380,
            target,
            reloc_type,
            symbol_index,
            addend,
        );
    }

    fn install_rela_section_at(
        image: &mut [u8],
        shoff: u64,
        rela_offset: u64,
        target: u64,
        reloc_type: u32,
        symbol_index: u64,
        addend: i64,
    ) {
        put_u64(image, 40, shoff);
        put_u16(image, 58, ELF64_SHDR_SIZE as u16);
        put_u16(image, 60, 1);
        put_u32(image, shoff as usize + 4, SHT_RELA);
        put_u64(image, shoff as usize + 24, rela_offset);
        put_u64(image, shoff as usize + 32, ELF64_RELA_SIZE as u64);
        put_u64(image, shoff as usize + 56, ELF64_RELA_SIZE as u64);
        put_u64(image, rela_offset as usize, target);
        put_u64(
            image,
            rela_offset as usize + 8,
            (symbol_index << 32) | u64::from(reloc_type),
        );
        put_i64(image, rela_offset as usize + 16, addend);
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

    fn put_i64(bytes: &mut [u8], offset: usize, value: i64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
