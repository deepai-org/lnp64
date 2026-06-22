#![allow(dead_code)]

pub use crate::personality_lowering::*;

#[cfg(test)]
mod tests {
    use crate::native::{CloneProfile, MetadataOp, ObjectKind, ObjectProfile};

    use super::*;

    fn manifest_field<'a>(manifest: &'a str, key: &str) -> &'a str {
        let prefix = format!("{key}=");
        manifest
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .unwrap_or_else(|| panic!("missing manifest field {key}"))
    }

    fn manifest_csv_contains(manifest: &str, key: &str, value: &str) -> bool {
        manifest_field(manifest, key)
            .split(',')
            .any(|entry| entry == value)
    }

    fn relocation_rows(manifest: &str) -> Vec<(u16, &str, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, ',');
                let number = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing relocation number in {line}"))
                    .parse()
                    .unwrap_or_else(|_| panic!("invalid relocation number in {line}"));
                let name = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing relocation name in {line}"));
                let calculation = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing relocation calculation in {line}"));
                let loader_status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing relocation loader status in {line}"));
                (number, name, calculation, loader_status)
            })
            .collect()
    }

    fn intrinsic_rows(manifest: &str) -> Vec<(&str, &str, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let name = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic name in {line}"));
                let primitive = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic primitive in {line}"));
                let result = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic result in {line}"));
                let operands = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic operands in {line}"));
                (name, primitive, result, operands)
            })
            .collect()
    }

    fn intrinsic_lowering_rows(manifest: &str) -> Vec<(&str, &str, &str, &str, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(6, '|');
                let name = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering name in {line}"));
                let primitive = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering primitive in {line}"));
                let abi_shape = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering ABI shape in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering status in {line}"));
                let evidence = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering evidence in {line}"))
                    .split(',')
                    .collect();
                let blocker = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering blocker in {line}"));
                (name, primitive, abi_shape, status, evidence, blocker)
            })
            .collect()
    }

    fn isel_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(3, '|');
                let group = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing isel group in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing isel status in {line}"));
                let opcodes = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing isel opcodes in {line}"))
                    .split(',')
                    .collect();
                (group, status, opcodes)
            })
            .collect()
    }

    fn mc_encoding_rows(
        manifest: &str,
    ) -> Vec<(&str, &str, Vec<&str>, &str, Vec<&str>, Vec<&str>)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(6, '|');
                let group = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding group in {line}"));
                let format = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding format in {line}"));
                let opcodes = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding opcodes in {line}"))
                    .split(',')
                    .collect();
                let operands = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding operands in {line}"));
                let relocations = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding relocations in {line}"))
                    .split(',')
                    .collect();
                let surfaces = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding surfaces in {line}"))
                    .split(',')
                    .collect();
                (group, format, opcodes, operands, relocations, surfaces)
            })
            .collect()
    }

    fn exec_plan_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(3, '|');
                let record = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing exec-plan record in {line}"));
                let requirement = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing exec-plan requirement in {line}"));
                let record_fields = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing exec-plan fields in {line}"))
                    .split(',')
                    .collect();
                (record, requirement, record_fields)
            })
            .collect()
    }

    fn loader_security_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let requirement = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing loader security requirement in {line}"));
                let boundary = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing loader security boundary in {line}"));
                let evidence = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing loader security evidence in {line}"))
                    .split(',')
                    .collect();
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing loader security status in {line}"));
                (requirement, boundary, evidence, status)
            })
            .collect()
    }

    fn contract_rows(manifest: &str) -> Vec<(&str, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(3, '|');
                let name = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing contract name in {line}"));
                let path = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing contract path in {line}"));
                let test = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing contract test in {line}"));
                (name, path, test)
            })
            .collect()
    }

    fn inline_asm_rows(manifest: &str) -> Vec<(&str, &str, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let constraint = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing inline-asm constraint in {line}"));
                let class = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing inline-asm class in {line}"));
                let values = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing inline-asm values in {line}"));
                let usage = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing inline-asm use in {line}"));
                (constraint, class, values, usage)
            })
            .collect()
    }

    fn crt_startup_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(3, '|');
                let item = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing crt startup item in {line}"));
                let requirement = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing crt startup requirement in {line}"));
                let contract = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing crt startup contract in {line}"))
                    .split(',')
                    .collect();
                (item, requirement, contract)
            })
            .collect()
    }

    fn transition_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let phase = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing transition phase in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing transition status in {line}"));
                let artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing transition artifacts in {line}"))
                    .split(',')
                    .collect();
                let gate = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing transition gate in {line}"));
                (phase, status, artifacts, gate)
            })
            .collect()
    }

    fn register_class_rows(manifest: &str) -> Vec<(&str, &str, &str, &str, Vec<&str>, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(7, '|');
                let class = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing register class in {line}"));
                let values = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing register values in {line}"));
                let width = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing register width in {line}"));
                let allocatable = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing allocatable register set in {line}"));
                let reserved = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing reserved register set in {line}"))
                    .split(',')
                    .collect();
                let role = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing register role in {line}"));
                let debug = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing debug register role in {line}"));
                (class, values, width, allocatable, reserved, role, debug)
            })
            .collect()
    }

    fn netbsd_layer_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let layer = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing NetBSD layer in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing NetBSD layer status in {line}"));
                let artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing NetBSD layer artifacts in {line}"))
                    .split(',')
                    .collect();
                let gate = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing NetBSD layer gate in {line}"));
                let next_blocker = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing NetBSD layer blocker in {line}"));
                (layer, status, artifacts, gate, next_blocker)
            })
            .collect()
    }

    fn real_program_ladder_rows(
        manifest: &str,
    ) -> Vec<(&str, &str, Vec<&str>, &str, &str, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(7, '|');
                let stage = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing real-program stage in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing real-program status in {line}"));
                let artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing real-program artifacts in {line}"))
                    .split(',')
                    .collect();
                let gate = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing real-program gate in {line}"));
                let goal = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing real-program goal in {line}"));
                let focus = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing real-program focus in {line}"))
                    .split(',')
                    .collect();
                let next_blocker = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing real-program blocker in {line}"));
                (stage, status, artifacts, gate, goal, focus, next_blocker)
            })
            .collect()
    }

    fn conformance_gate_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let category = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing conformance category in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing conformance status in {line}"));
                let artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing conformance artifacts in {line}"))
                    .split(',')
                    .collect();
                let gate = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing conformance gate in {line}"));
                let coverage = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing conformance coverage in {line}"));
                (category, status, artifacts, gate, coverage)
            })
            .collect()
    }

    fn llvm_bootstrap_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let case = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm bootstrap case in {line}"));
                let source = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm bootstrap source in {line}"));
                let backend = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm bootstrap backend contracts in {line}"))
                    .split(',')
                    .collect();
                let runtime = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm bootstrap runtime contracts in {line}"))
                    .split(',')
                    .collect();
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm bootstrap status in {line}"));
                (case, source, backend, runtime, status)
            })
            .collect()
    }

    fn llvm_gate_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let gate = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm gate name in {line}"));
                let command = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm gate command in {line}"));
                let requires = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm gate requirements in {line}"))
                    .split(',')
                    .collect();
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm gate status in {line}"));
                (gate, command, requires, status)
            })
            .collect()
    }

    fn run_elf_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let stage = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing run-elf stage in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing run-elf status in {line}"));
                let artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing run-elf artifacts in {line}"))
                    .split(',')
                    .collect();
                let evidence = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing run-elf evidence in {line}"));
                let blocker = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing run-elf blocker in {line}"));
                (stage, status, artifacts, evidence, blocker)
            })
            .collect()
    }

    fn llvm_filemap_rows(manifest: &str) -> Vec<(&str, &str, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let layer = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm filemap layer in {line}"));
                let path = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm filemap path in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm filemap status in {line}"));
                let purpose = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm filemap purpose in {line}"));
                (layer, path, status, purpose)
            })
            .collect()
    }

    fn libc_shim_rows(manifest: &str) -> Vec<(&str, Vec<&str>, Vec<&str>, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let group = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing libc shim group in {line}"));
                let public_surface = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing libc shim public surface in {line}"))
                    .split(',')
                    .collect();
                let native_lowering = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing libc shim native lowering in {line}"))
                    .split(',')
                    .collect();
                let evidence = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing libc shim evidence in {line}"))
                    .split(',')
                    .collect();
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing libc shim status in {line}"));
                (group, public_surface, native_lowering, evidence, status)
            })
            .collect()
    }

    #[test]
    fn toolchain_contract_index_is_complete() {
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let rows = contract_rows(contract_index);
        let mut names = std::collections::BTreeSet::new();
        let mut paths = std::collections::BTreeSet::new();
        let mut tests = std::collections::BTreeSet::new();
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

        for (name, path, test) in rows {
            assert!(names.insert(name), "duplicate contract name {name}");
            assert!(paths.insert(path), "duplicate contract path {path}");
            assert!(tests.insert(test), "duplicate contract test {test}");
            assert!(
                manifest_root.join(path).is_file(),
                "contract {name} path {path} does not exist"
            );
            assert!(!test.is_empty(), "empty test for contract {name}");
        }
        for name in [
            "contract_index",
            "target",
            "registers",
            "relocations",
            "mc_encoding",
            "psabi",
            "intrinsics",
            "intrinsic_lowering",
            "intrinsic_header",
            "clang_driver",
            "llvm_filemap",
            "libc_shim",
            "netbsd_layers",
            "conformance_gates",
            "real_program_ladder",
            "isel",
            "llvm_bootstrap",
            "llvm_gates",
            "run_elf",
            "linker_script",
            "exec_plan",
            "loader_security",
            "loader",
            "exec_descriptor_validator",
            "debug_unwind",
            "inline_asm",
            "crt_startup",
            "crt0",
            "sysroot",
            "minilibc_smoke",
            "transition",
            "toy_compiler_retirement",
        ] {
            assert!(names.contains(name), "missing contract index row {name}");
        }
    }

    #[test]
    fn llvm_gate_manifest_pins_clang_lld_loader_commands() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let gate_manifest = include_str!("../toolchain/lnp64_llvm_gates.manifest");
        let gate_driver = include_str!("../scripts/run_llvm_bootstrap_gates.sh");
        let libc_test_driver = include_str!("../scripts/run_libc_test.sh");
        let real_tblgen = include_str!("../scripts/run_real_llvm_tblgen.sh");
        let real_tblgen_docker = include_str!("../scripts/run_real_llvm_tblgen_docker.sh");
        let bootstrap_smokes = include_str!("../scripts/run_real_llvm_bootstrap_smokes.sh");
        let real_llc = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let real_llc_docker = include_str!("../scripts/run_real_llvm_lnp64_docker.sh");
        let real_objects_docker = include_str!("../scripts/run_real_llvm_lnp64_objects_docker.sh");
        let real_mc_docker = include_str!("../scripts/run_real_llvm_lnp64_mc_docker.sh");
        let real_clang_target = include_str!("../clang/lib/Basic/Targets/LNP64.cpp");
        let llvm_dockerfile = include_str!("../Dockerfile.llvm");
        let errno_header = include_str!("../toolchain/include/errno.h");
        let netinet_in_header = include_str!("../toolchain/include/netinet/in.h");
        let poll_header = include_str!("../toolchain/include/poll.h");
        let search_header = include_str!("../toolchain/include/search.h");
        let setjmp_header = include_str!("../toolchain/include/setjmp.h");
        let pthread_header = include_str!("../toolchain/include/pthread.h");
        let semaphore_header = include_str!("../toolchain/include/semaphore.h");
        let signal_header = include_str!("../toolchain/include/signal.h");
        let stdarg_header = include_str!("../toolchain/include/stdarg.h");
        let stddef_header = include_str!("../toolchain/include/stddef.h");
        let stdint_header = include_str!("../toolchain/include/stdint.h");
        let stdio_header = include_str!("../toolchain/include/stdio.h");
        let stdlib_header = include_str!("../toolchain/include/stdlib.h");
        let string_header = include_str!("../toolchain/include/string.h");
        let sys_mman_header = include_str!("../toolchain/include/sys/mman.h");
        let sys_epoll_header = include_str!("../toolchain/include/sys/epoll.h");
        let sys_event_header = include_str!("../toolchain/include/sys/event.h");
        let sys_select_header = include_str!("../toolchain/include/sys/select.h");
        let sys_auxv_header = include_str!("../toolchain/include/sys/auxv.h");
        let sys_socket_header = include_str!("../toolchain/include/sys/socket.h");
        let sys_timerfd_header = include_str!("../toolchain/include/sys/timerfd.h");
        let time_header = include_str!("../toolchain/include/time.h");
        let unistd_header = include_str!("../toolchain/include/unistd.h");
        let libc_string_min = include_str!("../toolchain/liblnp64_string_min.c");
        let libc_convert_min = include_str!("../toolchain/liblnp64_convert_min.c");
        let libc_path_min = include_str!("../toolchain/liblnp64_path_min.c");
        let libc_search_min = include_str!("../toolchain/liblnp64_search_min.c");
        let libc_sort_min = include_str!("../toolchain/liblnp64_sort_min.c");
        let libc_alloc_min = include_str!("../toolchain/liblnp64_alloc_min.c");
        let libc_fd_min = include_str!("../toolchain/liblnp64_fd_min.c");
        let libc_meta_min = include_str!("../toolchain/liblnp64_meta_min.c");
        let libc_process_min = include_str!("../toolchain/liblnp64_process_min.c");
        let libc_setjmp_min = include_str!("../toolchain/liblnp64_setjmp_min.s");
        let libc_errno_min = include_str!("../toolchain/liblnp64_errno_min.c");
        let libc_startup_min = include_str!("../toolchain/liblnp64_startup_min.c");
        let libc_random_min = include_str!("../toolchain/liblnp64_random_min.c");
        let libc_stdio_min = include_str!("../toolchain/liblnp64_stdio_min.c");
        let libc_time_min = include_str!("../toolchain/liblnp64_time_min.c");
        let libc_vma_min = include_str!("../toolchain/liblnp64_vma_min.c");
        let libc_futex_min = include_str!("../toolchain/liblnp64_futex_min.c");
        let lnp64_futex_header = include_str!("../toolchain/include/lnp64/futex.h");
        let lnp64_intrinsics_target_header =
            include_str!("../toolchain/include/lnp64/intrinsics.h");
        let libc_pthread_min = include_str!("../toolchain/liblnp64_pthread_min.c");
        let libc_sem_min = include_str!("../toolchain/liblnp64_sem_min.c");
        let libc_poll_min = include_str!("../toolchain/liblnp64_poll_min.c");
        let libc_signal_min = include_str!("../toolchain/liblnp64_signal_min.c");
        let libc_socket_min = include_str!("../toolchain/liblnp64_socket_min.c");
        let libc_sbase_min = include_str!("../toolchain/liblnp64_sbase_min.c");
        let libc_sbase_fs_min = include_str!("../toolchain/liblnp64_sbase_fs_min.c");
        let libc_sbase_recurse_min = include_str!("../toolchain/liblnp64_sbase_recurse_min.c");
        let libc_sbase_move_min = include_str!("../toolchain/liblnp64_sbase_move_min.c");
        let libc_sbase_time_min = include_str!("../toolchain/liblnp64_sbase_time_min.c");
        let libc_sbase_ls_min = include_str!("../toolchain/liblnp64_sbase_ls_min.c");
        let libc_sbase_find_min = include_str!("../toolchain/liblnp64_sbase_find_min.c");
        let libc_sbase_accounts_min = include_str!("../toolchain/liblnp64_sbase_accounts_min.c");
        let libc_sbase_wc_min = include_str!("../toolchain/liblnp64_sbase_wc_min.c");
        let libc_sbase_head_min = include_str!("../toolchain/liblnp64_sbase_head_min.c");
        let elf_exec_test_clang = include_str!("../userland/elf_exec_test_clang.c");
        let spawn_task_clang = include_str!("../userland/spawn_task_clang.c");
        let gate_trace_test_clang = include_str!("../userland/gate_trace_test_clang.c");
        let fd_passing_test_clang = include_str!("../userland/fd_passing_test_clang.c");
        let classifier_test_clang = include_str!("../userland/classifier_test_clang.c");
        let domain_ctl_clang = include_str!("../userland/domain_ctl_clang.h");
        let netbsd_init_clang = include_str!("../userland/netbsd_init_clang.c");
        let netbsd_personality_clang = include_str!("../userland/netbsd_personality_clang_smoke.c");
        let netbsd_sh_clang = include_str!("../userland/netbsd_sh_clang.c");
        let fork_wait_test_clang = include_str!("../userland/fork_wait_test_clang.c");
        let poll_test_clang = include_str!("../userland/poll_test_clang.c");
        let signal_gate_test_clang = include_str!("../userland/signal_gate_test_clang.c");
        let signal_fault_test_clang = include_str!("../userland/signal_fault_test_clang.c");
        let socket_loopback_test_clang = include_str!("../userland/socket_loopback_test_clang.c");
        let timer_test_clang = include_str!("../userland/timer_test_clang.c");
        let httpd_demo = include_str!("../demos/httpd.c");
        let netcat_demo = include_str!("../demos/netcat.c");
        let parallel_hash_demo = include_str!("../demos/parallel_hash.c");
        let ping_pong_demo = include_str!("../demos/ping_pong.c");
        let producer_consumer_demo = include_str!("../demos/producer_consumer.c");
        let sqlite_lite_demo = include_str!("../demos/sqlite_lite.c");
        let lnp64_isel_lowering = include_str!("../llvm/lib/Target/LNP64/LNP64ISelLowering.cpp");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");

        for minilibc_intrinsic_source in [
            libc_alloc_min,
            libc_fd_min,
            libc_futex_min,
            libc_poll_min,
            libc_process_min,
            libc_pthread_min,
            libc_sem_min,
            libc_signal_min,
            libc_socket_min,
            libc_startup_min,
            libc_time_min,
            libc_vma_min,
        ] {
            assert!(minilibc_intrinsic_source.contains("#include <lnp64/intrinsics.h>"));
            assert!(!minilibc_intrinsic_source.contains("#include \"lnp64_intrinsics.h\""));
        }
        for native_demo_source in [
            httpd_demo,
            netcat_demo,
            parallel_hash_demo,
            ping_pong_demo,
            producer_consumer_demo,
            sqlite_lite_demo,
        ] {
            assert!(native_demo_source.contains("#include <lnp64/intrinsics.h>"));
            assert!(!native_demo_source.contains("#include \"lnp64_intrinsics.h\""));
        }
        assert!(netcat_demo.contains("netcat self-test ok"));
        assert!(httpd_demo.contains("httpd self-test ok"));
        let real_llc_lines: Vec<_> = real_llc.lines().collect();
        for (index, line) in real_llc_lines.iter().enumerate() {
            if line.trim_start().starts_with("-I toolchain ") && line.trim_end().ends_with('\\') {
                let previous = index
                    .checked_sub(1)
                    .and_then(|previous| real_llc_lines.get(previous))
                    .expect("private include line must have a preceding target include line");
                assert!(
                    previous.contains("-I toolchain/include"),
                    "real LLVM compile line {} must search installed target headers before private root",
                    index + 1
                );
            }
            assert!(
                !line.contains("-I toolchain -c")
                    || line.contains("-I toolchain/include -I toolchain -c"),
                "single-line real LLVM compile line {} must include installed target headers",
                index + 1
            );
        }
        assert!(
            !real_llc.contains("-I toolchain \\\n  -I toolchain/include"),
            "real LLVM compile lines must search installed target headers before private toolchain root"
        );
        assert!(
            !real_llc.contains("-I toolchain -I toolchain/include"),
            "single-line real LLVM compile lines must search installed target headers first"
        );
        let rows = llvm_gate_rows(gate_manifest);
        let mut gates = std::collections::BTreeSet::new();
        let mut commands = std::collections::BTreeMap::new();
        let mut requirements_by_gate = std::collections::BTreeMap::new();
        let mut statuses = std::collections::BTreeMap::new();
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

        assert_eq!(
            manifest_field(target_manifest, "llvm_gate_contract"),
            "toolchain/lnp64_llvm_gates.manifest"
        );
        assert!(
            manifest_root
                .join("scripts/run_llvm_bootstrap_gates.sh")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_tblgen.sh")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_tblgen_docker.sh")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_bootstrap_smokes.sh")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_lnp64.sh")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_lnp64_docker.sh")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_lnp64_mc_docker.sh")
                .is_file()
        );
        assert!(manifest_root.join("Dockerfile.llvm").is_file());
        assert!(contract_index.contains(
            "llvm_gates|toolchain/lnp64_llvm_gates.manifest|llvm_gate_manifest_pins_clang_lld_loader_commands"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_llvm_gates.manifest"));
        assert!(transition_manifest.contains("scripts/run_llvm_bootstrap_gates.sh"));
        assert!(roadmap.contains("toolchain/lnp64_llvm_gates.manifest"));
        assert!(roadmap.contains("scripts/run_llvm_bootstrap_gates.sh --dry-run"));
        assert!(roadmap.contains("scripts/run_real_llvm_tblgen_docker.sh"));
        assert!(roadmap.contains("scripts/run_real_llvm_lnp64_docker.sh"));
        assert!(roadmap.contains("Dockerfile.llvm"));

        for (gate, command, requirements, status) in rows {
            assert!(gates.insert(gate), "duplicate llvm gate {gate}");
            commands.insert(gate, command);
            requirements_by_gate.insert(gate, requirements.clone());
            statuses.insert(gate, status);
            assert!(
                ["planned", "tested"].contains(&status),
                "unknown LLVM gate status {status} for {gate}"
            );
            assert!(!command.is_empty(), "empty llvm gate command for {gate}");
            assert!(
                !requirements.is_empty(),
                "empty llvm gate requirements for {gate}"
            );
        }
        assert_eq!(statuses["gate_driver"], "tested");
        assert_eq!(statuses["real_llc_build"], "tested");
        assert_eq!(statuses["real_mc_build"], "tested");
        assert_eq!(statuses["real_tblgen"], "tested");
        assert_eq!(statuses["sysroot_package"], "tested");
        assert_eq!(statuses["simple_libc_gate"], "tested");
        for gate in [
            "compile_hello",
            "compile_arithmetic",
            "compile_memory",
            "compile_calls",
            "assemble_crt0",
            "link_static",
            "inspect_exec_plan",
            "run_through_loader",
        ] {
            assert_eq!(statuses[gate], "tested", "{gate} must be tested");
            assert!(
                commands[gate].contains("scripts/run_real_llvm_bootstrap_smokes.sh"),
                "{gate} must use the narrow real LLVM bootstrap smoke script"
            );
        }
        assert!(
            requirements_by_gate["real_llc_build"].contains(&"packaged_sysroot"),
            "full real LLVM gate must advertise the packaged sysroot it now builds and uses"
        );
        for requirement in [
            "packaged_sysroot",
            "crt0_object",
            "target_headers",
            "libc_shim_objects",
            "sysroot_static_link",
            "sysroot_run_elf",
            "clang_zlib_adler32_object",
            "clang_zlib_crc32_object",
            "clang_zlib_package_object",
            "clang_natsort_package_object",
            "clang_jsmn_package_object",
            "clang_inih_package_object",
            "clang_cwalk_package_object",
            "clang_varargs_call_object",
            "clang_large_frame_object",
            "clang_sbase_command_objects",
            "clang_sbase_libutil_objects",
            "clang_sbase_support_object",
            "clang_userland_ucat_object",
            "clang_userland_init_object",
            "clang_userland_lnpsh_object",
            "clang_userland_spawn_task_object",
            "clang_netbsd_init_object",
            "clang_netbsd_shell_object",
            "clang_netbsd_loader_target_child_object",
            "clang_netbsd_elf_exec_parent_object",
            "clang_netbsd_fork_wait_child_object",
            "clang_netbsd_thread_child_object",
            "clang_netbsd_poll_child_object",
            "clang_netbsd_signal_gate_child_object",
            "clang_netbsd_signal_fault_child_object",
            "clang_netbsd_timer_child_object",
            "clang_netbsd_mmap_child_object",
            "clang_netbsd_fd_passing_child_object",
            "clang_netbsd_fs_service_child_object",
            "clang_netbsd_classifier_child_object",
            "clang_netbsd_socket_loopback_child_object",
            "clang_netbsd_gate_trace_child_object",
            "clang_netbsd_domain_nested_child_object",
            "clang_netbsd_domain_budget_child_object",
            "clang_minilibc_meta_impl_object",
            "clang_meta_libc_object",
            "clang_minilibc_random_impl_object",
            "clang_minilibc_stdio_impl_object",
            "clang_libc_test_argv_object",
            "clang_libc_test_env_object",
            "clang_libc_test_random_object",
            "clang_libc_test_string_memcpy_bounded_object",
            "clang_libc_test_string_memmove_bounded_object",
            "clang_libc_test_search_insque_object",
            "clang_libc_test_malloc_0_object",
            "clang_libc_test_fgets_eof_object",
            "clang_libc_test_access_bounded_object",
            "clang_libc_test_stat_object",
            "clang_libc_test_utime_object",
            "clang_libc_test_fdopen_object",
            "clang_libc_test_fcntl_basic_bounded_object",
            "clang_libc_test_fcntl_object",
            "clang_libc_test_pthread_tsd_object",
            "clang_libc_test_sem_init_object",
            "clang_minilibc_pthread_impl_object",
            "clang_minilibc_sem_impl_object",
            "zlib_package_static_link",
            "natsort_package_static_link",
            "jsmn_package_static_link",
            "inih_package_static_link",
            "cwalk_package_static_link",
            "libc_test_argv_static_link",
            "libc_test_env_static_link",
            "libc_test_random_static_link",
            "libc_test_ctype_static_link",
            "libc_test_string_static_link",
            "libc_test_string_memcpy_bounded_static_link",
            "libc_test_string_memmove_bounded_static_link",
            "libc_test_string_memmem_static_link",
            "libc_test_string_strchr_static_link",
            "libc_test_string_strcspn_static_link",
            "libc_test_string_strstr_static_link",
            "libc_test_udiv_static_link",
            "libc_test_basename_static_link",
            "libc_test_dirname_static_link",
            "libc_test_strtol_static_link",
            "libc_test_clock_gettime_static_link",
            "libc_test_access_bounded_static_link",
            "libc_test_stat_static_link",
            "libc_test_utime_static_link",
            "libc_test_fdopen_static_link",
            "libc_test_fcntl_basic_bounded_static_link",
            "libc_test_fcntl_static_link",
            "libc_test_pthread_tsd_static_link",
            "libc_test_sem_init_static_link",
            "libc_test_qsort_bounded_static_link",
            "libc_test_search_insque_static_link",
            "libc_test_malloc_0_static_link",
            "libc_test_fgets_eof_static_link",
            "zlib_package_run_elf",
            "natsort_package_run_elf",
            "jsmn_package_run_elf",
            "inih_package_run_elf",
            "cwalk_package_run_elf",
            "netcat_demo_run_elf",
            "httpd_demo_run_elf",
            "libc_test_argv_run_elf",
            "libc_test_env_run_elf",
            "libc_test_random_run_elf",
            "libc_test_ctype_run_elf",
            "libc_test_string_run_elf",
            "libc_test_string_memcpy_bounded_run_elf",
            "libc_test_string_memmove_bounded_run_elf",
            "libc_test_string_memmem_run_elf",
            "libc_test_string_strchr_run_elf",
            "libc_test_string_strcspn_run_elf",
            "libc_test_string_strstr_run_elf",
            "libc_test_udiv_run_elf",
            "libc_test_basename_run_elf",
            "libc_test_dirname_run_elf",
            "libc_test_strtol_run_elf",
            "libc_test_clock_gettime_run_elf",
            "libc_test_access_bounded_run_elf",
            "libc_test_stat_run_elf",
            "libc_test_utime_run_elf",
            "libc_test_fdopen_run_elf",
            "libc_test_fcntl_basic_bounded_run_elf",
            "libc_test_fcntl_run_elf",
            "libc_test_pthread_tsd_run_elf",
            "libc_test_sem_init_run_elf",
            "libc_test_qsort_bounded_run_elf",
            "libc_test_search_insque_run_elf",
            "libc_test_malloc_0_run_elf",
            "libc_test_fgets_eof_run_elf",
            "sbase_echo_static_link",
            "sbase_echo_run_elf",
            "sbase_yes_static_link",
            "sbase_yes_exec_plan",
            "sbase_path_static_link",
            "sbase_path_run_elf",
            "sbase_cat_static_link",
            "sbase_cat_run_elf",
            "clang_sbase_fs_support_object",
            "sbase_mkdir_static_link",
            "sbase_mkdir_run_elf",
            "sbase_ln_static_link",
            "sbase_ln_run_elf",
            "sbase_chmod_static_link",
            "sbase_chmod_run_elf",
            "clang_sbase_recurse_support_object",
            "clang_sbase_move_support_object",
            "clang_sbase_time_support_object",
            "clang_sbase_ls_support_object",
            "clang_sbase_find_support_object",
            "clang_sbase_accounts_support_object",
            "clang_sbase_wc_support_object",
            "clang_sbase_head_support_object",
            "sbase_cmp_static_link",
            "sbase_cmp_run_elf",
            "sbase_cksum_static_link",
            "sbase_cksum_run_elf",
            "sbase_uniq_static_link",
            "sbase_uniq_run_elf",
            "sbase_tail_static_link",
            "sbase_tail_run_elf",
            "sbase_tee_static_link",
            "sbase_tee_run_elf",
            "sbase_cp_static_link",
            "sbase_cp_run_elf",
            "sbase_cut_static_link",
            "sbase_cut_run_elf",
            "sbase_tr_static_link",
            "sbase_tr_run_elf",
            "sbase_sort_static_link",
            "sbase_sort_run_elf",
            "sbase_grep_static_link",
            "sbase_grep_fixed_string_run_elf",
            "sbase_sed_static_link",
            "sbase_sed_no_regex_run_elf",
            "sbase_ls_static_link",
            "sbase_ls_run_elf",
            "sbase_find_static_link",
            "sbase_find_run_elf",
            "sbase_chown_static_link",
            "sbase_chown_run_elf",
            "sbase_wc_static_link",
            "sbase_wc_run_elf",
            "sbase_head_static_link",
            "sbase_head_run_elf",
            "sbase_touch_static_link",
            "sbase_touch_run_elf",
            "sbase_mv_static_link",
            "sbase_mv_run_elf",
            "sbase_rm_static_link",
            "sbase_rm_run_elf",
            "userland_ucat_static_link",
            "userland_ucat_run_elf",
            "userland_init_static_link",
            "userland_init_run_elf",
            "userland_lnpsh_static_link",
            "userland_lnpsh_run_elf",
            "userland_spawn_task_static_link",
            "userland_spawn_task_run_elf",
            "netbsd_init_static_link",
            "netbsd_shell_static_link",
            "netbsd_init_shell_system_run_elf",
            "netbsd_loader_target_child_static_link",
            "netbsd_loader_target_child_run_elf",
            "netbsd_elf_exec_parent_static_link",
            "netbsd_elf_exec_parent_run_elf",
            "netbsd_fork_wait_child_static_link",
            "netbsd_fork_wait_child_run_elf",
            "netbsd_thread_child_static_link",
            "netbsd_thread_child_run_elf",
            "netbsd_poll_child_static_link",
            "netbsd_poll_child_run_elf",
            "netbsd_signal_gate_child_static_link",
            "netbsd_signal_gate_child_run_elf",
            "netbsd_signal_fault_child_static_link",
            "netbsd_signal_fault_child_run_elf",
            "netbsd_timer_child_static_link",
            "netbsd_timer_child_run_elf",
            "netbsd_mmap_child_static_link",
            "netbsd_mmap_child_run_elf",
            "netbsd_fd_passing_child_static_link",
            "netbsd_fd_passing_child_run_elf",
            "netbsd_namespace_child_static_link",
            "netbsd_namespace_child_run_elf",
            "netbsd_fs_service_child_static_link",
            "netbsd_fs_service_child_run_elf",
            "netbsd_classifier_child_static_link",
            "netbsd_classifier_child_run_elf",
            "netbsd_socket_loopback_child_static_link",
            "netbsd_socket_loopback_child_run_elf",
            "netbsd_gate_trace_child_static_link",
            "netbsd_gate_trace_child_run_elf",
            "netbsd_domain_nested_child_static_link",
            "netbsd_domain_nested_child_run_elf",
            "netbsd_domain_budget_child_static_link",
            "netbsd_domain_budget_child_run_elf",
            "metadata_libc_static_link",
            "metadata_libc_run_elf",
        ] {
            assert!(
                gate_manifest.contains(requirement),
                "real LLVM gate manifest missing package requirement {requirement}"
            );
        }
        assert!(
            !lnp64_isel_lowering.contains("varargs call lowering is not implemented yet"),
            "real LLVM backend must lower ordinary calls to variadic prototypes"
        );

        for gate in [
            "gate_driver",
            "real_tblgen",
            "real_mc_build",
            "real_objects_build",
            "real_llc_build",
            "compile_hello",
            "compile_arithmetic",
            "compile_memory",
            "compile_calls",
            "assemble_crt0",
            "link_static",
            "inspect_exec_plan",
            "run_through_loader",
            "simple_libc_gate",
        ] {
            assert!(gates.contains(gate), "missing llvm gate {gate}");
        }
        assert!(
            commands["gate_driver"].contains("scripts/run_llvm_bootstrap_gates.sh --dry-run"),
            "llvm gate driver must expose the dry-run script"
        );
        assert!(
            commands["real_tblgen"].contains("scripts/run_real_llvm_tblgen_docker.sh"),
            "real LLVM TableGen gate must run through the Docker-backed script"
        );
        assert!(
            commands["real_llc_build"].contains("scripts/run_real_llvm_lnp64_docker.sh"),
            "real LLVM llc gate must run through the Docker-backed script"
        );
        assert!(
            commands["sysroot_package"].contains("scripts/package_lnp64_sysroot.sh"),
            "sysroot package gate must run through the checked package script"
        );
        assert!(
            commands["real_objects_build"]
                .contains("bash scripts/run_real_llvm_lnp64_objects_docker.sh"),
            "real LLVM object gate must run through the Docker-backed script"
        );
        assert!(
            commands["real_mc_build"].contains("scripts/run_real_llvm_lnp64_mc_docker.sh"),
            "real LLVM MC gate must run through the Docker-backed script"
        );
        assert!(
            commands["simple_libc_gate"]
                .contains("scripts/run_libc_test.sh --backend llvm --loader exec-plan"),
            "simple libc replacement gate must request the LLVM/exec-plan backend"
        );
        assert!(gate_driver.contains("toolchain/lnp64_llvm_gates.manifest"));
        assert!(gate_driver.contains("--dry-run"));
        assert!(gate_driver.contains("--run"));
        assert!(gate_driver.contains("LNP64_LLVM_GATE_FILTER"));
        assert!(gate_driver.contains("filter_allows_gate"));
        assert!(gate_driver.contains("no LLVM gate rows matched"));
        assert!(gate_driver.contains("LNP64_RUN_PLANNED_LLVM_GATES"));
        assert!(gate_driver.contains("skipping planned gate"));
        assert!(gate_driver.contains(r"command//\{build\}/"));
        assert!(libc_test_driver.contains("--backend llvm"));
        assert!(libc_test_driver.contains("backend=\"llvm\""));
        assert!(libc_test_driver.contains("loader=\"exec-plan\""));
        assert!(libc_test_driver.contains("--loader exec-plan"));
        assert!(libc_test_driver.contains("exec bash scripts/run_real_llvm_lnp64_docker.sh"));
        assert!(libc_test_driver.contains("llvm backend requires --loader exec-plan"));
        assert!(bootstrap_smokes.contains("LNP64_BOOTSTRAP_CASES"));
        assert!(bootstrap_smokes.contains("demos/hello.c"));
        assert!(bootstrap_smokes.contains("demos/factorial.c"));
        assert!(bootstrap_smokes.contains("demos/allocator.c"));
        assert!(bootstrap_smokes.contains("demos/fibonacci.c"));
        assert!(bootstrap_smokes.contains("elf-plan"));
        assert!(bootstrap_smokes.contains("run-elf"));
        assert!(bootstrap_smokes.contains("real LLVM bootstrap smokes passed"));
        assert!(real_tblgen.contains("llvm-tblgen"));
        assert!(real_tblgen.contains("llvm-config"));
        assert!(real_tblgen.contains("-gen-register-info"));
        assert!(real_tblgen.contains("-gen-instr-info"));
        assert!(real_tblgen.contains("-gen-callingconv"));
        assert!(real_tblgen.contains("-gen-subtarget"));
        assert!(real_tblgen.contains("LNP64GenRegisterInfo.inc"));
        assert!(real_tblgen.contains("LNP64GenInstrInfo.inc"));
        assert!(real_tblgen.contains("LNP64GenCallingConv.inc"));
        assert!(real_tblgen.contains("LNP64GenSubtargetInfo.inc"));
        assert!(real_tblgen.contains("real LLVM TableGen outputs written to"));
        assert!(real_tblgen_docker.contains("Dockerfile.llvm"));
        assert!(real_tblgen_docker.contains("scripts/run_real_llvm_tblgen.sh"));
        assert!(real_tblgen_docker.contains("LNP64_LLVM_DOCKER_SKIP_BUILD"));
        assert!(real_tblgen_docker.contains(r#"--user "$uid:$gid""#));
        assert!(real_llc_docker.contains("LNP64_LLVM_DOCKER_SKIP_BUILD"));
        assert!(real_llc_docker.contains("LNP64_LLVM_DOCKER_SKIP_RUN_ELF"));
        assert!(real_llc_docker.contains(r#"LNP64_LLVM_GATE="${LNP64_LLVM_GATE:-full}""#));
        assert!(real_llc_docker.contains("run-elf execution skipped by LNP64_LLVM_GATE"));
        assert!(real_objects_docker.contains("LNP64_LLVM_GATE=objects"));
        assert!(real_objects_docker.contains("scripts/run_real_llvm_lnp64_docker.sh"));
        assert!(real_mc_docker.contains("LNP64_LLVM_DOCKER_SKIP_BUILD"));
        assert!(real_llc.contains("llvmorg-14.0.6"));
        assert!(real_llc.contains("LNP64_LLVM_GATE"));
        assert!(real_llc.contains("full|mc|objects"));
        assert!(real_llc.contains("usage: scripts/run_real_llvm_lnp64.sh"));
        assert!(real_llc.contains("unknown option: %s"));
        assert!(real_llc.contains("missing required tool for real LLVM LNP64 gate"));
        assert!(real_llc.contains("for tool in git perl cmake ninja; do"));
        assert!(real_llc.contains("ninja -C \"$build_dir\" -j \"$jobs\" llvm-mc llvm-objdump"));
        assert!(
            real_llc
                .contains("ninja -C \"$build_dir\" -j \"$jobs\" llc llvm-mc llvm-objdump clang")
        );
        assert!(real_llc.contains("real LLVM LNP64 object-only gate passed"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-objdump crt0 decode smoke passed"));
        assert!(real_llc.contains("git clone"));
        assert!(
            real_llc.contains("git -C \"$project_dir\" sparse-checkout set llvm cmake clang lld")
        );
        assert!(real_llc.contains("LLVM_ENABLE_PROJECTS=\"clang;lld\""));
        assert!(real_llc.contains("LLVM_TARGETS_TO_BUILD=LNP64"));
        assert!(real_llc.contains(r#"ninja -C "$build_dir""#));
        assert!(real_llc.contains("llc llvm-mc llvm-objdump clang lld"));
        assert!(real_llc.contains(r#"llc="$build_dir/bin/llc""#));
        assert!(real_llc.contains(r#"clang="$build_dir/bin/clang""#));
        assert!(real_llc.contains(r#"llvm_mc="$build_dir/bin/llvm-mc""#));
        assert!(real_llc.contains(r#"llvm_objdump="$build_dir/bin/llvm-objdump""#));
        assert!(real_llc.contains(r#"lld="$build_dir/bin/lld""#));
        assert!(real_llc.contains(r#""$llc" --version"#));
        assert!(real_llc.contains("clang/lib/Basic/Targets/LNP64.h"));
        assert!(real_llc.contains("clang/lib/Basic/Targets/LNP64.cpp"));
        assert!(real_clang_target.contains("MaxAtomicInlineWidth = 64"));
        assert!(real_clang_target.contains("MaxAtomicPromoteWidth = MaxAtomicInlineWidth"));
        assert!(real_llc.contains("clang/lib/Driver/ToolChains/Arch/LNP64.cpp"));
        assert!(real_llc.contains("Targets/LNP64.cpp"));
        assert!(real_llc.contains("BareMetal(Triple)"));
        assert!(real_llc.contains("lld/ELF/Arch/LNP64.cpp"));
        assert!(real_llc.contains("elf64lnp64"));
        assert!(real_llc.contains("getLNP64TargetInfo"));
        assert!(real_llc.contains("ELF-only LNP64 smoke linker"));
        assert!(real_llc.contains("-verify-machineinstrs"));
        assert!(real_llc.contains("-filetype=obj"));
        assert!(real_llc.contains("real LLVM LNP64 llc smoke passed"));
        assert!(real_llc.contains("--target=lnp64-unknown-none"));
        assert!(real_llc.contains("-fno-jump-tables"));
        assert!(real_llc.contains("int main(void)"));
        assert!(real_llc.contains("scalar-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang scalar compile smoke passed"));
        assert!(real_llc.contains("scalar-arith-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'addi r'"));
        assert!(real_llc.contains("grep -q 'udiv r'"));
        assert!(real_llc.contains("grep -q 'srem r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang scalar arithmetic object smoke passed"));
        assert!(real_llc.contains("high-mul-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'mulhu r'"));
        assert!(real_llc.contains("grep -q 'mulh r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang high-multiply object smoke passed"));
        assert!(real_llc.contains("high-mul-mc-smoke.o"));
        assert!(real_llc.contains("mulhsu r7, r8, r9"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc high-multiply smoke passed"));
        assert!(real_llc.contains("auipc-mc-smoke.o"));
        assert!(real_llc.contains("auipc r1, 4096"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc auipc smoke passed"));
        assert!(real_llc.contains("mmap-mc-smoke.o"));
        assert!(real_llc.contains("mmap r1, r2, r3, r4"));
        assert!(real_llc.contains("munmap r5, r6"));
        assert!(real_llc.contains("mprotect r7, r8, r9, r10"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc mmap opcode smoke passed"));
        assert!(real_llc.contains("env-get-mc-smoke.o"));
        assert!(real_llc.contains("env_get r1, r2, r3, r4"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc env_get opcode smoke passed"));
        assert!(real_llc.contains("get-pcr-mc-smoke.o"));
        assert!(real_llc.contains("get_pcr r1, PID"));
        assert!(real_llc.contains("set_pcr r3, SIGMASK, r2"));
        assert!(real_llc.contains("stale two-operand SET_PCR unexpectedly assembled"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc GET_PCR opcode smoke passed"));
        assert!(real_llc.contains("open-at-mc-smoke.o"));
        assert!(real_llc.contains("open_at r1, r2, r3, r4"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc OPEN_AT opcode smoke passed"));
        assert!(real_llc.contains("clone-control-mc-smoke.o"));
        assert!(real_llc.contains("clone.spawn r1, r2, r3"));
        assert!(real_llc.contains("thread_join r4, r5, r6"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc clone control opcode smoke passed"));
        assert!(real_llc.contains("compat-meta-mc-smoke.o"));
        assert!(real_llc.contains("stat_path_at r1, r2, r3, r4"));
        assert!(real_llc.contains("stat_fd_dyn r5, r6"));
        assert!(real_llc.contains("utime_path_at r7, r8, r9, r10"));
        assert!(real_llc.contains("utime_fd_dyn r11, r12"));
        assert!(real_llc.contains("fcntl_fd_dyn r13, r14, r15"));
        assert!(
            real_llc.contains("real LLVM LNP64 llvm-mc compatibility metadata opcode smoke passed")
        );
        assert!(real_llc.contains("cap-control-mc-smoke.o"));
        assert!(real_llc.contains("cap_dup r1, r2"));
        assert!(real_llc.contains("cap_send r3, r4"));
        assert!(real_llc.contains("cap_recv r5, r6"));
        assert!(real_llc.contains("cap_revoke r7, r8"));
        assert!(
            real_llc.contains("real LLVM LNP64 llvm-mc capability control opcode smoke passed")
        );
        assert!(real_llc.contains("atomic-mc-smoke.o"));
        assert!(real_llc.contains("lr.d r13, (r14)"));
        assert!(real_llc.contains("sc.d r15, r16, (r14)"));
        assert!(real_llc.contains("futex_wait r20, r21"));
        assert!(real_llc.contains("futex_wake r22, r23"));
        assert!(real_llc.contains("fence.acq_rel"));
        assert!(real_llc.contains("isync r24, r25, r26"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc atomic opcode smoke passed"));
        assert!(real_llc.contains("signal-alias-mc-smoke.o"));
        assert!(real_llc.contains("sigaction r1, r2"));
        assert!(real_llc.contains("sigmask_set r3"));
        assert!(real_llc.contains("kill r4, r5"));
        assert!(real_llc.contains("alarm r6, r7"));
        assert!(real_llc.contains("yield"));
        assert!(real_llc.contains("sigret"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc signal alias opcode smoke passed"));
        assert!(real_llc.contains("scalar-extend-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'zext.w r'"));
        assert!(real_llc.contains("grep -q 'sext.b r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang scalar extension object smoke passed"));
        assert!(real_llc.contains("signed-load-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'sext.h r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang signed-load object smoke passed"));
        assert!(real_llc.contains("bitmanip-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'clz r'"));
        assert!(real_llc.contains("grep -q 'bswap64 r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang bit-manip object smoke passed"));
        assert!(real_llc.contains("csel-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'slt r' \"$csel_dump\""));
        assert!(real_llc.contains("grep -q 'sltu r' \"$csel_dump\""));
        assert!(real_llc.contains("grep -q 'bne r' \"$csel_dump\""));
        assert!(real_llc.contains("real LLVM LNP64 clang csel object smoke passed"));
        assert!(real_llc.contains("call-clobber-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang call-clobber object smoke passed"));
        assert!(real_llc.contains("debug-line-clang-smoke.o"));
        assert!(real_llc.contains("debug-line-clang-smoke.s"));
        assert!(real_llc.contains("-g -gdwarf-5"));
        assert!(real_llc.contains(r#"grep -Eq '\.cfi_def_cfa_offset|\.cfi_def_cfa"#));
        assert!(real_llc.contains("grep -q '.cfi_offset 32'"));
        assert!(real_llc.contains("grep -q '.debug_info'"));
        assert!(real_llc.contains("grep -q '.debug_line'"));
        assert!(real_llc.contains("grep -q '.debug_frame'"));
        assert!(real_llc.contains("grep -q '.rela.debug_line'"));
        assert!(real_llc.contains("real LLVM LNP64 clang debug section smoke passed"));
        assert!(real_llc.contains("sysroot=\"${LNP64_SYSROOT_DIR:-target/lnp64-sysroot}\""));
        assert!(
            real_llc.contains("lnp64_target_include_flags=(-I toolchain/include -I toolchain)")
        );
        assert!(real_llc.contains("scripts/package_lnp64_sysroot.sh"));
        assert!(real_llc.contains(
            "lnp64_target_include_flags=(-isystem \"$sysroot/usr/include\" -I toolchain)"
        ));
        assert!(real_llc.contains("\"${lnp64_target_include_flags[@]}\" -S \"$debug_line_c\""));
        assert!(real_llc.contains("\"${lnp64_target_include_flags[@]}\" -c \"$debug_line_c\""));
        assert!(real_llc.contains("-c demos/hello.c"));
        assert!(real_llc.contains("hello-clang-smoke.o"));
        assert!(real_llc.contains("\"${lnp64_target_include_flags[@]}\" \\\n  -c demos/hello.c"));
        assert!(real_llc.contains("hello-clang-smoke.dump"));
        assert!(real_llc.contains("real LLVM LNP64 clang hello object smoke passed"));
        assert!(real_llc.contains("-c demos/factorial.c"));
        assert!(real_llc.contains("factorial-clang-smoke.o"));
        assert!(
            real_llc.contains("\"${lnp64_target_include_flags[@]}\" \\\n  -c demos/factorial.c")
        );
        assert!(real_llc.contains("factorial-clang-smoke.dump"));
        assert!(real_llc.contains("ld.w r"));
        assert!(real_llc.contains("st.w r"));
        assert!(real_llc.contains("mul r"));
        assert!(real_llc.contains("grep -q 'blt r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang factorial object smoke passed"));
        assert!(real_llc.contains("-c demos/allocator.c"));
        assert!(real_llc.contains("allocator-clang-smoke.o"));
        assert!(real_llc.contains("allocator-clang-smoke.dump"));
        assert!(real_llc.contains("grep -q 'beq r' \"$allocator_dump\""));
        assert!(real_llc.contains("real LLVM LNP64 clang allocator object smoke passed"));
        assert!(real_llc.contains("-c demos/fibonacci.c"));
        assert!(real_llc.contains("fibonacci-clang-smoke.o"));
        assert!(real_llc.contains("fibonacci-clang-smoke.dump"));
        assert!(real_llc.contains("<fib_recursive>:"));
        assert!(real_llc.contains("<main>:"));
        assert!(real_llc.contains("ret"));
        assert!(real_llc.contains("real LLVM LNP64 clang fibonacci object smoke passed"));
        assert!(real_llc.contains("indirect-call-clang-smoke.o"));
        assert!(real_llc.contains("call_reg"));
        assert!(real_llc.contains("real LLVM LNP64 clang indirect call object smoke passed"));
        assert!(real_llc.contains("intrinsic-await-clang-smoke.o"));
        assert!(real_llc.contains("await r"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic await object smoke passed"));
        assert!(real_llc.contains("intrinsic-call-clang-smoke.o"));
        assert!(real_llc.contains("gate_call r"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic call object smoke passed"));
        assert!(real_llc.contains("intrinsic-gate-return-clang-smoke.o"));
        assert!(real_llc.contains("gate_return r"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang intrinsic gate return object smoke passed")
        );
        assert!(real_llc.contains("intrinsic-control-clang-smoke.o"));
        assert!(real_llc.contains("object_ctl r"));
        assert!(real_llc.contains("domain_ctl r"));
        assert!(real_llc.contains("__lnp_object_create(999"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic control object smoke passed"));
        assert!(real_llc.contains("intrinsic-cap-control-clang-smoke.o"));
        assert!(real_llc.contains("cap_dup r"));
        assert!(real_llc.contains("cap_send r"));
        assert!(real_llc.contains("cap_recv r"));
        assert!(real_llc.contains("cap_revoke r"));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang intrinsic capability control object smoke passed")
        );
        assert!(real_llc.contains("lnp64-intrinsic-cap-control-linked.elf"));
        assert!(
            real_llc.contains("real LLVM LNP64 lld intrinsic capability control link smoke passed")
        );
        assert!(real_llc.contains("intrinsic-amo-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'lr.d r' \"$intrinsic_amo_dump\""));
        assert!(real_llc.contains("grep -q 'sc.d r' \"$intrinsic_amo_dump\""));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic AMO object smoke passed"));
        assert!(real_llc.contains("c11-atomic-clang-smoke.o"));
        assert!(real_llc.contains("__atomic_load_n"));
        assert!(real_llc.contains("__atomic_store_n"));
        assert!(real_llc.contains("__atomic_fetch_add"));
        assert!(real_llc.contains("__atomic_fetch_xor"));
        assert!(real_llc.contains("__atomic_compare_exchange_n"));
        assert!(real_llc.contains("grep -q 'lr.d r' \"$c11_atomic_dump\""));
        assert!(real_llc.contains("grep -q 'sc.d r' \"$c11_atomic_dump\""));
        assert!(real_llc.contains("real LLVM LNP64 clang C11 atomic object smoke passed"));
        assert!(real_llc.contains("exit-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang exit object smoke passed"));
        assert!(real_llc.contains("setjmp-clang-smoke.o"));
        assert!(real_llc.contains("#include <setjmp.h>"));
        assert!(setjmp_header.contains("typedef unsigned long jmp_buf"));
        assert!(setjmp_header.contains("LNP64_JMPBUF_THREAD_COOKIE"));
        assert!(setjmp_header.contains("LNP64_JMPBUF_STACK_POINTER"));
        assert!(setjmp_header.contains("LNP64_JMPBUF_LINK_REGISTER"));
        assert!(setjmp_header.contains("LNP64_JMPBUF_CALLEE_SAVED_BASE"));
        assert!(setjmp_header.contains("LNP64_JMPBUF_WORDS 15"));
        // longjmp must restore the callee-saved set s0..s9 = r18..r27.
        assert!(libc_setjmp_min.contains("SD r18, 40(r2)"));
        assert!(libc_setjmp_min.contains("SD r27, 112(r2)"));
        assert!(libc_setjmp_min.contains("LD r18, 40(r2)"));
        assert!(libc_setjmp_min.contains("LD r27, 112(r2)"));
        assert!(setjmp_header.contains("__attribute__((returns_twice))"));
        assert!(setjmp_header.contains("__attribute__((noreturn))"));
        assert!(real_llc.contains("toolchain/liblnp64_setjmp_min.s"));
        assert!(libc_setjmp_min.contains("SD r1, 32(r2)"));
        assert!(libc_setjmp_min.contains("MOV r1, r5"));
        assert!(libc_setjmp_min.contains("ADD r31, r4, r0"));
        assert!(libc_setjmp_min.contains("SD r0, 0(r2)"));
        assert!(libc_setjmp_min.contains("SD r0, 8(r2)"));
        assert!(libc_setjmp_min.contains("SD r0, 16(r2)"));
        assert!(libc_setjmp_min.contains("BNE r3, r0, longjmp_value_ready"));
        assert!(real_llc.contains("liblnp64-setjmp-min.o"));
        assert!(real_llc.contains("grep -q '<setjmp>:'"));
        assert!(real_llc.contains("grep -q '<longjmp>:'"));
        assert!(real_llc.contains("real LLVM LNP64 clang setjmp object smoke passed"));
        assert!(
            real_llc.contains("real LLVM LNP64 llvm-mc setjmp implementation object smoke passed")
        );
        assert!(real_llc.contains("toolchain/liblnp64_process_min.c"));
        assert!(libc_process_min.contains("__lnp_exit"));
        assert!(libc_process_min.contains("void abort(void)"));
        assert!(libc_process_min.contains("int pid(void)"));
        assert!(libc_process_min.contains("int getpid(void)"));
        assert!(libc_process_min.contains("int getppid(void)"));
        assert!(libc_process_min.contains("unsigned int getuid(void)"));
        assert!(libc_process_min.contains("unsigned int getegid(void)"));
        assert!(libc_process_min.contains("__lnp_get_pid"));
        assert!(libc_process_min.contains("int fork(void)"));
        assert!(libc_process_min.contains("int pthread_atfork("));
        assert!(libc_process_min.contains("lnp64_run_atfork_prepare"));
        assert!(libc_process_min.contains("lnp64_run_atfork_parent"));
        assert!(libc_process_min.contains("lnp64_run_atfork_child"));
        assert!(pthread_header.contains("int pthread_atfork("));
        assert!(libc_process_min.contains("int waitpid(int pid, int *status, int options)"));
        assert!(libc_process_min.contains("int execve(const char *path"));
        assert!(libc_process_min.contains("int execv(const char *path"));
        assert!(libc_process_min.contains("int execvp(const char *file"));
        assert!(libc_process_min.contains("int execl(const char *path"));
        assert!(libc_process_min.contains("#include <stdarg.h>"));
        assert!(libc_process_min.contains("lnp64_exec_compat"));
        assert!(libc_process_min.contains("lnp64_fork_compat"));
        assert!(libc_process_min.contains("lnp64_wait_pid_compat"));
        assert!(real_llc.contains("liblnp64-process-min.o"));
        assert!(
            real_llc.contains(
                "-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_process_impl_c\""
            )
        );
        assert!(real_llc.contains("grep -q 'exit r'"));
        assert!(real_llc.contains("grep -q 'get_pcr r'"));
        assert!(real_llc.contains("grep -q 'fork r'"));
        assert!(real_llc.contains("grep -q 'wait_pid r'"));
        assert!(real_llc.contains("grep -q 'exec r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc process implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("toolchain/liblnp64_errno_min.c"));
        assert!(libc_errno_min.contains("ERRNO_GET") || libc_errno_min.contains("errno_get"));
        assert!(libc_errno_min.contains("ERRNO_SET") || libc_errno_min.contains("errno_set"));
        assert!(libc_errno_min.contains("lnp64_errno_initialized"));
        assert!(libc_errno_min.contains("__errno_location"));
        assert!(errno_header.contains("int *__errno_location(void);"));
        assert!(errno_header.contains("#define errno (*__errno_location())"));
        for signal_define in [
            "#define SIGINT  2",
            "#define SIGFPE  8",
            "#define SIGSEGV 11",
            "#define SIGALRM 14",
            "#define SIGTERM 15",
        ] {
            assert!(signal_header.contains(signal_define));
        }
        assert!(signal_header.contains("__lnp64_sighandler_t signal"));
        assert!(signal_header.contains("int raise(int signum);"));
        assert!(real_llc.contains("liblnp64-errno-min.o"));
        assert!(real_llc.contains("grep -q 'errno_get r'"));
        assert!(real_llc.contains("grep -q 'errno_set r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc errno implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("toolchain/liblnp64_startup_min.c"));
        assert!(libc_startup_min.contains("getauxval"));
        assert!(sys_auxv_header.contains("#define AT_PAGESZ 6"));
        assert!(sys_auxv_header.contains("#define AT_HWCAP 16"));
        assert!(sys_auxv_header.contains("#define AT_RANDOM 25"));
        assert!(sys_auxv_header.contains("unsigned long getauxval(unsigned long type);"));
        assert!(libc_startup_min.contains("char **environ"));
        assert!(libc_startup_min.contains("char *getenv("));
        assert!(libc_startup_min.contains("int setenv("));
        assert!(libc_startup_min.contains("int unsetenv("));
        assert!(libc_startup_min.contains("int clearenv("));
        assert!(libc_startup_min.contains("int putenv("));
        assert!(libc_startup_min.contains("env_get %0, %1, %2, %3"));
        assert!(unistd_header.contains("extern char **environ;"));
        assert!(real_llc.contains("liblnp64-startup-min.o"));
        assert!(
            real_llc.contains(
                "-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_startup_impl_c\""
            )
        );
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc startup implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("toolchain/liblnp64_random_min.c"));
        assert!(libc_random_min.contains("#include <stdlib.h>"));
        assert!(libc_random_min.contains("long random(void)"));
        assert!(libc_random_min.contains("void srandom(unsigned int seed)"));
        assert!(libc_random_min.contains("char *initstate("));
        assert!(libc_random_min.contains("char *setstate("));
        assert!(real_llc.contains("liblnp64-random-min.o"));
        assert!(real_llc.contains("grep -q '<random>:'"));
        assert!(real_llc.contains("grep -q '<srandom>:'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc random implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("toolchain/liblnp64_time_min.c"));
        assert!(libc_time_min.contains("clock_gettime"));
        assert!(libc_time_min.contains("get_pcr %0, REALTIME_SEC"));
        assert!(libc_time_min.contains("get_pcr %0, REALTIME_NSEC"));
        assert!(libc_time_min.contains("int usleep(unsigned int usec)"));
        assert!(libc_time_min.contains("unsigned int sleep(unsigned int seconds)"));
        assert!(libc_time_min.contains("int timerfd_create(int clockid, int flags)"));
        assert!(libc_time_min.contains("int timerfd_settime("));
        assert!(libc_time_min.contains("int timerfd_gettime("));
        assert!(libc_time_min.contains("LNP64_OBJECT_KIND_TIMER"));
        assert!(libc_time_min.contains("__lnp_object_ctl"));
        assert!(
            libc_time_min.contains("long status = (long)__lnp_object_ctl((lnp64_word_t)record);")
        );
        assert!(libc_time_min.contains("errno = (int)-status;"));
        assert!(libc_time_min.contains("__lnp_push"));
        assert!(libc_time_min.contains("__lnp_yield"));
        assert!(time_header.contains("struct itimerspec"));
        assert!(time_header.contains("#include <stddef.h>"));
        assert!(time_header.contains("size_t strftime(char *s, size_t max"));
        assert!(sys_timerfd_header.contains("int timerfd_create(int clockid, int flags);"));
        assert!(sys_timerfd_header.contains("int timerfd_settime("));
        assert!(sys_timerfd_header.contains("int timerfd_gettime("));
        assert!(unistd_header.contains("unsigned int alarm(unsigned int seconds);"));
        assert!(unistd_header.contains("int usleep(unsigned int usec);"));
        assert!(real_llc.contains("liblnp64-time-min.o"));
        assert!(real_llc.contains("grep -q 'get_pcr r'"));
        assert!(real_llc.contains("grep -q 'yield' \"$libc_time_impl_dump\""));
        assert!(real_llc.contains("grep -q 'object_ctl r' \"$libc_time_impl_dump\""));
        assert!(real_llc.contains("grep -q 'push r' \"$libc_time_impl_dump\""));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang minilibc time implementation object smoke passed")
        );
        assert!(libc_string_min.contains("int isascii(int ch)"));
        assert!(libc_string_min.contains("int isblank(int ch)"));
        assert!(libc_string_min.contains("int iscntrl(int ch)"));
        assert!(libc_string_min.contains("int isprint(int ch)"));
        assert!(libc_string_min.contains("int isgraph(int ch)"));
        assert!(libc_string_min.contains("int ispunct(int ch)"));
        assert!(real_llc.contains("libc-test-print-clang-smoke.o"));
        assert!(real_llc.contains("libc-test-argv-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/argv.c"));
        assert!(real_llc.contains("libc-test-env-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/env.c"));
        assert!(real_llc.contains("libc-test-random-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/random.c"));
        assert!(real_llc.contains("libc-test-ctype-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/ctype_bounded.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test harness object smoke passed"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test argv object smoke passed"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test env object smoke passed"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test random object smoke passed"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test ctype_bounded object smoke passed")
        );
        assert!(real_llc.contains("libc-test-string-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test string object smoke passed"));
        assert!(real_llc.contains("libc-test-string-memcpy-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_memcpy_bounded.c"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang libc-test string_memcpy_bounded object smoke passed"
            )
        );
        assert!(real_llc.contains("libc-test-string-memmove-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_memmove_bounded.c"));
        assert!(real_llc.contains(
            "real LLVM LNP64 clang libc-test string_memmove_bounded object smoke passed"
        ));
        assert!(real_llc.contains("libc-test-string-memmem-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_memmem.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test string_memmem object smoke passed")
        );
        assert!(real_llc.contains("libc-test-string-strchr-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_strchr.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test string_strchr object smoke passed")
        );
        assert!(real_llc.contains("libc-test-string-strcspn-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_strcspn.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test string_strcspn object smoke passed")
        );
        assert!(real_llc.contains("libc-test-string-strstr-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_strstr.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test string_strstr object smoke passed")
        );
        assert!(real_llc.contains("libc-test-udiv-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/udiv.c"));
        assert!(real_llc.contains("grep -q 'udiv r'"));
        assert!(real_llc.contains("grep -q 'urem r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test udiv object smoke passed"));
        assert!(real_llc.contains("libc-test-basename-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/basename.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test basename object smoke passed"));
        assert!(real_llc.contains("libc-test-dirname-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/dirname.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test dirname object smoke passed"));
        assert!(real_llc.contains("libc-test-strtol-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/strtol.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test strtol object smoke passed"));
        assert!(real_llc.contains("libc-test-clock-gettime-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/clock_gettime.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test clock_gettime object smoke passed")
        );
        assert!(real_llc.contains("libc-test-access-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/access_bounded.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test access_bounded object smoke passed")
        );
        assert!(real_llc.contains("libc-test-stat-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/stat.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test stat object smoke passed"));
        assert!(real_llc.contains("libc-test-utime-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/utime.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test utime object smoke passed"));
        assert!(real_llc.contains("libc-test-ungetc-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/ungetc.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test ungetc object smoke passed"));
        assert!(real_llc.contains("libc-test-fdopen-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/fdopen.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test fdopen object smoke passed"));
        assert!(real_llc.contains("libc-test-fcntl-basic-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/fcntl_basic_bounded.c"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang libc-test fcntl_basic_bounded object smoke passed"
            )
        );
        assert!(real_llc.contains("libc-test-pthread-tsd-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/pthread_tsd.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test pthread_tsd object smoke passed")
        );
        assert!(real_llc.contains("libc-test-sem-init-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/sem_init.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test sem_init object smoke passed"));
        assert!(real_llc.contains("libc-test-qsort-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/qsort_bounded.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test qsort_bounded object smoke passed")
        );
        assert!(real_llc.contains("libc-test-search-insque-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/search_insque.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test search_insque object smoke passed")
        );
        assert!(real_llc.contains("libc-test-search-lsearch-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/search_lsearch.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test search_lsearch object smoke passed")
        );
        assert!(real_llc.contains("libc-test-malloc-0-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/regression/malloc-0.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test malloc-0 object smoke passed"));
        assert!(real_llc.contains("libc-test-fgets-eof-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/regression/fgets-eof.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test fgets-eof object smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-ctype-bounded-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test ctype_bounded link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-string-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test string link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-string-memcpy-bounded-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_memcpy_bounded_obj" \"#));
        assert!(
            real_llc
                .contains("real LLVM LNP64 lld libc-test string_memcpy_bounded link smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-test-string-memmove-bounded-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_memmove_bounded_obj" \"#));
        assert!(
            real_llc
                .contains("real LLVM LNP64 lld libc-test string_memmove_bounded link smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-test-string-memmem-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test string_memmem link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-string-strchr-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test string_strchr link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-string-strcspn-linked.elf"));
        assert!(
            real_llc.contains("real LLVM LNP64 lld libc-test string_strcspn link smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-test-string-strstr-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test string_strstr link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-udiv-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test udiv link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-basename-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test basename link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-dirname-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test dirname link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-strtol-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test strtol link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-clock-gettime-linked.elf"));
        assert!(real_llc.contains(
            r#""$libc_test_clock_gettime_obj" \
  "$libc_test_print_obj" "$libc_time_impl_obj" "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test clock_gettime link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-access-bounded-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_access_bounded_obj""#));
        assert!(real_llc.contains(r#""$libc_meta_impl_obj" "$libc_fd_impl_obj""#));
        assert!(
            real_llc.contains("real LLVM LNP64 lld libc-test access_bounded link smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-test-stat-linked.elf"));
        assert!(real_llc.contains(
            r#""$libc_test_stat_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_meta_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj" "$libc_time_impl_obj" \
  "$libc_process_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test stat link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-utime-linked.elf"));
        assert!(real_llc.contains(
            r#""$libc_test_utime_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_meta_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj" "$libc_time_impl_obj" \
  "$libc_process_impl_obj" "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test utime link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-ungetc-linked.elf"));
        assert!(real_llc.contains(
            r#""$libc_test_ungetc_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_string_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test ungetc link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-fdopen-linked.elf"));
        assert!(real_llc.contains(
            r#""$libc_test_fdopen_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_string_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test fdopen link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-fcntl-basic-bounded-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_fcntl_basic_obj""#));
        assert!(real_llc.contains(r#""$libc_stdio_impl_obj" "$libc_meta_impl_obj""#));
        assert!(
            real_llc
                .contains("real LLVM LNP64 lld libc-test fcntl_basic_bounded link smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-test-fcntl-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_fcntl_obj""#));
        assert!(real_llc.contains(r#""$libc_process_impl_obj" "$libc_errno_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test fcntl link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-pthread-tsd-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_pthread_tsd_obj""#));
        assert!(real_llc.contains(r#""$libc_pthread_impl_obj" "$libc_alloc_impl_obj""#));
        assert!(real_llc.contains(r#""$libc_alloc_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test pthread_tsd link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-sem-init-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_sem_init_obj""#));
        assert!(real_llc.contains(r#""$libc_pthread_impl_obj" "$libc_sem_impl_obj""#));
        assert!(real_llc.contains(r#""$libc_sem_impl_obj" "$libc_futex_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test sem_init link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-qsort-bounded-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_qsort_bounded_obj" \"#));
        assert!(real_llc.contains(r#""$libc_sort_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test qsort_bounded link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-search-insque-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_search_insque_obj" \"#));
        assert!(real_llc.contains(r#""$libc_search_impl_obj" "$libc_alloc_impl_obj""#));
        assert!(real_llc.contains(r#""$libc_string_impl_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test search_insque link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-search-lsearch-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_search_lsearch_obj""#));
        assert!(
            real_llc.contains("real LLVM LNP64 lld libc-test search_lsearch link smoke passed")
        );
        assert!(search_header.contains("void insque(void *elem, void *pred);"));
        assert!(search_header.contains("void remque(void *elem);"));
        assert!(real_llc.contains("lnp64-libc-test-malloc-0-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_malloc_0_obj" \"#));
        assert!(real_llc.contains(r#""$libc_alloc_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test malloc-0 link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-fgets-eof-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_fgets_eof_obj" \"#));
        assert!(real_llc.contains(r#""$libc_stdio_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test fgets-eof link smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_futex_min.c"));
        assert!(lnp64_futex_header.contains("int futex_wait("));
        assert!(lnp64_futex_header.contains("int futex_wake("));
        assert!(libc_futex_min.contains("#include <lnp64/futex.h>"));
        assert!(libc_futex_min.contains("__lnp_futex_wait"));
        assert!(libc_futex_min.contains("__lnp_futex_wake"));
        assert!(real_llc.contains("liblnp64-futex-min.o"));
        assert!(
            real_llc.contains(
                "-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_futex_impl_c\""
            )
        );
        assert!(real_llc.contains("grep -q 'futex_wait r'"));
        assert!(real_llc.contains("grep -q 'futex_wake r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc futex implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("futex-libc-clang-smoke.o"));
        assert!(real_llc.contains("#include <lnp64/futex.h>"));
        assert!(
            real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$futex_libc_c\"")
        );
        assert!(real_llc.contains("real LLVM LNP64 clang futex libc object smoke passed"));
        assert!(pthread_header.contains("int pthread_create("));
        assert!(pthread_header.contains("int pthread_key_create("));
        assert!(pthread_header.contains("void *pthread_getspecific("));
        assert!(libc_pthread_min.contains("__lnp_spawn_entry"));
        assert!(libc_pthread_min.contains("__lnp_thread_join"));
        assert!(libc_pthread_min.contains("lnp64_run_tsd_destructors"));
        assert!(real_llc.contains("toolchain/liblnp64_pthread_min.c"));
        assert!(real_llc.contains("liblnp64-pthread-min.o"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc pthread implementation object smoke passed"
            )
        );
        assert!(semaphore_header.contains("typedef struct"));
        assert!(semaphore_header.contains("int sem_init("));
        assert!(semaphore_header.contains("int sem_timedwait("));
        assert!(libc_sem_min.contains("__lnp_futex_wait"));
        assert!(libc_sem_min.contains("__lnp_futex_wake"));
        assert!(libc_sem_min.contains("__atomic_compare_exchange_n"));
        assert!(real_llc.contains("toolchain/liblnp64_sem_min.c"));
        assert!(real_llc.contains("liblnp64-sem-min.o"));
        assert!(real_llc.contains(
            "real LLVM LNP64 clang minilibc semaphore implementation object smoke passed"
        ));
        assert!(real_llc.contains("toolchain/liblnp64_poll_min.c"));
        assert!(libc_poll_min.contains("#include <poll.h>"));
        assert!(libc_poll_min.contains("#include <sys/epoll.h>"));
        assert!(libc_poll_min.contains("#include <sys/event.h>"));
        assert!(libc_poll_min.contains("#include <sys/select.h>"));
        assert!(!libc_poll_min.contains("typedef unsigned long nfds_t;"));
        assert!(libc_poll_min.contains("int poll(struct pollfd *fds"));
        assert!(libc_poll_min.contains("int select(int nfds"));
        assert!(libc_poll_min.contains("int epoll_create1(int flags)"));
        assert!(libc_poll_min.contains("int epoll_ctl(int epfd, int op, int fd"));
        assert!(libc_poll_min.contains("int epoll_wait(int epfd"));
        assert!(libc_poll_min.contains("int kqueue(void)"));
        assert!(libc_poll_min.contains("int lnp64_kqueue_close(int fd)"));
        assert!(libc_poll_min.contains("lnp64_kqueue_write_error"));
        assert!(libc_poll_min.contains("LNP64_EVFILT_USER"));
        assert!(libc_poll_min.contains("LNP64_EV_ENABLE"));
        assert!(libc_poll_min.contains("LNP64_EV_DISABLE"));
        assert!(libc_poll_min.contains("LNP64_EV_RECEIPT"));
        assert!(libc_poll_min.contains("LNP64_NOTE_TRIGGER"));
        assert!(libc_poll_min.contains("int kevent(int kq"));
        assert!(libc_poll_min.contains("__lnp_await"));
        assert!(poll_header.contains("struct pollfd"));
        assert!(poll_header.contains("#define POLLIN"));
        assert!(poll_header.contains("int poll(struct pollfd *fds"));
        assert!(sys_select_header.contains("typedef struct"));
        assert!(sys_select_header.contains("int select(int nfds"));
        assert!(sys_epoll_header.contains("struct epoll_event"));
        assert!(sys_epoll_header.contains("#define EPOLL_CTL_ADD"));
        assert!(sys_epoll_header.contains("int epoll_ctl(int epfd"));
        assert!(sys_event_header.contains("struct kevent"));
        assert!(sys_event_header.contains("#define EVFILT_READ"));
        assert!(sys_event_header.contains("#define EVFILT_USER"));
        assert!(sys_event_header.contains("#define EV_ENABLE"));
        assert!(sys_event_header.contains("#define EV_DISABLE"));
        assert!(sys_event_header.contains("#define EV_ONESHOT"));
        assert!(sys_event_header.contains("#define EV_RECEIPT"));
        assert!(sys_event_header.contains("#define EV_ERROR"));
        assert!(sys_event_header.contains("#define NOTE_TRIGGER"));
        assert!(sys_event_header.contains("int kevent(int kq"));
        assert!(real_llc.contains("#include <poll.h>"));
        assert!(real_llc.contains("#include <sys/epoll.h>"));
        assert!(real_llc.contains("#include <sys/event.h>"));
        assert!(real_llc.contains("#include <sys/select.h>"));
        assert!(
            real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$poll_libc_c\"")
        );
        assert!(real_llc.contains("select(1, &readfds, &writefds, &exceptfds, &timeout)"));
        assert!(real_llc.contains("epoll_create1(0)"));
        assert!(real_llc.contains("epoll_ctl(ep, EPOLL_CTL_ADD, 0, &ev)"));
        assert!(real_llc.contains("epoll_ctl(ep, EPOLL_CTL_MOD, 0, &ev)"));
        assert!(real_llc.contains("epoll_wait(ep, &out, 1, 0)"));
        assert!(real_llc.contains("epoll_ctl(ep, EPOLL_CTL_DEL, 0, 0)"));
        assert!(real_llc.contains("kqueue()"));
        assert!(real_llc.contains("change.filter = EVFILT_READ"));
        assert!(real_llc.contains("change.flags = EV_ADD"));
        assert!(real_llc.contains("change.flags = EV_ADD | EV_ONESHOT"));
        assert!(real_llc.contains("change.flags = EV_DELETE"));
        assert!(real_llc.contains("change.filter = 99"));
        assert!(real_llc.contains("kevent(kq, &change, 1, 0, 0, &ts) != -1"));
        assert!(real_llc.contains("change.filter = EVFILT_WRITE"));
        assert!(real_llc.contains("kevent(kq, &change, 1, 0, 0, &ts)"));
        assert!(real_llc.contains("poll-libc-clang-smoke.o"));
        assert!(real_llc.contains("liblnp64-poll-min.o"));
        assert!(
            real_llc
                .contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_poll_impl_c\"")
        );
        assert!(real_llc.contains("grep -q 'await r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang poll/select/epoll/kqueue libc object smoke passed"
            )
        );
        assert!(real_llc.contains(
            "real LLVM LNP64 clang minilibc poll/select/epoll/kqueue implementation object smoke passed"
        ));
        assert!(real_llc.contains("toolchain/liblnp64_signal_min.c"));
        assert!(libc_signal_min.contains("#include <signal.h>"));
        assert!(libc_signal_min.contains("#include <unistd.h>"));
        assert!(!real_llc.contains("#include \"lnp64_intrinsics.h\""));
        assert!(!libc_signal_min.contains("typedef unsigned long sigset_t;"));
        assert!(!libc_signal_min.contains("struct sigaction {"));
        assert!(libc_signal_min.contains("sighandler_t signal"));
        assert!(libc_signal_min.contains("int sigaction(int signum"));
        assert!(libc_signal_min.contains("int sigprocmask(int how"));
        assert!(libc_signal_min.contains("int kill(int pid"));
        assert!(libc_signal_min.contains("lnp64_word_t status = __lnp_kill"));
        assert!(libc_signal_min.contains("int raise(int signum"));
        assert!(libc_signal_min.contains("kill((int)__lnp_get_pid(), signum)"));
        assert!(libc_signal_min.contains("unsigned int alarm(unsigned int seconds)"));
        assert!(signal_header.contains("struct sigaction"));
        assert!(signal_header.contains("#define SIG_SETMASK"));
        assert!(signal_header.contains("int sigaction(int signum"));
        assert!(signal_header.contains("int sigprocmask(int how"));
        assert!(signal_header.contains("int kill(int pid"));
        assert!(real_llc.contains("signal-libc-clang-smoke.o"));
        assert!(real_llc.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("#include <signal.h>"));
        assert!(real_llc.contains("#include <unistd.h>"));
        assert!(
            real_llc
                .contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$signal_libc_c\"")
        );
        assert!(real_llc.contains("signal(10, SIG_IGN)"));
        assert!(real_llc.contains("sigaction(12, &act, 0)"));
        assert!(real_llc.contains("sigprocmask(SIG_SETMASK, &mask, 0)"));
        assert!(real_llc.contains("kill((int)__lnp_get_pid(), 10)"));
        assert!(real_llc.contains("raise(12)"));
        assert!(real_llc.contains("real LLVM LNP64 clang signal libc object smoke passed"));
        assert!(real_llc.contains("liblnp64-signal-min.o"));
        assert!(
            real_llc.contains(
                "-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_signal_impl_c\""
            )
        );
        assert!(real_llc.contains("grep -q 'sigaction r'"));
        assert!(real_llc.contains("grep -q 'sigmask_set r'"));
        assert!(real_llc.contains("grep -q 'kill r'"));
        assert!(real_llc.contains("grep -q 'alarm r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc signal implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("toolchain/liblnp64_socket_min.c"));
        assert!(libc_socket_min.contains("#include <sys/socket.h>"));
        assert!(!libc_socket_min.contains("typedef unsigned long socklen_t;"));
        assert!(libc_socket_min.contains("int socket(int domain"));
        assert!(libc_socket_min.contains("int bind(int fd"));
        assert!(libc_socket_min.contains("int listen(int fd"));
        assert!(libc_socket_min.contains("int connect(int fd"));
        assert!(libc_socket_min.contains("int accept(int fd"));
        assert!(libc_socket_min.contains("int getsockname(int fd"));
        assert!(libc_socket_min.contains("int getsockopt(int fd"));
        assert!(libc_socket_min.contains("int setsockopt(int fd"));
        assert!(libc_socket_min.contains("long send(int fd"));
        assert!(libc_socket_min.contains("long recv(int fd"));
        assert!(libc_socket_min.contains("lnp64_complete_status"));
        assert!(libc_socket_min.contains("lnp64_errno_store(lnp64_errno_load())"));
        assert!(libc_socket_min.contains("__lnp_object_ctl"));
        assert!(libc_socket_min.contains("__lnp_push"));
        assert!(libc_socket_min.contains("__lnp_pull"));
        assert!(sys_socket_header.contains("#define AF_INET"));
        assert!(sys_socket_header.contains("#define MSG_NOSIGNAL"));
        assert!(sys_socket_header.contains("int socket(int domain, int type, int protocol);"));
        assert!(netinet_in_header.contains("#define IPPROTO_TCP"));
        assert!(real_llc.contains("socket-libc-clang-smoke.o"));
        assert!(real_llc.contains("#include <sys/socket.h>"));
        assert!(
            real_llc.contains(
                "-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_socket_impl_c\""
            )
        );
        assert!(
            real_llc
                .contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$socket_libc_c\"")
        );
        assert!(real_llc.contains("socket(AF_INET, SOCK_STREAM, 0)"));
        assert!(real_llc.contains("setsockopt(server, SOL_SOCKET, SO_REUSEADDR"));
        assert!(real_llc.contains("getsockopt(server, SOL_SOCKET, SO_ERROR"));
        assert!(real_llc.contains("bind(server, \"127.0.0.1:0\", 0)"));
        assert!(real_llc.contains("connect(client, addr, addrlen)"));
        assert!(real_llc.contains("accept(server, 0, 0)"));
        assert!(real_llc.contains("send(client, \"z\", 1, MSG_NOSIGNAL)"));
        assert!(real_llc.contains("recv(accepted, buf, 1, 0)"));
        assert!(real_llc.contains("real LLVM LNP64 clang socket libc object smoke passed"));
        assert!(real_llc.contains("userland/netbsd_personality_clang_smoke.c"));
        assert!(real_llc.contains("netbsd-personality-clang-smoke.o"));
        assert!(netbsd_personality_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(netbsd_personality_clang.contains("#include <poll.h>"));
        assert!(netbsd_personality_clang.contains("#include <sys/mman.h>"));
        assert!(netbsd_personality_clang.contains("#include <sys/socket.h>"));
        assert!(netbsd_personality_clang.contains("MAP_FAILED"));
        assert!(netbsd_personality_clang.contains("PROT_READ | PROT_WRITE"));
        assert!(!netbsd_personality_clang.contains("void *mmap(void *addr"));
        assert!(!netbsd_personality_clang.contains("int socket(int domain"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD personality smoke object passed"));
        assert!(real_llc.contains("liblnp64-socket-min.o"));
        assert!(real_llc.contains("grep -q 'object_ctl r'"));
        assert!(real_llc.contains("grep -q 'push r'"));
        assert!(real_llc.contains("grep -q 'pull r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc socket implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("errno-clang-smoke.o"));
        assert!(real_llc.contains("lnp64_errno_store(22)"));
        assert!(real_llc.contains("real LLVM LNP64 clang errno object smoke passed"));
        assert!(real_llc.contains("startup-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang startup argv/envp object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_stdio_min.c"));
        assert!(libc_stdio_min.contains("#include <stdio.h>"));
        assert!(!libc_stdio_min.contains("typedef struct __lnp64_file FILE;"));
        assert!(libc_stdio_min.contains("int vsnprintf("));
        assert!(libc_stdio_min.contains("int snprintf("));
        assert!(libc_stdio_min.contains("FILE *fmemopen("));
        assert!(libc_stdio_min.contains("int vsnprintf(char *str, size_t size"));
        assert!(libc_stdio_min.contains("int snprintf(char *str, size_t size"));
        assert!(libc_stdio_min.contains("FILE *fmemopen(void *buf, size_t size"));
        assert!(libc_stdio_min.contains("size_t fread(void *ptr, size_t size, size_t count"));
        assert!(
            libc_stdio_min.contains("size_t fwrite(const void *ptr, size_t size, size_t count")
        );
        assert!(libc_stdio_min.contains("char *fgets("));
        assert!(libc_stdio_min.contains("int fseek(FILE *stream"));
        assert!(libc_stdio_min.contains("long ftell(FILE *stream)"));
        assert!(libc_stdio_min.contains("int fseeko(FILE *stream, off_t offset, int whence)"));
        assert!(libc_stdio_min.contains("off_t ftello(FILE *stream)"));
        assert!(libc_stdio_min.contains("int fscanf(FILE *stream"));
        assert!(libc_stdio_min.contains("FILE *tmpfile(void)"));
        assert!(libc_stdio_min.contains("int fileno(FILE *stream)"));
        assert!(stdio_header.contains("#include <stdarg.h>"));
        assert!(stdio_header.contains("#include <stddef.h>"));
        assert!(stdio_header.contains("#include <sys/types.h>"));
        assert!(
            stdio_header.contains("int vfprintf(FILE *stream, const char *format, va_list ap);")
        );
        assert!(stdio_header.contains("int fseeko(FILE *stream, off_t offset, int whence);"));
        assert!(stdio_header.contains("off_t ftello(FILE *stream);"));
        assert!(
            stdio_header
                .contains("int vsnprintf(char *str, size_t size, const char *format, va_list ap);")
        );
        assert!(
            stdio_header.contains("int snprintf(char *str, size_t size, const char *format, ...);")
        );
        assert!(stdio_header.contains("ssize_t getline(char **lineptr, size_t *n, FILE *stream);"));
        assert!(stdio_header.contains("size_t fread(void *ptr, size_t size, size_t count"));
        assert!(stdio_header.contains("size_t fwrite(const void *ptr, size_t size, size_t count"));
        assert!(stdio_header.contains("FILE *fmemopen(void *buf, size_t size"));
        assert!(!stdio_header.contains("__builtin_va_list"));
        assert!(real_llc.contains("liblnp64-stdio-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_stdio_impl_c\""));
        assert!(real_llc.contains("grep -q '<vsnprintf>:'"));
        assert!(real_llc.contains("grep -q '<snprintf>:'"));
        assert!(real_llc.contains("grep -q '<tmpfile>:'"));
        assert!(real_llc.contains("grep -q '<fileno>:'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc stdio implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("getauxval-clang-smoke.o"));
        assert!(real_llc.contains("#include <sys/auxv.h>"));
        assert!(
            real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$getauxval_c\"")
        );
        assert!(real_llc.contains("real LLVM LNP64 clang getauxval object smoke passed"));
        assert!(real_llc.contains("libc-string-clang-smoke.o"));
        assert!(real_llc.contains("#include <ctype.h>"));
        assert!(real_llc.contains("#include <string.h>"));
        assert!(real_llc.contains("strcmp(\"abc\", \"abc\")"));
        assert!(real_llc.contains("strncmp(\"abcdef\", \"abcxyz\", 3)"));
        assert!(real_llc.contains("strcpy(dst, \"xy\")"));
        assert!(real_llc.contains("strncpy(bounded, \"abc\", 6)"));
        assert!(real_llc.contains("strncat(bounded, \"zpq\", 1)"));
        assert!(real_llc.contains("strchr(\"abcd\", 'c')"));
        assert!(real_llc.contains("strrchr(scan, 'a')"));
        assert!(real_llc.contains("strstr(\"abcde\", \"bcd\")"));
        assert!(real_llc.contains("strspn(\"abc123\", \"abc\")"));
        assert!(real_llc.contains("strcspn(\"abc123\", \"321\")"));
        assert!(real_llc.contains("strpbrk(\"abc123\", \"29\")"));
        assert!(real_llc.contains("strtok(tokens, \",\")"));
        assert!(real_llc.contains("strlcpy(small, \"abcdef\", sizeof(small))"));
        assert!(real_llc.contains("strlcat(small, \"cdef\", sizeof(small))"));
        assert!(real_llc.contains("memmem(hay, 7, needle, 2)"));
        assert!(real_llc.contains("tolower('Q')"));
        assert!(real_llc.contains("toupper('q')"));
        assert!(real_llc.contains("grep -q 'sext.w'"));
        assert!(stdarg_header.contains("typedef __builtin_va_list va_list;"));
        assert!(stdarg_header.contains("#define va_start(ap, last) __builtin_va_start(ap, last)"));
        assert!(stdarg_header.contains("#define va_arg(ap, type) __builtin_va_arg(ap, type)"));
        assert!(stddef_header.contains("typedef unsigned long size_t;"));
        assert!(stddef_header.contains("typedef long ptrdiff_t;"));
        assert!(stddef_header.contains("#define NULL ((void *)0)"));
        assert!(string_header.contains("#include <stddef.h>"));
        assert!(libc_string_min.contains("#include <string.h>"));
        assert!(!libc_string_min.contains("typedef unsigned long size_t;"));
        assert!(libc_string_min.contains("void *memmove"));
        assert!(libc_string_min.contains("int strcmp"));
        assert!(libc_string_min.contains("int strncmp"));
        assert!(libc_string_min.contains("char *strcpy"));
        assert!(libc_string_min.contains("char *strncpy"));
        assert!(libc_string_min.contains("char *strncat"));
        assert!(libc_string_min.contains("char *strchr"));
        assert!(libc_string_min.contains("char *strrchr"));
        assert!(libc_string_min.contains("char *strstr"));
        assert!(libc_string_min.contains("size_t strspn"));
        assert!(libc_string_min.contains("size_t strcspn"));
        assert!(libc_string_min.contains("char *strpbrk"));
        assert!(libc_string_min.contains("char *strtok"));
        assert!(libc_string_min.contains("size_t strlcpy"));
        assert!(libc_string_min.contains("size_t strlcat"));
        assert!(libc_string_min.contains("void *memmem"));
        assert!(libc_string_min.contains("int isalpha"));
        assert!(libc_string_min.contains("int isdigit"));
        assert!(libc_string_min.contains("int isspace"));
        assert!(libc_string_min.contains("int isxdigit"));
        assert!(libc_string_min.contains("int tolower"));
        assert!(libc_string_min.contains("int toupper"));
        assert!(real_llc.contains("real LLVM LNP64 clang minilibc string object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_string_min.c"));
        assert!(real_llc.contains("liblnp64-string-min.o"));
        assert!(
            real_llc.contains(
                "-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_string_impl_c\""
            )
        );
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc string implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("convert-clang-smoke.o"));
        assert!(stdlib_header.contains("char *getenv(const char *name);"));
        assert!(
            stdlib_header
                .contains("int setenv(const char *name, const char *value, int overwrite);")
        );
        assert!(stdlib_header.contains("int unsetenv(const char *name);"));
        assert!(stdlib_header.contains("int clearenv(void);"));
        assert!(stdlib_header.contains("int putenv(char *string);"));
        assert!(stdlib_header.contains("long random(void);"));
        assert!(stdlib_header.contains("void srandom(unsigned int seed);"));
        assert!(
            stdlib_header.contains("char *initstate(unsigned int seed, char *state, size_t size);")
        );
        assert!(stdlib_header.contains("char *setstate(char *state);"));
        assert!(stdlib_header.contains("int atoi(const char *nptr);"));
        assert!(stdlib_header.contains("long atol(const char *nptr);"));
        assert!(stdlib_header.contains("long strtol(const char *nptr, char **endptr, int base);"));
        assert!(
            stdlib_header
                .contains("unsigned long strtoul(const char *nptr, char **endptr, int base);")
        );
        assert!(
            stdlib_header.contains("long long strtoll(const char *nptr, char **endptr, int base);")
        );
        assert!(
            stdlib_header.contains(
                "unsigned long long strtoull(const char *nptr, char **endptr, int base);"
            )
        );
        assert!(real_llc.contains("#include <errno.h>"));
        assert!(real_llc.contains("#include <stdlib.h>"));
        assert!(
            real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$convert_c\"")
        );
        assert!(real_llc.contains("errno = 0;"));
        assert!(real_llc.contains("strtol(s, &end, 8)"));
        assert!(real_llc.contains("strtol(s, &end, 37)"));
        assert!(real_llc.contains("strtoull(s, &end, 0)"));
        assert!(real_llc.contains("real LLVM LNP64 clang numeric conversion object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_convert_min.c"));
        assert!(real_llc.contains("liblnp64-convert-min.o"));
        assert!(
            real_llc.contains(
                "-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_convert_impl_c\""
            )
        );
        assert!(libc_convert_min.contains("#include <errno.h>"));
        assert!(libc_convert_min.contains("#include <stdlib.h>"));
        assert!(libc_convert_min.contains("strtoull"));
        assert!(libc_convert_min.contains("strtoll"));
        assert!(libc_convert_min.contains("double strtod(const char *nptr, char **endptr)"));
        assert!(libc_convert_min.contains("int __ltdf2(double lhs, double rhs)"));
        assert!(libc_convert_min.contains("int __gtdf2(double lhs, double rhs)"));
        assert!(libc_convert_min.contains("lnp64_errno_store(EINVAL)"));
        assert!(libc_convert_min.contains("lnp64_errno_store(ERANGE)"));
        assert!(real_llc.contains(
            "real LLVM LNP64 clang minilibc numeric conversion implementation object smoke passed"
        ));
        assert!(real_llc.contains("path-clang-smoke.o"));
        assert!(real_llc.contains("#include <libgen.h>"));
        assert!(real_llc.contains("#include <string.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$path_c\""));
        assert!(real_llc.contains("check_basename(\"/usr/lib\", \"lib\")"));
        assert!(real_llc.contains("check_dirname(\"/usr/lib\", \"/usr\")"));
        assert!(real_llc.contains("real LLVM LNP64 clang path helper object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_path_min.c"));
        assert!(real_llc.contains("liblnp64-path-min.o"));
        assert!(
            real_llc
                .contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_path_impl_c\"")
        );
        assert!(libc_path_min.contains("#include <libgen.h>"));
        assert!(libc_path_min.contains("#include <string.h>"));
        assert!(libc_path_min.contains("char *basename"));
        assert!(libc_path_min.contains("char *dirname"));
        assert!(libc_path_min.contains("end = strlen(path);"));
        assert!(libc_path_min.contains("lnp64_dot"));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang minilibc path implementation object smoke passed")
        );
        assert!(real_llc.contains("search-clang-smoke.o"));
        assert!(real_llc.contains("#include <search.h>"));
        assert!(real_llc.contains("#include <string.h>"));
        assert!(
            real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$search_c\"")
        );
        assert!(real_llc.contains("get(key_a)"));
        assert!(real_llc.contains("remque(p->p)"));
        assert!(real_llc.contains("real LLVM LNP64 clang search helper object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_search_min.c"));
        assert!(real_llc.contains("liblnp64-search-min.o"));
        assert!(libc_search_min.contains("#include <search.h>"));
        assert!(libc_search_min.contains("#include <string.h>"));
        assert!(!libc_search_min.contains("typedef unsigned long size_t;"));
        assert!(libc_search_min.contains("void *lfind"));
        assert!(libc_search_min.contains("void *lsearch"));
        assert!(libc_search_min.contains("void insque"));
        assert!(libc_search_min.contains("void remque"));
        assert!(libc_search_min.contains("lnp64_search_copy_key(found, key, width)"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc search implementation object smoke passed"
            )
        );
        assert!(
            real_llc.contains(
                "-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_search_impl_c\""
            )
        );
        assert!(real_llc.contains("sort-clang-smoke.o"));
        assert!(stdint_header.contains("typedef unsigned long uint64_t;"));
        assert!(stdint_header.contains("typedef long intmax_t;"));
        assert!(stdint_header.contains("typedef unsigned long uintmax_t;"));
        assert!(stdint_header.contains("#define INTMAX_MAX INT64_MAX"));
        assert!(stdint_header.contains("#define UINTMAX_MAX UINT64_MAX"));
        assert!(stdint_header.contains("#define SIZE_MAX UINT64_MAX"));
        assert!(real_llc.contains("#include <stdint.h>"));
        assert!(real_llc.contains("#include <stdlib.h>"));
        assert!(real_llc.contains("#include <string.h>"));
        assert!(!real_llc.contains("typedef unsigned long uint64_t;"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$sort_c\""));
        assert!(real_llc.contains("qsort(names, 6"));
        assert!(real_llc.contains("qsort(nums, 8"));
        assert!(real_llc.contains("qsort(chars, sizeof chars - 1"));
        assert!(real_llc.contains("qsort(wide, 6"));
        assert!(real_llc.contains("real LLVM LNP64 clang sort helper object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_sort_min.c"));
        assert!(real_llc.contains("liblnp64-sort-min.o"));
        assert!(libc_sort_min.contains("#include <stdlib.h>"));
        assert!(!libc_sort_min.contains("typedef unsigned long size_t;"));
        assert!(libc_sort_min.contains("void qsort"));
        assert!(libc_sort_min.contains("lnp64_swap_bytes"));
        assert!(libc_sort_min.contains("compar(prev, cur)"));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang minilibc sort implementation object smoke passed")
        );
        assert!(
            real_llc
                .contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_sort_impl_c\"")
        );
        assert!(real_llc.contains("toolchain/liblnp64_alloc_min.c"));
        assert!(libc_alloc_min.contains("#include <stdlib.h>"));
        assert!(libc_alloc_min.contains("#include <string.h>"));
        assert!(!libc_alloc_min.contains("typedef unsigned long size_t;"));
        assert!(libc_alloc_min.contains("void *alloc(size_t size)"));
        assert!(libc_alloc_min.contains("__lnp_alloc(size)"));
        assert!(libc_alloc_min.contains("__lnp_alloc_size(ptr)"));
        assert!(stdlib_header.contains("void *calloc(size_t count, size_t size);"));
        assert!(stdlib_header.contains("void *realloc(void *ptr, size_t size);"));
        assert!(stdlib_header.contains("void free(void *ptr);"));
        assert!(real_llc.contains("liblnp64-alloc-min.o"));
        assert!(
            real_llc.contains(
                "-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_alloc_impl_c\""
            )
        );
        assert!(real_llc.contains("grep -q 'alloc r'"));
        assert!(real_llc.contains("grep -q 'alloc_size r'"));
        assert!(real_llc.contains("grep -q 'free r'"));
        assert!(real_llc.contains(
            "real LLVM LNP64 clang minilibc allocation implementation object smoke passed"
        ));
        assert!(real_llc.contains("calloc-clang-smoke.o"));
        assert!(
            real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$calloc_c\"")
        );
        assert!(real_llc.contains("real LLVM LNP64 clang calloc object smoke passed"));
        assert!(real_llc.contains("realloc-clang-smoke.o"));
        assert!(
            real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$realloc_c\"")
        );
        assert!(real_llc.contains("real LLVM LNP64 clang realloc object smoke passed"));
        assert!(real_llc.contains("read-clang-smoke.o"));
        assert!(unistd_header.contains("ssize_t read(int fd, void *buf, size_t count);"));
        assert!(unistd_header.contains("ssize_t write(int fd, const void *buf, size_t count);"));
        assert!(real_llc.contains("#include <unistd.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$read_c\""));
        assert!(real_llc.contains("real LLVM LNP64 clang read object smoke passed"));
        assert!(real_llc.contains("write-clang-smoke.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$write_c\""));
        assert!(real_llc.contains("fd write ok"));
        assert!(real_llc.contains("real LLVM LNP64 clang write object smoke passed"));
        assert!(real_llc.contains("userland/ucat_clang.c"));
        assert!(real_llc.contains("userland-ucat-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang userland ucat object smoke passed"));
        assert!(real_llc.contains("userland/init_clang.c"));
        assert!(real_llc.contains("userland-init-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang userland init object smoke passed"));
        assert!(real_llc.contains("userland/lnpsh_clang.c"));
        assert!(real_llc.contains("userland-lnpsh-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang userland lnpsh object smoke passed"));
        assert!(real_llc.contains("userland/spawn_task_clang.c"));
        assert!(lnp64_intrinsics_target_header.contains("../../lnp64_intrinsics.h"));
        assert!(spawn_task_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("userland-spawn-task-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'clone.spawn r'"));
        assert!(real_llc.contains("grep -q 'thread_join r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang userland spawn task object smoke passed"));
        assert!(real_llc.contains("userland/netbsd_init_clang.c"));
        assert!(real_llc.contains("netbsd-init-clang-smoke.o"));
        assert!(netbsd_init_clang.contains("execl(\"/bin/netbsd_sh.elf\""));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD init object passed"));
        assert!(real_llc.contains("userland/netbsd_sh_clang.c"));
        assert!(real_llc.contains("netbsd-sh-clang-smoke.o"));
        assert!(netbsd_sh_clang.contains("\"/bin/fork_wait_test.elf\""));
        assert!(netbsd_sh_clang.contains("\"/bin/domain_budget_test.elf\""));
        assert!(!netbsd_sh_clang.contains(".s\""));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD shell object passed"));
        assert!(real_llc.contains("userland/loader_target_clang.c"));
        assert!(real_llc.contains("netbsd-loader-target-clang-smoke.o"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang NetBSD loader target child object passed")
        );
        assert!(real_llc.contains("userland/elf_exec_test_clang.c"));
        assert!(real_llc.contains("netbsd-elf-exec-test-clang-smoke.o"));
        assert!(
            elf_exec_test_clang.contains("execl(\"/bin/loader_target.elf\", \"loader_target\", 0)")
        );
        assert!(elf_exec_test_clang.contains("execv(\"/bin/loader_target.elf\", argv)"));
        assert!(elf_exec_test_clang.contains("execve(\"/bin/loader_target.elf\", argv, envp)"));
        assert!(elf_exec_test_clang.contains("execvp(\"/bin/loader_target.elf\", argv)"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD ELF exec parent object passed"));
        assert!(real_llc.contains("userland/fork_wait_test_clang.c"));
        assert!(real_llc.contains("netbsd-fork-wait-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'call ' "$netbsd_fork_wait_test_dump""#));
        assert!(fork_wait_test_clang.contains("#include <pthread.h>"));
        assert!(fork_wait_test_clang.contains("pthread_atfork("));
        assert!(fork_wait_test_clang.contains("atfork_prepare_count != 1"));
        assert!(fork_wait_test_clang.contains("atfork_parent_count != 1"));
        assert!(fork_wait_test_clang.contains("atfork_child_count != 0"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD fork/wait child object passed"));
        assert!(real_llc.contains("userland/thread_test_clang.c"));
        assert!(real_llc.contains("netbsd-thread-test-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD thread child object passed"));
        assert!(real_llc.contains("userland/poll_test_clang.c"));
        assert!(real_llc.contains("netbsd-poll-test-clang-smoke.o"));
        assert!(poll_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(poll_test_clang.contains("#include <poll.h>"));
        assert!(poll_test_clang.contains("#include <sys/epoll.h>"));
        assert!(poll_test_clang.contains("#include <sys/event.h>"));
        assert!(poll_test_clang.contains("#include <sys/select.h>"));
        assert!(poll_test_clang.contains("kqueue()"));
        assert!(poll_test_clang.contains("change.filter = EVFILT_READ"));
        assert!(poll_test_clang.contains("kevent(kq, &change, 1, 0, 0, &ktimeout)"));
        assert!(poll_test_clang.contains("kevent(kq, 0, 0, &kout, 1, &ktimeout)"));
        assert!(poll_test_clang.contains("epoll_ctl(ep, EPOLL_CTL_MOD"));
        assert!(poll_test_clang.contains("epoll_ctl(ep, EPOLL_CTL_DEL"));
        assert!(poll_test_clang.contains("out.data != read_cap + 1"));
        assert!(poll_test_clang.contains("change.flags = EV_ADD | EV_ONESHOT"));
        assert!(poll_test_clang.contains("change.flags = EV_ADD | EV_DISABLE"));
        assert!(poll_test_clang.contains("change.flags = EV_ENABLE"));
        assert!(poll_test_clang.contains("change.flags = EV_ADD | EV_RECEIPT"));
        assert!(poll_test_clang.contains("kout.data != 0"));
        assert!(poll_test_clang.contains("change.flags = EV_DELETE"));
        assert!(poll_test_clang.contains("change.filter = 99"));
        assert!(poll_test_clang.contains("kevent(kq, &change, 1, 0, 0, &ktimeout) != -1"));
        assert!(poll_test_clang.contains("kout.flags & EV_ERROR"));
        assert!(poll_test_clang.contains("kout.data != 22"));
        assert!(poll_test_clang.contains("change.filter = EVFILT_USER"));
        assert!(poll_test_clang.contains("change.fflags = NOTE_TRIGGER"));
        assert!(poll_test_clang.contains("kout.fflags != NOTE_TRIGGER"));
        assert!(poll_test_clang.contains("change.filter = EVFILT_WRITE"));
        assert!(poll_test_clang.contains("kout.filter != EVFILT_WRITE"));
        assert!(poll_test_clang.contains("close(kq) != 0"));
        assert!(poll_test_clang.contains("kevent(kq, 0, 0, &kout, 1, &ktimeout) != -1"));
        assert!(!poll_test_clang.contains("typedef unsigned long nfds_t"));
        assert!(!poll_test_clang.contains("int poll(struct pollfd"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD poll child object passed"));
        assert!(real_llc.contains("userland/signal_gate_test_clang.c"));
        assert!(signal_gate_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-signal-gate-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'yield' "$netbsd_signal_gate_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD signal gate child object passed"));
        assert!(real_llc.contains("userland/signal_fault_test_clang.c"));
        assert!(signal_fault_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-signal-fault-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'div r' "$netbsd_signal_fault_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'sigret' "$netbsd_signal_fault_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD signal fault child object passed"));
        assert!(real_llc.contains("userland/timer_test_clang.c"));
        assert!(real_llc.contains("netbsd-timer-test-clang-smoke.o"));
        assert!(timer_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(timer_test_clang.contains("#include <poll.h>"));
        assert!(!timer_test_clang.contains("typedef unsigned long nfds_t"));
        assert!(!timer_test_clang.contains("int poll(struct pollfd"));
        assert!(real_llc.contains(r#"grep -q 'yield' "$netbsd_timer_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'sigret' "$netbsd_timer_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD timer child object passed"));
        assert!(real_llc.contains("userland/mmap_test_clang.c"));
        assert!(real_llc.contains("netbsd-mmap-test-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD mmap child object passed"));
        assert!(real_llc.contains("userland/socket_loopback_test_clang.c"));
        assert!(real_llc.contains("netbsd-socket-loopback-test-clang-smoke.o"));
        assert!(socket_loopback_test_clang.contains("#include <poll.h>"));
        assert!(!socket_loopback_test_clang.contains("typedef unsigned long nfds_t"));
        assert!(!socket_loopback_test_clang.contains("int poll(struct pollfd"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang NetBSD socket loopback child object passed")
        );
        assert!(real_llc.contains("userland/gate_trace_test_clang.c"));
        assert!(gate_trace_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-gate-trace-test-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_domain_create"));
        assert!(real_llc.contains("__lnp_call_gate_create"));
        assert!(real_llc.contains(r#"grep -q 'domain_ctl r' "$netbsd_gate_trace_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'object_ctl r' "$netbsd_gate_trace_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'gate_call r' "$netbsd_gate_trace_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'gate_return r' "$netbsd_gate_trace_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD gate trace child object passed"));
        assert!(real_llc.contains("toolchain/liblnp64_fd_min.c"));
        assert!(libc_fd_min.contains("#include <fcntl.h>"));
        assert!(libc_fd_min.contains("#include <unistd.h>"));
        assert!(!libc_fd_min.contains("typedef unsigned long size_t;"));
        assert!(libc_fd_min.contains("int openat(int dirfd, const char *path, int flags, ...)"));
        assert!(libc_fd_min.contains("int open(const char *path, int flags, ...)"));
        assert!(libc_fd_min.contains("int creat(const char *path, mode_t mode)"));
        assert!(libc_fd_min.contains("ssize_t read(int fd, void *buf, size_t len)"));
        assert!(libc_fd_min.contains("ssize_t write(int fd, const void *buf, size_t len)"));
        assert!(libc_fd_min.contains("__lnp_pull"));
        assert!(libc_fd_min.contains("__lnp_push"));
        assert!(libc_fd_min.contains("off_t lseek(int fd, off_t offset, int whence)"));
        assert!(libc_fd_min.contains("fd_seek_dyn %1, %2, %3"));
        assert!(libc_fd_min.contains("LNP64_FDR_TOKEN_MARKER"));
        assert!(libc_fd_min.contains("LNP64_FDR_TOKEN_INDEX_MASK"));
        assert!(libc_fd_min.contains("int close(int fd)"));
        assert!(libc_fd_min.contains("lnp64_kqueue_close(fd)"));
        assert!(libc_fd_min.contains("__lnp_cap_revoke"));
        assert!(real_llc.contains("liblnp64-fd-min.o"));
        assert!(
            real_llc
                .contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_fd_impl_c\"")
        );
        assert!(real_llc.contains("grep -q 'pull r'"));
        assert!(real_llc.contains("grep -q 'fd_seek_dyn r'"));
        assert!(real_llc.contains("grep -q 'push r'"));
        assert!(real_llc.contains("grep -q 'cap_revoke r'"));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang minilibc fd implementation object smoke passed")
        );
        assert!(real_llc.contains("toolchain/liblnp64_meta_min.c"));
        assert!(libc_meta_min.contains("stat_path_at"));
        assert!(libc_meta_min.contains("stat_fd_dyn"));
        assert!(libc_meta_min.contains("utime_path_at"));
        assert!(libc_meta_min.contains("utime_fd_dyn"));
        assert!(libc_meta_min.contains("fcntl_fd_dyn"));
        assert!(libc_meta_min.contains("int access(const char *path, int mode)"));
        assert!(libc_meta_min.contains("static struct stat lnp64_access_stat"));
        assert!(libc_meta_min.contains("int mknod(const char *path, mode_t mode, dev_t dev)"));
        assert!(libc_meta_min.contains("lnp64_errno_store(ENOSYS)"));
        assert!(
            libc_meta_min.contains("lnp64_stat_path_at(AT_FDCWD, path, &lnp64_access_stat, 0)")
        );
        assert!(libc_meta_min.contains("DIR *opendir(const char *name)"));
        assert!(libc_meta_min.contains("int mkdirat(int dirfd, const char *path, mode_t mode)"));
        assert!(libc_meta_min.contains("int renameat(int olddirfd, const char *oldpath"));
        assert!(libc_meta_min.contains("ssize_t readlinkat("));
        assert!(libc_meta_min.contains("int fchmodat("));
        assert!(libc_meta_min.contains("int fchownat("));
        assert!(libc_meta_min.contains("int faccessat("));
        assert!(libc_meta_min.contains("va_arg(ap, long)"));
        assert!(libc_meta_min.contains("lnp64_complete_status"));
        assert!(real_llc.contains("liblnp64-meta-min.o"));
        assert!(real_llc.contains("grep -q 'stat_path_at r'"));
        assert!(real_llc.contains("grep -q 'stat_fd_dyn r'"));
        assert!(real_llc.contains("grep -q 'utime_path_at r'"));
        assert!(real_llc.contains("grep -q 'utime_fd_dyn r'"));
        assert!(real_llc.contains("grep -q 'fcntl_fd_dyn r'"));
        assert!(real_llc.contains("grep -q 'open_dir_dyn r'"));
        assert!(real_llc.contains("grep -q 'mkdir_path_at r'"));
        assert!(real_llc.contains("grep -q 'rename_path_at r'"));
        assert!(real_llc.contains("grep -q 'link_path_at r'"));
        assert!(real_llc.contains("grep -q 'symlink_path_at r'"));
        assert!(real_llc.contains("grep -q 'readlink_path_at r'"));
        assert!(real_llc.contains("grep -q 'getcwd_path r'"));
        assert!(real_llc.contains("grep -q 'readdir_fd_dyn r1, r2'"));
        assert!(real_llc.contains("grep -q 'chmod_path_at r'"));
        assert!(real_llc.contains("grep -q 'chown_path_at r'"));
        assert!(real_llc.contains("grep -q 'errno_get r'"));
        assert!(real_llc.contains(
            "real LLVM LNP64 clang minilibc metadata implementation object smoke passed"
        ));
        assert!(real_llc.contains("meta-libc-clang-smoke.o"));
        assert!(real_llc.contains("mkdirat(AT_FDCWD"));
        assert!(real_llc.contains("renameat(AT_FDCWD"));
        assert!(real_llc.contains("readlinkat(AT_FDCWD"));
        assert!(real_llc.contains("opendir(\"target/llvm-lnp64-build\")"));
        assert!(real_llc.contains("S_ISREG(st.st_mode)"));
        assert!(real_llc.contains("st.st_nlink <= 0"));
        assert!(real_llc.contains("futimens(-1, omit)"));
        assert!(real_llc.contains("errno != EBADF"));
        assert!(real_llc.contains("real LLVM LNP64 clang metadata libc object smoke passed"));
        assert!(real_llc.contains("stack-args-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang stack-argument object smoke passed"));
        assert!(real_llc.contains("large-frame-clang-smoke.o"));
        // v2/E8: large frames adjust the SP with a single ADDI (full signed-32
        // immediate), not a `li r30; sub` scratch sequence -- and r30 never
        // appears (it is a normal allocatable GPR, not a reserved scratch).
        assert!(real_llc.contains("grep -q 'addi r31, r31, -40008'"));
        assert!(real_llc.contains("grep -q 'addi r31, r31, 40008'"));
        assert!(real_llc.contains("! grep -q 'r30'"));
        assert!(real_llc.contains("real LLVM LNP64 clang large-frame object smoke passed"));
        assert!(real_llc.contains("toolchain/crt0_lnp64.s"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc crt0 smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_min.s"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc minilibc smoke passed"));
        assert!(real_llc.contains("liblnp64-min-smoke.dump"));
        assert!(real_llc.contains("pull r2, r2, r3, r4"));
        assert!(real_llc.contains("alloc r2, r2"));
        assert!(real_llc.contains("alloc_size r4, r3"));
        assert!(real_llc.contains("free r2"));
        assert!(
            real_llc.contains("real LLVM LNP64 llvm-objdump minilibc native decode smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-string-linked.elf"));
        assert!(real_llc.contains(r#""$libc_string_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld minilibc string link smoke passed"));
        assert!(real_llc.contains("lnp64-convert-linked.elf"));
        assert!(real_llc.contains(
            r#""$convert_obj" "$libc_convert_impl_obj" \
  "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld numeric conversion link smoke passed"));
        assert!(real_llc.contains("lnp64-path-linked.elf"));
        assert!(real_llc.contains(
            r#""$path_obj" "$libc_path_impl_obj" \
  "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld path helper link smoke passed"));
        assert!(real_llc.contains("lnp64-search-linked.elf"));
        assert!(real_llc.contains(
            r#""$search_obj" "$libc_search_impl_obj" \
  "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld search helper link smoke passed"));
        assert!(real_llc.contains("lnp64-sort-linked.elf"));
        assert!(real_llc.contains(
            r#""$sort_obj" "$libc_sort_impl_obj" \
  "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld sort helper link smoke passed"));
        assert!(real_llc.contains("lnp64-calloc-linked.elf"));
        assert!(real_llc.contains(
            r#""$calloc_obj" "$libc_alloc_impl_obj" \
  "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld calloc link smoke passed"));
        assert!(real_llc.contains("lnp64-realloc-linked.elf"));
        assert!(real_llc.contains(
            r#""$realloc_obj" "$libc_alloc_impl_obj" \
  "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld realloc link smoke passed"));
        assert!(real_llc.contains("lnp64-read-linked.elf"));
        assert!(real_llc.contains(r#""$read_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld read link smoke passed"));
        assert!(real_llc.contains("lnp64-write-linked.elf"));
        assert!(real_llc.contains(r#""$write_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld write link smoke passed"));
        assert!(real_llc.contains("lnp64-userland-ucat-linked.elf"));
        assert!(real_llc.contains(r#""$userland_ucat_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld userland ucat link smoke passed"));
        assert!(real_llc.contains("lnp64-userland-init-linked.elf"));
        assert!(real_llc.contains(r#""$userland_init_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld userland init link smoke passed"));
        assert!(real_llc.contains("lnp64-userland-lnpsh-linked.elf"));
        assert!(real_llc.contains(r#""$userland_lnpsh_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld userland lnpsh link smoke passed"));
        assert!(real_llc.contains("lnp64-userland-spawn-task-linked.elf"));
        assert!(real_llc.contains(r#""$userland_spawn_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld userland spawn task link smoke passed"));
        assert!(real_llc.contains("lnp64-netbsd-init-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_init_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD init link passed"));
        assert!(real_llc.contains("lnp64-netbsd-sh-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_sh_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD shell link passed"));
        assert!(real_llc.contains("lnp64-netbsd-loader-target-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_loader_target_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD loader target child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-elf-exec-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_elf_exec_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD ELF exec parent link passed"));
        assert!(real_llc.contains("lnp64-netbsd-fork-wait-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_fork_wait_test_obj" \"#));
        assert!(
            real_llc
                .contains(r#""$libc_process_impl_obj" "$libc_errno_impl_obj" "$libc_fd_impl_obj""#)
        );
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD fork/wait child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-thread-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_thread_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_string_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD thread child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-poll-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_poll_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD poll child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-signal-gate-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_signal_gate_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD signal gate child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-signal-fault-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_signal_fault_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD signal fault child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-timer-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_timer_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_time_impl_obj" "$libc_signal_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD timer child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-mmap-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_mmap_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_vma_impl_obj" "$libc_errno_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD mmap child link passed"));
        assert!(real_llc.contains("userland/fd_passing_test_clang.c"));
        assert!(fd_passing_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-fd-passing-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'cap_dup r' "$netbsd_fd_passing_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'cap_send r' "$netbsd_fd_passing_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'cap_recv r' "$netbsd_fd_passing_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'cap_revoke r' "$netbsd_fd_passing_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD fd passing child object passed"));
        assert!(real_llc.contains("lnp64-netbsd-fd-passing-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_fd_passing_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD fd passing child link passed"));
        assert!(real_llc.contains("userland/namespace_test_clang.c"));
        assert!(real_llc.contains("netbsd-namespace-test-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD namespace child object passed"));
        assert!(real_llc.contains("lnp64-netbsd-namespace-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_namespace_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_meta_impl_obj" "$libc_errno_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD namespace child link passed"));
        assert!(real_llc.contains("userland/fs_service_test_clang.c"));
        assert!(real_llc.contains("netbsd-fs-service-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'ld.b r' "$netbsd_fs_service_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'st.b r' "$netbsd_fs_service_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD fs service child object passed"));
        assert!(real_llc.contains("lnp64-netbsd-fs-service-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_fs_service_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_alloc_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD fs service child link passed"));
        assert!(real_llc.contains("userland/classifier_test_clang.c"));
        assert!(classifier_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-classifier-test-clang-smoke.o"));
        assert!(classifier_test_clang.contains("#include <poll.h>"));
        assert!(!classifier_test_clang.contains("typedef unsigned long nfds_t"));
        assert!(!classifier_test_clang.contains("int poll(struct pollfd"));
        assert!(real_llc.contains(r#"grep -q 'object_ctl r' "$netbsd_classifier_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'cap_dup r' "$netbsd_classifier_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'pull r' "$netbsd_classifier_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD classifier child object passed"));
        assert!(real_llc.contains("lnp64-netbsd-classifier-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_classifier_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_poll_impl_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("netbsd-classifier-test-linked.dump"));
        assert!(real_llc.contains(r#"grep -q 'await r' "$netbsd_classifier_test_linked_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD classifier child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-socket-loopback-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_socket_loopback_test_obj" "$libc_socket_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD socket loopback child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-gate-trace-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_gate_trace_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD gate trace child link passed"));
        assert!(real_llc.contains("userland/domain_nested_test_clang.c"));
        assert!(domain_ctl_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-domain-nested-test-clang-smoke.o"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang NetBSD domain nested child object passed")
        );
        assert!(real_llc.contains("lnp64-netbsd-domain-nested-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_domain_nested_test_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD domain nested child link passed"));
        assert!(real_llc.contains("userland/domain_budget_test_clang.c"));
        assert!(real_llc.contains("netbsd-domain-budget-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'alloc r' "$netbsd_domain_budget_test_dump""#));
        assert!(
            real_llc.contains("real LLVM LNP64 clang NetBSD domain budget child object passed")
        );
        assert!(real_llc.contains("lnp64-netbsd-domain-budget-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_domain_budget_test_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD domain budget child link passed"));
        assert!(real_llc.contains("lnp64-meta-libc-linked.elf"));
        assert!(real_llc.contains(
            r#""$meta_libc_obj" "$libc_meta_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld metadata libc link smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_vma_min.c"));
        assert!(libc_vma_min.contains("#include <sys/mman.h>"));
        assert!(!libc_vma_min.contains("typedef unsigned long size_t;"));
        assert!(libc_vma_min.contains(
            "void *mmap(void *addr, size_t len, int prot, int flags, int fd, off_t offset)"
        ));
        assert!(libc_vma_min.contains("int mprotect("));
        assert!(libc_vma_min.contains("int munmap("));
        assert!(libc_vma_min.contains("lnp64_complete_status"));
        assert!(libc_vma_min.contains("lnp64_complete_ptr"));
        assert!(libc_vma_min.contains("lnp64_errno_store(lnp64_errno_load())"));
        assert!(libc_vma_min.contains("__lnp_mmap_bootstrap"));
        assert!(libc_vma_min.contains("__lnp_mprotect_bootstrap"));
        assert!(libc_vma_min.contains("__lnp_munmap_bootstrap"));
        assert!(sys_mman_header.contains("#define MAP_FAILED"));
        assert!(sys_mman_header.contains("void *mmap("));
        assert!(sys_mman_header.contains("int mprotect("));
        assert!(sys_mman_header.contains("int munmap("));
        assert!(real_llc.contains("liblnp64-vma-min.o"));
        assert!(
            real_llc
                .contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$libc_vma_impl_c\"")
        );
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang minilibc VMA implementation object smoke passed")
        );
        assert!(real_llc.contains("mmap-libc-clang-smoke.o"));
        assert!(real_llc.contains("#include <sys/mman.h>"));
        assert!(
            real_llc.contains("-I toolchain/include \\\n  -I toolchain \\\n  -c \"$mmap_libc_c\"")
        );
        assert!(real_llc.contains("MAP_FAILED"));
        assert!(real_llc.contains("PROT_READ | PROT_WRITE"));
        assert!(real_llc.contains("real LLVM LNP64 clang mmap libc object smoke passed"));
        assert!(real_llc.contains("lnp64-mmap-libc-linked.elf"));
        assert!(real_llc.contains(r#""$mmap_libc_obj" "$libc_vma_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld mmap libc link smoke passed"));
        assert!(real_llc.contains("lnp64-futex-libc-linked.elf"));
        assert!(real_llc.contains(r#""$futex_libc_obj" "$libc_futex_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld futex libc link smoke passed"));
        assert!(real_llc.contains("lnp64-poll-libc-linked.elf"));
        assert!(real_llc.contains(r#""$poll_libc_obj" "$libc_poll_impl_obj""#));
        assert!(
            real_llc
                .contains("real LLVM LNP64 lld poll/select/epoll/kqueue libc link smoke passed")
        );
        assert!(real_llc.contains("lnp64-signal-libc-linked.elf"));
        assert!(real_llc.contains(
            r#""$signal_libc_obj" \
  "$libc_signal_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld signal libc link smoke passed"));
        assert!(real_llc.contains("lnp64-socket-libc-linked.elf"));
        assert!(real_llc.contains(
            r#""$socket_libc_obj" \
  "$libc_socket_impl_obj" "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld socket libc link smoke passed"));
        assert!(real_llc.contains("lnp64-netbsd-personality-clang-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_personality_clang_obj" "$libc_fd_impl_obj" \"#));
        assert!(
            real_llc.contains("real LLVM LNP64 lld NetBSD personality clang smoke link passed")
        );
        assert!(real_llc.contains("lnp64-exit-linked.elf"));
        assert!(real_llc.contains(r#""$exit_obj" "$libc_process_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld exit link smoke passed"));
        assert!(real_llc.contains("lnp64-errno-linked.elf"));
        assert!(real_llc.contains(r#""$errno_obj" "$libc_errno_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld errno link smoke passed"));
        assert!(real_llc.contains("lnp64-startup-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld startup argv/envp link smoke passed"));
        assert!(real_llc.contains("lnp64-getauxval-linked.elf"));
        assert!(real_llc.contains(r#""$getauxval_obj" "$libc_startup_impl_obj" \"#));
        assert!(real_llc.contains(r#""$libc_errno_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld getauxval link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-argv-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_argv_obj" \"#));
        assert!(real_llc.contains(r#""$libc_stdio_impl_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test argv link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-env-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_env_obj" \"#));
        assert!(real_llc.contains(r#""$libc_startup_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains(r#""$libc_errno_impl_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test env link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-random-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_random_obj" \"#));
        assert!(real_llc.contains(r#""$libc_random_impl_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test random link smoke passed"));
        assert!(real_llc.contains("lnp64-scalar-arith-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld scalar arithmetic link smoke passed"));
        assert!(real_llc.contains("lnp64-high-mul-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld high-multiply link smoke passed"));
        assert!(real_llc.contains("lnp64-scalar-extend-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld scalar extension link smoke passed"));
        assert!(real_llc.contains("lnp64-bitmanip-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld bit-manip link smoke passed"));
        assert!(real_llc.contains("lnp64-csel-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld csel link smoke passed"));
        assert!(real_llc.contains("lnp64-call-clobber-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld call-clobber link smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-await-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic await link smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-call-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic call link smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-gate-return-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic gate return link smoke passed"));
        assert!(real_llc.contains("--triple=lnp64-unknown-none"));
        assert!(real_llc.contains("errno_set r0"));
        assert!(real_llc.contains("exit r2"));
        assert!(real_llc.contains("crt0_smoke_obj=\"$build_dir/crt0-smoke.o\""));
        assert!(real_llc.contains("real LLVM LNP64 llvm-objdump crt0 decode smoke passed"));
        assert!(real_llc.contains("native-heap-smoke.o"));
        assert!(real_llc.contains("alloc_ex r3, r1, r2"));
        assert!(real_llc.contains("real LLVM LNP64 native heap opcode smoke passed"));
        assert!(real_llc.contains("linker_script=\"$sysroot/usr/lib/lnp64/lnp64_static.ld\""));
        assert!(real_llc.contains("crt0_obj=\"$sysroot/usr/lib/lnp64/crt0.o\""));
        assert!(real_llc.contains("-T \"$linker_script\""));
        assert!(real_llc.contains("real LLVM LNP64 lld static link smoke passed"));
        assert!(real_llc.contains("lnp64-native-heap-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld native heap link smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-control-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic control link smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-cap-control-linked.elf"));
        assert!(
            real_llc.contains("real LLVM LNP64 lld intrinsic capability control link smoke passed")
        );
        assert!(real_llc.contains("intrinsic-mmap-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_mmap_bootstrap"));
        assert!(real_llc.contains("__lnp_mprotect_bootstrap"));
        assert!(real_llc.contains("__lnp_munmap_bootstrap"));
        assert!(real_llc.contains("grep -q 'mmap r'"));
        assert!(real_llc.contains("grep -q 'mprotect r'"));
        assert!(real_llc.contains("grep -q 'munmap r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic mmap object smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-mmap-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic mmap link smoke passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-mmap-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic mmap execution passed")
        );
        assert!(real_llc.contains("intrinsic-get-pcr-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_get_pid"));
        assert!(real_llc.contains("grep -q 'get_pcr r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic GET_PCR object smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-get-pcr-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic GET_PCR link smoke passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-get-pcr-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic GET_PCR execution passed")
        );
        assert!(real_llc.contains("intrinsic-set-pcr-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_set_thread_pointer"));
        assert!(real_llc.contains("__lnp_set_event_mask"));
        assert!(real_llc.contains("grep -q 'set_pcr r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic SET_PCR object smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-set-pcr-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic SET_PCR link smoke passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-set-pcr-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic SET_PCR execution passed")
        );
        assert!(real_llc.contains("intrinsic-openat-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_openat"));
        assert!(real_llc.contains("grep -q 'open_at r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic OPEN_AT object smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-openat-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic OPEN_AT link smoke passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-openat-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic OPEN_AT execution passed")
        );
        assert!(real_llc.contains("intrinsic-clone-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_spawn_entry"));
        assert!(real_llc.contains("__lnp_thread_join"));
        assert!(real_llc.contains("grep -q 'clone.spawn r'"));
        assert!(real_llc.contains("grep -q 'thread_join r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic CLONE object smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-clone-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic CLONE link smoke passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-clone-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic CLONE execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-poll-libc-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf poll/select/epoll/kqueue libc execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-signal-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf signal libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-socket-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf socket libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-netbsd-personality-clang-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf NetBSD personality clang smoke passed")
        );
        assert!(real_llc_docker.contains("netbsd clang personality smoke ok"));
        assert!(real_llc.contains("lnp64-intrinsic-amo-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic AMO link smoke passed"));
        assert!(real_llc.contains("lnp64-c11-atomic-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld C11 atomic link smoke passed"));
        assert!(real_llc.contains("lnp64-stack-args-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld stack-argument link smoke passed"));
        assert!(real_llc.contains("pcr-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/pcr.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang PCR demo object smoke passed"));
        assert!(real_llc.contains("cat-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/cat.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang cat demo object smoke passed"));
        assert!(real_llc.contains("json-parser-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/json_parser.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang json parser demo object smoke passed"));
        assert!(real_llc.contains("rot13-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/rot13.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang rot13 demo object smoke passed"));
        assert!(real_llc.contains("producer-consumer-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/producer_consumer.c"));
        assert!(real_llc.contains("grep -q 'clone.spawn r'"));
        assert!(real_llc.contains("grep -q 'thread_join r'"));
        assert!(real_llc.contains("grep -q 'lr.d r' \"$producer_consumer_dump\""));
        assert!(real_llc.contains("grep -q 'sc.d r' \"$producer_consumer_dump\""));
        assert!(
            real_llc.contains("real LLVM LNP64 clang producer consumer demo object smoke passed")
        );
        assert!(real_llc.contains("parallel-hash-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/parallel_hash.c"));
        assert!(real_llc.contains("grep -q 'lr.d r' \"$parallel_hash_dump\""));
        assert!(real_llc.contains("grep -q 'sc.d r' \"$parallel_hash_dump\""));
        assert!(real_llc.contains("real LLVM LNP64 clang parallel hash demo object smoke passed"));
        assert!(real_llc.contains("sqlite-lite-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/sqlite_lite.c"));
        assert!(real_llc.contains("grep -q 'mmap r'"));
        assert!(real_llc.contains("grep -q 'fence'"));
        assert!(real_llc.contains("real LLVM LNP64 clang sqlite lite demo object smoke passed"));
        assert!(real_llc.contains("ping-pong-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/ping_pong.c"));
        assert!(real_llc.contains("grep -q 'object_ctl r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang ping pong demo object smoke passed"));
        assert!(real_llc.contains("zlib-adler32-clang-smoke.o"));
        assert!(real_llc.contains("-c third_party/zlib/adler32.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang zlib adler32 object smoke passed"));
        assert!(real_llc.contains("zlib-crc32-clang-smoke.o"));
        assert!(real_llc.contains("-c third_party/zlib/crc32.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang zlib crc32 object smoke passed"));
        assert!(real_llc.contains("zlib-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang zlib package object smoke passed"));
        assert!(real_llc.contains("lnp64-zlib-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld zlib package link smoke passed"));
        assert!(real_llc.contains("natsort-strnatcmp-clang-smoke.o"));
        assert!(real_llc.contains("-c third_party/natsort/strnatcmp.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang natsort implementation object smoke passed")
        );
        assert!(real_llc.contains("natsort-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang natsort package object smoke passed"));
        assert!(real_llc.contains("lnp64-natsort-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld natsort package link smoke passed"));
        assert!(real_llc.contains("jsmn-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang jsmn package object smoke passed"));
        assert!(real_llc.contains("lnp64-jsmn-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld jsmn package link smoke passed"));
        assert!(real_llc.contains("inih-clang-smoke.o"));
        assert!(real_llc.contains("-O0 -ffreestanding"));
        assert!(real_llc.contains("real LLVM LNP64 clang inih package object smoke passed"));
        assert!(real_llc.contains("lnp64-inih-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld inih package link smoke passed"));
        assert!(real_llc.contains("cwalk-clang-impl.o"));
        assert!(real_llc.contains("-c third_party/cwalk/src/cwalk.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang cwalk implementation object smoke passed")
        );
        assert!(real_llc.contains("cwalk-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang cwalk package object smoke passed"));
        assert!(real_llc.contains("lnp64-cwalk-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld cwalk package link smoke passed"));
        assert!(real_llc.contains("varargs-call-clang-smoke.o"));
        assert!(real_llc.contains("printf(\"lnp64 %d %s"));
        assert!(real_llc.contains("real LLVM LNP64 clang varargs call object smoke passed"));
        assert!(real_llc.contains("sbase_commands=("));
        for command in [
            "echo", "cat", "wc", "yes", "basename", "dirname", "head", "tee", "cksum", "tail",
            "cmp", "uniq", "sort", "grep", "sed", "cp", "mv", "ls", "chmod", "chown", "ln",
            "mkdir", "rm", "cut", "tr", "touch", "find",
        ] {
            assert!(real_llc.contains(command));
        }
        assert!(real_llc.contains("-Werror=implicit-function-declaration"));
        assert!(real_llc.contains("sbase-$sbase_cmd-clang-smoke.o"));
        assert!(real_llc.contains("third_party/sbase/$sbase_cmd.c"));
        assert!(transition_manifest.contains("third_party/sbase/fs.h"));
        assert!(real_llc.contains("real LLVM LNP64 clang sbase command object smokes passed"));
        assert!(real_llc.contains("sbase_libutil_sources=("));
        for source in [
            "concat", "confirm", "cp", "enmasse", "fnck", "getlines", "linecmp", "writeall",
        ] {
            assert!(real_llc.contains(source));
        }
        assert!(real_llc.contains("sbase-libutil-$sbase_libutil-clang-smoke.o"));
        assert!(real_llc.contains("third_party/sbase/libutil/$sbase_libutil.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang sbase libutil object smokes passed"));
        assert!(real_llc.contains("toolchain/liblnp64_sbase_min.c"));
        assert!(libc_sbase_min.contains("void putword(FILE *stream, const char *word)"));
        assert!(libc_sbase_min.contains("void eprintf(const char *fmt, ...)"));
        assert!(libc_sbase_min.contains("void weprintf(const char *fmt, ...)"));
        assert!(libc_sbase_min.contains("char *argv0;"));
        assert!(real_llc.contains("liblnp64-sbase-min.o"));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang sbase support implementation object smoke passed")
        );
        assert!(real_llc.contains("lnp64-sbase-echo-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-echo-clang-smoke.o" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase echo link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-yes-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-yes-clang-smoke.o" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase yes link smoke passed"));
        assert!(real_llc.contains("for sbase_path_cmd in basename dirname"));
        assert!(real_llc.contains("lnp64-sbase-$sbase_path_cmd-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase path command link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-cat-linked.elf"));
        assert!(real_llc.contains("sbase-libutil-concat-clang-smoke.o"));
        assert!(real_llc.contains("sbase-libutil-writeall-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase cat link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-ls-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-ls-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_ls_support_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase ls link smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_sbase_fs_min.c"));
        assert!(libc_sbase_fs_min.contains("mode_t parsemode("));
        assert!(libc_sbase_fs_min.contains("int mkdirp("));
        assert!(real_llc.contains("liblnp64-sbase-fs-min.o"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang sbase filesystem support object smoke passed")
        );
        assert!(real_llc.contains("toolchain/liblnp64_sbase_recurse_min.c"));
        assert!(libc_sbase_recurse_min.contains("void recurse("));
        assert!(libc_sbase_recurse_min.contains("void rm("));
        assert!(real_llc.contains("liblnp64-sbase-recurse-min.o"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang sbase recurse support object smoke passed")
        );
        assert!(real_llc.contains("toolchain/liblnp64_sbase_move_min.c"));
        assert!(libc_sbase_move_min.contains("int cp("));
        assert!(libc_sbase_move_min.contains("void fnck("));
        assert!(libc_sbase_move_min.contains("void enmasse("));
        assert!(real_llc.contains("liblnp64-sbase-move-min.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang sbase move support object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_sbase_time_min.c"));
        assert!(libc_sbase_time_min.contains("long long estrtonum("));
        assert!(libc_sbase_time_min.contains("struct tm *localtime("));
        assert!(libc_sbase_time_min.contains("time_t mktime("));
        assert!(libc_sbase_time_min.contains("char *strptime("));
        assert!(real_llc.contains("liblnp64-sbase-time-min.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang sbase time support object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_sbase_ls_min.c"));
        assert!(libc_sbase_ls_min.contains("struct dirent *readdir("));
        assert!(libc_sbase_ls_min.contains("int printf("));
        assert!(libc_sbase_ls_min.contains("char *estrdup("));
        assert!(libc_sbase_ls_min.contains("int chartorune("));
        assert!(real_llc.contains("liblnp64-sbase-ls-min.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang sbase ls support object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_sbase_find_min.c"));
        assert!(libc_sbase_find_min.contains("int fnmatch("));
        assert!(libc_sbase_find_min.contains("struct dirent *readdir("));
        assert!(libc_sbase_find_min.contains("long sysconf("));
        assert!(libc_sbase_find_min.contains("void *ereallocarray("));
        assert!(real_llc.contains("liblnp64-sbase-find-min.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang sbase find support object smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-find-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-find-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_find_support_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase find link smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_sbase_accounts_min.c"));
        assert!(libc_sbase_accounts_min.contains("struct passwd *getpwnam("));
        assert!(libc_sbase_accounts_min.contains("struct group *getgrnam("));
        assert!(real_llc.contains("liblnp64-sbase-accounts-min.o"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang sbase accounts support object smoke passed")
        );
        assert!(real_llc.contains("toolchain/liblnp64_sbase_wc_min.c"));
        assert!(libc_sbase_wc_min.contains("int printf("));
        assert!(libc_sbase_wc_min.contains("FILE *fopen("));
        assert!(libc_sbase_wc_min.contains("int efgetrune("));
        assert!(libc_sbase_wc_min.contains("int isspacerune("));
        assert!(real_llc.contains("liblnp64-sbase-wc-min.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang sbase wc support object smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-wc-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-wc-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_wc_support_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase wc link smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_sbase_head_min.c"));
        assert!(libc_sbase_head_min.contains("ssize_t getline("));
        assert!(libc_sbase_head_min.contains("FILE *fopen("));
        assert!(libc_sbase_head_min.contains("int fshut("));
        assert!(libc_sbase_head_min.contains("void *erealloc(void *ptr, size_t size)"));
        assert!(libc_sbase_head_min.contains("void *ecalloc(size_t count, size_t size)"));
        assert!(libc_sbase_head_min.contains("void *ereallocarray(void *ptr"));
        assert!(libc_sbase_head_min.contains("int charntorune(Rune *r"));
        assert!(libc_sbase_head_min.contains("int fullrune(const char *s, size_t n)"));
        assert!(libc_sbase_head_min.contains("size_t unescape(char *s)"));
        assert!(libc_sbase_head_min.contains("void *xmemmem("));
        assert!(libc_sbase_head_min.contains("int snprintf(char *str, size_t size"));
        assert!(libc_sbase_head_min.contains("size_t estrlcpy(char *dst"));
        assert!(libc_sbase_head_min.contains("size_t estrlcat(char *dst"));
        assert!(libc_sbase_head_min.contains("int getchar(void)"));
        assert!(libc_sbase_head_min.contains("void xvprintf("));
        assert!(libc_sbase_head_min.contains("struct dirent *readdir(DIR *dirp)"));
        assert!(libc_sbase_head_min.contains("void *emalloc(size_t size)"));
        assert!(libc_sbase_head_min.contains("void *enmalloc(int status, size_t size)"));
        assert!(libc_sbase_head_min.contains("FILE *fmemopen(void *buf, size_t size"));
        assert!(libc_sbase_head_min.contains("int ungetc(int ch, FILE *stream)"));
        assert!(libc_sbase_head_min.contains("int feof(FILE *stream)"));
        assert!(libc_sbase_head_min.contains("void clearerr(FILE *stream)"));
        assert!(libc_sbase_head_min.contains("void efshut(FILE *stream"));
        assert!(libc_sbase_head_min.contains("int puts(const char *s)"));
        assert!(libc_sbase_head_min.contains("int sprintf(char *str"));
        assert!(libc_sbase_head_min.contains("char *strcat(char *dst"));
        assert!(libc_sbase_head_min.contains("char *estrndup(const char *s, size_t n)"));
        assert!(libc_sbase_head_min.contains("int runelen(Rune r)"));
        assert!(libc_sbase_head_min.contains("size_t utfnlen(const char *s, size_t n)"));
        assert!(libc_sbase_head_min.contains("size_t xstrlcat(char *dst"));
        assert!(libc_sbase_head_min.contains("int strcasecmp(const char *lhs"));
        assert!(libc_sbase_head_min.contains("char *xstrcasestr("));
        assert!(libc_sbase_head_min.contains("int enregcomp(int status"));
        assert!(libc_sbase_head_min.contains("unsupported regex"));
        assert!(libc_sbase_head_min.contains("int chartorune(Rune *r"));
        assert!(libc_sbase_head_min.contains("size_t utflen(const char *s)"));
        assert!(libc_sbase_head_min.contains("size_t utftorunestr(const char *s, Rune *r)"));
        assert!(libc_sbase_head_min.contains("int efgetrune(Rune *r, FILE *stream"));
        assert!(libc_sbase_head_min.contains("int efputrune(const Rune *r, FILE *stream"));
        assert!(libc_sbase_head_min.contains("int isalnumrune(Rune r)"));
        assert!(libc_sbase_head_min.contains("Rune toupperrune(Rune r)"));
        assert!(libc_sbase_head_min.contains("void weprintf("));
        assert!(libc_sbase_head_min.contains("void enprintf("));
        assert!(libc_sbase_head_min.contains("int fprintf(FILE *stream"));
        assert!(libc_sbase_head_min.contains("int vfprintf(FILE *stream"));
        assert!(libc_sbase_head_min.contains("*format == 'z' && format[1] == 'u'"));
        assert!(libc_sbase_head_min.contains("*format == 'l' && format[1] == 'd'"));
        assert!(libc_sbase_head_min.contains("*format == 'u'"));
        assert!(libc_sbase_head_min.contains("*format == 'o'"));
        assert!(real_llc.contains("liblnp64-sbase-head-min.o"));
        assert!(real_llc.contains("grep -q '<enprintf>:'"));
        assert!(real_llc.contains("real LLVM LNP64 clang sbase head support object smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-head-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-head-clang-smoke.o" \"#));
        assert!(
            real_llc.contains(r#""$sbase_head_support_impl_obj" "$sbase_time_support_impl_obj" \"#)
        );
        assert!(real_llc.contains("real LLVM LNP64 lld sbase head link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-cmp-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-cmp-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_head_support_impl_obj" "$libc_alloc_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase cmp link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-cksum-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-cksum-clang-smoke.o" \"#));
        assert!(real_llc.contains(
            r#""$sbase_head_support_impl_obj" "$libc_alloc_impl_obj" "$libc_fd_impl_obj" \"#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase cksum link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-uniq-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-uniq-clang-smoke.o" \"#));
        assert!(
            real_llc.contains(r#""$sbase_head_support_impl_obj" "$sbase_time_support_impl_obj" \"#)
        );
        assert!(real_llc.contains("real LLVM LNP64 lld sbase uniq link smoke passed"));
        assert!(libc_sbase_time_min.contains("long long llabs(long long value)"));
        assert!(real_llc.contains("lnp64-sbase-tail-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-tail-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$build_dir/sbase-libutil-concat-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$build_dir/sbase-libutil-writeall-clang-smoke.o" \"#));
        assert!(
            real_llc
                .contains(r#""$libc_alloc_impl_obj" "$libc_fd_impl_obj" "$libc_meta_impl_obj" \"#)
        );
        assert!(real_llc.contains("real LLVM LNP64 lld sbase tail link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-tee-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-tee-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$build_dir/sbase-libutil-writeall-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_head_support_impl_obj" "$libc_alloc_impl_obj""#));
        assert!(real_llc.contains(r#""$libc_signal_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase tee link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-cp-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-cp-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$build_dir/sbase-libutil-cp-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$build_dir/sbase-libutil-enmasse-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$build_dir/sbase-libutil-fnck-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$build_dir/sbase-libutil-confirm-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$build_dir/sbase-libutil-writeall-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$libc_meta_impl_obj" "$libc_path_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase cp link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-cut-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-cut-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$libc_fd_impl_obj" "$libc_convert_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase cut link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-tr-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-tr-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$libc_fd_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase tr link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-sort-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-sort-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$build_dir/sbase-libutil-getlines-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$build_dir/sbase-libutil-linecmp-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_head_support_impl_obj" "$libc_sort_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase sort link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-grep-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-grep-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_head_support_impl_obj" "$libc_alloc_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase grep link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-sed-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-sed-clang-smoke.o" \"#));
        assert!(
            real_llc.contains(
                r#""$libc_fd_impl_obj" "$libc_string_impl_obj" "$libc_convert_impl_obj" \"#
            )
        );
        assert!(real_llc.contains("real LLVM LNP64 lld sbase sed link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-mkdir-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-mkdir-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_fs_support_impl_obj" \"#));
        assert!(real_llc.contains("liblnp64-meta-min.o"));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase mkdir link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-ln-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-ln-clang-smoke.o" \"#));
        assert!(real_llc.contains("liblnp64-path-min.o"));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase ln link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-chmod-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-chmod-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_fs_support_impl_obj" \"#));
        assert!(real_llc.contains(r#""$libc_fd_impl_obj" "$libc_string_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase chmod link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-chown-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-chown-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_accounts_support_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase chown link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-touch-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-touch-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_time_support_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase touch link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-mv-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-mv-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_move_support_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase mv link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-rm-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-rm-clang-smoke.o" \"#));
        assert!(real_llc.contains(r#""$sbase_recurse_support_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase rm link smoke passed"));
        assert!(real_llc.contains("netcat-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/netcat.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang netcat demo object smoke passed"));
        assert!(real_llc.contains("lnp64-netcat-clang-linked.elf"));
        assert!(real_llc.contains(r#""$netcat_obj" "$libc_fd_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld netcat demo link smoke passed"));
        assert!(real_llc.contains("httpd-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/httpd.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang httpd demo object smoke passed"));
        assert!(real_llc.contains("lnp64-httpd-clang-linked.elf"));
        assert!(real_llc.contains(r#""$httpd_obj" "$libc_fd_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld httpd demo link smoke passed"));
        assert!(real_llc_docker.contains("netcat --self-test --expect 'netcat self-test ok'"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf netcat self-test passed"));
        assert!(real_llc_docker.contains("httpd-fixture-root"));
        assert!(real_llc_docker.contains("httpd --self-test"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf httpd self-test passed"));
        assert!(real_llc.contains("lnp64-$demo-clang-linked.elf"));
        assert!(real_llc.contains(
            r#""$demo_obj" "$libc_fd_impl_obj" \
    "$libc_alloc_impl_obj" "$libc_string_impl_obj" "$libc_process_impl_obj" \
    "$libc_futex_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld clang demo link smoke passed"));
        assert!(real_llc.contains("rewrite_with_perl"));
        assert!(real_llc_docker.contains("Dockerfile.llvm"));
        assert!(real_llc_docker.contains("scripts/run_real_llvm_lnp64.sh"));
        assert!(real_llc_docker.contains(r#"--user "$uid:$gid""#));
        assert!(real_mc_docker.contains("Dockerfile.llvm"));
        assert!(real_mc_docker.contains("LNP64_LLVM_GATE=mc"));
        assert!(real_mc_docker.contains("scripts/run_real_llvm_lnp64.sh"));
        assert!(real_mc_docker.contains(r#"--user "$uid:$gid""#));
        assert!(llvm_dockerfile.contains("llvm-dev"));
        assert!(llvm_dockerfile.contains("llvm-runtime"));
        assert!(llvm_dockerfile.contains("clang"));
        assert!(llvm_dockerfile.contains("lld"));
        assert!(
            bootstrap_smokes.contains(r#"-T "$linker_script""#)
                && bootstrap_smokes.contains("lnp64_static.ld")
                && bootstrap_smokes.contains("scripts/package_lnp64_sysroot.sh"),
            "static link gate must use checked LNP64 linker script"
        );
        assert!(
            bootstrap_smokes.contains("run-elf"),
            "execution gate must route through the checked run-elf boundary"
        );
        assert!(
            bootstrap_smokes.contains("crt0.o") && bootstrap_smokes.contains("run_crt0_smoke"),
            "crt0 gate must assemble checked startup stub"
        );
        for gate in [
            "compile_hello",
            "compile_arithmetic",
            "compile_memory",
            "compile_calls",
        ] {
            assert!(
                bootstrap_smokes.contains("-I toolchain"),
                "{gate} must include checked private intrinsic header path"
            );
        }
    }

    #[test]
    fn static_linker_script_records_loader_mapping_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let linker_script = include_str!("../toolchain/lnp64_static.ld");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let object_format = include_str!("../object_format.md");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let linker_path = manifest_field(target_manifest, "linker_script_contract");

        assert_eq!(linker_path, "toolchain/lnp64_static.ld");
        assert!(manifest_root.join(linker_path).is_file());
        assert!(contract_index.contains(
            "linker_script|toolchain/lnp64_static.ld|static_linker_script_records_loader_mapping_contract"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_static.ld"));
        assert!(roadmap.contains("toolchain/lnp64_static.ld"));

        for required in [
            "OUTPUT_ARCH(lnp64)",
            "ENTRY(_start)",
            "PHDRS",
            "text PT_LOAD FLAGS(5);",
            "rodata PT_LOAD FLAGS(4);",
            "data PT_LOAD FLAGS(6);",
            "tls PT_TLS FLAGS(4);",
            "note PT_NOTE FLAGS(4);",
            ". = 0x400000;",
            "__lnp64_image_base = .;",
            "__lnp64_image_end = .;",
            "__lnp64_tls_start = .;",
            "__lnp64_tls_end = .;",
            ".text",
            ".rodata",
            ".data",
            ".bss",
            ".tdata",
            ".tbss",
            ".note.lnp64.startup",
            ".note.lnp64.capreq",
        ] {
            assert!(
                linker_script.contains(required),
                "linker script missing {required}"
            );
        }
        for section in [
            ".text",
            ".rodata",
            ".data",
            ".bss",
            ".tdata",
            ".tbss",
            ".note.lnp64.startup",
            ".note.lnp64.capreq",
        ] {
            assert!(
                object_format.contains(section),
                "object format missing linked section {section}"
            );
        }
        for permission_rule in [
            "| `PF_R` | read |",
            "| `PF_W` | read/write, non-executable |",
            "| `PF_X` | read/execute, non-writable |",
            "| `PF_W | PF_X` | rejected",
        ] {
            assert!(
                object_format.contains(permission_rule),
                "object format missing executable mapping rule {permission_rule}"
            );
        }
        assert!(linker_script.contains("*(.note.GNU-stack)"));
        assert!(
            !linker_script.contains("FLAGS(7)"),
            "static v0 linker script must not emit writable executable PHDRs"
        );
        assert!(
            !linker_script.contains("PT_DYNAMIC"),
            "static v0 linker script must not emit PT_DYNAMIC"
        );
    }

    #[test]
    fn clang_driver_manifest_matches_llvm_gates() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let driver_manifest = include_str!("../toolchain/lnp64_clang_driver.manifest");
        let gate_manifest = include_str!("../toolchain/lnp64_llvm_gates.manifest");
        let real_llvm_runner = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let driver_path = manifest_field(target_manifest, "clang_driver_contract");

        assert_eq!(driver_path, "toolchain/lnp64_clang_driver.manifest");
        assert!(manifest_root.join(driver_path).is_file());
        assert!(contract_index.contains(
            "clang_driver|toolchain/lnp64_clang_driver.manifest|clang_driver_manifest_matches_llvm_gates"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_clang_driver.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_clang_driver.manifest"));
        assert_eq!(
            manifest_field(driver_manifest, "triple"),
            manifest_field(target_manifest, "triple")
        );
        for flag in [
            "-ffreestanding",
            "-fno-pic",
            "-fno-jump-tables",
            "-isystem",
            "target/lnp64-sysroot/usr/include",
            "-Itoolchain",
        ] {
            assert!(
                manifest_csv_contains(driver_manifest, "cflags", flag),
                "driver cflags missing {flag}"
            );
        }
        assert_eq!(manifest_field(driver_manifest, "assembler"), "llvm-mc");
        assert!(manifest_csv_contains(
            driver_manifest,
            "assembler_flags",
            "-triple=lnp64-unknown-none"
        ));
        assert!(manifest_csv_contains(
            driver_manifest,
            "assembler_flags",
            "-filetype=obj"
        ));
        assert_eq!(manifest_field(driver_manifest, "linker"), "ld.lld");
        for flag in [
            "-static",
            "-m",
            "elf64lnp64",
            "-T",
            "target/lnp64-sysroot/usr/lib/lnp64/lnp64_static.ld",
        ] {
            assert!(
                manifest_csv_contains(driver_manifest, "linker_flags", flag),
                "driver linker flags missing {flag}"
            );
        }
        assert_eq!(
            manifest_field(driver_manifest, "crt0"),
            "target/lnp64-sysroot/usr/lib/lnp64/crt0.o"
        );
        assert_eq!(
            manifest_field(driver_manifest, "intrinsic_header"),
            "target/lnp64-sysroot/usr/include/lnp64/intrinsics.h"
        );
        assert_eq!(
            manifest_field(driver_manifest, "loader_probe"),
            "lnp64 elf-plan"
        );
        assert_eq!(
            manifest_field(driver_manifest, "status"),
            "active_real_backend"
        );

        assert!(
            gate_manifest.contains("real_llc_build|bash scripts/run_real_llvm_lnp64_docker.sh")
        );
        assert!(real_llvm_runner.contains("\"$clang\" --target=lnp64-unknown-none"));
        assert!(real_llvm_runner.contains("-ffreestanding -fno-pic -fno-jump-tables"));
        assert!(real_llvm_runner.contains("\"$llvm_mc\" -triple=lnp64-unknown-none"));
        assert!(real_llvm_runner.contains("toolchain/crt0_lnp64.s"));
        assert!(real_llvm_runner.contains("\"$lld\" -flavor gnu -static -m elf64lnp64"));
        assert!(real_llvm_runner.contains("lnp64_static.ld"));
        assert!(gate_manifest.contains("inspect_exec_plan|LNP64_BOOTSTRAP_CASES=hello"));
    }

    #[test]
    fn run_elf_manifest_records_execution_boundary() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let run_elf_manifest = include_str!("../toolchain/lnp64_run_elf.manifest");
        let gate_manifest = include_str!("../toolchain/lnp64_llvm_gates.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let loader_security = include_str!("../toolchain/lnp64_loader_security.manifest");
        let main_source = include_str!("main.rs");
        let loader_source = include_str!("loader.rs");
        let emulator_source = include_str!("emulator.rs");
        let lowering_source = include_str!("lowering.rs");
        let personality_lowering_source = include_str!("personality_lowering.rs");
        let real_llc_docker = include_str!("../scripts/run_real_llvm_lnp64_docker.sh");
        let real_llc_runner = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let bootstrap_smokes = include_str!("../scripts/run_real_llvm_bootstrap_smokes.sh");
        let fd_shim_source = include_str!("../toolchain/liblnp64_fd_min.c");
        let process_shim_source = include_str!("../toolchain/liblnp64_process_min.c");
        let evidence_corpus = format!(
            "{main_source}\n{loader_source}\n{emulator_source}\n{lowering_source}\n{personality_lowering_source}\n{real_llc_docker}\n{real_llc_runner}\n{bootstrap_smokes}\n{fd_shim_source}\n{process_shim_source}"
        );
        let rows = run_elf_rows(run_elf_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut stages = std::collections::BTreeMap::new();

        assert_eq!(
            manifest_field(target_manifest, "run_elf_contract"),
            "toolchain/lnp64_run_elf.manifest"
        );
        assert!(contract_index.contains(
            "run_elf|toolchain/lnp64_run_elf.manifest|run_elf_manifest_records_execution_boundary"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_run_elf.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_run_elf.manifest"));
        assert!(conformance.contains("toolchain/lnp64_run_elf.manifest"));
        assert!(gate_manifest.contains("scripts/run_real_llvm_bootstrap_smokes.sh"));
        assert!(bootstrap_smokes.contains("elf-plan"));
        assert!(bootstrap_smokes.contains("run-elf"));
        assert!(real_llc_docker.contains("cargo build --quiet --bin lnp64"));
        assert!(real_llc_docker.contains(r#""$lnp64_bin" elf-plan"#));
        assert!(real_llc_docker.contains(r#""$lnp64_bin" run-elf"#));
        assert!(real_llc_docker.contains("LNP64_LLVM_DOCKER_SKIP_RUN_ELF"));
        assert!(!real_llc_docker.contains("cargo run --quiet -- elf-plan"));
        assert!(!real_llc_docker.contains("cargo run --quiet -- run-elf"));
        assert!(real_llc_docker.contains("lnp64-$demo-clang-linked.elf"));
        assert!(real_llc_docker.contains("hello from LNP64"));
        assert!(real_llc_docker.contains("factorial ok"));
        assert!(real_llc_docker.contains("alloc ok"));
        assert!(real_llc_docker.contains("fibonacci ok"));
        assert!(real_llc_docker.contains("pcr ok"));
        assert!(real_llc_docker.contains("cat ok"));
        assert!(real_llc_docker.contains("json parser ok"));
        assert!(real_llc_docker.contains("rot13 ok"));
        assert!(real_llc_docker.contains("producer consumer ok"));
        assert!(real_llc_docker.contains("parallel hash ok"));
        assert!(real_llc_docker.contains("sqlite lite ok"));
        assert!(real_llc_docker.contains("ping pong ok"));
        assert!(real_llc_docker.contains("exit=0"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf clang demo execution passed"));
        assert!(real_llc_docker.contains("lnp64-native-heap-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf native heap execution passed"));
        assert!(real_llc_docker.contains("lnp64-indirect-call-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf indirect call execution passed"));
        assert!(real_llc_docker.contains("lnp64-scalar-arith-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf scalar arithmetic execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-high-mul-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf high-multiply execution passed"));
        assert!(real_llc_docker.contains("lnp64-scalar-extend-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf scalar extension execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-bitmanip-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf bit-manip execution passed"));
        assert!(real_llc_docker.contains("lnp64-csel-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf csel execution passed"));
        assert!(real_llc_docker.contains("lnp64-call-clobber-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf call-clobber execution passed"));
        assert!(real_llc_docker.contains("lnp64-stack-args-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf stack-argument execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-string-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf minilibc string execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-convert-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf numeric conversion execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-path-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf path helper execution passed"));
        assert!(real_llc_docker.contains("lnp64-search-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf search helper execution passed"));
        assert!(real_llc_docker.contains("lnp64-sort-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sort helper execution passed"));
        assert!(real_llc_docker.contains("lnp64-zlib-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf zlib package execution passed"));
        assert!(real_llc_docker.contains("lnp64-natsort-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf natsort package execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-jsmn-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf jsmn package execution passed"));
        assert!(real_llc_docker.contains("lnp64-inih-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf inih package execution passed"));
        assert!(real_llc_docker.contains("lnp64-cwalk-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf cwalk package execution passed"));
        assert!(real_llc_docker.contains("lnp64-libc-test-ctype-bounded-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test ctype_bounded execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test string execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-memcpy-bounded-linked.elf"));
        assert!(
            real_llc_docker.contains(
                "real LLVM LNP64 run-elf libc-test string_memcpy_bounded execution passed"
            )
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-memmove-bounded-linked.elf"));
        assert!(
            real_llc_docker.contains(
                "real LLVM LNP64 run-elf libc-test string_memmove_bounded execution passed"
            )
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-memmem-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test string_memmem execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-strchr-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test string_strchr execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-strcspn-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test string_strcspn execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-strstr-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test string_strstr execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-udiv-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test udiv execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-basename-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test basename execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-dirname-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test dirname execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-strtol-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test strtol execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-clock-gettime-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test clock_gettime execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-access-bounded-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test access_bounded execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-stat-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test stat execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-utime-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test utime execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-ungetc-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test ungetc execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-fdopen-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test fdopen execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-fcntl-basic-bounded-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test fcntl_basic_bounded execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-pthread-tsd-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test pthread_tsd execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-sem-init-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test sem_init execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-qsort-bounded-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test qsort_bounded execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-search-insque-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test search_insque execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-search-lsearch-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test search_lsearch execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-malloc-0-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test malloc-0 execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-fgets-eof-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test fgets-eof execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-calloc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf calloc execution passed"));
        assert!(real_llc_docker.contains("lnp64-realloc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf realloc execution passed"));
        assert!(real_llc_docker.contains("lnp64-read-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf read execution passed"));
        assert!(real_llc_docker.contains("lnp64-write-linked.elf"));
        assert!(real_llc_docker.contains("fd write ok"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf write execution passed"));
        assert!(real_llc_docker.contains("lnp64-meta-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf metadata libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-mmap-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf mmap libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-futex-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf futex libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-poll-libc-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf poll/select/epoll/kqueue libc execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-signal-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf signal libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-socket-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf socket libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-netbsd-personality-clang-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf NetBSD personality clang smoke passed")
        );
        assert!(real_llc_docker.contains("netbsd clang personality smoke ok"));
        assert!(real_llc_docker.contains("netbsd-elf-exec-fixture-root"));
        assert!(real_llc_docker.contains("lnp64-netbsd-elf-exec-test-linked.elf"));
        assert!(real_llc_docker.contains("loader_target ok"));
        assert!(real_llc_docker.contains("elf_exec_test ok"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf NetBSD ELF exec parent passed"));
        assert!(real_llc_docker.contains("netbsd-namespace-fixture-root"));
        assert!(real_llc_docker.contains("lnp64-netbsd-namespace-test-linked.elf"));
        assert!(real_llc_docker.contains("namespace_test ok"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf NetBSD namespace child passed"));
        assert!(real_llc_docker.contains("lnp64-netbsd-fork-wait-test-linked.elf"));
        assert!(real_llc_docker.contains("fork_wait_test ok"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf NetBSD fork/wait child passed"));
        assert!(real_llc_docker.contains("LNP64_LLVM_PACKAGE_FILTER=netbsd"));
        assert!(real_llc_docker.contains(
            "LNP64_LLVM_PACKAGE_FILTER=netbsd bash scripts/run_real_llvm_package_gate.sh"
        ));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf NetBSD package/system gate passed")
        );
        assert!(real_llc_docker.contains("lnp64-sbase-echo-linked.elf"));
        assert!(real_llc_docker.contains("echo hello clang --expect 'hello clang'"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase echo execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-yes-linked.elf"));
        assert!(
            real_llc_docker.contains("elf-plan target/llvm-lnp64-build/lnp64-sbase-yes-linked.elf")
        );
        assert!(
            real_llc_docker.contains("real LLVM LNP64 elf-plan sbase yes static boundary passed")
        );
        assert!(real_llc_docker.contains("lnp64-sbase-basename-linked.elf"));
        assert!(real_llc_docker.contains("basename /usr/local/bin/clang --expect '^clang$'"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf sbase basename execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-sbase-dirname-linked.elf"));
        assert!(
            real_llc_docker.contains("dirname /usr/local/bin/clang --expect '^/usr/local/bin$'")
        );
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase dirname execution passed"));
        assert!(real_llc_docker.contains("run-elf --namespace-root \"$sbase_fixture_root\""));
        assert!(real_llc_docker.contains("lnp64-sbase-cat-linked.elf"));
        assert!(real_llc_docker.contains("cat input/cat.txt"));
        assert!(real_llc_docker.contains("cat via clang"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase cat execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-wc-linked.elf"));
        assert!(real_llc_docker.contains("wc input/wc.txt"));
        assert!(real_llc_docker.contains("^2 3 14 input/wc.txt$"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase wc execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-head-linked.elf"));
        assert!(real_llc_docker.contains("head -n 2 input/head.txt"));
        assert!(real_llc_docker.contains("sbase head printed too many lines"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase head execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-cmp-linked.elf"));
        assert!(real_llc_docker.contains("cmp input/cmp-a.txt input/cmp-b.txt"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase cmp execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-cksum-linked.elf"));
        assert!(real_llc_docker.contains("cksum input/cksum.txt"));
        assert!(real_llc_docker.contains("^622224091 16 input/cksum.txt$"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase cksum execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-uniq-linked.elf"));
        assert!(real_llc_docker.contains("uniq input/uniq.txt"));
        assert!(real_llc_docker.contains("grep -c '^alpha$'"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase uniq execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-tail-linked.elf"));
        assert!(real_llc_docker.contains("tail -n 2 input/tail.txt"));
        assert!(real_llc_docker.contains("sbase tail printed too many lines"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase tail execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-tee-linked.elf"));
        assert!(real_llc_docker.contains("tee tee-copy.txt"));
        assert!(real_llc_docker.contains("tee-stdout.txt"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase tee execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-cp-linked.elf"));
        assert!(real_llc_docker.contains("cp input/cp-source.txt cp-copy.txt"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase cp execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-cut-linked.elf"));
        assert!(real_llc_docker.contains("cut -f 2 input/cut.txt"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase cut execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-tr-linked.elf"));
        assert!(real_llc_docker.contains("tr 'a-z' 'A-Z'"));
        assert!(real_llc_docker.contains("^MIXED CASE 123$"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase tr execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-sort-linked.elf"));
        assert!(real_llc_docker.contains("sort input/sort.txt"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase sort execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-grep-linked.elf"));
        assert!(real_llc_docker.contains("grep -F alpha input/grep.txt"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf sbase grep fixed-string execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-sbase-sed-linked.elf"));
        assert!(real_llc_docker.contains("sed -n p input/sed.txt"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf sbase sed no-regex execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-sbase-ls-linked.elf"));
        assert!(real_llc_docker.contains("ls input"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase ls execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-mkdir-linked.elf"));
        assert!(real_llc_docker.contains("mkdir made"));
        assert!(real_llc_docker.contains("test -d \"$sbase_fixture_root/made\""));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase mkdir execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-ln-linked.elf"));
        assert!(real_llc_docker.contains("ln input/cat.txt linked.txt"));
        assert!(real_llc_docker.contains("cmp -s \"$sbase_fixture_root/input/cat.txt\""));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase ln execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-chmod-linked.elf"));
        assert!(real_llc_docker.contains("chmod 700 chmod.txt"));
        assert!(real_llc_docker.contains("stat -c '%a'"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase chmod execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-chown-linked.elf"));
        assert!(real_llc_docker.contains("chown :\"$(id -g)\" chown.txt"));
        assert!(real_llc_docker.contains("stat -c '%g'"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase chown execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-touch-linked.elf"));
        assert!(real_llc_docker.contains("touch touched.txt"));
        assert!(real_llc_docker.contains("test -f \"$sbase_fixture_root/touched.txt\""));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase touch execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-mv-linked.elf"));
        assert!(real_llc_docker.contains("mv move-source.txt moved.txt"));
        assert!(real_llc_docker.contains("test ! -e \"$sbase_fixture_root/move-source.txt\""));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase mv execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-rm-linked.elf"));
        assert!(real_llc_docker.contains("rm remove.txt"));
        assert!(real_llc_docker.contains("test ! -e \"$sbase_fixture_root/remove.txt\""));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase rm execution passed"));
        assert!(real_llc_docker.contains("lnp64-userland-ucat-linked.elf"));
        assert!(real_llc_docker.contains("userland-fixture-root"));
        assert!(real_llc_docker.contains("ucat etc/motd"));
        assert!(real_llc_docker.contains("welcome from clang ucat"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf userland ucat execution passed"));
        assert!(real_llc_docker.contains("lnp64-userland-init-linked.elf"));
        assert!(real_llc_docker.contains("init /"));
        assert!(real_llc_docker.contains("lnp64 clang init: boot"));
        assert!(real_llc_docker.contains("lnp64 clang init: root /"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf userland init execution passed"));
        assert!(real_llc_docker.contains("lnp64-userland-lnpsh-linked.elf"));
        assert!(real_llc_docker.contains("lnpsh clang: scripted console"));
        assert!(real_llc_docker.contains("console"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf userland lnpsh execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-userland-spawn-task-linked.elf"));
        assert!(real_llc_docker.contains("userland spawn: parent"));
        assert!(real_llc_docker.contains("userland spawn: child"));
        assert!(real_llc_docker.contains("userland spawn: joined"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf userland spawn task execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-errno-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf errno execution passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-push-linked.elf"));
        assert!(real_llc_docker.contains("intrinsic push ok"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic push execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-await-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic await execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-call-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic call execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-gate-return-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf intrinsic gate return execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-control-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic control execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-cap-control-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf intrinsic capability control execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-mmap-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic mmap execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-clone-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic CLONE execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-amo-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic AMO execution passed"));
        assert!(real_llc_docker.contains("lnp64-c11-atomic-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf C11 atomic execution passed"));
        assert!(real_llc_docker.contains("lnp64-exit-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf exit execution passed"));
        assert!(real_llc_docker.contains("lnp64-setjmp-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf setjmp/longjmp execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-startup-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf startup argv/envp execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-getauxval-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf getauxval execution passed"));
        assert!(real_llc_docker.contains("lnp64-libc-test-argv-linked.elf"));
        assert!(real_llc_docker.contains("lnp64-argv --expect"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test argv execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-env-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf libc-test env execution passed"));
        assert!(real_llc_docker.contains("lnp64-libc-test-random-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test random execution passed")
        );
        assert!(main_source.contains("\"run-elf\""));
        assert!(main_source.contains("take_run_namespace_root(&mut args)?"));
        assert!(main_source.contains("probe.machine.set_namespace_root(root)?"));
        assert!(main_source.contains("run_committed_exec"));
        assert!(loader_security.contains("submit_exec_plan"));
        assert!(
            loader_security.contains("emulator_commits_exec_descriptor_memory_image_atomically")
        );
        assert!(
            emulator_source.contains("exec_descriptor_startup_metadata_base_is_runtime_visible")
        );
        assert!(emulator_source.contains("fn startup_metadata_base(&self)"));
        assert!(
            emulator_source
                .contains("ENV_KEY_STARTUP_METADATA_PTR => Some(self.startup_metadata_base()?")
        );
        assert!(emulator_source.contains("fn exec_static_elf_image("));
        assert!(emulator_source.contains("committed_exec_opcode_loads_static_elf_child"));
        assert!(emulator_source.contains("crate::loader::load_static_elf"));
        assert!(emulator_source.contains("0x7f => Instr::Exec(a, b, c)"));

        for (stage, status, artifacts, evidence, blocker) in rows {
            assert!(
                stages
                    .insert(stage, (status, artifacts.clone(), evidence, blocker))
                    .is_none(),
                "duplicate run-elf stage {stage}"
            );
            assert!(
                ["tested", "partial", "planned"].contains(&status),
                "unknown run-elf status {status} for {stage}"
            );
            assert!(
                !artifacts.is_empty(),
                "empty artifacts for run-elf stage {stage}"
            );
            for artifact in artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "run-elf stage {stage} names missing artifact {artifact}"
                );
            }
            assert!(!evidence.is_empty(), "empty run-elf evidence for {stage}");
            if status == "tested" {
                assert_eq!(blocker, "none", "tested run-elf stage {stage} has blocker");
                assert!(
                    evidence_corpus.contains(evidence),
                    "tested run-elf evidence {evidence} for {stage} is not present"
                );
            } else {
                assert_ne!(
                    blocker, "none",
                    "unfinished run-elf stage {stage} lacks blocker"
                );
            }
        }

        for stage in [
            "load_static_elf",
            "materialize_vmas",
            "descriptor_validate",
            "descriptor_commit",
            "cli_probe",
            "cli_surface",
            "real_clang_lld_probe",
            "real_clang_demo_execution",
            "real_clang_bootstrap_smoke",
            "real_native_heap_execution",
            "real_libc_test_ctype_execution",
            "real_libc_test_string_execution",
            "real_libc_test_string_memcpy_bounded_execution",
            "real_libc_test_string_memmove_bounded_execution",
            "real_libc_test_string_memmem_execution",
            "real_libc_test_string_strchr_execution",
            "real_libc_test_string_strcspn_execution",
            "real_libc_test_string_strstr_execution",
            "real_libc_test_udiv_execution",
            "real_libc_test_basename_execution",
            "real_libc_test_dirname_execution",
            "real_libc_test_strtol_execution",
            "real_libc_test_clock_gettime_execution",
            "real_libc_test_access_bounded_execution",
            "real_libc_test_stat_execution",
            "real_libc_test_utime_execution",
            "real_libc_test_ungetc_execution",
            "real_libc_test_fdopen_execution",
            "real_libc_test_fcntl_basic_bounded_execution",
            "real_libc_test_fcntl_execution",
            "real_libc_test_pthread_tsd_execution",
            "real_libc_test_sem_init_execution",
            "real_libc_test_qsort_bounded_execution",
            "real_libc_test_search_insque_execution",
            "real_libc_test_search_lsearch_execution",
            "real_libc_test_malloc_0_execution",
            "real_libc_test_fgets_eof_execution",
            "real_numeric_conversion_execution",
            "real_path_helper_execution",
            "real_search_helper_execution",
            "real_sort_helper_execution",
            "real_read_execution",
            "real_write_execution",
            "real_userland_ucat_execution",
            "real_userland_init_execution",
            "real_userland_lnpsh_execution",
            "real_userland_spawn_task_execution",
            "real_netbsd_loader_target_child_execution",
            "real_netbsd_elf_exec_parent_execution",
            "real_netbsd_fork_wait_child_execution",
            "real_netbsd_thread_child_execution",
            "real_netbsd_poll_child_execution",
            "real_netbsd_signal_gate_child_execution",
            "real_netbsd_signal_fault_child_execution",
            "real_netbsd_timer_child_execution",
            "real_netbsd_mmap_child_execution",
            "real_netbsd_fd_passing_child_execution",
            "real_netbsd_namespace_child_execution",
            "real_netbsd_fs_service_child_execution",
            "real_netbsd_classifier_child_execution",
            "real_netbsd_socket_loopback_child_execution",
            "real_netbsd_gate_trace_child_execution",
            "real_netbsd_domain_nested_child_execution",
            "real_netbsd_domain_budget_child_execution",
            "real_netbsd_init_shell_system_execution",
            "real_metadata_libc_execution",
            "real_mmap_libc_execution",
            "real_futex_libc_execution",
            "real_poll_select_epoll_kqueue_libc_execution",
            "real_signal_libc_execution",
            "real_socket_libc_execution",
            "real_netcat_self_test_execution",
            "real_httpd_self_test_execution",
            "real_netbsd_personality_clang_execution",
            "real_sbase_echo_execution",
            "real_sbase_yes_exec_plan",
            "real_sbase_basename_execution",
            "real_sbase_dirname_execution",
            "real_sbase_cat_execution",
            "real_sbase_wc_execution",
            "real_sbase_head_execution",
            "real_sbase_cmp_execution",
            "real_sbase_cksum_execution",
            "real_sbase_uniq_execution",
            "real_sbase_tail_execution",
            "real_sbase_tee_execution",
            "real_sbase_cp_execution",
            "real_sbase_cut_execution",
            "real_sbase_tr_execution",
            "real_sbase_sort_execution",
            "real_sbase_grep_fixed_string_execution",
            "real_sbase_sed_no_regex_execution",
            "real_sbase_ls_execution",
            "real_sbase_find_execution",
            "real_sbase_mkdir_execution",
            "real_sbase_ln_execution",
            "real_sbase_chmod_execution",
            "real_sbase_chown_execution",
            "real_sbase_touch_execution",
            "real_sbase_mv_execution",
            "real_sbase_rm_execution",
            "real_errno_execution",
            "real_startup_execution",
            "real_getauxval_execution",
            "real_libc_test_argv_execution",
            "real_libc_test_env_execution",
            "real_libc_test_random_execution",
            "real_intrinsic_await_execution",
            "real_intrinsic_call_execution",
            "real_intrinsic_gate_return_execution",
            "real_intrinsic_push_execution",
            "real_intrinsic_control_execution",
            "real_intrinsic_capability_control_execution",
            "real_intrinsic_mmap_execution",
            "real_intrinsic_get_pcr_execution",
            "real_intrinsic_set_pcr_execution",
            "real_intrinsic_openat_execution",
            "real_intrinsic_clone_execution",
            "real_intrinsic_amo_execution",
            "real_c11_atomic_execution",
            "real_stack_argument_execution",
            "real_exit_execution",
            "entry_state",
            "exec_opcode_static_elf",
            "text_fetch_decode",
            "stdout_exit",
            "real_clang_loader_path",
        ] {
            assert!(stages.contains_key(stage), "missing run-elf stage {stage}");
        }
        for stage in [
            "load_static_elf",
            "materialize_vmas",
            "descriptor_validate",
            "descriptor_commit",
            "cli_probe",
            "cli_surface",
            "real_clang_lld_probe",
            "real_clang_demo_execution",
            "real_clang_bootstrap_smoke",
            "real_native_heap_execution",
            "real_libc_test_ctype_execution",
            "real_libc_test_string_execution",
            "real_libc_test_string_memcpy_bounded_execution",
            "real_libc_test_string_memmove_bounded_execution",
            "real_libc_test_string_memmem_execution",
            "real_libc_test_string_strchr_execution",
            "real_libc_test_string_strcspn_execution",
            "real_libc_test_string_strstr_execution",
            "real_libc_test_udiv_execution",
            "real_libc_test_basename_execution",
            "real_libc_test_dirname_execution",
            "real_libc_test_strtol_execution",
            "real_libc_test_clock_gettime_execution",
            "real_libc_test_access_bounded_execution",
            "real_libc_test_stat_execution",
            "real_libc_test_utime_execution",
            "real_libc_test_ungetc_execution",
            "real_libc_test_fdopen_execution",
            "real_libc_test_fcntl_basic_bounded_execution",
            "real_libc_test_fcntl_execution",
            "real_libc_test_pthread_tsd_execution",
            "real_libc_test_sem_init_execution",
            "real_libc_test_qsort_bounded_execution",
            "real_libc_test_search_insque_execution",
            "real_libc_test_search_lsearch_execution",
            "real_libc_test_malloc_0_execution",
            "real_libc_test_fgets_eof_execution",
            "real_numeric_conversion_execution",
            "real_path_helper_execution",
            "real_search_helper_execution",
            "real_sort_helper_execution",
            "real_write_execution",
            "real_userland_ucat_execution",
            "real_userland_init_execution",
            "real_userland_lnpsh_execution",
            "real_userland_spawn_task_execution",
            "real_netbsd_loader_target_child_execution",
            "real_netbsd_elf_exec_parent_execution",
            "real_netbsd_fork_wait_child_execution",
            "real_netbsd_thread_child_execution",
            "real_netbsd_poll_child_execution",
            "real_netbsd_signal_gate_child_execution",
            "real_netbsd_signal_fault_child_execution",
            "real_netbsd_timer_child_execution",
            "real_netbsd_mmap_child_execution",
            "real_netbsd_fd_passing_child_execution",
            "real_netbsd_namespace_child_execution",
            "real_netbsd_fs_service_child_execution",
            "real_netbsd_classifier_child_execution",
            "real_netbsd_socket_loopback_child_execution",
            "real_netbsd_gate_trace_child_execution",
            "real_netbsd_domain_nested_child_execution",
            "real_netbsd_domain_budget_child_execution",
            "real_netbsd_init_shell_system_execution",
            "real_metadata_libc_execution",
            "real_mmap_libc_execution",
            "real_futex_libc_execution",
            "real_poll_select_epoll_kqueue_libc_execution",
            "real_signal_libc_execution",
            "real_socket_libc_execution",
            "real_netcat_self_test_execution",
            "real_httpd_self_test_execution",
            "real_netbsd_personality_clang_execution",
            "real_sbase_echo_execution",
            "real_sbase_yes_exec_plan",
            "real_sbase_basename_execution",
            "real_sbase_dirname_execution",
            "real_sbase_cat_execution",
            "real_sbase_wc_execution",
            "real_sbase_head_execution",
            "real_sbase_cmp_execution",
            "real_sbase_cksum_execution",
            "real_sbase_uniq_execution",
            "real_sbase_tail_execution",
            "real_sbase_tee_execution",
            "real_sbase_cp_execution",
            "real_sbase_cut_execution",
            "real_sbase_tr_execution",
            "real_sbase_sort_execution",
            "real_sbase_grep_fixed_string_execution",
            "real_sbase_sed_no_regex_execution",
            "real_sbase_ls_execution",
            "real_sbase_find_execution",
            "real_sbase_mkdir_execution",
            "real_sbase_ln_execution",
            "real_sbase_chmod_execution",
            "real_sbase_chown_execution",
            "real_sbase_touch_execution",
            "real_sbase_mv_execution",
            "real_sbase_rm_execution",
            "real_intrinsic_push_execution",
            "real_intrinsic_control_execution",
            "real_libc_test_argv_execution",
            "real_intrinsic_mmap_execution",
            "real_intrinsic_amo_execution",
            "real_c11_atomic_execution",
            "real_exit_execution",
            "real_errno_execution",
            "real_startup_execution",
            "real_getauxval_execution",
            "real_libc_test_env_execution",
            "real_libc_test_random_execution",
            "entry_state",
            "exec_opcode_static_elf",
            "text_fetch_decode",
            "real_clang_loader_path",
        ] {
            assert_eq!(stages[stage].0, "tested", "{stage} should be tested");
        }
        assert_eq!(stages["stdout_exit"].0, "partial");
        assert_eq!(
            stages["stdout_exit"].2,
            "real_clang_stdout_exit_run_elf_smokes"
        );
        assert_eq!(stages["stdout_exit"].3, "needs_full_libc_runtime_packaging");
        for artifact in [
            "scripts/run_real_llvm_lnp64_docker.sh",
            "scripts/run_real_llvm_lnp64.sh",
            "toolchain/liblnp64_fd_min.c",
            "toolchain/liblnp64_process_min.c",
            "toolchain/lnp64_llvm_gates.manifest",
        ] {
            assert!(
                stages["stdout_exit"].1.contains(&artifact),
                "stdout/exit partial row must name artifact {artifact}"
            );
        }
        assert!(real_llc_runner.contains("const char msg[] = \"fd write ok\\n\";"));
        assert!(real_llc_docker.contains("lnp64-write-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf write execution passed"));
        assert!(fd_shim_source.contains("ssize_t write(int fd, const void *buf, size_t len)"));
        assert!(fd_shim_source.contains("__lnp_push"));
        assert!(process_shim_source.contains("void _exit(int status)"));
        assert!(process_shim_source.contains("__lnp_exit"));
        assert!(conformance.contains(
            "stdout/exit compatibility row remains partial until the production libc/runtime path replaces the smoke-only shim"
        ));
        assert!(conformance.contains(
            "Replace the smoke-only libc shim with Clang-built crt/libc runtime support"
        ));
        assert!(roadmap.contains("The run-elf path is tested"));
    }

    #[test]
    fn llvm_filemap_manifest_names_backend_source_surface() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let filemap_manifest = include_str!("../toolchain/lnp64_llvm_filemap.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = llvm_filemap_rows(filemap_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let filemap_path = manifest_field(target_manifest, "llvm_filemap_contract");
        let mut layers = std::collections::BTreeSet::new();
        let mut paths = std::collections::BTreeSet::new();
        let mut purposes = Vec::new();
        let mut statuses = std::collections::BTreeMap::new();

        assert_eq!(filemap_path, "toolchain/lnp64_llvm_filemap.manifest");
        assert!(manifest_root.join(filemap_path).is_file());
        assert!(contract_index.contains(
            "llvm_filemap|toolchain/lnp64_llvm_filemap.manifest|llvm_filemap_manifest_names_backend_source_surface"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_llvm_filemap.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_llvm_filemap.manifest"));
        assert!(roadmap.contains("under `llvm/lib/Target/LNP64/`"));
        assert!(roadmap.contains("top-level `clang/`"));
        assert!(roadmap.contains("top-level `lld/`"));
        assert!(!roadmap.contains("under the matching\nllvm-project paths"));

        for (layer, path, status, purpose) in rows {
            layers.insert(layer);
            assert!(paths.insert(path), "duplicate llvm-project path {path}");
            statuses.insert(path, status);
            assert!(
                ["planned", "scaffolded"].contains(&status),
                "unknown llvm-project status {status} for {path}"
            );
            if status == "scaffolded" {
                assert!(
                    manifest_root.join(path).is_file(),
                    "scaffolded llvm-project file {path} is missing"
                );
            }
            assert!(
                path.starts_with("llvm/") || path.starts_with("clang/") || path.starts_with("lld/"),
                "llvm filemap path {path} must name an llvm-project source tree path"
            );
            assert!(
                !purpose.is_empty(),
                "llvm filemap path {path} must describe its purpose"
            );
            purposes.push(purpose);
        }

        for layer in [
            "llvm_target",
            "llvm_mc",
            "llvm_asmparser",
            "llvm_disassembler",
            "llvm_targetinfo",
            "lld",
            "clang_basic",
            "clang_driver",
            "llvm_tests",
            "clang_tests",
        ] {
            assert!(layers.contains(layer), "missing llvm filemap layer {layer}");
        }
        for path in [
            "llvm/lib/Target/LNP64/CMakeLists.txt",
            "llvm/lib/Target/LNP64/LNP64.td",
            "llvm/lib/Target/LNP64/LNP64RegisterInfo.td",
            "llvm/lib/Target/LNP64/LNP64InstrInfo.td",
            "llvm/lib/Target/LNP64/LNP64CallingConv.td",
            "llvm/lib/Target/LNP64/LNP64TargetMachine.cpp",
            "llvm/lib/Target/LNP64/LNP64Subtarget.cpp",
            "llvm/lib/Target/LNP64/LNP64InstrInfo.cpp",
            "llvm/lib/Target/LNP64/LNP64RegisterInfo.cpp",
            "llvm/lib/Target/LNP64/LNP64ISelLowering.cpp",
            "llvm/lib/Target/LNP64/LNP64ISelDAGToDAG.cpp",
            "llvm/lib/Target/LNP64/LNP64FrameLowering.cpp",
            "llvm/lib/Target/LNP64/LNP64AsmPrinter.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCTargetDesc.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmInfo.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCCodeEmitter.cpp",
            "llvm/lib/Target/LNP64/AsmParser/LNP64AsmParser.cpp",
            "llvm/lib/Target/LNP64/Disassembler/LNP64Disassembler.cpp",
            "llvm/lib/Target/LNP64/TargetInfo/LNP64TargetInfo.cpp",
            "lld/ELF/Arch/LNP64.cpp",
            "clang/lib/Basic/Targets/LNP64.h",
            "clang/lib/Basic/Targets/LNP64.cpp",
            "clang/lib/Driver/ToolChains/Arch/LNP64.cpp",
            "llvm/test/CodeGen/LNP64/hello.ll",
            "llvm/test/MC/LNP64/basic.s",
            "clang/test/Driver/lnp64.c",
        ] {
            assert!(paths.contains(path), "missing llvm filemap path {path}");
        }
        for path in [
            "llvm/lib/Target/LNP64/CMakeLists.txt",
            "llvm/lib/Target/LNP64/LNP64.td",
            "llvm/lib/Target/LNP64/LNP64RegisterInfo.td",
            "llvm/lib/Target/LNP64/LNP64InstrInfo.td",
            "llvm/lib/Target/LNP64/LNP64CallingConv.td",
            "llvm/lib/Target/LNP64/LNP64TargetMachine.cpp",
            "llvm/lib/Target/LNP64/LNP64Subtarget.cpp",
            "llvm/lib/Target/LNP64/LNP64InstrInfo.cpp",
            "llvm/lib/Target/LNP64/LNP64RegisterInfo.cpp",
            "llvm/lib/Target/LNP64/LNP64ISelLowering.cpp",
            "llvm/lib/Target/LNP64/LNP64ISelDAGToDAG.cpp",
            "llvm/lib/Target/LNP64/LNP64FrameLowering.cpp",
            "llvm/lib/Target/LNP64/LNP64AsmPrinter.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCTargetDesc.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmInfo.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCCodeEmitter.cpp",
            "llvm/lib/Target/LNP64/AsmParser/LNP64AsmParser.cpp",
            "llvm/lib/Target/LNP64/Disassembler/LNP64Disassembler.cpp",
            "llvm/lib/Target/LNP64/TargetInfo/LNP64TargetInfo.cpp",
            "lld/ELF/Arch/LNP64.cpp",
            "clang/lib/Basic/Targets/LNP64.h",
            "clang/lib/Basic/Targets/LNP64.cpp",
            "clang/lib/Driver/ToolChains/Arch/LNP64.cpp",
            "llvm/test/CodeGen/LNP64/hello.ll",
            "llvm/test/MC/LNP64/basic.s",
            "clang/test/Driver/lnp64.c",
        ] {
            assert_eq!(statuses[path], "scaffolded", "{path} should be scaffolded");
        }
        for concept in [
            "register",
            "calling",
            "relocation",
            "inline asm",
            "driver",
            "static",
            "driver-surface",
        ] {
            assert!(
                purposes.iter().any(|purpose| purpose.contains(concept)),
                "llvm filemap must cover {concept}"
            );
        }
        let target_td = include_str!("../llvm/lib/Target/LNP64/LNP64.td");
        let registers_td = include_str!("../llvm/lib/Target/LNP64/LNP64RegisterInfo.td");
        let calling_td = include_str!("../llvm/lib/Target/LNP64/LNP64CallingConv.td");
        let instr_td = include_str!("../llvm/lib/Target/LNP64/LNP64InstrInfo.td");
        let instr_info = include_str!("../llvm/lib/Target/LNP64/LNP64InstrInfo.cpp");
        let cmake = include_str!("../llvm/lib/Target/LNP64/CMakeLists.txt");
        let target_info = include_str!("../llvm/lib/Target/LNP64/TargetInfo/LNP64TargetInfo.cpp");
        let mc_desc_header =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCTargetDesc.h");
        let mc_desc_cmake = include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/CMakeLists.txt");
        let mc_desc = include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCTargetDesc.cpp");
        let mc_asm_info = include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmInfo.h");
        let mc_emitter =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCCodeEmitter.cpp");
        let mc_asm_backend =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmBackend.cpp");
        let inst_printer =
            include_str!("../llvm/lib/Target/LNP64/InstPrinter/LNP64InstPrinter.cpp");
        let asm_parser = include_str!("../llvm/lib/Target/LNP64/AsmParser/LNP64AsmParser.cpp");
        let disassembler =
            include_str!("../llvm/lib/Target/LNP64/Disassembler/LNP64Disassembler.cpp");
        let target_machine = include_str!("../llvm/lib/Target/LNP64/LNP64TargetMachine.cpp");
        let asm_printer = include_str!("../llvm/lib/Target/LNP64/LNP64AsmPrinter.cpp");
        let subtarget = include_str!("../llvm/lib/Target/LNP64/LNP64Subtarget.cpp");
        let isel = include_str!("../llvm/lib/Target/LNP64/LNP64ISelLowering.cpp");
        let dag_isel = include_str!("../llvm/lib/Target/LNP64/LNP64ISelDAGToDAG.cpp");
        let isel_header = include_str!("../llvm/lib/Target/LNP64/LNP64ISelLowering.h");
        let frame = include_str!("../llvm/lib/Target/LNP64/LNP64FrameLowering.cpp");
        let reginfo = include_str!("../llvm/lib/Target/LNP64/LNP64RegisterInfo.cpp");
        let clang_target_header = include_str!("../clang/lib/Basic/Targets/LNP64.h");
        let clang_target = include_str!("../clang/lib/Basic/Targets/LNP64.cpp");
        let clang_driver = include_str!("../clang/lib/Driver/ToolChains/Arch/LNP64.cpp");
        let lld_arch = include_str!("../lld/ELF/Arch/LNP64.cpp");
        let codegen_test = include_str!("../llvm/test/CodeGen/LNP64/hello.ll");
        let mc_test = include_str!("../llvm/test/MC/LNP64/basic.s");
        let clang_driver_test = include_str!("../clang/test/Driver/lnp64.c");

        assert!(target_td.contains("def LNP64 : Target"));
        for required in ["GPR", "FDR", "FPR", "VR", "PCR", "LR", "FLAGS", "R31"] {
            assert!(
                registers_td.contains(required),
                "register TableGen missing {required}"
            );
        }
        assert!(registers_td.contains(r#"sequence "FD%u", 0, 255"#));
        assert!(registers_td.contains("class LNP64GPR<bits<16> Enc"));
        assert!(calling_td.contains("CC_LNP64"));
        assert!(calling_td.contains("R2, R3, R4, R5, R6, R7, R8, R9"));
        assert!(calling_td.contains("iPTR"));
        for opcode in [
            "ADD",
            "LIU",
            "LD",
            "JAL",
            "YIELD",
            "AUIPC",
            "JALR",
            "RET",
            "SLT",
            "SLTU",
            "BLTU",
            "ERRNO_SET",
            "FORK",
            "WAIT_PID",
            "GET_PCR",
            "SET_PCR",
            "OPEN_AT",
            "CLONE_SPAWN",
            "THREAD_JOIN",
            "EXIT",
            "AWAIT",
            "GATE_CALL",
            "GATE_RETURN",
            "PULL",
            "OBJECT_CTL",
            "CAP_REVOKE",
        ] {
            assert!(instr_td.contains(opcode), "instr TableGen missing {opcode}");
        }
        for shape in [
            "class LNP64RRR",
            "(outs GPR:$rd)",
            "(ins GPR:$rs1, GPR:$rs2)",
            "class LNP64MemLoad",
            "${offset}(${base})",
            "class LNP64Native4",
            "(ins GPR:$cap, GPR:$arg0, GPR:$arg1)",
            "class LNP64PcrGet",
            "(ins PCR:$pcr)",
            "class LNP64PcrSet",
        ] {
            assert!(instr_td.contains(shape), "instr TableGen missing {shape}");
        }
        assert!(cmake.contains("LNP64GenRegisterInfo.inc"));
        assert!(cmake.contains("LNP64GenDAGISel.inc"));
        assert!(cmake.contains("AsmPrinter"));
        assert!(cmake.contains("SelectionDAG"));
        assert!(cmake.contains("add_llvm_component_group(LNP64)"));
        assert!(cmake.contains("ADD_TO_COMPONENT"));
        for source in [
            "LNP64TargetMachine.cpp",
            "LNP64AsmPrinter.cpp",
            "LNP64Subtarget.cpp",
            "LNP64ISelLowering.cpp",
            "LNP64ISelDAGToDAG.cpp",
            "LNP64FrameLowering.cpp",
            "add_subdirectory(InstPrinter)",
            "add_subdirectory(AsmParser)",
            "add_subdirectory(Disassembler)",
        ] {
            assert!(cmake.contains(source), "CMake missing {source}");
        }
        assert!(cmake.contains("add_llvm_target(LNP64CodeGen"));
        assert!(mc_desc_cmake.contains("LNP64MCAsmBackend.cpp"));
        assert!(mc_desc_cmake.contains("LNP64MCAsmInfo.cpp"));
        assert!(mc_desc_cmake.contains("LNP64InstPrinter"));
        assert!(target_info.contains("LLVMInitializeLNP64TargetInfo"));
        assert!(target_info.contains("RegisterTarget<Triple::lnp64>"));
        assert!(mc_desc.contains("LLVMInitializeLNP64TargetMC"));
        assert!(mc_desc.contains("RegisterMCAsmInfo<LNP64MCAsmInfo>"));
        assert!(mc_desc.contains("RegisterMCCodeEmitter"));
        assert!(mc_desc.contains("RegisterMCAsmBackend"));
        assert!(mc_desc.contains("RegisterMCInstPrinter"));
        assert!(mc_desc_header.contains("fixup_lnp64_branch"));
        assert!(mc_desc_header.contains("fixup_lnp64_auipc"));
        assert!(mc_desc_header.contains("fixup_lnp64_jump"));
        assert!(mc_desc_header.contains("fixup_lnp64_pcrel32"));
        assert!(mc_desc_header.contains("fixup_lnp64_abs32"));
        assert!(mc_desc_header.contains("fixup_lnp64_tls_tprel_slot64"));
        assert!(mc_asm_info.contains("MCAsmInfoELF"));
        assert!(mc_emitter.contains("createLNP64MCCodeEmitter"));
        assert!(mc_asm_backend.contains("createLNP64AsmBackend"));
        assert!(mc_asm_backend.contains("LNP64ELFObjectWriter"));
        assert!(mc_asm_backend.contains("R_LNP64_BRANCH"));
        for (fixup, relocation) in [
            ("fixup_lnp64_auipc", "R_LNP64_AUIPC"),
            ("fixup_lnp64_branch", "R_LNP64_BRANCH"),
            ("fixup_lnp64_jump", "R_LNP64_JUMP"),
            ("fixup_lnp64_pcrel32", "R_LNP64_PC32"),
            ("fixup_lnp64_abs32", "R_LNP64_ABS32"),
            ("fixup_lnp64_tls_tprel_slot64", "R_LNP64_TLS_TPREL_SLOT64"),
        ] {
            assert!(mc_asm_backend.contains(fixup), "MC backend missing {fixup}");
            assert!(
                mc_asm_backend.contains(relocation),
                "MC backend missing relocation mapping {relocation}"
            );
        }
        // The InstPrinter is now TableGen-generated: mnemonics, operand order
        // and the mov/li/ret aliases come from the .td AsmStrings via
        // LNP64GenAsmWriter.inc. This file only provides the printOperand /
        // register-name hooks -- no hand-written per-opcode mnemonic switch.
        assert!(inst_printer.contains("createLNP64MCInstPrinter"));
        assert!(inst_printer.contains("#define PRINT_ALIAS_INSTR"));
        assert!(inst_printer.contains("LNP64GenAsmWriter.inc"));
        assert!(inst_printer.contains("printInstruction(MI, Address, O)"));
        assert!(inst_printer.contains("printAliasInstr(MI, Address, O)"));
        assert!(inst_printer.contains("getRegisterName"));
        assert!(!inst_printer.contains("getLNP64Mnemonic"));
        // The encoder is now TableGen-generated: encodeInstruction defers to the
        // generated getBinaryCodeForInstr over the `bits<64> Inst` fields, with
        // custom operand encoders for the branch/jump/auipc targets that emit
        // relocations. The per-opcode bit layout lives in the TableGen, not a
        // hand-written switch.
        assert!(mc_emitter.contains("getBinaryCodeForInstr"));
        assert!(mc_emitter.contains("LNP64GenMCCodeEmitter.inc"));
        assert!(mc_emitter.contains("getMachineOpValue"));
        assert!(mc_emitter.contains("getBranchTargetOpValue"));
        assert!(mc_emitter.contains("getJumpTargetOpValue"));
        assert!(mc_emitter.contains("getAUIPCTargetOpValue"));
        assert!(mc_emitter.contains("fixup_lnp64_auipc"));
        assert!(mc_emitter.contains("fixup_lnp64_branch"));
        assert!(mc_emitter.contains("fixup_lnp64_jump"));
        // The generated emitter is built from the `field bits<64> Inst` layout.
        assert!(cmake.contains("LNP64GenMCCodeEmitter.inc"));
        assert!(instr_td.contains("field bits<64> Inst"));
        assert!(instr_td.contains("let Inst{63-56} = Opcode"));
        // The AsmParser now defers instruction matching to the TableGen
        // -gen-asm-matcher tables (MatchInstructionImpl): mnemonics and per-
        // instruction operand shape/typing come from the .td AsmStrings, not a
        // hand StringSwitch + per-opcode operand dispatch. This file only lexes
        // operands (registers incl. PCR names, immediates, off(base) memory).
        assert!(asm_parser.contains("LLVMInitializeLNP64AsmParser"));
        assert!(asm_parser.contains("RegisterMCAsmParser"));
        assert!(asm_parser.contains("tryParseRegister"));
        assert!(asm_parser.contains("MatchInstructionImpl"));
        assert!(asm_parser.contains("GET_ASSEMBLER_HEADER"));
        assert!(asm_parser.contains("GET_MATCHER_IMPLEMENTATION"));
        assert!(asm_parser.contains("LNP64GenAsmMatcher.inc"));
        assert!(asm_parser.contains("parseRegisterName"));
        assert!(asm_parser.contains("addImmOperands"));
        assert!(asm_parser.contains("addRegOperands"));
        assert!(!asm_parser.contains("buildInstruction"));
        assert!(!asm_parser.contains(".Case(\"addi\""));
        // The .td is the single source of truth for these mnemonics/shapes.
        assert!(instr_td.contains(r#"def CLONE_SPAWN : LNP64RRR<"clone.spawn">"#));
        assert!(instr_td.contains("def OPEN_AT : LNP64Native4"));
        assert!(instr_td.contains("def FORK : LNP64RuntimeGet"));
        assert!(instr_td.contains("def WAIT_PID : LNP64RR"));
        assert!(instr_td.contains("def CLONE_SPAWN : LNP64RRR"));
        assert!(instr_td.contains("def THREAD_JOIN : LNP64RRR"));
        assert!(instr_td.contains("def GET_PCR : LNP64PcrGet"));
        assert!(instr_td.contains("def SET_PCR : LNP64PcrSet"));
        // The disassembler is now TableGen-generated: it defers to
        // decodeInstruction over the generated DecoderTable64 (the verified
        // inverse of the generated encoder), with only register-class and
        // immediate decode hooks hand-provided. No hand-written per-opcode
        // switch (`case 0x..` / MI.setOpcode), and the isCodeGenOnly aliases
        // (li/mov/ret) are excluded from the decoder -- one decodable
        // instruction per opcode -- and re-spelled by the printer's InstAliases.
        assert!(disassembler.contains("LLVMInitializeLNP64Disassembler"));
        assert!(disassembler.contains("RegisterMCDisassembler"));
        assert!(disassembler.contains("LNP64GenDisassemblerTables.inc"));
        assert!(disassembler.contains("decodeInstruction(DecoderTable64"));
        assert!(disassembler.contains("DecodeGPRRegisterClass"));
        assert!(disassembler.contains("DecodePCRRegisterClass"));
        assert!(disassembler.contains("decodeShiftedTarget"));
        assert!(disassembler.contains("decodeSImm32"));
        assert!(disassembler.contains("ArrayRef<uint8_t> Bytes"));
        assert!(!disassembler.contains("MemoryObject"));
        assert!(!disassembler.contains("MI.setOpcode("));
        assert!(disassembler.contains("SignExtend64<32>"));
        assert!(disassembler.contains("MCDisassembler::Fail"));
        assert!(target_machine.contains("LLVMInitializeLNP64Target"));
        assert!(target_machine.contains("LLVMInitializeLNP64AsmPrinter"));
        assert!(target_machine.contains("createPassConfig"));
        assert!(target_machine.contains("addInstSelector"));
        assert!(target_machine.contains("createLNP64ISelDag"));
        assert!(target_machine.contains("TargetLoweringObjectFileELF"));
        assert!(target_machine.contains("e-m:e-p:64:64-i64:64-n64-S128"));
        assert!(target_machine.contains("initAsmInfo()"));
        assert!(dag_isel.contains("SelectionDAGISel"));
        assert!(dag_isel.contains("LNP64GenDAGISel.inc"));
        assert!(dag_isel.contains("SelectCode(Node)"));
        assert!(dag_isel.contains("SelectFrameIndexValue"));
        // A bare frame-index value selects to `addi rd, <fi>, 0` in place
        // (SelectNodeTo), resolved by the generic eliminateFrameIndex path --
        // no dedicated frame-address pseudo.
        assert!(dag_isel.contains("SelectNodeTo(Node, LNP64::ADDI"));
        assert!(dag_isel.contains("SelectFrameIndexLoad"));
        assert!(dag_isel.contains("SelectFrameIndexStore"));
        assert!(dag_isel.contains("getTargetFrameIndex"));
        assert!(dag_isel.contains("ISD::SEXTLOAD"));
        assert!(dag_isel.contains("ISD::EXTLOAD"));
        assert!(dag_isel.contains("MemVT == MVT::i1"));
        assert!(dag_isel.contains("LNP64::LW"));
        assert!(dag_isel.contains("LNP64::LH"));
        assert!(dag_isel.contains("LNP64::LB"));
        assert!(dag_isel.contains("LNP64::LWU"));
        assert!(dag_isel.contains("LNP64::SW"));
        assert!(dag_isel.contains("LNP64::SH"));
        assert!(dag_isel.contains("LNP64::SB"));
        assert!(asm_printer.contains("RegisterAsmPrinter<LNP64AsmPrinter>"));
        assert!(asm_printer.contains("void LNP64AsmPrinter::emitInstruction"));
        assert!(asm_printer.contains("PrintAsmOperand"));
        assert!(asm_printer.contains("printLNP64AsmReg"));
        // Mnemonics (get_pcr/open_at/clone.spawn/...) and register names
        // (PID/SIGMASK/...) come from the .td AsmStrings and register AsmNames
        // via the generated AsmWriter; the printer no longer hand-codes them.
        assert!(inst_printer.contains("getRegisterName(Op.getReg())"));
        assert!(!inst_printer.contains("case LNP64::"));
        assert!(asm_printer.contains("PrintAsmMemoryOperand"));
        assert!(asm_printer.contains("MachineOperand::MO_MachineBasicBlock"));
        assert!(asm_printer.contains("MachineOperand::MO_GlobalAddress"));
        assert!(asm_printer.contains("MachineOperand::MO_ExternalSymbol"));
        assert!(asm_printer.contains("EmitToStreamer(*OutStreamer, Inst)"));
        assert!(subtarget.contains("TLInfo(TM, *this)"));
        assert!(isel.contains("addRegisterClass(MVT::i64"));
        assert!(isel.contains("ISD::ADD"));
        assert!(isel.contains("ISD::SDIV"));
        assert!(isel.contains("setOperationAction(ISD::BRCOND, MVT::Other, Custom)"));
        assert!(isel.contains("getBranchForCC"));
        assert!(isel.contains("LNP64::PseudoLI64"));
        assert!(isel.contains("TII.get(LNP64::LI)"));
        assert!(isel.contains("TII.get(LNP64::LIU)"));
        assert!(isel.contains("LNP64GenCallingConv.inc"));
        assert!(isel.contains("LowerOperation"));
        assert!(isel.contains("setOperationAction(ISD::GlobalAddress, MVT::i64, Custom)"));
        assert!(isel.contains("ISD::GlobalAddress"));
        assert!(isel.contains("LNP64ISD::WRAPPER"));
        assert!(isel.contains("ISD::BR_CC"));
        assert!(isel.contains("ISD::BRCOND"));
        assert!(isel.contains("EmitInstrWithCustomInserter"));
        assert!(isel.contains("LNP64::PseudoSELECT_CC"));
        assert!(isel.contains("LNP64::BEQ"));
        assert!(isel.contains("LNP64::BLTU"));
        assert!(isel.contains("getBranchForCC(CC, SwapOps)"));
        assert!(isel.contains("TII.get(BrOpc)"));
        assert!(isel.contains("LowerFormalArguments"));
        assert!(isel.contains("CCInfo.AnalyzeFormalArguments(Ins, CC_LNP64)"));
        assert!(isel.contains("CreateFixedObject"));
        assert!(isel.contains("MachinePointerInfo::getFixedStack"));
        assert!(isel.contains("MF.addLiveIn(VA.getLocReg(), &LNP64::GPRRegClass)"));
        assert!(isel.contains("LowerReturn"));
        assert!(isel.contains("CCInfo.AnalyzeReturn(Outs, RetCC_LNP64)"));
        assert!(isel.contains("DAG.getCopyToReg"));
        assert!(isel.contains("LowerCall"));
        assert!(isel.contains("ArgCCInfo.AnalyzeCallOperands(CLI.Outs, CC_LNP64)"));
        assert!(isel.contains("DAG.getCALLSEQ_START"));
        assert!(isel.contains("DAG.getCALLSEQ_END"));
        assert!(isel.contains("DAG.getTargetGlobalAddress"));
        assert!(isel.contains("DAG.getTargetExternalSymbol"));
        assert!(isel.contains("indirect call callee must lower to an i64 register"));
        assert!(isel.contains("ISD::ATOMIC_LOAD"));
        assert!(isel.contains("ISD::ATOMIC_STORE"));
        assert!(isel.contains("setMaxAtomicSizeInBitsSupported(64)"));
        assert!(isel.contains("AtomicExpansionKind::LLSC"));
        assert!(isel.contains("shouldExpandAtomicRMWInIR"));
        assert!(isel.contains("LNP64ISD::CALL"));
        assert!(isel.contains("CalleeName == \"__lnp_await\" || CalleeName == \"__lnp_call\""));
        assert!(
            isel.contains(
                "CalleeName == \"__lnp_domain_ctl\" || CalleeName == \"__lnp_object_ctl\""
            )
        );
        assert!(isel.contains("LNP64ISD::AWAIT"));
        assert!(isel.contains("LNP64ISD::DOMAIN_CTL"));
        assert!(isel.contains("LNP64ISD::GATE_CALL"));
        assert!(isel.contains("LNP64ISD::GATE_RETURN"));
        assert!(isel.contains("LNP64ISD::OBJECT_CTL"));
        assert!(isel.contains("LNP64ISD::PULL"));
        assert!(isel.contains("LNP64ISD::PUSH"));
        assert!(isel.contains("RetCCInfo.AnalyzeCallResult(CLI.Ins, RetCC_LNP64)"));
        assert!(isel.contains("native shim lowering expects three arguments and a result"));
        assert!(isel.contains("native control lowering expects one argument and a result"));
        assert!(isel.contains("LNP64ISD::RET_FLAG"));
        assert!(isel.contains("setLoadExtAction(ISD::ZEXTLOAD, MVT::i64, MemVT, Legal)"));
        assert!(isel.contains("MVT::i1"));
        assert!(isel.contains("setLoadExtAction(ISD::SEXTLOAD, MVT::i64, MemVT, Legal)"));
        assert!(isel.contains("setLoadExtAction(ISD::EXTLOAD, MVT::i64, MemVT, Legal)"));
        assert!(instr_td.contains("zextloadi1"));
        assert!(isel.contains("setTruncStoreAction(MVT::i64, MemVT, Legal)"));
        assert!(isel.contains("LNP64TargetLowering::getConstraintType"));
        assert!(isel.contains("return C_RegisterClass"));
        assert!(isel.contains("LNP64TargetLowering::getRegForInlineAsmConstraint"));
        assert!(isel.contains("return std::make_pair(0U, &LNP64::GPRRegClass)"));
        assert!(isel.contains("computeRegisterProperties"));
        assert!(isel.contains("CCState ArgCCInfo(CLI.CallConv, CLI.IsVarArg"));
        assert!(isel.contains("unsigned VarArgStackBytes = 0"));
        assert!(isel.contains("if (CLI.IsVarArg && !CLI.Outs[I].IsFixed)"));
        assert!(isel.contains("VarArgStackOffset += alignTo"));
        assert!(isel.contains("setOperationAction(ISD::VASTART, MVT::Other, Custom)"));
        assert!(isel.contains("setOperationAction(ISD::VAEND, MVT::Other, Expand)"));
        assert!(isel.contains("case ISD::VASTART"));
        assert!(isel.contains("CreateFixedObject(8, 0, /*IsImmutable=*/true)"));
        assert!(!isel.contains("varargs lowering is not implemented yet"));
        assert!(isel_header.contains("getTargetNodeName"));
        assert!(isel_header.contains("LowerOperation"));
        assert!(isel_header.contains("getConstraintType"));
        assert!(isel_header.contains("getRegForInlineAsmConstraint"));
        assert!(isel_header.contains("EmitInstrWithCustomInserter"));
        assert!(isel_header.contains("SELECT_CC"));
        assert!(isel_header.contains("LowerFormalArguments"));
        assert!(isel_header.contains("LowerReturn"));
        assert!(isel_header.contains("LowerCall"));
        assert!(isel_header.contains("AWAIT"));
        assert!(isel_header.contains("CALL"));
        assert!(isel_header.contains("DOMAIN_CTL"));
        assert!(isel_header.contains("GATE_CALL"));
        assert!(isel_header.contains("GATE_RETURN"));
        assert!(isel_header.contains("OBJECT_CTL"));
        assert!(isel_header.contains("PULL"));
        assert!(isel_header.contains("PUSH"));
        assert!(isel_header.contains("WRAPPER"));
        assert!(isel_header.contains("RET_FLAG"));
        assert!(instr_td.contains("def simm32_imm"));
        assert!(instr_td.contains("def simm32_i64_imm"));
        assert!(instr_td.contains("def wide64_imm"));
        assert!(instr_td.contains("def all_ones_imm"));
        assert!(instr_td.contains("def brtarget : Operand<OtherVT>"));
        assert!(instr_td.contains("(ins GPR:$rs1, GPR:$rs2, brtarget:$target)"));
        assert!(instr_td.contains("(brcc SETEQ"));
        assert!(instr_td.contains("def BEQ"));
        assert!(instr_td.contains("def BNE"));
        assert!(instr_td.contains("def BLTU"));
        assert!(instr_td.contains("class LNP64Branch"));
        assert!(instr_td.contains("def SLT"));
        assert!(instr_td.contains("def SLTI"));
        assert!(instr_td.contains("def LB"));
        assert!(instr_td.contains("usesCustomInserter = 1"));
        assert!(instr_td.contains("def PseudoSELECT_CC"));
        assert!(instr_td.contains("(brcc SETEQ, GPR:$a, GPR:$b, bb:$t), (BEQ GPR:$a, GPR:$b, bb:$t)"));
        assert!(instr_td.contains("(brcc SETULT, GPR:$a, GPR:$b, bb:$t), (BLTU GPR:$a, GPR:$b, bb:$t)"));
        assert!(instr_td.contains("(setcc GPR:$rs, simm32_i64_imm:$imm, SETLT)"));
        assert!(instr_td.contains("(setcc GPR:$rs, simm32_i64_imm:$imm, SETULT)"));
        assert!(instr_td.contains("(i64 (sextloadi8 (add GPR:$base, simm32_imm:$offset)))"));
        assert!(instr_td.contains("(i64 (extloadi16 (add GPR:$base, simm32_imm:$offset)))"));
        assert!(instr_td.contains("(LH GPR:$base, simm32_imm:$offset)"));
        assert!(instr_td.contains("def LNP64retflag"));
        assert!(instr_td.contains("def SDT_LNP64Call"));
        assert!(instr_td.contains("SDTypeProfile<0, -1, []>"));
        assert!(instr_td.contains("def LNP64call"));
        assert!(instr_td.contains("def LNP64domainctl"));
        assert!(instr_td.contains("def LNP64gatecall"));
        assert!(instr_td.contains("def LNP64objectctl"));
        assert!(instr_td.contains("def LNP64pull"));
        assert!(instr_td.contains("def LNP64push"));
        assert!(instr_td.contains("def LNP64wrapper"));
        assert!(instr_td.contains("(add GPR:$rs, simm32_i64_imm:$imm)"));
        assert!(instr_td.contains("def AUIPC"));
        assert!(instr_td.contains("def LI "));
        assert!(instr_td.contains("def LIU "));
        assert!(instr_td.contains("def PseudoLI64"));
        assert!(instr_td.contains("let Size = 8"));
        assert!(instr_td.contains("(i64 (LNP64wrapper tglobaladdr:$target))"));
        assert!(instr_td.contains("(AUIPC tglobaladdr:$target)"));
        assert!(!instr_td.contains("(LA tglobaladdr:$target)"));
        assert!(instr_td.contains("(PseudoLI64 wide64_imm:$imm)"));
        assert!(instr_td.contains("(set GPR:$rd, (add GPR:$rs1, GPR:$rs2))"));
        assert!(instr_td.contains("(set GPR:$rd, (xor GPR:$rs, all_ones_imm))"));
        assert!(instr_td.contains("(set GPR:$rd, (shl GPR:$rs1, GPR:$rs2))"));
        assert!(instr_td.contains("let Pattern = [(br bb:$target)]"));
        assert!(instr_td.contains("(LNP64call tglobaladdr:$target)"));
        assert!(instr_td.contains("(LNP64call texternalsym:$target)"));
        assert!(instr_td.contains("(LNP64call GPR:$target)"));
        assert!(instr_td.contains("(i64 (load (add GPR:$base, simm32_imm:$offset)))"));
        assert!(instr_td.contains("(i64 (zextloadi32 (add GPR:$base, simm32_imm:$offset)))"));
        assert!(instr_td.contains("(i64 (zextloadi16 (add GPR:$base, simm32_imm:$offset)))"));
        assert!(instr_td.contains("(i64 (zextloadi8 (add GPR:$base, simm32_imm:$offset)))"));
        assert!(instr_td.contains("(SD GPR:$rs, GPR:$base, simm32_imm:$offset)"));
        assert!(instr_td.contains("(SW GPR:$rs, GPR:$base, simm32_imm:$offset)"));
        assert!(instr_td.contains("(SH GPR:$rs, GPR:$base, simm32_imm:$offset)"));
        assert!(instr_td.contains("(SB GPR:$rs, GPR:$base, simm32_imm:$offset)"));
        assert!(instr_td.contains("(LNP64domainctl GPR:$arg)"));
        assert!(instr_td.contains("(LNP64gatecall GPR:$cap, GPR:$arg0, GPR:$arg1)"));
        assert!(instr_td.contains("(LNP64objectctl GPR:$arg)"));
        assert!(instr_td.contains("(LNP64pull GPR:$cap, GPR:$arg0, GPR:$arg1)"));
        assert!(instr_td.contains("(LNP64push GPR:$cap, GPR:$arg0, GPR:$arg1)"));
        assert!(instr_td.contains("isReturn = 1"));
        assert!(instr_td.contains("Defs = [R1, R2"));
        // Calls clobber only caller-saved GPRs; the callee-saved set
        // s0..s9 = r18..r27 is preserved and must not be in the call Defs list.
        assert!(instr_td.contains("R15, R16, R17, R28, R29, R30] in {"));
        assert!(isel.contains("getCallPreservedMask"));
        assert!(isel.contains("DAG.getRegisterMask(Mask)"));
        assert!(reginfo.contains("CSR_LNP64_RegMask"));
        assert!(!instr_td.contains("Defs = [FLAGS]"));
        assert!(!instr_td.contains("Uses = [FLAGS]"));
        assert!(instr_td.contains("Uses = [R1]"));
        assert!(instr_td.contains("let Pattern = [(LNP64retflag)]"));
        assert!(instr_td.contains("isBranch = 1"));
        assert!(instr_td.contains("ADJCALLSTACKDOWN"));
        assert!(instr_td.contains("ADJCALLSTACKUP"));
        assert!(instr_info.contains("/*CFSetupOpcode=*/LNP64::ADJCALLSTACKDOWN"));
        assert!(instr_info.contains("/*CFDestroyOpcode=*/LNP64::ADJCALLSTACKUP"));
        assert!(instr_info.contains("/*ReturnOpcode=*/LNP64::RET"));
        assert!(instr_info.contains("copyPhysReg"));
        assert!(instr_info.contains("BuildMI(MBB, I, DL, get(LNP64::MOV), DestReg)"));
        assert!(instr_info.contains("storeRegToStackSlot"));
        assert!(instr_info.contains("loadRegFromStackSlot"));
        assert!(instr_info.contains("addFrameIndex(FrameIndex)"));
        assert!(isel.contains("setStackPointerRegisterToSaveRestore(LNP64::R31)"));
        assert!(frame.contains("StackGrowsDown"));
        assert!(frame.contains("bool LNP64FrameLowering::hasFP"));
        assert!(frame.contains("/*LocalAreaOffset=*/0"));
        assert!(frame.contains("Align(16)"));
        assert!(frame.contains("emitSPAdjust"));
        // E8: SP adjusted by a single ADDI (full signed-32 immediate), no
        // scratch register, no SUB/ADD-with-materialized-magnitude.
        assert!(frame.contains("TII.get(LNP64::ADDI)"));
        assert!(frame.contains("stack adjustment exceeds 32-bit immediate"));
        assert!(!frame.contains("LNP64::R30"));
        assert!(frame.contains("MCCFIInstruction::cfiDefCfa"));
        assert!(frame.contains("LNP64DwarfSP = 31"));
        assert!(frame.contains("LNP64DwarfRA = 1"));
        assert!(frame.contains("MCCFIInstruction::createOffset"));
        assert!(frame.contains("TargetOpcode::CFI_INSTRUCTION"));
        assert!(frame.contains("LNP64DwarfRA,"));
        assert!(reginfo.contains("Reserved.set(LNP64::R0)"));
        assert!(reginfo.contains("Reserved.set(LNP64::R31)"));
        assert!(reginfo.contains("eliminateFrameIndex"));
        assert!(reginfo.contains("void LNP64RegisterInfo::eliminateFrameIndex"));
        // Frame indices (loads, stores, and the `addi rd, <fi>, 0` frame-
        // address form) are resolved uniformly: base operand -> r31, following
        // immediate -> resolved offset. No PseudoFRAMEADDR, no scratch register.
        assert!(reginfo.contains("ChangeToRegister(LNP64::R31"));
        assert!(reginfo.contains("ChangeToImmediate(Offset)"));
        assert!(reginfo.contains("MFI.getObjectOffset"));
        assert!(reginfo.contains("32-bit signed"));
        // v2 callee-saved set s0..s9 = r18..r27.
        assert!(reginfo.contains("getCalleeSavedRegs"));
        assert!(reginfo.contains("LNP64::R18"));
        assert!(reginfo.contains("LNP64::R27"));
        assert!(clang_target.contains("resetDataLayout(\"e-m:e-p:64:64-i64:64-n64-S128\")"));
        assert!(clang_target.contains("__LNP64__"));
        assert!(clang_target_header.contains("getTargetBuiltins()"));
        assert!(clang_target_header.contains("isValidCPUName(StringRef Name)"));
        assert!(clang_target.contains("Name == \"generic-lnp64\""));
        assert!(clang_target_header.contains("setCPU(const std::string &Name)"));
        assert!(clang_target_header.contains("hasFeature(StringRef Feature)"));
        assert!(clang_target.contains("const char *LNP64TargetInfo::getClobbers() const"));
        for constraint in ["case 'r'", "case 'f'", "case 'p'", "case 'm'", "case 'i'"] {
            assert!(
                clang_target.contains(constraint),
                "clang target missing asm constraint {constraint}"
            );
        }
        assert!(clang_driver.contains("getLNP64TargetCPU"));
        assert!(clang_driver.contains("target/lnp64-sysroot/usr/include"));
        assert!(clang_driver.contains("target/lnp64-sysroot/usr/lib/lnp64/crt0.o"));
        assert!(clang_driver.contains("elf64lnp64"));
        assert!(clang_driver.contains("target/lnp64-sysroot/usr/lib/lnp64/lnp64_static.ld"));
        assert!(lld_arch.contains("getLNP64TargetInfo"));
        assert!(lld_arch.contains("copyRel = R_LNP64_NONE"));
        assert!(lld_arch.contains("relativeRel = R_LNP64_RELATIVE"));
        assert!(lld_arch.contains("switch (Rel.type)"));
        for reloc in [
            "R_LNP64_ABS64",
            "R_LNP64_RELATIVE",
            "R_LNP64_TLS_TPREL64",
            "R_LNP64_FDR_DESC64",
            "R_LNP64_BRANCH",
        ] {
            assert!(lld_arch.contains(reloc), "lld arch missing {reloc}");
        }
        assert!(codegen_test.contains("llc -mtriple=lnp64-unknown-none"));
        assert!(codegen_test.contains("XFAIL: *"));
        assert!(codegen_test.contains("define i64 @arith"));
        assert!(codegen_test.contains("define i64 @invert"));
        assert!(codegen_test.contains("define i64 @control"));
        assert!(codegen_test.contains("define i64 @gate"));
        assert!(codegen_test.contains("define i64 @read_stream"));
        assert!(codegen_test.contains("define i64 @wait_ready"));
        assert!(codegen_test.contains("define i64 @jump"));
        assert!(codegen_test.contains("define i64 @branch_if"));
        assert!(codegen_test.contains("define i64 @call_direct"));
        assert!(codegen_test.contains("define i64 @call_indirect"));
        assert!(codegen_test.contains("define i64 @memory"));
        assert!(codegen_test.contains("%biased = add i64 %sum, 7"));
        assert!(codegen_test.contains("br label %exit"));
        assert!(codegen_test.contains("call i64 @callee"));
        assert!(codegen_test.contains("; CHECK: call callee"));
        assert!(codegen_test.contains("; CHECK: cmp"));
        assert!(codegen_test.contains("; CHECK: beq"));
        assert!(codegen_test.contains("; CHECK: call_reg"));
        assert!(codegen_test.contains("; CHECK: jmp"));
        for mnemonic in ["ld.b", "ld.h", "ld.w", "st.b", "st.h", "st.w"] {
            assert!(
                codegen_test.contains(&format!("; CHECK: {mnemonic}")),
                "codegen fixture missing narrow memory check for {mnemonic}"
            );
        }
        assert!(codegen_test.contains("; CHECK: lsl"));
        assert!(codegen_test.contains("; CHECK: not"));
        assert!(codegen_test.contains("; CHECK: ret"));
        assert!(codegen_test.contains("__lnp_call"));
        assert!(codegen_test.contains("__lnp_domain_ctl"));
        assert!(codegen_test.contains("__lnp_object_ctl"));
        assert!(codegen_test.contains("__lnp_await"));
        assert!(codegen_test.contains("__lnp_pull"));
        assert!(codegen_test.contains("__lnp_push"));
        assert!(codegen_test.contains("__lnp_gate_return"));
        assert!(codegen_test.contains("; CHECK: domain_ctl"));
        assert!(codegen_test.contains("; CHECK: gate_call"));
        assert!(codegen_test.contains("; CHECK: gate_return"));
        assert!(codegen_test.contains("; CHECK: object_ctl"));
        assert!(codegen_test.contains("; CHECK: await"));
        assert!(codegen_test.contains("; CHECK: pull"));
        assert!(codegen_test.contains("; CHECK: push"));
        assert!(mc_test.contains("llvm-mc -triple=lnp64-unknown-none"));
        assert!(mc_test.contains("li r1, 42"));
        assert!(mc_test.contains("ld.h r5, 18(r31)"));
        assert!(mc_test.contains("st.h r5, 26(r31)"));
        assert!(mc_test.contains("XFAIL: *"));
        assert!(clang_driver_test.contains("--target=lnp64-unknown-none"));
        assert!(clang_driver_test.contains("elf64lnp64"));
        assert!(clang_driver_test.contains("target/lnp64-sysroot/usr/include"));
        assert!(clang_driver_test.contains("target/lnp64-sysroot/usr/lib/lnp64/crt0.o"));
        assert!(clang_driver_test.contains("XFAIL: *"));
    }

    #[test]
    fn libc_shim_manifest_covers_runtime_surfaces() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let shim_manifest = include_str!("../toolchain/lnp64_libc_shim.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let libc_roadmap = include_str!("../libc_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let run_elf_manifest = include_str!("../toolchain/lnp64_run_elf.manifest");
        let real_llc = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let real_llc_docker = include_str!("../scripts/run_real_llvm_lnp64_docker.sh");
        let emulator = include_str!("emulator.rs");
        let evidence_corpus =
            format!("{conformance}\n{run_elf_manifest}\n{real_llc}\n{real_llc_docker}\n{emulator}");
        let rows = libc_shim_rows(shim_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let shim_path = manifest_field(target_manifest, "libc_shim_contract");
        let mut groups = std::collections::BTreeMap::new();
        let mut group_evidence = std::collections::BTreeMap::new();

        assert_eq!(shim_path, "toolchain/lnp64_libc_shim.manifest");
        assert!(manifest_root.join(shim_path).is_file());
        assert!(contract_index.contains(
            "libc_shim|toolchain/lnp64_libc_shim.manifest|libc_shim_manifest_covers_runtime_surfaces"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_libc_shim.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_libc_shim.manifest"));
        assert!(libc_roadmap.contains("toolchain/lnp64_libc_shim.manifest"));

        for (group, public_surface, native_lowering, evidence, status) in rows {
            assert!(
                groups
                    .insert(
                        group,
                        (public_surface.clone(), native_lowering.clone(), status),
                    )
                    .is_none(),
                "duplicate libc shim group {group}"
            );
            assert!(
                group_evidence.insert(group, evidence.clone()).is_none(),
                "duplicate libc shim evidence group {group}"
            );
            assert!(
                ["tested", "partial", "planned"].contains(&status),
                "unknown libc shim status {status} for {group}"
            );
            assert!(
                !public_surface.is_empty(),
                "empty public surface for libc shim group {group}"
            );
            assert!(
                !native_lowering.is_empty(),
                "empty native lowering for libc shim group {group}"
            );
            assert!(
                !evidence.is_empty(),
                "empty evidence for libc shim group {group}"
            );
            for item in public_surface.iter().chain(native_lowering.iter()) {
                assert!(!item.is_empty(), "empty item in libc shim group {group}");
            }
            for item in evidence {
                assert!(
                    manifest_root.join(item).exists() || evidence_corpus.contains(item),
                    "libc shim evidence {item} for {group} is not present in repo evidence"
                );
            }
        }

        for group in [
            "startup_env_auxv",
            "errno_tls",
            "string_ctype",
            "numeric_conversion",
            "process_identity",
            "process_lifecycle_compat",
            "nonlocal_jump",
            "random_state",
            "path_helpers",
            "search_helpers",
            "sort_helpers",
            "time_clock",
            "fd_io",
            "malloc_heap",
            "pthread_futex",
            "poll_select_epoll_kqueue",
            "mmap_mprotect",
            "signals_as_events",
            "sockets_endpoints",
        ] {
            assert!(
                groups.contains_key(group),
                "missing libc shim group {group}"
            );
        }
        assert_eq!(
            groups["poll_select_epoll_kqueue"].2, "partial",
            "broader kqueue filters and semantics must stay partial"
        );
        assert_eq!(groups["process_lifecycle_compat"].2, "tested");
        assert_eq!(groups["nonlocal_jump"].2, "tested");
        assert!(conformance.contains("`setjmp`, `longjmp`"));
        assert!(conformance.contains("user-context restores"));
        assert!(conformance.contains("Stable generation-cookie validation"));
        assert!(shim_manifest.contains("toolchain/include/setjmp.h"));
        assert!(shim_manifest.contains("toolchain/liblnp64_setjmp_min.s"));
        assert!(
            group_evidence["nonlocal_jump"]
                .contains(&"real LLVM LNP64 run-elf setjmp/longjmp execution passed"),
            "nonlocal_jump row must name run-elf setjmp evidence"
        );
        assert!(shim_manifest.contains("userland/fork_wait_test_clang.c"));
        assert!(shim_manifest.contains("userland/elf_exec_test_clang.c"));
        assert!(shim_manifest.contains("pthread_atfork"));
        assert!(shim_manifest.contains("atfork_handlers"));
        assert!(shim_manifest.contains("toolchain/include/pthread.h"));
        assert!(libc_roadmap.contains("pthread_atfork"));
        assert!(libc_roadmap.contains("signal dispositions"));
        assert!(
            group_evidence["signals_as_events"]
                .contains(&"sigret_rejects_stale_signal_frame_generation_without_restore"),
            "signals_as_events row must name stale SIGRET generation evidence"
        );
        assert!(
            conformance.contains("`SIGRET` signal-frame generation rejection")
                || conformance.contains("stale `SIGRET` signal-frame generation rejection"),
            "signal conformance must record SIGRET generation coverage"
        );
        assert!(conformance.contains("trusted user-visible `SIGRET` tokens"));
        assert!(conformance.contains("fork_clone_inherits_signal_state_and_clears_pending_events"));
        assert!(conformance.contains("fork_clone_does_not_copy_in_flight_ipc_or_waiters"));
        assert!(conformance.contains("fork_clone_copies_vma_heap_metadata_and_isolates_memory"));
        assert!(conformance.contains("fork_clone_stresses_vma_heap_mutation_independence"));
        assert!(
            conformance
                .contains("copied signal masks/dispositions with cleared child pending signals")
        );
        assert!(conformance.contains("no copied in-flight IPC inbox entries or waiter ownership"));
        assert!(
            conformance
                .contains("copied VMA/heap metadata with independent post-fork memory writes")
        );
        assert!(conformance.contains("larger forked VMA and heap mutation independence"));
        assert!(shim_manifest.contains("userland/poll_test_clang.c"));
        assert!(shim_manifest.contains("real LLVM LNP64 run-elf NetBSD poll child passed"));
        for evidence in [
            "toolchain/liblnp64_poll_min.c",
            "userland/poll_test_clang.c",
            "real LLVM LNP64 run-elf poll/select/epoll/kqueue libc execution passed",
            "real LLVM LNP64 run-elf NetBSD poll child passed",
        ] {
            assert!(
                group_evidence["poll_select_epoll_kqueue"].contains(&evidence),
                "poll/select/epoll/kqueue row must name evidence {evidence}"
            );
        }
        assert!(conformance.contains("| `kqueue`, `kevent` | partial |"));
        assert!(conformance.contains("real-Clang `EPOLL_CTL_MOD` data replacement"));
        assert!(conformance.contains("EVFILT_READ and EVFILT_WRITE readiness"));
        assert!(conformance.contains("EV_DELETE removes a registered readiness source"));
        assert!(conformance.contains("EV_DISABLE suppresses delivery without removing"));
        assert!(conformance.contains("EV_ENABLE resumes delivery"));
        assert!(conformance.contains("EV_ONESHOT removes a source after first delivery"));
        assert!(conformance.contains("EV_RECEIPT returns an `EV_ERROR` success receipt"));
        assert!(
            conformance.contains("EVFILT_USER/NOTE_TRIGGER runs as a process-local trigger source")
        );
        assert!(conformance.contains("unsupported filter registrations fail closed"));
        assert!(conformance.contains("report `EV_ERROR`/`EINVAL`"));
        assert!(conformance.contains("`close(kq)` invalidates the process-local kqueue"));
        assert!(conformance.contains("Broader kernel-backed filters remain partial"));
        assert!(conformance.contains("`COMPAT-STRESS-005` | poll/epoll races"));
        for group in [
            "startup_env_auxv",
            "errno_tls",
            "string_ctype",
            "numeric_conversion",
            "process_identity",
            "nonlocal_jump",
            "random_state",
            "path_helpers",
            "search_helpers",
            "sort_helpers",
            "time_clock",
            "fd_io",
            "malloc_heap",
            "pthread_futex",
            "mmap_mprotect",
            "signals_as_events",
            "sockets_endpoints",
        ] {
            assert_eq!(groups[group].2, "tested", "{group} should be tested");
        }

        for (group, required_public, required_native) in [
            (
                "startup_env_auxv",
                vec![
                    "_start",
                    "argv",
                    "envp",
                    "environ",
                    "getenv",
                    "setenv",
                    "unsetenv",
                    "clearenv",
                    "putenv",
                    "getauxval",
                ],
                vec!["crt0", "TLS", "ENV_GET", "EXIT"],
            ),
            (
                "errno_tls",
                vec!["errno", "__errno_location", "strerror"],
                vec!["TLS", "ERRNO_SET", "completion_helpers"],
            ),
            (
                "string_ctype",
                vec!["strlen", "strcmp", "memcpy", "isalpha", "tolower"],
                vec!["load_store", "integer_alu", "static_link"],
            ),
            (
                "numeric_conversion",
                vec!["atoi", "strtol", "strtoull"],
                vec!["integer_alu", "ERRNO_SET", "static_link"],
            ),
            (
                "process_identity",
                vec![
                    "pid", "getpid", "getppid", "getuid", "geteuid", "getgid", "getegid",
                ],
                vec!["GET_PCR"],
            ),
            (
                "process_lifecycle_compat",
                vec!["_exit", "fork", "waitpid", "execve", "execvp"],
                vec!["EXIT", "FORK", "WAIT_PID", "EXEC", "errno_tls"],
            ),
            (
                "nonlocal_jump",
                vec!["setjmp", "longjmp", "jmp_buf"],
                vec![
                    "load_store",
                    "link_register_r1",
                    "stack_pointer_restore",
                    "user_context_only",
                    "validation_cookies_reserved",
                ],
            ),
            (
                "random_state",
                vec!["random", "srandom", "initstate", "setstate"],
                vec!["integer_alu", "load_store", "static_link"],
            ),
            (
                "path_helpers",
                vec!["basename", "dirname"],
                vec!["load_store", "integer_alu", "static_link"],
            ),
            (
                "search_helpers",
                vec!["lfind", "lsearch", "insque", "remque"],
                vec!["load_store", "integer_alu", "static_link"],
            ),
            (
                "sort_helpers",
                vec!["qsort"],
                vec!["load_store", "integer_alu", "static_link"],
            ),
            (
                "time_clock",
                vec![
                    "clock_gettime",
                    "time",
                    "usleep",
                    "sleep",
                    "timerfd_create",
                    "timerfd_settime",
                    "timerfd_gettime",
                ],
                vec![
                    "GET_PCR",
                    "REALTIME_SEC",
                    "REALTIME_NSEC",
                    "YIELD",
                    "OBJECT_CTL",
                    "PUSH",
                    "AWAIT",
                    "PULL",
                    "errno_tls",
                ],
            ),
            (
                "fd_io",
                vec![
                    "openat", "read", "write", "fcntl", "stat", "fstat", "futimens", "stdio",
                ],
                vec![
                    "__lnp_openat",
                    "__lnp_pull",
                    "__lnp_push",
                    "CAP_DUP",
                    "FDR",
                    "GET_META",
                    "SET_META",
                    "FD_SEEK_DYN",
                ],
            ),
            (
                "malloc_heap",
                vec!["malloc", "free", "posix_memalign"],
                vec!["ALLOC", "ALLOC_EX", "ALLOC_SIZE", "FREE"],
            ),
            (
                "pthread_futex",
                vec!["pthread_create", "pthread_join", "futex"],
                vec!["CLONE", "FUTEX_WAIT", "FUTEX_WAKE", "AWAIT"],
            ),
            (
                "poll_select_epoll_kqueue",
                vec!["poll", "select", "epoll_wait", "kqueue"],
                vec!["event_queue", "AWAIT", "OBJECT_CTL", "waitable_generation"],
            ),
            (
                "mmap_mprotect",
                vec!["mmap", "munmap", "mprotect"],
                vec!["MMAP", "MUNMAP", "MPROTECT", "VMA"],
            ),
            (
                "signals_as_events",
                vec!["sigaction", "signal", "SIGRET"],
                vec!["event_delivery", "signal_frame", "SIGRET"],
            ),
            (
                "sockets_endpoints",
                vec!["socket", "accept", "getsockopt", "recv"],
                vec!["OBJECT_CTL", "endpoint_profile", "GET_META", "PULL", "PUSH"],
            ),
        ] {
            let (public_surface, native_lowering, _) = &groups[group];
            for item in required_public {
                assert!(
                    public_surface.contains(&item),
                    "libc shim group {group} missing public surface {item}"
                );
            }
            for item in required_native {
                assert!(
                    native_lowering.contains(&item),
                    "libc shim group {group} missing native lowering {item}"
                );
            }
        }
    }

    #[test]
    fn llvm_bootstrap_manifest_names_first_clang_gate() {
        let bootstrap_manifest = include_str!("../toolchain/lnp64_llvm_bootstrap.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = llvm_bootstrap_rows(bootstrap_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut cases = std::collections::BTreeSet::new();
        let mut statuses = std::collections::BTreeMap::new();

        assert!(contract_index.contains(
            "llvm_bootstrap|toolchain/lnp64_llvm_bootstrap.manifest|llvm_bootstrap_manifest_names_first_clang_gate"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_llvm_bootstrap.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_llvm_bootstrap.manifest"));
        for case in ["hello", "arithmetic", "memory", "calls", "simple_libc"] {
            assert!(
                roadmap.contains(case),
                "roadmap must describe llvm bootstrap case {case}"
            );
        }

        for (case, source, backend_contracts, runtime_contracts, status) in rows {
            assert!(cases.insert(case), "duplicate llvm bootstrap case {case}");
            statuses.insert(case, status);
            assert!(
                manifest_root.join(source).exists(),
                "llvm bootstrap case {case} names missing source/gate {source}"
            );
            assert!(
                ["tested", "partial", "planned"].contains(&status),
                "unknown LLVM bootstrap status {status} for {case}"
            );
            if status == "tested" {
                assert!(
                    backend_contracts.contains(&"static_link"),
                    "tested case {case} must require static linking"
                );
            }
            assert!(
                !runtime_contracts.is_empty(),
                "case {case} must name runtime expectations"
            );
        }

        for case in [
            "hello",
            "arithmetic",
            "memory",
            "calls",
            "pcr",
            "cat",
            "json_parser",
            "rot13",
            "producer_consumer",
            "parallel_hash",
            "sqlite_lite",
            "ping_pong",
            "zlib_checksum",
            "natsort",
            "jsmn",
            "inih_parse_string",
            "cwalk",
            "sbase_commands",
            "userland_ucat",
            "userland_init",
            "userland_lnpsh",
            "userland_spawn_task",
            "netbsd_init_root",
            "netbsd_shell_root",
            "netbsd_loader_target_child",
            "netbsd_elf_exec_parent",
            "netbsd_fork_wait_child",
            "netbsd_thread_child",
            "netbsd_poll_child",
            "netbsd_signal_gate_child",
            "netbsd_signal_fault_child",
            "netbsd_timer_child",
            "netbsd_mmap_child",
            "netbsd_fd_passing_child",
            "netbsd_namespace_child",
            "netbsd_fs_service_child",
            "netbsd_classifier_child",
            "netbsd_socket_loopback_child",
            "netbsd_gate_trace_child",
            "netbsd_domain_nested_child",
            "netbsd_domain_budget_child",
            "netbsd_personality_clang",
            "netcat",
            "httpd",
            "simple_libc",
        ] {
            assert!(cases.contains(case), "missing llvm bootstrap case {case}");
        }
        for (case, status) in statuses {
            assert_eq!(status, "tested", "{case} should be tested");
        }
    }

    #[test]
    fn crt0_startup_stub_matches_crt_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let crt_manifest = include_str!("../toolchain/lnp64_crt_startup.manifest");
        let crt0 = include_str!("../toolchain/crt0_lnp64.s");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let crt0_path = manifest_field(target_manifest, "crt0_contract");

        assert_eq!(crt0_path, "toolchain/crt0_lnp64.s");
        assert!(manifest_root.join(crt0_path).is_file());
        assert!(
            contract_index
                .contains("crt0|toolchain/crt0_lnp64.s|crt0_startup_stub_matches_crt_contract")
        );
        assert!(transition_manifest.contains("toolchain/crt0_lnp64.s"));
        assert!(roadmap.contains("toolchain/crt0_lnp64.s"));

        for required in [
            "_start:",
            ".globl _start",
            ".type _start,@function",
            "li r7, 0x7000",
            "li r8, 0x100",
            "mul r7, r7, r8",
            "ld r1, 0(r7)",
            "li r2, 8",
            "add r2, r7, r2",
            "mul r3, r1, r8",
            "add r3, r3, r2",
            "add r3, r3, r8",
            "errno_set r0",
            "jal r1, main",
            "exit r2",
        ] {
            assert!(crt0.contains(required), "crt0 missing {required}");
        }
        assert!(crt_manifest.contains("entry_symbol|required|_start"));
        assert!(crt_manifest.contains("main_signature|required|main(argc,argv,envp)"));
        assert!(crt_manifest.contains("process_exit|required|EXIT"));
    }

    #[test]
    fn sysroot_manifest_records_packaged_crt_layout() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let sysroot_manifest = include_str!("../toolchain/lnp64_sysroot.manifest");
        let package_script = include_str!("../scripts/package_lnp64_sysroot.sh");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

        assert_eq!(
            manifest_field(target_manifest, "sysroot_contract"),
            "toolchain/lnp64_sysroot.manifest"
        );
        assert!(
            manifest_root
                .join("toolchain/lnp64_sysroot.manifest")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/package_lnp64_sysroot.sh")
                .is_file()
        );
        assert!(contract_index.contains(
            "sysroot|toolchain/lnp64_sysroot.manifest|sysroot_manifest_records_packaged_crt_layout"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_sysroot.manifest"));
        assert!(transition_manifest.contains("scripts/package_lnp64_sysroot.sh"));
        assert!(roadmap.contains("toolchain/lnp64_sysroot.manifest"));
        assert!(roadmap.contains("scripts/package_lnp64_sysroot.sh"));
        assert!(conformance.contains("sysroot_manifest_records_packaged_crt_layout"));

        for row in [
            "sysroot_root|target/lnp64-sysroot|scripts/package_lnp64_sysroot.sh|generated",
            "target_headers|usr/include|toolchain/include|generated",
            "private_intrinsics|usr/include/lnp64/intrinsics.h|toolchain/include/lnp64/intrinsics.h|generated",
            "canonical_intrinsics|usr/lnp64_intrinsics.h|toolchain/lnp64_intrinsics.h|generated",
            "crt0_object|usr/lib/lnp64/crt0.o|toolchain/crt0_lnp64.s|generated",
            "linker_script|usr/lib/lnp64/lnp64_static.ld|toolchain/lnp64_static.ld|generated",
            "legacy_minilibc_object|usr/lib/lnp64/liblnp64_min.o|toolchain/liblnp64_min.s|generated",
            "libc_shim_objects|usr/lib/lnp64/liblnp64-*.o|toolchain/liblnp64_*_min.c,toolchain/liblnp64_*_min.s|generated",
            "sysroot_static_link|target/lnp64-sysroot-smoke/sysroot-smoke.elf|scripts/package_lnp64_sysroot.sh|tested",
            "sysroot_run_elf|target/lnp64-sysroot-smoke/sysroot-smoke.elf|lnp64 run-elf|tested",
        ] {
            assert!(
                sysroot_manifest.contains(row),
                "sysroot manifest missing {row}"
            );
        }
        for script_piece in [
            "LNP64_SYSROOT_DIR:-target/lnp64-sysroot",
            "LNP64_SYSROOT_SMOKE_DIR:-target/lnp64-sysroot-smoke",
            "mkdir -p \"$sysroot/usr/include\" \"$sysroot/usr/lib/lnp64\"",
            "cp -a toolchain/include/. \"$sysroot/usr/include/\"",
            "cp -a toolchain/lnp64_intrinsics.h \"$sysroot/usr/lnp64_intrinsics.h\"",
            "toolchain/crt0_lnp64.s -o \"$sysroot/usr/lib/lnp64/crt0.o\"",
            "toolchain/liblnp64_min.s -o \"$sysroot/usr/lib/lnp64/liblnp64_min.o\"",
            "for source in toolchain/liblnp64_*_min.c",
            "for source in toolchain/liblnp64_*_min.s",
            "base=\"${base//_/-}\"",
            "-I \"$sysroot/usr/include\" -I toolchain",
            "\"$sysroot/usr/lib/lnp64/${base}-min.o\"",
            "sysroot-smoke.c",
            "\"$lld\" -flavor gnu -static -m elf64lnp64",
            "-T \"$sysroot/usr/lib/lnp64/lnp64_static.ld\"",
            "\"$sysroot/usr/lib/lnp64/crt0.o\"",
            "\"$sysroot/usr/lib/lnp64/liblnp64-fd-min.o\"",
            "\"$lnp64_bin\" elf-plan \"$smoke_elf\"",
            "\"$lnp64_bin\" run-elf \"$smoke_elf\"",
            "grep -q 'exit=0'",
            "LNP64 sysroot run-elf smoke passed",
        ] {
            assert!(
                package_script.contains(script_piece),
                "sysroot package script missing {script_piece}"
            );
        }
    }

    #[test]
    fn minilibc_smoke_stub_matches_real_llvm_gate() {
        let minilibc = include_str!("../toolchain/liblnp64_min.s");
        let real_llc = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");

        assert!(contract_index.contains(
            "minilibc_smoke|toolchain/liblnp64_min.s|minilibc_smoke_stub_matches_real_llvm_gate"
        ));
        for required in [
            ".globl write",
            "write:",
            "push r2, r2, r3, r4",
            ".globl read",
            "read:",
            "pull r2, r2, r3, r4",
            ".globl alloc",
            "alloc:",
            "alloc r2, r2",
            ".globl malloc",
            "malloc:",
            "jal r1, alloc",
            ".globl calloc",
            "calloc:",
            "jal r1, memset",
            ".globl realloc",
            "realloc:",
            "alloc_size r4, r3",
            "jal r1, memcpy",
            ".globl free",
            "free:",
            "free r2",
            "li r2, 0",
            ".globl strlen",
            "strlen:",
            ".globl memcpy",
            "memcpy:",
            ".globl memmove",
            "memmove:",
            "memmove_backward_loop:",
            "jal r1, memcpy",
            ".globl memcmp",
            "memcmp:",
            "memcmp_diff:",
            ".globl memset",
            "memset:",
            ".globl _exit",
            "_exit:",
            "exit r2",
            ".globl exit",
            "exit:",
            "__lnp64_min_realloc_old:",
            "__lnp64_min_realloc_size:",
            "__lnp64_min_realloc_new:",
        ] {
            assert!(minilibc.contains(required), "minilibc missing {required}");
        }
        assert!(real_llc.contains("toolchain/liblnp64_min.s"));
        assert!(real_llc.contains("liblnp64-min-smoke.o"));
        assert!(real_llc.contains("lnp64-$demo-clang-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld clang demo link smoke passed"));
        assert!(roadmap.contains("toolchain/liblnp64_min.s"));
    }

    #[test]
    fn toolchain_transition_manifest_records_layered_deliverables() {
        let manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let psabi = include_str!("../psABI.md");
        let conformance = include_str!("../conformance_matrix.md");
        let libc = include_str!("../libc_roadmap.md");
        let object_format = include_str!("../object_format.md");
        let run_elf_manifest = include_str!("../toolchain/lnp64_run_elf.manifest");
        let llvm_gates_manifest = include_str!("../toolchain/lnp64_llvm_gates.manifest");
        let llvm_bootstrap_manifest = include_str!("../toolchain/lnp64_llvm_bootstrap.manifest");
        let rows = transition_rows(manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut phases = std::collections::BTreeMap::new();

        for (phase, status, artifacts, gate) in rows {
            assert!(
                phases
                    .insert(phase, (status, artifacts.clone(), gate))
                    .is_none(),
                "duplicate transition phase {phase}"
            );
            assert!(
                ["required", "partial", "planned"].contains(&status),
                "unknown transition status {status} for {phase}"
            );
            assert!(!artifacts.is_empty(), "empty artifacts for {phase}");
            assert!(!gate.is_empty(), "empty gate for {phase}");
            for artifact in artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "transition phase {phase} names missing artifact {artifact}"
                );
            }
        }

        for phase in [
            "real_toolchain_target",
            "toy_compiler_retirement",
            "minimal_llvm_clang_path",
            "libc_runtime_shim",
            "software_loader_exec_plan",
            "netbsd_personality_layers",
            "conformance_gates",
        ] {
            assert!(
                phases.contains_key(phase),
                "missing transition phase {phase}"
            );
        }

        assert_eq!(phases["real_toolchain_target"].0, "required");
        for artifact in [
            "toolchain/lnp64_target.manifest",
            "toolchain/lnp64_registers.manifest",
            "toolchain/lnp64_psabi.manifest",
            "toolchain/lnp64_relocations.manifest",
            "toolchain/lnp64_mc_encoding.manifest",
            "toolchain/lnp64_inline_asm.manifest",
            "toolchain/lnp64_debug_unwind.manifest",
            "toolchain/lnp64_crt_startup.manifest",
            "toolchain/crt0_lnp64.s",
            "toolchain/lnp64_intrinsics.manifest",
            "toolchain/lnp64_intrinsic_lowering.manifest",
            "toolchain/lnp64_intrinsics.h",
            "toolchain/lnp64_isel.manifest",
            "toolchain/lnp64_exec_plan.manifest",
            "toolchain/lnp64_clang_driver.manifest",
            "toolchain/lnp64_static.ld",
            "toolchain/lnp64_run_elf.manifest",
            "psABI.md",
            "object_format.md",
        ] {
            assert!(
                phases["real_toolchain_target"].1.contains(&artifact),
                "real_toolchain_target is missing artifact {artifact}"
            );
        }

        assert!(roadmap.contains("## First Acceptance Gates"));
        assert!(roadmap.contains("## Checked Transition Deliverables"));
        assert_eq!(phases["minimal_llvm_clang_path"].0, "partial");
        assert_eq!(
            phases["minimal_llvm_clang_path"].2,
            "llvm_bootstrap_manifest_names_first_clang_gate"
        );
        for artifact in [
            "scripts/run_llvm_bootstrap_gates.sh",
            "scripts/run_real_llvm_bootstrap_smokes.sh",
            "scripts/package_lnp64_sysroot.sh",
            "toolchain/lnp64_llvm_bootstrap.manifest",
            "toolchain/lnp64_llvm_gates.manifest",
            "toolchain/lnp64_run_elf.manifest",
            "toolchain/lnp64_clang_driver.manifest",
            "toolchain/lnp64_static.ld",
            "toolchain/crt0_lnp64.s",
            "toolchain/lnp64_sysroot.manifest",
            "toolchain/liblnp64_fd_min.c",
            "toolchain/liblnp64_process_min.c",
            "toolchain/liblnp64_startup_min.c",
            "toolchain/liblnp64_random_min.c",
            "toolchain/liblnp64_stdio_min.c",
            "toolchain/liblnp64_pthread_min.c",
            "toolchain/liblnp64_sem_min.c",
            "toolchain/liblnp64_signal_min.c",
            "userland/loader_target_clang.c",
            "userland/elf_exec_test_clang.c",
            "userland/fork_wait_test_clang.c",
            "userland/thread_test_clang.c",
            "userland/poll_test_clang.c",
            "userland/signal_gate_test_clang.c",
            "userland/signal_fault_test_clang.c",
            "userland/timer_test_clang.c",
            "userland/mmap_test_clang.c",
            "userland/namespace_test_clang.c",
            "userland/socket_loopback_test_clang.c",
            "toolchain/include/poll.h",
            "toolchain/include/pthread.h",
            "toolchain/include/search.h",
            "toolchain/include/semaphore.h",
            "toolchain/include/stdarg.h",
            "toolchain/include/stddef.h",
            "toolchain/include/stdint.h",
            "toolchain/include/sys/epoll.h",
            "toolchain/include/sys/event.h",
            "toolchain/include/sys/select.h",
            "toolchain/include/unistd.h",
            "toolchain/include/lnp64/futex.h",
            "toolchain/lnp64_intrinsics.h",
            "toolchain/include/lnp64/intrinsics.h",
        ] {
            assert!(
                phases["minimal_llvm_clang_path"].1.contains(&artifact),
                "minimal_llvm_clang_path is missing artifact {artifact}"
            );
        }
        assert!(llvm_bootstrap_manifest.contains("hello"));
        assert!(llvm_bootstrap_manifest.contains("arithmetic"));
        assert!(llvm_bootstrap_manifest.contains("memory"));
        assert!(llvm_bootstrap_manifest.contains("calls"));
        assert!(llvm_gates_manifest.contains("scripts/run_real_llvm_lnp64_docker.sh"));
        assert!(llvm_gates_manifest.contains("clang_scalar_compile"));
        assert!(llvm_gates_manifest.contains("lld"));
        assert!(llvm_gates_manifest.contains("static_link_smoke"));
        assert!(llvm_gates_manifest.contains("real_clang_loader_path"));
        assert!(run_elf_manifest.contains(
            "stdout_exit|partial|scripts/run_real_llvm_lnp64_docker.sh,scripts/run_real_llvm_lnp64.sh,toolchain/liblnp64_fd_min.c,toolchain/liblnp64_process_min.c,toolchain/lnp64_llvm_gates.manifest|real_clang_stdout_exit_run_elf_smokes|needs_full_libc_runtime_packaging"
        ));
        assert!(roadmap.contains("`minimal_llvm_clang_path` transition row remains partial"));
        assert!(roadmap.contains("bootstrap manifest rows are tested"));
        assert!(roadmap.contains("smoke-only libc shim"));
        assert!(roadmap.contains("real Clang/lld and the software"));
        assert!(roadmap.contains("poll,pthread"));
        assert!(roadmap.contains("sys/epoll,sys/event,sys/mman,sys/select"));
        assert!(
            phases["minimal_llvm_clang_path"]
                .1
                .contains(&"scripts/run_real_llvm_bootstrap_smokes.sh")
        );
        assert!(roadmap.contains("scripts/run_real_llvm_bootstrap_smokes.sh"));
        assert!(roadmap.contains("all current rows in the manifest are tested"));
        assert!(psabi.contains("## Register Model"));
        assert!(psabi.contains("## Calling Convention"));
        assert!(psabi.contains("## Debug and Unwind Minimum"));
        assert!(psabi.contains("LLVM/Clang, lld, loader, and"));
        assert!(psabi.contains("libc/runtime process ABI"));
        assert!(psabi.contains("real Clang/lld path"));
        assert!(object_format.contains("## Relocation Model"));
        assert!(object_format.contains("## Exec-Plan Descriptor Boundary"));
        assert!(object_format.contains("Clang/lld-produced static ELF execution"));
        assert!(object_format.contains("bounded exec-plan path"));
        assert!(!object_format.contains("current emulator still executes LNP64 assembly directly"));
        assert!(!object_format.contains("target format for the future loader"));
        assert!(libc.contains("startup"));
        assert!(libc.contains("errno"));
        assert!(libc.contains("pthread"));
        assert!(conformance.contains("scripts/run_software_gates.sh"));
        assert!(conformance.contains("scripts/run_netbsd_personality_system.sh"));
        assert!(conformance.contains("the static software loader now parses ELF"));
        assert!(conformance.contains("Static v1 software-loader ELF loading"));
        assert!(!conformance.contains("Implement a software ELF loader"));
        assert!(
            !conformance.contains("Full ELF loading, loader-produced architectural exec plans")
        );
    }

    #[test]
    fn netbsd_layers_manifest_preserves_personality_order() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let layers_manifest = include_str!("../toolchain/lnp64_netbsd_layers.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let personality_doc = include_str!("../netbsd_personality_abi.md");
        let conformance = include_str!("../conformance_matrix.md");
        let system_gate = include_str!("../scripts/run_netbsd_personality_system.sh");
        let package_gate = include_str!("../scripts/run_real_llvm_package_gate.sh");
        let run_real_packages = include_str!("../scripts/run_real_packages.sh");
        let run_sbase = include_str!("../scripts/run_sbase.sh");
        let rows = netbsd_layer_rows(layers_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let layers_path = manifest_field(target_manifest, "netbsd_layers_contract");
        let mut seen = std::collections::BTreeSet::new();
        let mut ordered_layers = Vec::new();
        let mut statuses = std::collections::BTreeMap::new();
        let mut layer_artifacts = std::collections::BTreeMap::new();
        let mut gates = std::collections::BTreeMap::new();
        let mut blockers = std::collections::BTreeMap::new();

        assert_eq!(layers_path, "toolchain/lnp64_netbsd_layers.manifest");
        assert!(manifest_root.join(layers_path).is_file());
        assert!(contract_index.contains(
            "netbsd_layers|toolchain/lnp64_netbsd_layers.manifest|netbsd_layers_manifest_preserves_personality_order"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_netbsd_layers.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_netbsd_layers.manifest"));
        assert!(personality_doc.contains("toolchain/lnp64_netbsd_layers.manifest"));
        assert!(personality_doc.contains("No full monolithic NetBSD kernel port"));
        assert!(personality_doc.contains("ELF-to-exec-plan loading is now covered"));
        assert!(!personality_doc.contains("full ELF-to-exec-plan loader remains future work"));
        assert!(system_gate.contains("LNP64_LLVM_PACKAGE_FILTER=netbsd"));
        assert!(system_gate.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(
            system_gate.contains(
                "netbsd_system_gate_canonical_native_primitives_cover_runner_requirements"
            )
        );
        assert!(system_gate.contains("demos/stale_fd_token.s"));

        for (layer, status, artifacts, gate, next_blocker) in rows {
            assert!(seen.insert(layer), "duplicate NetBSD layer {layer}");
            ordered_layers.push(layer);
            statuses.insert(layer, status);
            layer_artifacts.insert(layer, artifacts.clone());
            gates.insert(layer, gate);
            blockers.insert(layer, next_blocker);
            assert!(
                ["bootstrap_gate", "scaffolded", "planned", "blocked"].contains(&status),
                "unknown NetBSD layer status {status} for {layer}"
            );
            assert!(
                !artifacts.is_empty(),
                "empty artifacts for NetBSD layer {layer}"
            );
            for artifact in artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "NetBSD layer {layer} names missing artifact {artifact}"
                );
            }
            if gate != "none" {
                assert!(
                    manifest_root.join(gate).exists(),
                    "NetBSD layer {layer} names missing gate {gate}"
                );
            }
            assert!(
                !next_blocker.is_empty(),
                "NetBSD layer {layer} must name its next blocker"
            );
        }

        assert_eq!(
            ordered_layers,
            vec![
                "libc_userland_pieces",
                "rump_filesystem_components",
                "rump_network_socket_personality",
                "process_signal_thread_compat",
                "personality_escape_denials",
                "larger_userland_commands",
                "fuller_machine_port",
            ],
            "NetBSD personality layers must stay in the planned bring-up order"
        );
        assert_eq!(statuses["fuller_machine_port"], "blocked");
        assert_eq!(statuses["libc_userland_pieces"], "bootstrap_gate");
        assert_eq!(statuses["process_signal_thread_compat"], "bootstrap_gate");
        assert_eq!(statuses["personality_escape_denials"], "bootstrap_gate");
        assert_eq!(statuses["rump_filesystem_components"], "scaffolded");
        assert_eq!(statuses["rump_network_socket_personality"], "scaffolded");
        assert_eq!(statuses["larger_userland_commands"], "planned");
        assert_eq!(
            gates["personality_escape_denials"], "scripts/run_netbsd_personality_system.sh",
            "NetBSD denied escape boundary should stay under the system gate"
        );
        for artifact in [
            "netbsd_personality_abi.md",
            "src/personality_lowering.rs",
            "src/lowering.rs",
            "scripts/run_netbsd_personality_system.sh",
        ] {
            assert!(
                layer_artifacts["personality_escape_denials"].contains(&artifact),
                "NetBSD denied escape row must name artifact {artifact}"
            );
        }
        assert!(
            blockers["personality_escape_denials"].contains("negative_runtime_escape_tests"),
            "NetBSD denied escape row should name the runtime negative-test blocker"
        );
        assert!(personality_doc.contains("No personality-owned page tables"));
        assert!(personality_doc.contains("raw interrupt"));
        assert!(personality_doc.contains("raw DMA"));
        assert!(personality_doc.contains("capability minting"));
        assert_eq!(
            gates["larger_userland_commands"], "scripts/run_real_packages.sh",
            "larger NetBSD userland should remain delegated to the package gate"
        );
        for artifact in [
            "scripts/run_sbase.sh",
            "scripts/run_real_packages.sh",
            "conformance_matrix.md",
        ] {
            assert!(
                layer_artifacts["larger_userland_commands"].contains(&artifact),
                "larger NetBSD userland row must name artifact {artifact}"
            );
        }
        assert_eq!(
            blockers["larger_userland_commands"], "clang_lld_static_elf_package_builds",
            "larger NetBSD userland blocker should stay tied to Clang/lld static ELF packages"
        );
        assert!(run_real_packages.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(run_sbase.contains("LNP64_LLVM_PACKAGE_FILTER=sbase"));
        assert!(run_sbase.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(package_gate.contains("lnp64-sbase-ls-linked.elf"));
        assert!(package_gate.contains("real LLVM LNP64 run-elf sbase ls execution passed"));
        assert!(
            conformance.contains("| sbase subset | partial / passing current real-Clang smokes |")
        );
        assert!(
            conformance.contains("`yes` is Clang/lld-linked and exec-plan validated but not run")
        );
        assert!(
            blockers["fuller_machine_port"].contains("not_credible_yet"),
            "fuller machine port must remain blocked on rump services/static userland credibility"
        );
        assert!(
            blockers["rump_filesystem_components"].contains("rumpfs_service"),
            "rump filesystem layer must name the real rumpfs service blocker"
        );
        assert!(
            blockers["rump_network_socket_personality"].contains("socket_service"),
            "rump socket layer must name the real socket service blocker"
        );
        for gate_evidence in [
            "lnp64-netbsd-fs-service-test-linked.elf",
            "fs_service_test ok",
            "LNPFS2",
            "lnp64-netbsd-socket-loopback-test-linked.elf",
            "socket_loopback_test ok",
        ] {
            assert!(
                package_gate.contains(gate_evidence),
                "NetBSD delegated package gate missing scaffold evidence {gate_evidence}"
            );
        }
        assert_ne!(
            statuses["larger_userland_commands"], "bootstrap_gate",
            "larger NetBSD userland must not be treated as current bootstrap coverage"
        );
    }

    #[test]
    fn real_program_ladder_manifest_orders_lua_to_redis() {
        let ladder_manifest = include_str!("../toolchain/lnp64_real_program_ladder.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let conformance = include_str!("../conformance_matrix.md");
        let netbsd_layers = include_str!("../toolchain/lnp64_netbsd_layers.manifest");
        let libc_shim = include_str!("../toolchain/lnp64_libc_shim.manifest");
        let gate_manifest = include_str!("../toolchain/lnp64_conformance_gates.manifest");
        let rows = real_program_ladder_rows(ladder_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut seen = std::collections::BTreeSet::new();
        let mut ordered_stages = Vec::new();
        let mut statuses = std::collections::BTreeMap::new();
        let mut artifacts_by_stage = std::collections::BTreeMap::new();
        let mut gates = std::collections::BTreeMap::new();
        let mut goals = std::collections::BTreeMap::new();
        let mut focus_by_stage = std::collections::BTreeMap::new();
        let mut blockers = std::collections::BTreeMap::new();

        assert!(contract_index.contains(
            "real_program_ladder|toolchain/lnp64_real_program_ladder.manifest|real_program_ladder_manifest_orders_lua_to_redis"
        ));
        assert!(conformance.contains("toolchain/lnp64_real_program_ladder.manifest"));
        assert!(conformance.contains("| Redis upstream | passing (Phase E smoke gate) |"));
        assert!(conformance.contains("`COMPAT-PKG-008`"));
        assert!(conformance.contains("NetBSD personality system gate"));
        assert!(netbsd_layers.contains("process_signal_thread_compat"));
        assert!(netbsd_layers.contains("rump_network_socket_personality"));
        assert!(libc_shim.contains("pthread"));
        assert!(gate_manifest.contains("netbsd_personality"));

        for (stage, status, artifacts, gate, goal, focus, next_blocker) in rows {
            assert!(seen.insert(stage), "duplicate real-program stage {stage}");
            ordered_stages.push(stage);
            statuses.insert(stage, status);
            artifacts_by_stage.insert(stage, artifacts.clone());
            gates.insert(stage, gate);
            goals.insert(stage, goal);
            focus_by_stage.insert(stage, focus.clone());
            blockers.insert(stage, next_blocker);
            assert!(
                ["bootstrap_gate", "planned", "blocked", "tested"].contains(&status),
                "unknown real-program ladder status {status} for {stage}"
            );
            assert!(
                !artifacts.is_empty(),
                "empty artifacts for real-program stage {stage}"
            );
            for artifact in artifacts {
                if let Some(planned) = artifact.strip_prefix("planned:") {
                    assert!(
                        !planned.is_empty(),
                        "empty planned artifact in real-program stage {stage}"
                    );
                } else {
                    assert!(
                        manifest_root.join(artifact).exists(),
                        "real-program stage {stage} names missing artifact {artifact}"
                    );
                }
            }
            if let Some(planned_gate) = gate.strip_prefix("planned:") {
                assert!(
                    !planned_gate.is_empty(),
                    "empty planned gate in real-program stage {stage}"
                );
            } else {
                assert!(
                    manifest_root.join(gate).exists(),
                    "real-program stage {stage} names missing gate {gate}"
                );
            }
            assert!(
                !goal.is_empty(),
                "empty goal for real-program stage {stage}"
            );
            assert!(
                !focus.is_empty(),
                "empty focus for real-program stage {stage}"
            );
            assert!(
                !next_blocker.is_empty(),
                "empty blocker for real-program stage {stage}"
            );
        }

        assert_eq!(
            ordered_stages,
            vec![
                "minimal_lua",
                "sqlite_memory_file",
                "netbsd_posix_personality_closure",
                "tiny_network_daemons",
                "redis_configured_build",
                "redis_single_client",
                "redis_persistence_fork",
            ],
            "real-program ladder must keep generic compatibility gates before Redis"
        );
        assert_eq!(statuses["minimal_lua"], "tested");
        assert_eq!(statuses["sqlite_memory_file"], "bootstrap_gate");
        assert_eq!(
            statuses["netbsd_posix_personality_closure"],
            "bootstrap_gate"
        );
        assert_eq!(statuses["tiny_network_daemons"], "bootstrap_gate");
        assert_eq!(statuses["redis_configured_build"], "blocked");
        assert_eq!(statuses["redis_single_client"], "blocked");
        assert_eq!(statuses["redis_persistence_fork"], "blocked");
        assert_eq!(
            gates["netbsd_posix_personality_closure"],
            "scripts/run_netbsd_personality_system.sh"
        );
        assert_eq!(
            gates["redis_configured_build"],
            "planned:scripts/run_redis.sh"
        );

        for focus in [
            "real_c_stack_calls",
            "setjmp_longjmp",
            "varargs_formatting",
            "malloc_realloc_free",
            "basic_stdio_write",
        ] {
            assert!(
                focus_by_stage["minimal_lua"].contains(&focus),
                "minimal Lua rung must keep focus item {focus}"
            );
        }
        assert!(goals["minimal_lua"].contains("lua -e print_1_plus_2"));
        assert!(goals["sqlite_memory_file"].contains("in-memory database"));
        assert!(goals["sqlite_memory_file"].contains("file-backed"));
        assert!(focus_by_stage["sqlite_memory_file"].contains(&"file_semantics"));
        assert!(focus_by_stage["sqlite_memory_file"].contains(&"durability_api"));
        for artifact in [
            "toolchain/lnp64_netbsd_layers.manifest",
            "netbsd_personality_abi.md",
            "scripts/run_netbsd_personality_system.sh",
            "toolchain/lnp64_libc_shim.manifest",
        ] {
            assert!(
                artifacts_by_stage["netbsd_posix_personality_closure"].contains(&artifact),
                "NetBSD/POSIX closure stage must name artifact {artifact}"
            );
        }
        for focus in [
            "libpthread_libm_runtime",
            "files_sockets_signals_timers",
            "fork_waitpid_SIGCHLD",
            "errno_fidelity",
        ] {
            assert!(
                focus_by_stage["netbsd_posix_personality_closure"].contains(&focus),
                "NetBSD/POSIX closure stage must keep focus item {focus}"
            );
        }
        assert!(
            blockers["netbsd_posix_personality_closure"].contains("close_personality_gaps"),
            "NetBSD/POSIX closure must be the generic compatibility investment"
        );
        for focus in [
            "socket_bind_listen_accept_connect",
            "poll_select_epoll_kqueue",
            "O_NONBLOCK",
            "EINTR_EAGAIN_ECONNRESET",
            "signal_shutdown",
        ] {
            assert!(
                focus_by_stage["tiny_network_daemons"].contains(&focus),
                "network-daemon rung must keep focus item {focus}"
            );
        }
        assert!(goals["redis_configured_build"].contains("unmodified"));
        for focus in [
            "static_build",
            "no_tls",
            "no_modules",
            "minimal_persistence",
        ] {
            assert!(
                focus_by_stage["redis_configured_build"].contains(&focus),
                "Redis configured build must keep strict profile item {focus}"
            );
        }
        assert!(goals["redis_single_client"].contains("PING SET GET DEL"));
        for focus in [
            "file_durability",
            "rename_tempfile",
            "background_save",
            "fork_waitpid_SIGCHLD",
            "persistence_reload",
        ] {
            assert!(
                focus_by_stage["redis_persistence_fork"].contains(&focus),
                "Redis persistence rung must keep focus item {focus}"
            );
        }
        assert!(
            goals["redis_persistence_fork"].contains("temp rename fsync child handling"),
            "Redis persistence rung must name the hard fork/persistence semantics"
        );
    }

    #[test]
    fn conformance_gate_manifest_covers_required_layers() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let gate_manifest = include_str!("../toolchain/lnp64_conformance_gates.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let run_all = include_str!("../scripts/run_all_gates.sh");
        let run_software = include_str!("../scripts/run_software_gates.sh");
        let run_real_packages = include_str!("../scripts/run_real_packages.sh");
        let run_real_package_gate = include_str!("../scripts/run_real_llvm_package_gate.sh");
        let run_demos = include_str!("../scripts/run_demos.sh");
        let run_userland = include_str!("../scripts/run_userland.sh");
        let run_netbsd_smoke = include_str!("../scripts/run_netbsd_personality_smoke.sh");
        let run_netbsd_system = include_str!("../scripts/run_netbsd_personality_system.sh");
        let run_elf_manifest = include_str!("../toolchain/lnp64_run_elf.manifest");
        let emulator_source = include_str!("emulator.rs");
        let loader_source = include_str!("loader.rs");
        let asm_source = include_str!("asm.rs");
        let rows = conformance_gate_rows(gate_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let gate_path = manifest_field(target_manifest, "conformance_gate_contract");
        let mut categories = std::collections::BTreeMap::new();

        assert_eq!(gate_path, "toolchain/lnp64_conformance_gates.manifest");
        assert!(manifest_root.join(gate_path).is_file());
        assert!(contract_index.contains(
            "conformance_gates|toolchain/lnp64_conformance_gates.manifest|conformance_gate_manifest_covers_required_layers"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_conformance_gates.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_conformance_gates.manifest"));
        assert!(conformance.contains("toolchain/lnp64_conformance_gates.manifest"));

        for (category, status, artifacts, gate, coverage) in rows {
            assert!(
                categories
                    .insert(category, (status, artifacts.clone(), gate, coverage))
                    .is_none(),
                "duplicate conformance gate category {category}"
            );
            assert!(
                ["tested", "partial", "planned"].contains(&status),
                "unknown conformance gate status {status} for {category}"
            );
            assert!(
                !artifacts.is_empty(),
                "empty artifacts for conformance gate {category}"
            );
            for artifact in artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "conformance gate {category} names missing artifact {artifact}"
                );
            }
            assert!(
                !gate.is_empty(),
                "empty gate for conformance category {category}"
            );
            assert!(
                gate.starts_with("cargo test")
                    || gate == "simple_libc_gate"
                    || manifest_root.join(gate).exists(),
                "conformance gate {category} names missing gate {gate}"
            );
            assert!(
                !coverage.is_empty(),
                "empty coverage note for conformance category {category}"
            );
        }

        for category in [
            "asm_demos",
            "c_tests",
            "randomized_emulator",
            "adversarial_fault",
            "package_tests",
            "llvm_package_tests",
            "netbsd_personality",
            "llvm_built_versions",
            "aggregate_hygiene",
        ] {
            assert!(
                categories.contains_key(category),
                "missing conformance category {category}"
            );
        }
        assert_eq!(categories["llvm_built_versions"].0, "partial");
        assert!(
            categories["llvm_built_versions"]
                .1
                .contains(&"scripts/run_llvm_bootstrap_gates.sh")
        );
        assert!(
            categories["llvm_built_versions"]
                .1
                .contains(&"scripts/run_real_llvm_bootstrap_smokes.sh")
        );
        assert!(
            categories["llvm_built_versions"]
                .1
                .contains(&"scripts/run_real_llvm_lnp64_objects_docker.sh")
        );
        assert!(
            categories["llvm_built_versions"]
                .1
                .contains(&"toolchain/lnp64_run_elf.manifest")
        );
        assert_eq!(
            categories["llvm_built_versions"].2,
            "scripts/run_real_llvm_lnp64_docker.sh"
        );
        assert!(
            categories["llvm_built_versions"]
                .3
                .contains("real_clang_lld_run_elf_versions")
        );
        assert!(
            categories["llvm_built_versions"]
                .3
                .contains("all_tested_bootstrap_rows")
        );
        assert!(
            categories["llvm_built_versions"]
                .3
                .contains("full_libc_replacement_pending")
        );
        assert!(run_elf_manifest.contains("stdout_exit|partial"));
        assert!(run_elf_manifest.contains("needs_full_libc_runtime_packaging"));
        assert!(conformance.contains("stdout/exit compatibility row remains partial"));
        assert_eq!(
            categories["llvm_package_tests"].2,
            "scripts/run_real_llvm_lnp64_docker.sh"
        );
        assert!(categories["llvm_package_tests"].3.contains("zlib"));
        assert!(categories["llvm_package_tests"].3.contains("cwalk"));
        assert!(
            categories["llvm_package_tests"]
                .3
                .contains("sbase_commands")
        );
        for category in [
            "asm_demos",
            "c_tests",
            "randomized_emulator",
            "adversarial_fault",
            "package_tests",
            "llvm_package_tests",
            "netbsd_personality",
            "aggregate_hygiene",
        ] {
            assert_eq!(
                categories[category].0, "tested",
                "{category} should be tested by current gates"
            );
        }
        assert_eq!(
            categories["randomized_emulator"].2,
            "cargo test --quiet randomized_"
        );
        assert_eq!(
            categories["randomized_emulator"].3,
            "randomized_mmap_capability_domain_stress"
        );
        for test_name in [
            "randomized_mmap_mprotect_and_guard_stress_preserves_permissions",
            "randomized_capability_delegation_stress_preserves_authority",
            "randomized_domain_lifecycle_stress_rejects_stale_handles",
        ] {
            assert!(
                emulator_source.contains(test_name),
                "randomized conformance gate filter must match {test_name}"
            );
        }
        assert_eq!(
            categories["adversarial_fault"].2,
            "cargo test --quiet rejects_"
        );
        assert_eq!(
            categories["adversarial_fault"].3,
            "rejects_malformed_faulting_authority_broadening_and_precommit_side_effects"
        );
        for test_name in [
            "rejects_raw_hardware_and_syscall_escape_opcodes",
            "static_elf_loader_rejects_wrong_machine",
            "static_elf_loader_rejects_writable_executable_loads",
            "emulator_rejects_writable_executable_exec_descriptor_vma",
            "mmap_rejects_unknown_protection_bits_without_vma_side_effects",
            "div_rejects_locked_result_before_fault_event",
        ] {
            assert!(
                [asm_source, loader_source, emulator_source]
                    .iter()
                    .any(|source| source.contains(test_name)),
                "adversarial/fault conformance gate filter must match {test_name}"
            );
        }

        assert!(run_software.contains("cargo test"));
        assert!(run_software.contains("bash scripts/run_toolchain_contracts.sh"));
        assert!(run_software.contains("bash scripts/run_llvm_bootstrap_gates.sh --dry-run"));
        assert!(run_software.contains("bash scripts/run_demos.sh"));
        assert!(run_software.contains("bash scripts/run_userland.sh"));
        assert!(run_software.contains("bash scripts/run_netbsd_personality_system.sh"));
        assert!(!run_software.contains("bash scripts/run_netbsd_personality_smoke.sh"));
        assert!(run_software.contains("bash scripts/run_real_packages.sh"));
        assert!(run_all.contains("bash scripts/run_software_gates.sh"));
        assert!(run_all.contains("git diff --check"));
        assert!(run_real_packages.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(run_real_packages.contains("real LLVM LNP64 package gate"));
        assert!(run_real_package_gate.contains("demos)"));
        assert!(run_real_package_gate.contains("lnp64-netcat-clang-linked.elf"));
        assert!(run_real_package_gate.contains("lnp64-httpd-clang-linked.elf"));
        assert!(
            run_real_package_gate.contains(
                "for selected in zlib natsort jsmn inih cwalk demos sbase userland netbsd"
            )
        );
        assert!(run_real_package_gate.contains("real LLVM LNP64 run-elf netcat self-test passed"));
        assert!(run_real_package_gate.contains("real LLVM LNP64 run-elf httpd self-test passed"));
        for sbase_elf in [
            "lnp64-sbase-yes-linked.elf",
            "lnp64-sbase-wc-linked.elf",
            "lnp64-sbase-head-linked.elf",
            "lnp64-sbase-cmp-linked.elf",
            "lnp64-sbase-cksum-linked.elf",
            "lnp64-sbase-uniq-linked.elf",
            "lnp64-sbase-tail-linked.elf",
            "lnp64-sbase-tee-linked.elf",
            "lnp64-sbase-cp-linked.elf",
            "lnp64-sbase-cut-linked.elf",
            "lnp64-sbase-tr-linked.elf",
            "lnp64-sbase-sort-linked.elf",
            "lnp64-sbase-grep-linked.elf",
            "lnp64-sbase-sed-linked.elf",
            "lnp64-sbase-ls-linked.elf",
            "lnp64-sbase-find-linked.elf",
            "lnp64-sbase-mkdir-linked.elf",
            "lnp64-sbase-ln-linked.elf",
            "lnp64-sbase-chmod-linked.elf",
            "lnp64-sbase-chown-linked.elf",
            "lnp64-sbase-touch-linked.elf",
            "lnp64-sbase-mv-linked.elf",
            "lnp64-sbase-rm-linked.elf",
        ] {
            assert!(run_real_package_gate.contains(sbase_elf));
        }
        for sbase_message in [
            "real LLVM LNP64 elf-plan sbase yes static boundary passed",
            "real LLVM LNP64 run-elf sbase wc execution passed",
            "real LLVM LNP64 run-elf sbase head execution passed",
            "real LLVM LNP64 run-elf sbase cmp execution passed",
            "real LLVM LNP64 run-elf sbase cksum execution passed",
            "real LLVM LNP64 run-elf sbase uniq execution passed",
            "real LLVM LNP64 run-elf sbase tail execution passed",
            "real LLVM LNP64 run-elf sbase tee execution passed",
            "real LLVM LNP64 run-elf sbase cp execution passed",
            "real LLVM LNP64 run-elf sbase cut execution passed",
            "real LLVM LNP64 run-elf sbase tr execution passed",
            "real LLVM LNP64 run-elf sbase sort execution passed",
            "real LLVM LNP64 run-elf sbase grep fixed-string execution passed",
            "real LLVM LNP64 run-elf sbase sed no-regex execution passed",
            "real LLVM LNP64 run-elf sbase ls execution passed",
            "real LLVM LNP64 run-elf sbase find execution passed",
            "real LLVM LNP64 run-elf sbase mkdir execution passed",
            "real LLVM LNP64 run-elf sbase ln execution passed",
            "real LLVM LNP64 run-elf sbase chmod execution passed",
            "real LLVM LNP64 run-elf sbase chown execution passed",
            "real LLVM LNP64 run-elf sbase touch execution passed",
            "real LLVM LNP64 run-elf sbase mv execution passed",
            "real LLVM LNP64 run-elf sbase rm execution passed",
        ] {
            assert!(run_real_package_gate.contains(sbase_message));
        }
        assert!(run_real_package_gate.contains("netbsd)"));
        assert!(run_real_package_gate.contains("lnp64-netbsd-init-linked.elf"));
        assert!(run_real_package_gate.contains("lnp64-netbsd-sh-linked.elf"));
        assert!(run_real_package_gate.contains("netbsd-system-fixture-root"));
        assert!(run_real_package_gate.contains("netbsd personality system ok"));
        assert!(
            run_real_package_gate
                .contains("real LLVM LNP64 run-elf NetBSD init/shell system passed")
        );
        assert!(
            run_real_package_gate.contains(
                "for selected in zlib natsort jsmn inih cwalk demos sbase userland netbsd"
            )
        );
        assert!(run_demos.contains("scripts/run_real_llvm_lnp64_docker.sh"));
        assert!(!run_demos.contains("demos/netbsd_personality_smoke.c"));
        assert!(!run_demos.contains(&["include", "legacy", "c", "frontend"].join("_")));
        assert!(run_demos.contains("for src in demos/*.s"));
        assert!(run_userland.contains("usage: scripts/run_userland.sh [--backend llvm]"));
        assert!(run_netbsd_smoke.contains("LNP64_LLVM_PACKAGE_FILTER=netbsd"));
        assert!(run_netbsd_smoke.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(
            run_netbsd_smoke
                .contains("usage: scripts/run_netbsd_personality_smoke.sh [--backend llvm]")
        );
        assert!(run_netbsd_system.contains("LNP64_LLVM_PACKAGE_FILTER=netbsd"));
        assert!(run_netbsd_system.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(
            run_netbsd_system
                .contains("usage: scripts/run_netbsd_personality_system.sh [--backend llvm]")
        );
        assert_eq!(
            categories["netbsd_personality"].3,
            "real_clang_netbsd_child_elf_gate_through_real_clang_loader"
        );
        assert_eq!(
            categories["asm_demos"].3,
            "legacy_assembler_smoke_only_C_coverage_lives_in_real_clang_lld_run_elf"
        );
        assert!(
            categories["asm_demos"]
                .1
                .contains(&"demos/waitable_probe_no_consume.s")
        );
        assert!(
            categories["asm_demos"]
                .1
                .contains(&"demos/object_ctl_bad_profile_no_install.s")
        );
        assert!(
            categories["asm_demos"]
                .1
                .contains(&"demos/cap_dup_narrow_no_amplify.s")
        );
        assert!(run_demos.contains("legacy-assembler smoke demos only"));
        assert!(categories["c_tests"].3.contains("default_to_real_clang"));
        for migrated_demo in [
            "demos/allocator.c",
            "demos/cat.c",
            "demos/factorial.c",
            "demos/fibonacci.c",
            "demos/hello.c",
            "demos/json_parser.c",
            "demos/netcat.c",
            "demos/httpd.c",
            "demos/parallel_hash.c",
            "demos/pcr.c",
            "demos/ping_pong.c",
            "demos/producer_consumer.c",
            "demos/rot13.c",
            "demos/sqlite_lite.c",
        ] {
            assert!(
                !run_demos.contains(migrated_demo),
                "migrated real-Clang demo {migrated_demo} must not be routed through run_demos.sh"
            );
        }
    }

    #[test]
    fn feature_readiness_ledger_tracks_owner_authority_generation_and_comparison() {
        let readiness = include_str!("../feature_readiness.md");
        let waitable_demo = include_str!("../demos/waitable_probe_no_consume.s");
        let object_ctl_demo = include_str!("../demos/object_ctl_bad_profile_no_install.s");
        let cap_dup_demo = include_str!("../demos/cap_dup_narrow_no_amplify.s");
        let pcr_demo = include_str!("../demos/pcr_readonly_no_mutate.s");
        let conformance = include_str!("../toolchain/lnp64_conformance_gates.manifest");
        let top_program_manifest = include_str!("../tests/rtl/top_level_program_manifest.json");

        for required_column in [
            "Feature slice",
            "Object touched",
            "Owner",
            "Authority",
            "Stale-use generation",
            "Spec",
            "Model",
            "RTL",
            "Test",
            "Trace",
            "Proof",
            "Differential visibility / next blocker",
        ] {
            assert!(
                readiness.contains(required_column),
                "feature readiness ledger must retain column {required_column}"
            );
        }

        for feature in [
            "waitable_probe_no_consume",
            "object_ctl_bad_profile_no_install",
            "cap_dup_narrow_no_amplify",
            "pcr_readonly_no_mutate",
            "fdr_stale_generation_rejection",
            "real_clang_loader_exec",
            "libc_runtime_shim",
            "netbsd_personality_layers",
        ] {
            assert!(
                readiness.contains(feature),
                "feature readiness ledger must track {feature}"
            );
        }

        for required_phrase in [
            "OBJECT_CTL",
            "WAITABLE_PROBE",
            "failed create must not mutate caller-visible FDR state",
            "Object/queue owner engine",
            "Object/FDR owner engine",
            "M1 capability/FDR owner engine",
            "PCR/process metadata owner engine",
            "`fd3` read endpoint and `fd4` write endpoint",
            "requested `fd7` install slot",
            "`fd1` source capability with duplicate authority",
            "Writable TP/SIGMASK selectors",
            "FDR token generation",
            "`fd7` generation must not advance",
            "`fd4` and `fd5` tokens carry generations",
            "Process/thread metadata epoch",
            "typed_transition_trace",
            "retire_trace_and_final_state",
            "same source",
            "active top-program import",
            "stdout/result",
        ] {
            assert!(
                readiness.contains(required_phrase),
                "waitable readiness row missing {required_phrase}"
            );
        }

        for demo_contract in [
            "# Object touched: pipe-profile queue created through OBJECT_CTL.",
            "# Owner: object/queue owner engine, not core-private emulator state.",
            "# Authority: fd3 read endpoint and fd4 write endpoint returned by OBJECT_CTL.",
            "# Generation: FDR tokens carry generation; stale use would be rejected by FDR checks.",
            "# Trace: OBJECT_CTL, PUSH/WRITE_FD, WAITABLE_PROBE, PULL/READ_FD are observable.",
            "# Differential: same source runs under emulator and RTL top-program smoke input.",
        ] {
            assert!(
                waitable_demo.contains(demo_contract),
                "waitable stress demo must keep feature-readiness header: {demo_contract}"
            );
        }

        for demo_contract in [
            "# Object touched: requested queue-profile FDR slot in OBJECT_CTL create.",
            "# Owner: object/FDR owner engine, not caller-owned descriptor table mutation.",
            "# Authority: current domain object+FDR authority plus requested fd7 install slot.",
            "# Generation: fd7 generation must not advance because no object is installed.",
            "# Trace: OBJECT_CTL reject, ERRNO_GET, failed READ_FD on fd7, EXIT are observable.",
            "# Differential: same source runs under emulator and RTL top-program smoke input.",
        ] {
            assert!(
                object_ctl_demo.contains(demo_contract),
                "object_ctl stress demo must keep feature-readiness header: {demo_contract}"
            );
        }

        for demo_contract in [
            "# Object touched: FDR capability metadata for duplicated fd tokens.",
            "# Owner: M1 capability/FDR owner engine, not caller-side rights arithmetic.",
            "# Authority: fd1 source capability with duplicate authority.",
            "# Generation: fd4 and fd5 tokens carry generations for accepted duplicates.",
            "# Trace: CAP_DUP accept, CAP_DUP reject, ERRNO_GET, CAP_DUP accept, EXIT are observable.",
            "# Differential: same source runs under emulator and RTL top-program smoke input.",
        ] {
            assert!(
                cap_dup_demo.contains(demo_contract),
                "cap_dup stress demo must keep feature-readiness header: {demo_contract}"
            );
        }

        for demo_contract in [
            "# Object touched: process and thread PCR metadata.",
            "# Owner: PCR/process metadata owner engine, not caller-side cached register state.",
            "# Authority: writable TP/SIGMASK selectors only; PID/TID/credential/realtime selectors are read-only.",
            "# Generation: process/thread metadata epoch must not advance for rejected read-only writes.",
            "# Trace: GET_PCR, SET_PCR accept, SET_PCR reject, ERRNO_GET, WRITE_FD, EXIT are observable.",
            "# Differential: same source runs under emulator and RTL top-program smoke input.",
        ] {
            assert!(
                pcr_demo.contains(demo_contract),
                "pcr stress demo must keep feature-readiness header: {demo_contract}"
            );
        }

        assert!(conformance.contains("demos/waitable_probe_no_consume.s"));
        assert!(conformance.contains("demos/object_ctl_bad_profile_no_install.s"));
        assert!(conformance.contains("demos/cap_dup_narrow_no_amplify.s"));
        assert!(conformance.contains("demos/pcr_readonly_no_mutate.s"));
        assert!(top_program_manifest.contains("\"trace_target\": \"typed_transition_trace\""));
        assert!(top_program_manifest.contains("demos/waitable_probe_no_consume.s"));
        assert!(top_program_manifest.contains("\"waitable_probe_no_consume\""));
        assert!(top_program_manifest.contains("demos/object_ctl_bad_profile_no_install.s"));
        assert!(top_program_manifest.contains("\"object_ctl_bad_profile_rejection\""));
        assert!(top_program_manifest.contains("demos/cap_dup_narrow_no_amplify.s"));
        assert!(top_program_manifest.contains("\"no_authority_amplification\""));
        assert!(top_program_manifest.contains("tests/rtl/programs/top_cap_dup_no_amplify.s"));
        assert!(top_program_manifest.contains("demos/pcr_readonly_no_mutate.s"));
        assert!(top_program_manifest.contains("\"pcr_readonly_no_mutate\""));
        assert!(top_program_manifest.contains("tests/rtl/programs/top_set_pcr.s"));
        assert!(top_program_manifest.contains("\"status\": \"active\""));
        assert!(
            top_program_manifest.contains("\"rtl_gate\": \"scripts/run_rtl_top_program_smoke.sh\"")
        );
        assert!(top_program_manifest.contains("tests/rtl/programs/top_waitable_probe.s"));
        assert!(top_program_manifest.contains("tests/rtl/programs/top_pipe_push_pull.s"));
    }

    #[test]
    fn toy_compiler_retirement_manifest_limits_custom_frontend_scope() {
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let retirement_manifest =
            include_str!("../toolchain/lnp64_toy_compiler_retirement.manifest");
        let conformance_gates = include_str!("../toolchain/lnp64_conformance_gates.manifest");
        let run_demos = include_str!("../scripts/run_demos.sh");
        let run_software = include_str!("../scripts/run_software_gates.sh");
        let run_real_packages = include_str!("../scripts/run_real_packages.sh");
        let run_real_llvm = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let run_real_package_gate = include_str!("../scripts/run_real_llvm_package_gate.sh");
        let run_userland = include_str!("../scripts/run_userland.sh");
        let run_netbsd = include_str!("../scripts/run_netbsd_personality_system.sh");
        let rtl_program_smoke = include_str!("../scripts/run_rtl_top_program_smoke.sh");
        let rtl_clang_smoke = include_str!("../scripts/run_rtl_top_clang_smoke.sh");
        let rtl_linked_llvm_smoke = include_str!("../scripts/run_rtl_top_linked_llvm_smoke.sh");
        let rtl_manifest_runner = include_str!("../scripts/run_rtl_top_program_manifest.sh");
        let top_manifest = include_str!("../tests/rtl/top_level_program_manifest.json");
        let main_source = include_str!("main.rs");
        let rust_sources = [
            ("src/asm.rs", include_str!("asm.rs")),
            ("src/emulator.rs", include_str!("emulator.rs")),
            ("src/isa.rs", include_str!("isa.rs")),
            ("src/loader.rs", include_str!("loader.rs")),
            ("src/lowering.rs", include_str!("lowering.rs")),
            ("src/main.rs", main_source),
            ("src/native.rs", include_str!("native.rs")),
            (
                "src/personality_lowering.rs",
                include_str!("personality_lowering.rs"),
            ),
        ];
        let forbidden_frontend_hooks = [
            ["c", "compiler"].join("_"),
            ["compile", "c"].join("-"),
            ["compile", "c"].join("_"),
            ["include", "legacy", "c", "frontend"].join("_"),
            ["run", "c"].join("-"),
        ];
        let contains_retired_hook = |source: &str, hook: &str| {
            if hook.contains('-') {
                source.contains(hook)
            } else {
                source
                    .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
                    .any(|token| token == hook)
            }
        };
        let retirement_evidence = [
            "rust_sources_expose_no",
            "c",
            "compiler_and_legacy_scripts_reject_direct_c",
        ]
        .join("_");
        let rows: Vec<_> = retirement_manifest
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .map(|line| {
                let fields: Vec<_> = line.split('|').collect();
                assert_eq!(fields.len(), 5, "bad retirement manifest row {line}");
                (fields[0], fields[1], fields[2], fields[3], fields[4])
            })
            .collect();
        let scopes: std::collections::BTreeSet<_> =
            rows.iter().map(|(scope, _, _, _, _)| *scope).collect();

        assert!(contract_index.contains(
            "toy_compiler_retirement|toolchain/lnp64_toy_compiler_retirement.manifest|toy_compiler_retirement_manifest_limits_custom_frontend_scope"
        ));
        assert!(transition_manifest.contains(
            "toy_compiler_retirement|required|toolchain/lnp64_toy_compiler_retirement.manifest"
        ));
        assert!(scopes.contains("legacy_assembly_smokes"));
        assert!(scopes.contains("rtl_flat_exec_smokes"));
        assert!(scopes.contains("software_package_gates"));
        assert!(scopes.contains("custom_c_frontend"));
        for (_, status, artifacts, forbidden, evidence) in &rows {
            assert!(
                *status == "allowed_smoke_generator"
                    || *status == "real_toolchain_required"
                    || *status == "removed",
                "unknown retirement manifest status {status}"
            );
            assert!(!artifacts.is_empty());
            assert!(!forbidden.is_empty());
            assert!(!evidence.is_empty());
        }

        assert!(run_demos.contains("legacy-assembler smoke demos only"));
        assert!(run_demos.contains("scripts/run_real_llvm_lnp64_docker.sh"));
        assert!(run_demos.contains("for src in demos/*.s"));
        assert!(!run_demos.contains("for src in demos/*.c"));
        assert!(!run_demos.contains(&["include", "legacy", "c", "frontend"].join("_")));
        for (path, source) in rust_sources {
            for forbidden in &forbidden_frontend_hooks {
                assert!(
                    !contains_retired_hook(source, forbidden),
                    "{path} must not expose retired C frontend hook {forbidden}"
                );
            }
        }
        assert!(retirement_manifest.contains("custom_c_frontend|removed|none"));
        assert!(retirement_manifest.contains(&retirement_evidence));
        assert!(
            conformance_gates
                .contains("legacy_assembler_smoke_only_C_coverage_lives_in_real_clang_lld_run_elf")
        );

        assert!(
            rtl_program_smoke
                .contains("direct .c input to run_rtl_top_program_smoke.sh is retired")
        );
        let rtl_direct_c_rejection = rtl_program_smoke
            .find("direct .c input to run_rtl_top_program_smoke.sh is retired")
            .expect("missing direct C rejection in RTL program smoke");
        let rtl_verilator_probe = rtl_program_smoke
            .find("command -v verilator")
            .expect("missing Verilator probe in RTL program smoke");
        let rtl_filelist_read = rtl_program_smoke
            .find("mapfile -t rtl_files")
            .expect("missing RTL filelist read in RTL program smoke");
        assert!(
            rtl_direct_c_rejection < rtl_verilator_probe,
            "direct C input must be rejected before requiring Verilator"
        );
        assert!(
            rtl_direct_c_rejection < rtl_filelist_read,
            "direct C input must be rejected before reading RTL build inputs"
        );
        let rtl_manifest_no_selection = rtl_manifest_runner
            .find("no active top-level RTL programs selected")
            .expect("missing empty-selection rejection in RTL manifest runner");
        let rtl_manifest_direct_c_rejection = rtl_manifest_runner
            .find("direct .c input to run_rtl_top_program_manifest.sh is retired")
            .expect("missing direct C rejection in RTL manifest runner");
        let rtl_manifest_cargo_build = rtl_manifest_runner
            .find("cargo build --quiet")
            .expect("missing cargo build in RTL manifest runner");
        assert!(
            rtl_manifest_no_selection < rtl_manifest_cargo_build,
            "empty manifest selections must fail before building the Rust binary"
        );
        assert!(
            rtl_manifest_direct_c_rejection < rtl_manifest_cargo_build,
            "retired direct C manifest inputs must fail before building the Rust binary"
        );
        assert!(rtl_program_smoke.contains("scripts/run_rtl_top_clang_smoke.sh"));
        assert!(rtl_program_smoke.contains("scripts/run_rtl_top_linked_llvm_smoke.sh"));
        assert!(rtl_program_smoke.contains("asm-flat-exec"));
        assert!(rtl_program_smoke.contains("run-flat-exec"));
        assert!(rtl_clang_smoke.contains("clang"));
        assert!(rtl_clang_smoke.contains("--target=lnp64-unknown-none"));
        assert!(rtl_linked_llvm_smoke.contains("\"$lld\""));
        assert!(rtl_linked_llvm_smoke.contains("elf-flat-exec"));
        assert!(rtl_manifest_runner.contains("\"llvm_clang_programs\""));
        assert!(rtl_manifest_runner.contains("\"llvm_linked_programs\""));
        assert!(top_manifest.contains("\"llvm_clang_programs\""));
        assert!(top_manifest.contains("\"llvm_linked_programs\""));

        for gate in [
            "bash scripts/run_demos.sh",
            "bash scripts/run_userland.sh",
            "bash scripts/run_netbsd_personality_system.sh",
            "bash scripts/run_real_packages.sh",
        ] {
            assert!(
                run_software.contains(gate),
                "software gate must invoke {gate}"
            );
        }
        for (name, script) in [
            ("run_real_packages", run_real_packages),
            ("run_userland", run_userland),
            ("run_netbsd", run_netbsd),
        ] {
            assert!(
                !script.contains("scripts/run_demos.sh"),
                "{name} must not route through legacy demo smokes"
            );
            assert!(
                !script.contains("lnp64 run "),
                "{name} must not use legacy assembler execution for C/package/userland coverage"
            );
        }
        assert!(run_real_packages.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(run_real_llvm.contains(r#""$clang" --target=lnp64-unknown-none"#));
        assert!(run_real_package_gate.contains("run-elf --namespace-root"));
        assert!(run_userland.contains("LNP64_LLVM_PACKAGE_FILTER=userland"));
        assert!(run_netbsd.contains("LNP64_LLVM_PACKAGE_FILTER=netbsd"));
    }

    #[test]
    fn c_coverage_stays_on_real_clang_lld_surfaces() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let readme = include_str!("../README.md");
        let llvm_bootstrap = include_str!("../toolchain/lnp64_llvm_bootstrap.manifest");
        let run_elf = include_str!("../toolchain/lnp64_run_elf.manifest");
        let libc_test_readme = include_str!("../third_party/libc-test/README.lnp64.md");
        let intrinsics = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let intrinsic_header = include_str!("../toolchain/lnp64_intrinsics.h");
        let crt0 = include_str!("../toolchain/crt0_lnp64.s");
        let main_source = include_str!("main.rs");
        let rtl_top_manifest_checker =
            include_str!("../scripts/check_rtl_top_level_program_manifest.py");
        let clang_surface_scripts = [
            (
                "scripts/run_cwalk.sh",
                include_str!("../scripts/run_cwalk.sh"),
            ),
            (
                "scripts/run_demos.sh",
                include_str!("../scripts/run_demos.sh"),
            ),
            (
                "scripts/run_inih.sh",
                include_str!("../scripts/run_inih.sh"),
            ),
            (
                "scripts/run_jsmn.sh",
                include_str!("../scripts/run_jsmn.sh"),
            ),
            (
                "scripts/run_libc_test.sh",
                include_str!("../scripts/run_libc_test.sh"),
            ),
            (
                "scripts/run_natsort.sh",
                include_str!("../scripts/run_natsort.sh"),
            ),
            (
                "scripts/run_netbsd_personality_smoke.sh",
                include_str!("../scripts/run_netbsd_personality_smoke.sh"),
            ),
            (
                "scripts/run_netbsd_personality_system.sh",
                include_str!("../scripts/run_netbsd_personality_system.sh"),
            ),
            (
                "scripts/run_rtl_top_program_smoke.sh",
                include_str!("../scripts/run_rtl_top_program_smoke.sh"),
            ),
            (
                "scripts/run_sbase.sh",
                include_str!("../scripts/run_sbase.sh"),
            ),
            (
                "scripts/run_userland.sh",
                include_str!("../scripts/run_userland.sh"),
            ),
            (
                "scripts/run_zlib.sh",
                include_str!("../scripts/run_zlib.sh"),
            ),
        ];
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let clang_surface_script_corpus = clang_surface_scripts
            .iter()
            .map(|(_, script)| *script)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(roadmap.contains("real LLVM/Clang/lld based LNP64 toolchain"));
        assert!(readme.contains("C coverage belongs on the real LLVM/Clang/lld toolchain"));
        assert!(run_elf.contains("real_libc_test_pthread_tsd_execution"));
        assert!(run_elf.contains("real_libc_test_sem_init_execution"));
        assert!(run_elf.contains("real_libc_test_access_bounded_execution"));
        assert!(run_elf.contains("real_libc_test_fcntl_basic_bounded_execution"));
        assert!(run_elf.contains("real_libc_test_fcntl_execution"));
        assert!(libc_test_readme.contains("`fcntl.c` is the upstream file"));
        assert!(libc_test_readme.contains("owner reporting across `fork`"));
        for intrinsic in manifest_field(target_manifest, "intrinsics").split(',') {
            assert!(intrinsic.starts_with("__lnp_"));
            assert!(intrinsics.contains(intrinsic));
            assert!(intrinsic_header.contains(intrinsic));
        }
        assert!(!main_source.contains("\"cc\""));
        assert!(!main_source.contains(&["c", "compiler"].join("_")));
        assert!(crt0.contains("real LLVM/lld crt0 object"));
        for (script_name, script) in clang_surface_scripts {
            for (idx, line) in script.lines().enumerate() {
                if line.contains(" cc ") || line.contains(" -- cc ") {
                    panic!(
                        "{script_name}:{} must route C coverage through the real LLVM scripts: {line}",
                        idx + 1
                    );
                }
            }
        }
        for (script_name, script) in clang_surface_scripts {
            if matches!(
                script_name,
                "scripts/run_cwalk.sh"
                    | "scripts/run_inih.sh"
                    | "scripts/run_jsmn.sh"
                    | "scripts/run_natsort.sh"
                    | "scripts/run_sbase.sh"
                    | "scripts/run_zlib.sh"
            ) {
                assert!(
                    script.contains("scripts/run_real_llvm_package_gate.sh"),
                    "{script_name} should route package coverage through real LLVM"
                );
                let package_name = script_name
                    .strip_prefix("scripts/run_")
                    .and_then(|name| name.strip_suffix(".sh"))
                    .expect("legacy package script name shape");
                assert!(
                    script.contains(&format!("LNP64_LLVM_PACKAGE_FILTER={package_name}")),
                    "{script_name} should run only its own package subset"
                );
            }
        }
        assert!(
            clang_surface_script_corpus
                .contains("direct .c input to run_rtl_top_program_smoke.sh is retired")
        );
        assert!(
            clang_surface_script_corpus
                .contains("scripts/run_rtl_top_linked_llvm_smoke.sh for C inputs")
        );
        assert!(!readme.contains("run_rtl_top_program_smoke.sh demos/hello.c"));
        assert!(!readme.contains("run_rtl_top_program_smoke.sh demos/factorial.c"));
        assert!(!readme.contains("run_rtl_top_program_smoke.sh demos/allocator.c"));
        assert!(!readme.contains("run_rtl_top_program_smoke.sh demos/ping_pong.c"));
        assert!(readme.contains(
            "run_rtl_top_linked_llvm_smoke.sh tests/rtl/programs/top_linked_loop_branch.c"
        ));
        assert!(readme.contains(
            "run_rtl_top_linked_llvm_smoke.sh tests/rtl/programs/top_linked_clone_join.c"
        ));
        assert!(clang_surface_script_corpus.contains("LNP64_LLVM_PACKAGE_FILTER=userland"));
        assert!(clang_surface_script_corpus.contains("LNP64_LLVM_PACKAGE_FILTER=netbsd"));
        assert!(clang_surface_script_corpus.contains("scripts/run_real_llvm_package_gate.sh"));
        for clang_userland in [
            "userland/classifier_test_clang.c",
            "userland/domain_budget_test_clang.c",
            "userland/domain_nested_test_clang.c",
            "userland/fd_passing_test_clang.c",
            "userland/fs_service_test_clang.c",
            "userland/gate_trace_test_clang.c",
            "userland/loader_target_clang.c",
            "userland/mmap_test_clang.c",
            "userland/namespace_test_clang.c",
            "userland/netbsd_init_clang.c",
            "userland/netbsd_sh_clang.c",
            "userland/poll_test_clang.c",
            "userland/signal_fault_test_clang.c",
            "userland/signal_gate_test_clang.c",
            "userland/socket_loopback_test_clang.c",
            "userland/thread_test_clang.c",
            "userland/timer_test_clang.c",
        ] {
            assert!(
                manifest_root.join(clang_userland).is_file(),
                "real-Clang replacement fixture is missing: {clang_userland}"
            );
        }
        assert!(rtl_top_manifest_checker.contains("toolchain/lnp64_llvm_bootstrap.manifest"));
        assert!(!rtl_top_manifest_checker.contains("RUN_DEMOS"));
        assert!(!rtl_top_manifest_checker.contains("non_network"));
        for case in [
            "hello",
            "arithmetic",
            "memory",
            "calls",
            "pcr",
            "cat",
            "json_parser",
            "rot13",
            "producer_consumer",
            "parallel_hash",
            "sqlite_lite",
            "ping_pong",
            "netcat",
            "httpd",
            "userland_ucat",
            "userland_init",
            "userland_lnpsh",
            "userland_spawn_task",
            "netbsd_init_root",
            "netbsd_shell_root",
            "netbsd_loader_target_child",
            "netbsd_fork_wait_child",
            "netbsd_thread_child",
            "netbsd_poll_child",
            "netbsd_signal_gate_child",
            "netbsd_signal_fault_child",
            "netbsd_timer_child",
            "netbsd_mmap_child",
            "netbsd_fd_passing_child",
            "netbsd_namespace_child",
            "netbsd_fs_service_child",
            "netbsd_classifier_child",
            "netbsd_socket_loopback_child",
            "netbsd_gate_trace_child",
            "netbsd_domain_nested_child",
            "netbsd_domain_budget_child",
            "netbsd_personality_clang",
            "simple_libc",
        ] {
            assert!(
                llvm_bootstrap.contains(case),
                "replacement program set missing {case}"
            );
        }
    }

    #[test]
    fn rtl_c_top_level_smokes_have_direct_linked_llvm_coverage() {
        let manifest = include_str!("../tests/rtl/top_level_program_manifest.json");
        let linked_gate = "\"rtl_gate\": \"scripts/run_rtl_top_linked_llvm_smoke.sh\"";

        let entry_for = |source: &str| {
            let source_marker = format!("\"source\": \"{source}\"");
            let source_idx = manifest
                .find(&source_marker)
                .unwrap_or_else(|| panic!("missing RTL top-level manifest source {source}"));
            let entry_start = manifest[..source_idx]
                .rfind("    {")
                .unwrap_or_else(|| panic!("missing manifest entry start for {source}"));
            let entry_end = manifest[source_idx..]
                .find("\n    }")
                .map(|offset| source_idx + offset)
                .unwrap_or_else(|| panic!("missing manifest entry end for {source}"));
            &manifest[entry_start..entry_end]
        };

        assert!(!manifest.contains("generated_assembly"));
        assert!(!manifest.contains("\"status\": \"replaced_by_llvm\""));

        for (linked_source, feature) in [
            ("tests/rtl/programs/top_linked_main.c", "startup_call_main"),
            ("tests/rtl/programs/top_linked_loop_branch.c", "branch"),
            ("tests/rtl/programs/top_linked_loop_branch.c", "call_return"),
            (
                "tests/rtl/programs/top_linked_bitwise_shift.c",
                "bitwise_alu",
            ),
            ("tests/rtl/programs/top_linked_bitwise_shift.c", "shift_alu"),
            ("tests/rtl/programs/top_linked_factorial_mul.c", "mul"),
            (
                "tests/rtl/programs/top_linked_factorial_native.c",
                "push_pull",
            ),
            (
                "tests/rtl/programs/top_linked_fibonacci_native.c",
                "call_return",
            ),
            (
                "tests/rtl/programs/top_linked_divrem.c",
                "unsigned_division",
            ),
            ("tests/rtl/programs/top_linked_divrem.c", "signed_division"),
            (
                "tests/rtl/programs/top_linked_byte_array.c",
                "byte_load_store",
            ),
            ("tests/rtl/programs/top_linked_heap_byte_lanes.c", "heap"),
            ("tests/rtl/programs/top_linked_allocator_native.c", "heap"),
            ("tests/rtl/programs/top_linked_allocator_native.c", "free"),
            ("tests/rtl/programs/top_linked_json_parser_native.c", "heap"),
            ("tests/rtl/programs/top_linked_json_parser_native.c", "free"),
            ("tests/rtl/programs/top_linked_clone_join.c", "thread_join"),
            ("tests/rtl/programs/top_linked_hello_native.c", "push_pull"),
            ("tests/rtl/programs/top_linked_rot13_native.c", "push_pull"),
            ("tests/rtl/programs/top_linked_rot13_native.c", "free"),
        ] {
            let linked_entry = entry_for(linked_source);
            assert!(
                linked_entry.contains(linked_gate),
                "{linked_source} must use the linked LLVM RTL smoke gate"
            );
            assert!(
                linked_entry.contains("\"status\": \"active\""),
                "{linked_source} should be active replacement coverage"
            );
            assert!(
                linked_entry.contains(feature),
                "{linked_source} must advertise replacement feature {feature}"
            );
        }
    }

    #[test]
    fn llvm_target_manifest_records_required_backend_contract() {
        let manifest = include_str!("../toolchain/lnp64_target.manifest");
        let object_format = include_str!("../object_format.md");
        let psabi_doc = include_str!("../psABI.md");
        let roadmap = include_str!("../toolchain_roadmap.md");
        assert_eq!(manifest_field(manifest, "triple"), "lnp64-unknown-none");
        assert_eq!(manifest_field(manifest, "object_format"), "ELF64");
        assert_eq!(manifest_field(manifest, "endianness"), "little");
        assert_eq!(manifest_field(manifest, "data_model"), "LP64");
        assert_eq!(manifest_field(manifest, "pointer_width"), "64");
        assert_eq!(manifest_field(manifest, "e_machine"), "0x6c64");
        assert_eq!(manifest_field(manifest, "psabi"), "psABI.md");
        assert_eq!(
            manifest_field(manifest, "psabi_contract"),
            "toolchain/lnp64_psabi.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "register_contract"),
            "toolchain/lnp64_registers.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "object_contract"),
            "object_format.md"
        );
        assert_eq!(
            manifest_field(manifest, "relocation_contract"),
            "toolchain/lnp64_relocations.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "mc_encoding_contract"),
            "toolchain/lnp64_mc_encoding.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "intrinsic_contract"),
            "toolchain/lnp64_intrinsics.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "intrinsic_lowering_contract"),
            "toolchain/lnp64_intrinsic_lowering.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "intrinsic_header_contract"),
            "toolchain/lnp64_intrinsics.h"
        );
        assert_eq!(
            manifest_field(manifest, "target_intrinsic_header_contract"),
            "toolchain/include/lnp64/intrinsics.h"
        );
        assert_eq!(
            manifest_field(manifest, "clang_driver_contract"),
            "toolchain/lnp64_clang_driver.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "llvm_filemap_contract"),
            "toolchain/lnp64_llvm_filemap.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "libc_shim_contract"),
            "toolchain/lnp64_libc_shim.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "netbsd_layers_contract"),
            "toolchain/lnp64_netbsd_layers.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "conformance_gate_contract"),
            "toolchain/lnp64_conformance_gates.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "isel_contract"),
            "toolchain/lnp64_isel.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "exec_plan_contract"),
            "toolchain/lnp64_exec_plan.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "loader_security_contract"),
            "toolchain/lnp64_loader_security.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "debug_unwind_contract"),
            "toolchain/lnp64_debug_unwind.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "inline_asm_contract"),
            "toolchain/lnp64_inline_asm.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "crt_startup_contract"),
            "toolchain/lnp64_crt_startup.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "crt0_contract"),
            "toolchain/crt0_lnp64.s"
        );
        assert_eq!(
            manifest_field(manifest, "sysroot_contract"),
            "toolchain/lnp64_sysroot.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "llvm_gate_contract"),
            "toolchain/lnp64_llvm_gates.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "run_elf_contract"),
            "toolchain/lnp64_run_elf.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "linker_script_contract"),
            "toolchain/lnp64_static.ld"
        );
        assert_eq!(manifest_field(manifest, "gpr"), "r0-r31");
        assert_eq!(manifest_field(manifest, "fdr"), "fd0-fd255");
        assert_eq!(manifest_field(manifest, "fpr"), "f0-f31");
        assert_eq!(manifest_field(manifest, "vr"), "v0-v15");
        for pcr in [
            "PID",
            "PPID",
            "TID",
            "TP",
            "UID",
            "GID",
            "SIGMASK",
            "SIGPENDING",
            "REALTIME_SEC",
            "REALTIME_NSEC",
            "CRED_PROFILE",
            "CRED_HANDLE",
        ] {
            assert!(manifest_csv_contains(manifest, "pcr", pcr), "missing {pcr}");
        }
        assert!(manifest_csv_contains(
            manifest,
            "native_primitives",
            "CLONE"
        ));
        assert!(manifest_csv_contains(
            manifest,
            "native_primitives",
            "THREAD_JOIN"
        ));
        for profile in [
            "new_process_cow",
            "new_thread_shared_vm",
            "spawn_entry",
            "domain_task",
        ] {
            assert!(
                manifest_csv_contains(manifest, "clone_profiles", profile),
                "missing clone profile {profile}"
            );
            assert!(
                psabi_doc.contains(profile),
                "psABI.md is missing clone profile {profile}"
            );
        }
        for relocation in [
            "R_LNP64_NONE",
            "R_LNP64_ABS64",
            "R_LNP64_ABS32",
            "R_LNP64_PC32",
            "R_LNP64_GOT64",
            "R_LNP64_GLOB_DAT",
            "R_LNP64_RELATIVE",
            "R_LNP64_TLS_TPREL64",
            "R_LNP64_TLS_DTPREL64",
            "R_LNP64_FDR_DESC64",
            "R_LNP64_CAP_DESC64",
            "R_LNP64_CALLGATE64",
            "R_LNP64_TLS_TPREL_SLOT64",
            "R_LNP64_AUIPC",
            "R_LNP64_BRANCH",
            "R_LNP64_JUMP",
        ] {
            assert!(
                manifest_csv_contains(manifest, "relocations", relocation),
                "missing {relocation}"
            );
        }
        for relocation in manifest_field(manifest, "relocations").split(',') {
            assert!(
                object_format.contains(&format!("`{relocation}`")),
                "manifest relocation {relocation} is missing from object_format.md"
            );
        }
        for intrinsic in [
            "__lnp_openat",
            "__lnp_pull",
            "__lnp_push",
            "__lnp_mmap",
            "__lnp_await",
            "__lnp_gate_call",
            "__lnp_call",
            "__lnp_gate_return",
            "__lnp_domain_ctl",
            "__lnp_domain_create",
            "__lnp_object_ctl",
            "__lnp_object_create",
            "__lnp_call_gate_create",
            "__lnp_cap_dup",
            "__lnp_cap_send",
            "__lnp_cap_recv",
            "__lnp_cap_revoke",
            "__lnp_alloc",
            "__lnp_alloc_ex",
            "__lnp_alloc_size",
            "__lnp_free",
            "__lnp_get_pid",
            "__lnp_spawn_entry",
            "__lnp_thread_join",
            "__lnp_yield",
            "__lnp_mmap_bootstrap",
            "__lnp_munmap_bootstrap",
            "__lnp_mprotect_bootstrap",
            "__lnp_exit",
        ] {
            assert!(
                manifest_csv_contains(manifest, "intrinsics", intrinsic),
                "missing {intrinsic}"
            );
        }
        assert!(roadmap.contains("`CLONE` is a backend-visible native primitive"));
        assert!(roadmap.contains("new_thread_shared_vm"));
        assert!(psabi_doc.contains("## Native Clone Profiles"));
        assert!(roadmap.contains("C and package coverage now belongs"));
        assert!(roadmap.contains("real Clang/lld"));
    }

    #[test]
    fn intrinsic_manifest_matches_target_manifest() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let intrinsic_manifest = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let rows = intrinsic_rows(intrinsic_manifest);
        let mut names = std::collections::BTreeSet::new();
        let target_intrinsics: std::collections::BTreeSet<_> =
            manifest_field(target_manifest, "intrinsics")
                .split(',')
                .collect();
        let target_primitives: std::collections::BTreeSet<_> =
            manifest_field(target_manifest, "native_primitives")
                .split(',')
                .collect();

        assert_eq!(
            manifest_field(target_manifest, "intrinsic_contract"),
            "toolchain/lnp64_intrinsics.manifest"
        );
        assert_eq!(rows.len(), target_intrinsics.len());
        for (name, primitive, result, operands) in rows {
            assert!(
                name.starts_with("__lnp_"),
                "intrinsic {name} must stay in the private LNP namespace"
            );
            assert!(names.insert(name), "duplicate intrinsic {name}");
            assert!(
                target_intrinsics.contains(name),
                "intrinsic manifest names {name}, but target manifest does not"
            );
            assert!(
                target_primitives.contains(primitive),
                "intrinsic {name} maps to unknown primitive {primitive}"
            );
            assert!(!result.is_empty(), "intrinsic {name} has empty result");
            assert!(!operands.is_empty(), "intrinsic {name} has empty operands");
        }
        for name in target_intrinsics {
            assert!(
                names.contains(name),
                "target manifest intrinsic {name} is missing from intrinsic manifest"
            );
        }
    }

    #[test]
    fn intrinsic_lowering_manifest_matches_real_llvm_surface() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let intrinsic_manifest = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let lowering_manifest = include_str!("../toolchain/lnp64_intrinsic_lowering.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let intrinsic_header = include_str!("../toolchain/lnp64_intrinsics.h");
        let isel = include_str!("../llvm/lib/Target/LNP64/LNP64ISelLowering.cpp");
        let real_llc = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let fd_min = include_str!("../toolchain/liblnp64_fd_min.c");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let target_intrinsics: std::collections::BTreeSet<_> =
            manifest_field(target_manifest, "intrinsics")
                .split(',')
                .collect();
        let intrinsic_by_name: std::collections::BTreeMap<_, _> =
            intrinsic_rows(intrinsic_manifest)
                .into_iter()
                .map(|(name, primitive, result, operands)| (name, (primitive, result, operands)))
                .collect();
        let rows = intrinsic_lowering_rows(lowering_manifest);
        let mut names = std::collections::BTreeSet::new();

        assert_eq!(
            manifest_field(target_manifest, "intrinsic_lowering_contract"),
            "toolchain/lnp64_intrinsic_lowering.manifest"
        );
        assert!(contract_index.contains(
            "intrinsic_lowering|toolchain/lnp64_intrinsic_lowering.manifest|intrinsic_lowering_manifest_matches_real_llvm_surface"
        ));
        assert_eq!(rows.len(), target_intrinsics.len());
        assert!(roadmap.contains("toolchain/lnp64_intrinsic_lowering.manifest"));
        assert!(roadmap.contains("cannot silently lower"));

        for (name, primitive, abi_shape, status, evidence, blocker) in rows {
            assert!(
                names.insert(name),
                "duplicate intrinsic lowering row {name}"
            );
            assert!(
                target_intrinsics.contains(name),
                "lowering manifest names {name}, but target manifest does not"
            );
            let Some((declared_primitive, _, declared_operands)) = intrinsic_by_name.get(name)
            else {
                panic!("lowering manifest names {name}, but intrinsic manifest does not");
            };
            assert_eq!(
                primitive, *declared_primitive,
                "lowering primitive for {name} diverges from intrinsic manifest"
            );
            assert!(
                !abi_shape.is_empty() && !declared_operands.is_empty(),
                "intrinsic {name} must keep ABI operands explicit"
            );
            for path in evidence {
                assert!(
                    manifest_root.join(path).is_file(),
                    "lowering evidence path {path} for {name} does not exist"
                );
            }

            let callee_probe = format!("CalleeName == \"{name}\"");
            match status {
                "call_lowered" => {
                    assert_eq!(blocker, "none", "lowered intrinsic {name} has blocker");
                    assert!(
                        isel.contains(&callee_probe),
                        "call-lowered intrinsic {name} is missing from LLVM call lowering"
                    );
                    assert!(
                        real_llc.contains(name) || fd_min.contains(name),
                        "call-lowered intrinsic {name} is missing from real LLVM smoke coverage"
                    );
                }
                "inline_asm_lowered" => {
                    assert_eq!(blocker, "none", "inline intrinsic {name} has blocker");
                    let asm_mnemonic = primitive.to_ascii_lowercase().replace('.', ".");
                    assert!(
                        intrinsic_header.contains(&format!("static inline"))
                            && intrinsic_header.contains(name),
                        "inline intrinsic {name} is missing from the intrinsic header"
                    );
                    assert!(
                        intrinsic_header.contains(&format!("\"{asm_mnemonic} "))
                            || intrinsic_header.contains(&format!("\"{asm_mnemonic}")),
                        "inline intrinsic {name} is missing asm mnemonic {asm_mnemonic}"
                    );
                }
                "inline_record_builder_lowered" => {
                    assert_eq!(
                        blocker, "none",
                        "record-builder intrinsic {name} has blocker"
                    );
                    assert!(
                        intrinsic_header.contains("static inline")
                            && intrinsic_header.contains(name),
                        "record-builder intrinsic {name} is missing from the intrinsic header"
                    );
                    assert!(
                        real_llc.contains(name),
                        "record-builder intrinsic {name} lacks real LLVM smoke coverage"
                    );
                }
                "c11_atomic_lowered" => {
                    // v2: the LLVM backend expands C11 atomic builtins into
                    // LR.D/SC.D loops, so these intrinsics are plain C builtins
                    // (no hand-written inline asm) and the real-LLVM smoke proves
                    // they lower to lr.d/sc.d.
                    assert_eq!(blocker, "none", "atomic intrinsic {name} has blocker");
                    assert!(
                        intrinsic_header.contains("static inline")
                            && intrinsic_header.contains(name),
                        "atomic intrinsic {name} is missing from the intrinsic header"
                    );
                    assert!(
                        intrinsic_header.contains("__atomic_"),
                        "atomic intrinsic {name} must use a C11 atomic builtin"
                    );
                    assert!(
                        real_llc.contains("lr.d r") && real_llc.contains("sc.d r"),
                        "atomic intrinsic {name} lacks lr.d/sc.d real LLVM smoke coverage"
                    );
                }
                "pending_encoding" | "pending_argblock" | "pending_libc_record_builder" => {
                    assert_ne!(blocker, "none", "pending intrinsic {name} needs a blocker");
                    assert!(
                        !isel.contains(&callee_probe),
                        "pending intrinsic {name} must not have ad-hoc LLVM call lowering"
                    );
                    assert!(
                        intrinsic_header.contains(name),
                        "pending intrinsic {name} should remain declared at the ABI boundary"
                    );
                }
                _ => panic!("unknown intrinsic lowering status {status} for {name}"),
            }
        }

        for name in target_intrinsics {
            assert!(
                names.contains(name),
                "target manifest intrinsic {name} is missing from lowering manifest"
            );
        }
        assert!(intrinsic_header.contains("#define LNP64_OBJECT_CTL_CREATE 1UL"));
        assert!(intrinsic_header.contains("static inline lnp64_word_t __lnp_object_create"));
        assert!(intrinsic_header.contains("lnp64_word_t record[9];"));
        assert!(intrinsic_header.contains("record[0] = LNP64_OBJECT_CTL_CREATE;"));
        assert!(intrinsic_header.contains("record[8] = 0;"));
        assert!(intrinsic_header.contains("return __lnp_object_ctl((lnp64_word_t)record);"));
        assert!(intrinsic_header.contains("static inline lnp64_word_t __lnp_domain_create"));
        assert!(intrinsic_header.contains("lnp64_word_t record[25];"));
        assert!(intrinsic_header.contains("return __lnp_domain_ctl((lnp64_word_t)record);"));
        assert!(intrinsic_header.contains("static inline lnp64_word_t __lnp_call_gate_create"));
        assert!(intrinsic_header.contains("record[2] = 4;"));
        for (name, mnemonic) in [
            ("__lnp_cap_dup", "cap_dup"),
            ("__lnp_cap_send", "cap_send"),
            ("__lnp_cap_recv", "cap_recv"),
            ("__lnp_cap_revoke", "cap_revoke"),
        ] {
            assert!(
                intrinsic_header.contains(name) && intrinsic_header.contains(mnemonic),
                "capability intrinsic {name} must lower through {mnemonic} in the header"
            );
            assert!(
                real_llc.contains("intrinsic-cap-control-clang-smoke.o")
                    && real_llc.contains(mnemonic),
                "capability intrinsic {name} lacks real LLVM object smoke coverage"
            );
        }
        assert!(intrinsic_header.contains("lnp64_word_t record[4];"));
        assert!(
            intrinsic_header
                .contains("record[1] = 0;\n  record[2] = rights;\n  record[3] = flags;")
        );
        assert!(intrinsic_header.contains("record[2] = 0;\n  record[3] = flags;"));
        assert!(real_llc.contains("lnp64-intrinsic-cap-control-linked.elf"));
        assert!(
            real_llc.contains("real LLVM LNP64 lld intrinsic capability control link smoke passed")
        );
    }

    #[test]
    fn intrinsic_header_matches_intrinsic_manifest() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let intrinsic_manifest = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let intrinsic_header = include_str!("../toolchain/lnp64_intrinsics.h");
        let target_intrinsic_header = include_str!("../toolchain/include/lnp64/intrinsics.h");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = intrinsic_rows(intrinsic_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let header_path = manifest_field(target_manifest, "intrinsic_header_contract");
        let target_header_path =
            manifest_field(target_manifest, "target_intrinsic_header_contract");
        let mut declarations = std::collections::BTreeSet::new();

        assert_eq!(header_path, "toolchain/lnp64_intrinsics.h");
        assert_eq!(target_header_path, "toolchain/include/lnp64/intrinsics.h");
        assert!(manifest_root.join(header_path).is_file());
        assert!(manifest_root.join(target_header_path).is_file());
        assert!(contract_index.contains(
            "intrinsic_header|toolchain/lnp64_intrinsics.h|intrinsic_header_matches_intrinsic_manifest"
        ));
        assert!(contract_index.contains(
            "target_intrinsic_header|toolchain/include/lnp64/intrinsics.h|target_intrinsic_header_wraps_canonical_header"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_intrinsics.h"));
        assert!(transition_manifest.contains("toolchain/include/lnp64/intrinsics.h"));
        assert!(roadmap.contains("toolchain/lnp64_intrinsics.h"));
        assert!(target_intrinsic_header.contains("#include \"../../lnp64_intrinsics.h\""));
        assert!(intrinsic_header.contains("#ifndef LNP64_INTRINSICS_H"));
        assert!(intrinsic_header.contains("typedef unsigned long lnp64_word_t;"));
        assert!(intrinsic_header.contains("typedef lnp64_word_t lnp64_cap_t;"));

        for (name, primitive, _, operands) in rows {
            assert!(
                declarations.insert(name),
                "duplicate intrinsic declaration check for {name}"
            );
            assert!(
                intrinsic_header.contains(&format!(" {name}("))
                    || intrinsic_header.contains(&format!("*{name}(")),
                "intrinsic header is missing declaration for {name}"
            );
            assert!(
                !primitive.is_empty() && !operands.is_empty(),
                "manifest row for {name} must keep primitive and operands"
            );
        }
        for forbidden in [
            "fork", "pipe", "pthread", "signal", "poll", "select", "epoll",
        ] {
            assert!(
                !intrinsic_header.contains(forbidden),
                "private intrinsic header leaks compatibility word {forbidden}"
            );
        }
    }

    #[test]
    fn target_intrinsic_header_wraps_canonical_header() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let target_intrinsic_header = include_str!("../toolchain/include/lnp64/intrinsics.h");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let target_header_path =
            manifest_field(target_manifest, "target_intrinsic_header_contract");

        assert_eq!(target_header_path, "toolchain/include/lnp64/intrinsics.h");
        assert!(manifest_root.join(target_header_path).is_file());
        assert!(target_intrinsic_header.contains("#include \"../../lnp64_intrinsics.h\""));
        assert!(contract_index.contains(
            "target_intrinsic_header|toolchain/include/lnp64/intrinsics.h|target_intrinsic_header_wraps_canonical_header"
        ));
        assert!(transition_manifest.contains("toolchain/include/lnp64/intrinsics.h"));
    }

    #[test]
    fn private_intrinsics_do_not_expose_posix_compatibility_names() {
        let intrinsic_manifest = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let forbidden = [
            "fork", "pipe", "pthread", "signal", "sig", "errno", "poll", "select", "epoll",
            "socket",
        ];

        for (name, _, _, _) in intrinsic_rows(intrinsic_manifest) {
            for word in forbidden {
                assert!(
                    !name.contains(word),
                    "private native intrinsic {name} leaks compatibility spelling {word}"
                );
            }
        }
    }

    #[test]
    fn isel_manifest_covers_backend_starting_opcode_groups() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let isel_manifest = include_str!("../toolchain/lnp64_isel.manifest");
        let asm_source = include_str!("asm.rs");
        let rows = isel_rows(isel_manifest);
        let mut groups = std::collections::BTreeSet::new();
        let mut opcodes = std::collections::BTreeSet::new();

        assert_eq!(
            manifest_field(target_manifest, "isel_contract"),
            "toolchain/lnp64_isel.manifest"
        );
        for (group, status, group_opcodes) in rows {
            assert!(groups.insert(group), "duplicate isel group {group}");
            assert!(
                ["required", "profile", "intrinsic", "bootstrap"].contains(&status),
                "unknown isel status {status}"
            );
            assert!(!group_opcodes.is_empty(), "empty isel group {group}");
            for opcode in group_opcodes {
                assert!(!opcode.is_empty(), "empty opcode in {group}");
                assert!(opcodes.insert(opcode), "duplicate isel opcode {opcode}");
                assert!(
                    asm_source.contains(&format!("\"{opcode}\"")),
                    "isel opcode {opcode} is missing from the assembler parser"
                );
            }
        }
        for group in [
            "constants",
            "integer_alu",
            "control_flow",
            "memory",
            "atomics",
            "native_primitives",
        ] {
            assert!(groups.contains(group), "missing isel group {group}");
        }
    }

    #[test]
    fn mc_encoding_manifest_covers_initial_backend_opcodes() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let mc_manifest = include_str!("../toolchain/lnp64_mc_encoding.manifest");
        let isel_manifest = include_str!("../toolchain/lnp64_isel.manifest");
        let relocation_manifest = include_str!("../toolchain/lnp64_relocations.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let filemap = include_str!("../toolchain/lnp64_llvm_filemap.manifest");
        let mc_emitter =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCCodeEmitter.cpp");
        let disassembler =
            include_str!("../llvm/lib/Target/LNP64/Disassembler/LNP64Disassembler.cpp");
        let mc_asm_backend =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmBackend.cpp");
        let lld_arch = include_str!("../lld/ELF/Arch/LNP64.cpp");
        let rows = mc_encoding_rows(mc_manifest);
        let relocation_names: std::collections::BTreeSet<_> = relocation_rows(relocation_manifest)
            .into_iter()
            .map(|(_, name, _, _)| name)
            .collect();
        let mut groups = std::collections::BTreeMap::new();
        let mut encoded_opcodes = std::collections::BTreeSet::new();

        assert_eq!(
            manifest_field(target_manifest, "mc_encoding_contract"),
            "toolchain/lnp64_mc_encoding.manifest"
        );
        assert!(contract_index.contains(
            "mc_encoding|toolchain/lnp64_mc_encoding.manifest|mc_encoding_manifest_covers_initial_backend_opcodes"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_mc_encoding.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_mc_encoding.manifest"));
        assert!(conformance.contains("toolchain/lnp64_mc_encoding.manifest"));
        assert!(filemap.contains("LNP64MCCodeEmitter.cpp"));
        assert!(mc_manifest.contains("fixed64_no_operand"));
        assert!(mc_manifest.contains("opcode[63:56]"));
        assert!(mc_manifest.contains("fixed64_rrr"));
        assert!(mc_manifest.contains("fixed64_rri_simm32"));
        assert!(mc_manifest.contains("fixed64_mem_base_simm"));
        assert!(mc_manifest.contains("imm32[40:9]"));
        assert!(mc_manifest.contains("fixed64_mmap_bootstrap_control"));
        assert!(mc_manifest.contains("fixed64_env_get_control"));
        assert!(mc_manifest.contains("fixed64_pcr_control"));
        assert!(mc_manifest.contains("Final __lnp_mmap remains blocked"));
        assert!(mc_manifest.contains("F9 argument-block encoding"));

        for (group, format, opcodes, operands, relocations, surfaces) in rows {
            assert!(
                groups
                    .insert(
                        group,
                        (format, opcodes.clone(), operands, relocations.clone())
                    )
                    .is_none(),
                "duplicate MC encoding group {group}"
            );
            assert!(
                format.starts_with("fixed64_"),
                "v2 MC group {group} must use a fixed64 format class"
            );
            assert!(!opcodes.is_empty(), "empty MC opcode group {group}");
            assert!(!operands.is_empty(), "empty MC operands for {group}");
            assert!(!surfaces.is_empty(), "empty LLVM surfaces for {group}");
            for opcode in opcodes {
                assert!(
                    encoded_opcodes.insert(opcode),
                    "duplicate MC opcode {opcode}"
                );
            }
            for relocation in relocations {
                if relocation != "none" {
                    assert!(
                        relocation_names.contains(relocation),
                        "MC group {group} names unknown relocation {relocation}"
                    );
                }
            }
            for surface in surfaces {
                assert!(
                    surface.ends_with(".td") || surface.ends_with(".cpp"),
                    "MC group {group} names unexpected LLVM surface {surface}"
                );
            }
        }

        for group in [
            "constants",
            "wide_constants",
            "integer_alu_rrr",
            "integer_alu_rri",
            "integer_compare_value",
            "control_branch",
            "runtime_control",
            "memory",
            "atomics",
            "heap_rr",
            "heap_rrr",
            "heap_reg",
            "mmap_bootstrap_control",
            "env_get_control",
            "pcr_control",
            "native_primitives",
            "clone_control",
            "compat_metadata_control",
            "native_control_rr",
            "native_capability_rr",
        ] {
            assert!(
                groups.contains_key(group),
                "missing MC encoding group {group}"
            );
        }
        for (_group, status, opcodes) in isel_rows(isel_manifest) {
            if status == "required" || status == "intrinsic" {
                for opcode in opcodes {
                    assert!(
                        encoded_opcodes.contains(opcode),
                        "required/intrinsic isel opcode {opcode} lacks MC encoding coverage"
                    );
                }
            }
        }
        assert!(groups["control_branch"].3.contains(&"R_LNP64_BRANCH"));
        assert!(groups["control_branch"].3.contains(&"R_LNP64_PC32"));
        assert!(groups["control_jump"].3.contains(&"R_LNP64_JUMP"));
        assert!(groups["wide_constants"].3.contains(&"R_LNP64_AUIPC"));
        assert!(groups["wide_constants"].3.contains(&"R_LNP64_PC32"));
        assert!(
            groups["wide_constants"]
                .3
                .contains(&"R_LNP64_TLS_TPREL_SLOT64")
        );
        assert!(
            groups["native_primitives"]
                .3
                .contains(&"R_LNP64_CAP_DESC64")
        );
        assert!(
            groups["native_primitives"]
                .3
                .contains(&"R_LNP64_CALLGATE64")
        );
        assert!(groups["native_capability_rr"].1.contains(&"CAP_DUP"));
        assert!(groups["native_capability_rr"].1.contains(&"CAP_SEND"));
        assert!(groups["native_capability_rr"].1.contains(&"CAP_RECV"));
        assert!(groups["native_capability_rr"].1.contains(&"CAP_REVOKE"));
        assert!(groups["clone_control"].0.contains("fixed64_clone_control"));
        assert!(groups["clone_control"].1.contains(&"CLONE.SPAWN"));
        assert!(groups["clone_control"].1.contains(&"THREAD_JOIN"));
        assert!(
            groups["compat_process_control"]
                .0
                .contains("fixed64_compat_process")
        );
        assert!(groups["compat_process_control"].1.contains(&"FORK"));
        assert!(groups["compat_process_control"].1.contains(&"WAIT_PID"));
        assert!(groups["compat_process_control"].1.contains(&"EXEC"));
        // exec's mnemonic/encoding/decoding now come from the .td (def EXEC)
        // via the generated emitter/printer/disassembler, not hand-written
        // switch cases. The encoder is generated; the disassembler defers to
        // the generated decode table.
        assert!(mc_emitter.contains("getBinaryCodeForInstr"));
        assert!(disassembler.contains("decodeInstruction(DecoderTable64"));
        assert!(
            groups["compat_metadata_control"]
                .0
                .contains("fixed64_compat_metadata")
        );
        assert!(
            groups["compat_metadata_control"]
                .1
                .contains(&"STAT_PATH_AT")
        );
        assert!(groups["compat_metadata_control"].1.contains(&"STAT_FD_DYN"));
        assert!(
            groups["compat_metadata_control"]
                .1
                .contains(&"UTIME_PATH_AT")
        );
        assert!(
            groups["compat_metadata_control"]
                .1
                .contains(&"UTIME_FD_DYN")
        );
        assert!(
            groups["compat_metadata_control"]
                .1
                .contains(&"FCNTL_FD_DYN")
        );
        assert!(groups["compat_metadata_control"].1.contains(&"FD_SEEK_DYN"));
        assert!(
            groups["compat_metadata_control"]
                .1
                .contains(&"UNLINK_PATH_AT")
        );
        for opcode in [
            "OPEN_DIR_DYN",
            "MKDIR_PATH_AT",
            "RENAME_PATH_AT",
            "LINK_PATH_AT",
            "SYMLINK_PATH_AT",
            "READLINK_PATH_AT",
            "CHDIR_PATH",
            "GETCWD_PATH",
            "READDIR_FD_DYN",
            "CHMOD_PATH_AT",
            "CHOWN_PATH_AT",
        ] {
            assert!(
                groups["compat_namespace_control"].1.contains(&opcode),
                "missing namespace opcode {opcode}"
            );
        }
        assert!(!groups.contains_key("native_control_planned"));
        // Mnemonic recognition + operand shapes for all of these come from the
        // .td AsmStrings via the generated AsmMatcher (MatchInstructionImpl) --
        // no hand StringSwitch. The gate's assemble smokes exercise them.
        // The encoder is TableGen-generated: bytes come from the generated
        // getBinaryCodeForInstr over the `bits<64> Inst` layout (not a
        // hand-written per-opcode switch). The custom operand encoders below
        // handle the pc-relative branch/jump targets and the AUIPC U-type.
        assert!(mc_emitter.contains("getBinaryCodeForInstr"));
        assert!(mc_emitter.contains("LNP64GenMCCodeEmitter.inc"));
        assert!(mc_emitter.contains("getMachineOpValue"));
        assert!(mc_emitter.contains("getBranchTargetOpValue"));
        assert!(mc_emitter.contains("getJumpTargetOpValue"));
        assert!(mc_emitter.contains("getAUIPCTargetOpValue"));
        assert!(mc_emitter.contains("fixup_lnp64_branch"));
        assert!(mc_emitter.contains("fixup_lnp64_jump"));
        assert!(mc_emitter.contains("fixup_lnp64_auipc"));
        assert!(mc_asm_backend.contains("getRelocType"));
        assert!(mc_asm_backend.contains("fixup_lnp64_branch"));
        assert!(mc_asm_backend.contains("writeNopData"));
        assert!(lld_arch.contains("read64le(Loc)"));
    }

    #[test]
    fn exec_plan_manifest_matches_loader_boundary_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let exec_plan_manifest = include_str!("../toolchain/lnp64_exec_plan.manifest");
        let object_format = include_str!("../object_format.md");
        let rows = exec_plan_rows(exec_plan_manifest);
        let mut records = std::collections::BTreeSet::new();

        assert_eq!(
            manifest_field(target_manifest, "exec_plan_contract"),
            "toolchain/lnp64_exec_plan.manifest"
        );
        for (record, requirement, fields) in rows {
            assert!(
                records.insert(record),
                "duplicate exec-plan record {record}"
            );
            assert!(
                ["required", "optional"].contains(&requirement),
                "unknown exec-plan requirement {requirement}"
            );
            assert!(!fields.is_empty(), "empty exec-plan record {record}");
            let mut record_fields = std::collections::BTreeSet::new();
            for field in fields {
                assert!(
                    !field.is_empty(),
                    "empty field in exec-plan record {record}"
                );
                assert!(
                    record_fields.insert(field),
                    "duplicate field {field} in exec-plan record {record}"
                );
            }
        }
        for record in ["header", "entry", "vma", "fdr_grant"] {
            assert!(
                records.contains(record),
                "missing exec-plan record {record}"
            );
        }

        assert!(object_format.contains("## Exec-Plan Descriptor Boundary"));
        assert!(
            object_format.contains("exec-plan descriptor is the only object consumed by hardware")
        );
        assert!(object_format.contains("entry PC, initial SP"));
        assert!(object_format.contains("VMA records: target virtual address"));
        assert!(object_format.contains("mapping flags\n  (reserved zero in v1)"));
        assert!(object_format.contains("startup FDR grants"));
        assert!(object_format.contains("old image remains"));
    }

    #[test]
    fn loader_security_manifest_covers_exec_plan_security() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let security_manifest = include_str!("../toolchain/lnp64_loader_security.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let object_format = include_str!("../object_format.md");
        let loader_source = include_str!("loader.rs");
        let emulator_source = include_str!("emulator.rs");
        let lowering_source = include_str!("lowering.rs");
        let personality_lowering_source = include_str!("personality_lowering.rs");
        let conformance = include_str!("../conformance_matrix.md");
        let evidence_corpus = format!(
            "{loader_source}\n{emulator_source}\n{lowering_source}\n{personality_lowering_source}\n{conformance}"
        );
        let rows = loader_security_rows(security_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let security_path = manifest_field(target_manifest, "loader_security_contract");
        let mut requirements = std::collections::BTreeMap::new();

        assert_eq!(security_path, "toolchain/lnp64_loader_security.manifest");
        assert!(manifest_root.join(security_path).is_file());
        assert!(contract_index.contains(
            "loader_security|toolchain/lnp64_loader_security.manifest|loader_security_manifest_covers_exec_plan_security"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_loader_security.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_loader_security.manifest"));
        assert!(object_format.contains("The loader must choose ASLR layout"));
        assert!(object_format.contains("W^X/NX policy"));
        assert!(object_format.contains("executable provenance"));
        assert!(object_format.contains("old image remains"));

        for (requirement, boundary, evidence, status) in rows {
            assert!(
                requirements
                    .insert(requirement, (boundary, evidence.clone(), status))
                    .is_none(),
                "duplicate loader security requirement {requirement}"
            );
            assert!(
                [
                    "software_loader",
                    "loader_to_emulator",
                    "loader_and_exec_validator",
                    "software_loader_and_layout",
                    "exec_descriptor_validator",
                    "emulator_exec",
                ]
                .contains(&boundary),
                "unknown loader security boundary {boundary}"
            );
            assert!(
                ["tested", "partial"].contains(&status),
                "unknown loader security status {status} for {requirement}"
            );
            assert!(
                !evidence.is_empty(),
                "empty evidence for loader security requirement {requirement}"
            );
            for item in evidence {
                assert!(
                    evidence_corpus.contains(item),
                    "loader security evidence {item} for {requirement} is not present"
                );
            }
        }

        for requirement in [
            "parse_elf_headers",
            "apply_relocations",
            "prepare_vmas",
            "startup_metadata",
            "submit_exec_plan",
            "wx_nx_policy",
            "aslr_load_bias",
            "provenance_authority",
            "precommit_preservation",
        ] {
            assert!(
                requirements.contains_key(requirement),
                "missing loader security requirement {requirement}"
            );
        }
        assert_eq!(
            requirements["provenance_authority"].2, "partial",
            "generation/lineage authority validation must not be overclaimed"
        );
        for evidence in [
            "emulator_rejects_exec_descriptor_vma_without_source_capability",
            "emulator_rejects_exec_descriptor_vma_without_source_generation",
            "emulator_rejects_exec_descriptor_vma_without_lineage_epoch",
            "static_elf_loader_rejects_exec_descriptor_without_image_provenance",
            "static_elf_loader_rejects_exec_descriptor_without_fdr_grant_authority",
            "emulator_rejects_exec_descriptor_unsupported_vma_provenance",
            "emulator_rejects_exec_descriptor_executable_vma_without_image_text_provenance",
            "emulator_rejects_exec_descriptor_nonexecutable_vma_with_image_text_provenance",
            "emulator_rejects_prepared_exec_vma_source_authority_mismatch_before_commit",
            "static_elf_loader_rejects_exec_descriptor_bad_measurements",
            "emulator_rejects_exec_descriptor_measurement_without_authority",
            "static_elf_loader_rejects_exec_descriptor_unknown_vma_mapping_flags",
            "emulator_rejects_exec_descriptor_unknown_vma_mapping_flags",
            "emulator_rejects_exec_descriptor_fdr_grant_stale_source_fd_generation_before_commit",
            "emulator_rejects_exec_descriptor_stale_domain_generation_before_commit",
            "emulator_rejects_exec_descriptor_stale_process_generation_before_commit",
            "emulator_rejects_exec_descriptor_stale_lineage_epoch_before_commit",
            "emulator_preserves_old_image_when_exec_descriptor_validation_fails",
        ] {
            assert!(
                requirements["provenance_authority"].1.contains(&evidence),
                "provenance authority row must name evidence {evidence}"
            );
        }
        assert!(
            requirements["submit_exec_plan"]
                .1
                .contains(&"emulator_rejects_exec_descriptor_count_and_length_shape_fuzz"),
            "submit_exec_plan row must name descriptor count/length fuzz evidence"
        );
        assert!(
            requirements["apply_relocations"]
                .1
                .contains(&"static_elf_loader_rejects_malformed_rela_section_shapes"),
            "apply_relocations row must name malformed RELA section evidence"
        );
        for remaining_gap in [
            "external VMA source acquisition policy",
            "additional descriptor-shape fuzzing",
            "additional relocation/linker diagnostics",
            "archive/library search behavior",
            "dynamic-linking policy tests",
        ] {
            assert!(
                conformance.contains(remaining_gap),
                "COMPAT-BIN-001 must keep provenance authority partial gap visible: {remaining_gap}"
            );
        }
        for covered_rejection in [
            "entry-PC executable-VMA validation",
            "reserved mapping flags",
            "measurement authority",
            "descriptor count/length fuzzing",
            "prepared VMA source-authority mismatch",
            "malformed RELA section diagnostics",
        ] {
            assert!(
                conformance.contains(covered_rejection),
                "COMPAT-BIN-001 must keep loader rejection visible: {covered_rejection}"
            );
        }
        for requirement in [
            "parse_elf_headers",
            "apply_relocations",
            "prepare_vmas",
            "startup_metadata",
            "submit_exec_plan",
            "wx_nx_policy",
            "aslr_load_bias",
            "precommit_preservation",
        ] {
            assert_eq!(
                requirements[requirement].2, "tested",
                "{requirement} should be tested"
            );
        }
    }

    #[test]
    fn psabi_manifest_records_current_calling_convention_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let psabi_manifest = include_str!("../toolchain/lnp64_psabi.manifest");
        let psabi_doc = include_str!("../psABI.md");

        assert_eq!(
            manifest_field(target_manifest, "psabi_contract"),
            "toolchain/lnp64_psabi.manifest"
        );
        assert_eq!(
            manifest_field(psabi_manifest, "name"),
            manifest_field(target_manifest, "call_conv")
        );
        assert_eq!(
            manifest_field(psabi_manifest, "doc"),
            manifest_field(target_manifest, "psabi")
        );
        assert_eq!(
            manifest_field(psabi_manifest, "stack_alignment"),
            manifest_field(target_manifest, "stack_alignment")
        );
        assert_eq!(manifest_field(psabi_manifest, "gpr_count"), "32");
        assert_eq!(manifest_field(psabi_manifest, "fdr_count"), "256");
        assert_eq!(manifest_field(psabi_manifest, "fpr_count"), "32");
        assert_eq!(manifest_field(psabi_manifest, "vr_count"), "16");
        assert_eq!(manifest_field(psabi_manifest, "zero_register"), "r0");
        assert_eq!(manifest_field(psabi_manifest, "stack_pointer"), "r31");
        assert_eq!(manifest_field(psabi_manifest, "link_register"), "r1");
        assert_eq!(manifest_field(psabi_manifest, "argument_gprs"), "r2-r9");
        assert_eq!(manifest_field(psabi_manifest, "return_gprs"), "r2");
        assert_eq!(
            manifest_field(psabi_manifest, "caller_clobbered_gprs"),
            "r1-r17,r28-r30"
        );
        assert_eq!(
            manifest_field(psabi_manifest, "callee_saved_gprs"),
            "r18-r27"
        );
        // v2/E8: no backend scratch register -- ADDI's 32-bit immediate adjusts
        // SP / forms frame addresses directly. r30 is an ordinary allocatable
        // caller-clobbered temporary, so the manifest carries no such field.
        assert!(!psabi_manifest.contains("backend_scratch_gpr"));
        assert_eq!(
            manifest_field(psabi_manifest, "entry_page_base"),
            "0x700000"
        );
        assert_eq!(manifest_field(psabi_manifest, "entry_page_size"), "0x20000");
        assert_eq!(
            manifest_field(psabi_manifest, "entry_strings_base"),
            "0x701000"
        );
        assert_eq!(manifest_field(psabi_manifest, "thread_pointer_pcr"), "TP");
        assert!(manifest_csv_contains(
            psabi_manifest,
            "errno_ops",
            "ERRNO_GET"
        ));
        assert!(manifest_csv_contains(
            psabi_manifest,
            "errno_ops",
            "ERRNO_SET"
        ));
        assert!(manifest_csv_contains(
            psabi_manifest,
            "signal_return",
            "SIGRET"
        ));
        assert!(manifest_csv_contains(
            psabi_manifest,
            "signal_return",
            "GATE_RETURN"
        ));

        assert!(
            psabi_doc.contains("Integer and pointer arguments are passed in `r2` through `r9`.")
        );
        assert!(psabi_doc.contains("Return values are placed in `r2`."));
        assert!(psabi_doc.contains("`r1` is the return address (`ra`)"));
        assert!(psabi_doc.contains("callee-saved (preserved) GPR set `s0`-`s9` =\n`r18`-`r27`"));
        assert!(psabi_doc.contains("`r2`-`r17` and `r28`-`r30`"));
        assert!(psabi_doc.contains("Additional fixed arguments are passed"));
        assert!(psabi_doc.contains("`r31` points at the current thread's stack/local region."));
        assert!(psabi_doc.contains("The thread pointer is read and written through the `TP` PCR."));
        assert!(psabi_doc.contains("`SIGRET` is the POSIX spelling"));
        assert!(psabi_doc.contains("`GATE_RETURN`"));
    }

    #[test]
    fn register_manifest_records_backend_classes() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let register_manifest = include_str!("../toolchain/lnp64_registers.manifest");
        let psabi_manifest = include_str!("../toolchain/lnp64_psabi.manifest");
        let inline_asm_manifest = include_str!("../toolchain/lnp64_inline_asm.manifest");
        let debug_unwind_manifest = include_str!("../toolchain/lnp64_debug_unwind.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let rows = register_class_rows(register_manifest);
        let mut classes = std::collections::BTreeMap::new();

        assert_eq!(
            manifest_field(target_manifest, "register_contract"),
            "toolchain/lnp64_registers.manifest"
        );
        assert!(contract_index.contains(
            "registers|toolchain/lnp64_registers.manifest|register_manifest_records_backend_classes"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_registers.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_registers.manifest"));
        assert!(conformance.contains("toolchain/lnp64_registers.manifest"));

        for (class, values, width, allocatable, reserved, role, debug) in rows {
            assert!(
                classes
                    .insert(class, (values, width, allocatable, reserved, role, debug))
                    .is_none(),
                "duplicate register class {class}"
            );
            assert!(!values.is_empty(), "empty register values for {class}");
            assert!(!width.is_empty(), "empty register width for {class}");
            assert!(
                !allocatable.is_empty(),
                "empty allocatable register set for {class}"
            );
            assert!(!role.is_empty(), "empty register role for {class}");
            assert!(!debug.is_empty(), "empty debug register role for {class}");
        }

        for class in ["gpr", "fdr", "fpr", "vr", "pcr", "special"] {
            assert!(
                classes.contains_key(class),
                "missing register class {class}"
            );
        }
        assert_eq!(classes["gpr"].0, manifest_field(target_manifest, "gpr"));
        assert_eq!(classes["fdr"].0, manifest_field(target_manifest, "fdr"));
        assert_eq!(classes["fpr"].0, manifest_field(target_manifest, "fpr"));
        assert_eq!(classes["vr"].0, manifest_field(target_manifest, "vr"));
        assert_eq!(classes["gpr"].1, "64");
        // v2/E8: allocatable r2-r30; reserved r0 (zero), r1 (ra) and r31 (sp).
        // r30 is now an ordinary allocatable temporary (no backend scratch).
        assert_eq!(classes["gpr"].2, "r2-r30");
        assert!(classes["gpr"].3.contains(&"r0"));
        assert!(classes["gpr"].3.contains(&"r1"));
        assert!(!classes["gpr"].3.contains(&"r30"));
        assert!(
            classes["gpr"]
                .3
                .contains(&manifest_field(psabi_manifest, "stack_pointer"))
        );
        // v2: the link register (ra) is r1, a dedicated reserved link register
        // (held out of allocation so leaf functions cannot clobber the return
        // address), not a SPECIAL reg and not an allocatable temporary.
        assert_eq!(manifest_field(psabi_manifest, "link_register"), "r1");
        assert!(classes["gpr"].3.contains(&"r1"));
        // v2 dissolved the SPECIAL LR/FLAGS; only TP remains in the namespace.
        assert!(
            classes["special"]
                .0
                .split(',')
                .any(|value| value == manifest_field(psabi_manifest, "thread_pointer_pcr"))
        );
        assert!(!classes["special"].0.split(',').any(|value| value == "FLAGS"));
        assert!(!classes["special"].0.split(',').any(|value| value == "LR"));

        for pcr in [
            "PID",
            "PPID",
            "TID",
            "TP",
            "UID",
            "GID",
            "SIGMASK",
            "SIGPENDING",
            "REALTIME_SEC",
            "REALTIME_NSEC",
            "CRED_PROFILE",
            "CRED_HANDLE",
        ] {
            assert!(
                classes["pcr"].0.split(',').any(|value| value == pcr),
                "missing PCR {pcr}"
            );
        }
        for (constraint, class, values, _usage) in inline_asm_rows(inline_asm_manifest) {
            if ["gpr", "fdr", "fpr", "vr"].contains(&class) {
                assert_eq!(
                    classes[class].0, values,
                    "inline asm constraint {constraint} disagrees with register class {class}"
                );
            }
        }
        for register in ["r0-r31", "TP"] {
            assert!(manifest_csv_contains(
                debug_unwind_manifest,
                "register_numbers",
                register
            ));
        }
    }

    #[test]
    fn debug_unwind_manifest_records_minimum_backend_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let debug_unwind_manifest = include_str!("../toolchain/lnp64_debug_unwind.manifest");
        let frame_lowering = include_str!("../llvm/lib/Target/LNP64/LNP64FrameLowering.cpp");
        let psabi_doc = include_str!("../psABI.md");
        let roadmap = include_str!("../toolchain_roadmap.md");

        assert_eq!(
            manifest_field(target_manifest, "debug_unwind_contract"),
            "toolchain/lnp64_debug_unwind.manifest"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "debug_format"),
            "DWARFv5"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "line_tables"),
            "required"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "real_llvm_debug_sections"),
            "clang_debug_sections_object"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "line_table_decode"),
            "blocked_until_debug_relocation_decoding"
        );
        for register in ["r0-r31", "TP"] {
            assert!(manifest_csv_contains(
                debug_unwind_manifest,
                "register_numbers",
                register
            ));
        }
        assert_eq!(
            manifest_field(debug_unwind_manifest, "stack_pointer"),
            "r31"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "dwarf_register_map"),
            "r0-r31:0-31,TP:33"
        );
        assert_eq!(manifest_field(debug_unwind_manifest, "cfa_register"), "r31");
        assert_eq!(
            manifest_field(debug_unwind_manifest, "return_address"),
            "r1"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "cfi"),
            "required_for_non_leaf"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "exception_model"),
            "none_v0"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "signal_unwind"),
            "psabi_signal_frame"
        );

        assert!(psabi_doc.contains("## Debug and Unwind Minimum"));
        assert!(psabi_doc.contains("`r1` (ra) as DWARF register `1`, and `TP` as `33`"));
        assert!(psabi_doc.contains("`r31` as the CFA stack register"));
        assert!(psabi_doc.contains("There is no v0 language exception runtime"));
        assert!(roadmap.contains("toolchain/lnp64_debug_unwind.manifest"));
        assert!(frame_lowering.contains("LNP64DwarfSP = 31"));
        assert!(frame_lowering.contains("LNP64DwarfRA = 1"));
        assert!(frame_lowering.contains("MCCFIInstruction::cfiDefCfa"));
        assert!(frame_lowering.contains("MCCFIInstruction::createOffset"));
        assert!(frame_lowering.contains("TargetOpcode::CFI_INSTRUCTION"));
    }

    #[test]
    fn inline_asm_manifest_records_backend_constraints() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let inline_asm_manifest = include_str!("../toolchain/lnp64_inline_asm.manifest");
        let clang_target = include_str!("../clang/lib/Basic/Targets/LNP64.cpp");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = inline_asm_rows(inline_asm_manifest);
        let mut constraints = std::collections::BTreeMap::new();
        let assert_clang_names_range = |prefix: &str, end: usize| {
            for index in 0..=end {
                let name = format!(r#""{prefix}{index}""#);
                assert!(
                    clang_target.contains(&name),
                    "Clang target register names missing {name}"
                );
            }
        };

        assert_eq!(
            manifest_field(target_manifest, "inline_asm_contract"),
            "toolchain/lnp64_inline_asm.manifest"
        );
        for (constraint, class, values, usage) in rows {
            assert!(!class.is_empty(), "empty inline-asm class for {constraint}");
            assert!(
                !values.is_empty(),
                "empty inline-asm values for {constraint}"
            );
            assert!(!usage.is_empty(), "empty inline-asm use for {constraint}");
            assert!(
                constraints.insert(constraint, (class, values)).is_none(),
                "duplicate inline-asm constraint {constraint}"
            );
        }

        assert_eq!(constraints["r"], ("gpr", "r0-r31"));
        assert_eq!(constraints["f"], ("fdr", "fd0-fd255"));
        assert_eq!(
            constraints["d"],
            ("fpr", manifest_field(target_manifest, "fpr"))
        );
        assert_eq!(
            constraints["v"],
            ("vr", manifest_field(target_manifest, "vr"))
        );
        assert_eq!(
            constraints["p"],
            (
                "pcr",
                "PID,PPID,TID,TP,UID,GID,SIGMASK,SIGPENDING,REALTIME_SEC,REALTIME_NSEC,CRED_PROFILE,CRED_HANDLE"
            )
        );
        assert_eq!(constraints["m"], ("memory", "base_gpr_plus_signed_offset"));
        assert_eq!(constraints["i"], ("immediate", "signed_16_or_symbolic"));
        assert_clang_names_range("r", 31);
        assert_clang_names_range("fd", 255);
        assert_clang_names_range("f", 31);
        assert_clang_names_range("v", 15);
        for pcr in constraints["p"].1.split(',') {
            let name = format!(r#""{pcr}""#);
            assert!(
                clang_target.contains(&name),
                "Clang target register names missing PCR {name}"
            );
        }
        for constraint in ["case 'r'", "case 'f'", "case 'd'", "case 'v'", "case 'p'"] {
            assert!(
                clang_target.contains(constraint),
                "Clang target missing inline asm constraint {constraint}"
            );
        }
        assert!(roadmap.contains("toolchain/lnp64_inline_asm.manifest"));
    }

    #[test]
    fn crt_startup_manifest_records_process_entry_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let crt_manifest = include_str!("../toolchain/lnp64_crt_startup.manifest");
        let psabi_manifest = include_str!("../toolchain/lnp64_psabi.manifest");
        let psabi_doc = include_str!("../psABI.md");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = crt_startup_rows(crt_manifest);
        let mut contracts = std::collections::BTreeMap::new();

        assert_eq!(
            manifest_field(target_manifest, "crt_startup_contract"),
            "toolchain/lnp64_crt_startup.manifest"
        );
        for (item, requirement, contract) in rows {
            assert_eq!(requirement, "required", "crt startup item {item}");
            assert!(!contract.is_empty(), "empty crt startup contract {item}");
            assert!(
                contracts.insert(item, contract).is_none(),
                "duplicate crt startup item {item}"
            );
        }

        assert!(contracts["entry_symbol"].contains(&"_start"));
        assert!(contracts["main_signature"].contains(&"main(argc"));
        assert!(contracts["main_signature"].contains(&"argv"));
        assert!(contracts["main_signature"].contains(&"envp)"));
        assert!(contracts["startup_page"].contains(&"base=0x700000"));
        assert!(contracts["startup_page"].contains(&"size=0x20000"));
        assert_eq!(
            manifest_field(psabi_manifest, "entry_page_base"),
            "0x700000"
        );
        assert_eq!(manifest_field(psabi_manifest, "entry_page_size"), "0x20000");
        assert!(contracts["entry_strings"].contains(&"base=0x701000"));
        assert_eq!(
            manifest_field(psabi_manifest, "entry_strings_base"),
            "0x701000"
        );
        assert!(contracts["tls"].contains(&"thread_pointer_pcr=TP"));
        assert!(contracts["errno"].contains(&"ERRNO_GET"));
        assert!(contracts["errno"].contains(&"ERRNO_SET"));
        assert!(contracts["auxv"].contains(&"ENV_GET"));
        assert!(contracts["process_exit"].contains(&"EXIT"));

        assert!(psabi_doc.contains("The static crt0 startup stub initializes C `main`"));
        assert!(psabi_doc.contains(
            "Static Clang/lld driver defaults use\n`target/lnp64-sysroot/usr/lib/lnp64/crt0.o`"
        ));
        assert!(roadmap.contains("toolchain/lnp64_crt_startup.manifest"));
    }

    #[test]
    fn relocation_manifest_matches_object_format_and_target_manifest() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let relocation_manifest = include_str!("../toolchain/lnp64_relocations.manifest");
        let object_format = include_str!("../object_format.md");
        let loader_source = include_str!("loader.rs");
        let llvm_mc_backend =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmBackend.cpp");
        let lld_backend = include_str!("../lld/ELF/Arch/LNP64.cpp");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = relocation_rows(relocation_manifest);
        let target_relocations: std::collections::BTreeSet<_> =
            manifest_field(target_manifest, "relocations")
                .split(',')
                .collect();
        let mut numbers = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();

        assert!(roadmap.contains("no longer selects `LA` for globals"));
        assert!(roadmap.contains("object-writer relocation mapping"));
        assert!(roadmap.contains("Emitting those fixups from\n     SelectionDAG/asm parsing"));
        assert!(roadmap.contains("instruction-count branch/jump fixups"));
        assert!(object_format.contains("There is no split high/low pair"));
        assert!(object_format.contains("no companion `ADDI`/`LD` low relocation"));
        assert!(object_format.contains("there is no linker pair-binding question"));
        assert!(lld_backend.contains("case R_LNP64_AUIPC"));
        assert!(roadmap.contains("the single-word `AUIPC` form is the final object contract"));
        assert!(roadmap.contains("resolved\n     as `P + sext32(S + A - P)`"));
        assert_eq!(rows.len(), 16);
        assert_eq!(
            target_relocations.len(),
            rows.len(),
            "target manifest must enumerate the complete relocation contract"
        );
        for (idx, (number, name, calculation, loader_status)) in rows.iter().enumerate() {
            assert_eq!(*number as usize, idx, "relocation numbers must be dense");
            assert!(
                numbers.insert(*number),
                "duplicate relocation number {number}"
            );
            assert!(names.insert(*name), "duplicate relocation name {name}");
            assert!(!calculation.is_empty(), "empty calculation for {name}");
            assert!(
                loader_status.starts_with("supported_") || loader_status.starts_with("planned_"),
                "unknown loader status {loader_status} for {name}"
            );
            assert!(
                object_format.contains(&format!("| {number} | `{name}` |")),
                "relocation {number},{name} is missing from object_format.md"
            );
            assert!(
                target_relocations.contains(name),
                "relocation manifest {name} is missing from target manifest"
            );
            if *number >= 13 {
                // Relocations 13+ are the v2 LLVM MC backend code fixups (AUIPC /
                // instruction-count BRANCH / JUMP); they must be emitted by the
                // MC asm backend's getRelocType mapping.
                assert!(
                    llvm_mc_backend.contains(name),
                    "v2 MC fixup relocation {name} is missing from LLVM MC backend"
                );
            } else {
                // Relocations 0-12 are resolved by the in-tree loader/lld.
                assert!(
                    lld_backend.contains(name),
                    "relocation manifest {name} is missing from lld LNP64 backend"
                );
            }
            if loader_status.starts_with("supported_") {
                assert!(
                    loader_source.contains(&format!("const {name}:")),
                    "loader-supported relocation {name} is missing from loader constants"
                );
                if *name != "R_LNP64_NONE" {
                    assert!(
                        roadmap.contains(name),
                        "loader-supported relocation {name} is missing from toolchain roadmap"
                    );
                }
            }
        }
        for name in target_relocations {
            assert!(
                names.contains(name),
                "target manifest relocation {name} is missing from relocation manifest"
            );
        }
    }

    #[test]
    fn scheduler_heap_realtime_contracts_scope_current_architecture() {
        fn assert_contains(document: &str, needle: &str) {
            assert!(
                document.contains(needle),
                "missing architecture text: {needle}"
            );
        }

        let design = include_str!("../design.md");
        let hardware = include_str!("../hardware_design.md");
        let formal_roadmap = include_str!("../formal_rtl_codesign_roadmap.md");
        let formal_theorems = include_str!("../formal_theorems.md");
        let readme = include_str!("../README.md");

        assert_contains(design, "Fixed Weighted-Fair Virtual-Deadline Active-Window");
        assert_contains(hardware, "bounded active windows");
        assert_contains(hardware, "virtual-deadline buckets");
        assert_contains(formal_roadmap, "fixed monotonic weight table");
        assert_contains(formal_roadmap, "sticky");
        assert_contains(formal_roadmap, "bounded migration");
        assert_contains(formal_roadmap, "bounded wakeup insertion");
        assert_contains(formal_roadmap, "bounded preemption");
        assert_contains(formal_roadmap, "no scheduler bytecode");
        assert_contains(hardware, "red-black trees");
        assert_contains(hardware, "no red-black tree");

        assert_contains(design, "tightly synchronized global monotonic timebase");
        assert_contains(hardware, "Resource Domain id/generation");
        assert_contains(hardware, "submitter TID/generation");
        assert_contains(formal_theorems, "reservation/deadline metadata");
        assert_contains(hardware, "operation id");
        assert_contains(hardware, "cancellation epoch");
        assert_contains(hardware, "completion target");
        assert_contains(hardware, "No realtime-admitted Class D");
        assert_contains(hardware, "undifferentiated FIFO entry");

        assert_contains(design, "LNP64 Default Heap Algorithm");
        assert_contains(hardware, "fixed size-class dispatch");
        assert_contains(hardware, "per-thread allocation windows");
        assert_contains(hardware, "bounded transfer queues");
        assert_contains(hardware, "domain-owned slab/run pages");
        assert_contains(hardware, "generation fields");
        assert_contains(hardware, "exact-pointer free");
        assert_contains(hardware, "invalid pointers and double free");
        assert_contains(hardware, "NX heap backing");
        assert_contains(hardware, "bounded hot");
        assert_contains(hardware, "Class D owner-engine transactions with inherited");
        assert_contains(hardware, "Rust-style intra-program memory safety");
        assert_contains(hardware, "Ordinary `LD`/`ST`");

        assert_contains(formal_roadmap, "no-lost-wakeup");
        assert_contains(formal_roadmap, "bounded fairness");
        assert_contains(formal_theorems, "deadline comparison");
        assert_contains(formal_roadmap, "exact-pointer free");
        assert_contains(formal_roadmap, "invalid/double/foreign-free rejection");
        assert_contains(formal_roadmap, "domain accounting");
        assert_contains(formal_roadmap, "no hidden unbounded path in Class A/B/C");
        assert_contains(readme, "Realtime contract soundness");
    }

    #[test]
    fn resource_domain_tree_contracts_scope_current_architecture() {
        fn assert_contains(document: &str, needle: &str) {
            assert!(
                document.contains(needle),
                "missing Resource Domain contract text: {needle}"
            );
        }

        let design = include_str!("../design.md");
        let hardware = include_str!("../hardware_design.md");
        let formal_roadmap = include_str!("../formal_rtl_codesign_roadmap.md");
        let formal_theorems = include_str!("../formal_theorems.md");

        assert_contains(
            hardware,
            "V1 uses a **Fixed Monotonic Resource-Domain Tree**",
        );
        assert_contains(hardware, "do not change parentage or");
        assert_contains(hardware, "budget ownership");
        assert_contains(
            hardware,
            "Domain operations are fixed owner-engine transitions",
        );
        assert_contains(hardware, "software callbacks or policy bytecode");
        assert_contains(
            design,
            "Resource Domains are control-plane expensive and data-plane cheap",
        );
        assert_contains(design, "Hot scheduler and allocator paths must not walk");
        assert_contains(hardware, "Resident effective scheduling record");
        assert_contains(hardware, "not by walking the domain tree during dispatch");
        assert_contains(hardware, "resident effective heap-domain record");
        assert_contains(hardware, "must not walk the Resource Domain tree");
        assert_contains(hardware, "`ALLOC`/`FREE` hot path");
        assert_contains(hardware, "monotonic intersection");
        assert_contains(hardware, "must not walk an unbounded ancestor chain");
        assert_contains(hardware, "hierarchy depth is bounded");
        assert_contains(hardware, "Class D domain-engine work");
        assert_contains(hardware, "bounded cursors");
        assert_contains(hardware, "single-owner and monotonic");
        assert_contains(hardware, "stale attachments fail closed");
        assert_contains(
            formal_roadmap,
            "effective-domain records consumed by scheduler, heap",
        );
        assert_contains(
            formal_roadmap,
            "resident generation-checked effective records",
        );
        assert_contains(formal_roadmap, "resident effective scheduling records");
        assert_contains(formal_roadmap, "resident effective heap-domain records");
        assert_contains(
            formal_theorems,
            "flattened effective-domain records consumed by scheduler, heap",
        );
        assert_contains(
            formal_theorems,
            "does not require an unbounded ancestor walk",
        );
        assert_contains(
            formal_theorems,
            "Class D domain-engine refill/recompute of effective records",
        );
        assert_contains(formal_theorems, "scheduler dispatch consumes");
        assert_contains(formal_theorems, "heap hot paths consume");
        assert_contains(formal_theorems, "`ALLOC`/`FREE` hot paths do not walk");
    }

    #[test]
    fn compatibility_table_names_native_primitives() {
        assert_eq!(lowering_for(CompatSurface::Open), LOWER_OPEN);
        assert_eq!(lowering_for(CompatSurface::Read), LOWER_READ);
        assert_eq!(lowering_for(CompatSurface::Write), LOWER_WRITE);
        assert_eq!(lowering_for(CompatSurface::Close), LOWER_CLOSE);
        assert_eq!(
            lowering_for(CompatSurface::Pipe),
            &[NativePrimitive::ObjectCtl {
                kind: ObjectKind::Queue,
                profile: ObjectProfile::Pipe,
            }]
        );
        assert_eq!(
            lowering_for(CompatSurface::Fork),
            &[NativePrimitive::Clone {
                profile: CloneProfile::NewProcessCow,
            }]
        );
        assert_eq!(
            lowering_for(CompatSurface::PthreadCreate),
            &[NativePrimitive::Clone {
                profile: CloneProfile::NewThreadSharedVm,
            }]
        );
        assert!(lowering_for(CompatSurface::Exec).contains(&NativePrimitive::Exec));
        assert!(lowering_for(CompatSurface::Mmap).contains(&NativePrimitive::Mmap));
        assert!(lowering_for(CompatSurface::FdPassing).contains(&NativePrimitive::CapabilitySend));
        assert!(lowering_for(CompatSurface::SocketLoopback).contains(&NativePrimitive::Await));
        assert!(lowering_for(CompatSurface::Timer).contains(&NativePrimitive::EventDelivery));
        assert!(lowering_for(CompatSurface::CallGate).contains(&NativePrimitive::GateReturn));
        assert!(lowering_for(CompatSurface::Signal).contains(&NativePrimitive::EventDelivery));
        assert!(lowering_for(CompatSurface::Errno).contains(&NativePrimitive::TlsErrnoView));
        assert!(lowering_for(CompatSurface::ResourceDomain).contains(&NativePrimitive::DomainCtl));
        assert_eq!(
            lowering_for(CompatSurface::Stat),
            &[NativePrimitive::Metadata(MetadataOp::GetMeta)]
        );
        assert_eq!(
            lowering_for(CompatSurface::Chmod),
            &[NativePrimitive::Metadata(MetadataOp::SetMeta)]
        );
        assert_eq!(
            lowering_for(CompatSurface::Fcntl),
            &[
                NativePrimitive::Metadata(MetadataOp::GetMeta),
                NativePrimitive::Metadata(MetadataOp::SetMeta),
                NativePrimitive::Metadata(MetadataOp::ObjectCtl),
            ]
        );
    }

    #[test]
    fn compatibility_lowering_pins_native_architecture_boundaries() {
        assert_eq!(
            lowering_for(CompatSurface::PollSelectEpoll),
            &[
                NativePrimitive::EventQueue,
                NativePrimitive::Await,
                NativePrimitive::Pull,
            ]
        );
        assert_eq!(
            lowering_for(CompatSurface::Fork),
            &[NativePrimitive::Clone {
                profile: CloneProfile::NewProcessCow,
            }]
        );
        assert_eq!(
            lowering_for(CompatSurface::PthreadCreate),
            &[NativePrimitive::Clone {
                profile: CloneProfile::NewThreadSharedVm,
            }]
        );
        assert_eq!(
            lowering_for(CompatSurface::Signal),
            &[
                NativePrimitive::EventDelivery,
                NativePrimitive::AbiSignalFrame,
            ]
        );
        assert_eq!(
            lowering_for(CompatSurface::Errno),
            &[
                NativePrimitive::ExplicitResult,
                NativePrimitive::TlsErrnoView,
            ]
        );
    }

    #[test]
    fn netbsd_system_gate_surfaces_are_registered() {
        let surfaces = [
            CompatSurface::CwdRoot,
            CompatSurface::Open,
            CompatSurface::Read,
            CompatSurface::Write,
            CompatSurface::Close,
            CompatSurface::Pipe,
            CompatSurface::PollSelectEpoll,
            CompatSurface::Fork,
            CompatSurface::Exec,
            CompatSurface::PthreadCreate,
            CompatSurface::Mmap,
            CompatSurface::FdPassing,
            CompatSurface::SocketLoopback,
            CompatSurface::Timer,
            CompatSurface::CallGate,
            CompatSurface::Signal,
            CompatSurface::ResourceDomain,
            CompatSurface::Errno,
        ];
        for surface in surfaces {
            assert!(
                !lowering_for(surface).is_empty(),
                "missing lowering for {surface:?}"
            );
        }
    }

    #[test]
    fn netbsd_system_gate_canonical_native_primitives_cover_runner_requirements() {
        let personality_abi = include_str!("../netbsd_personality_abi.md");
        let system_gate = include_str!("../scripts/run_netbsd_personality_system.sh");

        fn gate_has(
            surfaces: &[CompatSurface],
            mut required: impl FnMut(&NativePrimitive) -> bool,
        ) -> bool {
            surfaces
                .iter()
                .flat_map(|surface| lowering_for(*surface))
                .any(|primitive| required(primitive))
        }

        let surfaces = [
            CompatSurface::CwdRoot,
            CompatSurface::Open,
            CompatSurface::Read,
            CompatSurface::Write,
            CompatSurface::Close,
            CompatSurface::Pipe,
            CompatSurface::PollSelectEpoll,
            CompatSurface::Fork,
            CompatSurface::Exec,
            CompatSurface::PthreadCreate,
            CompatSurface::Mmap,
            CompatSurface::FdPassing,
            CompatSurface::SocketLoopback,
            CompatSurface::Timer,
            CompatSurface::CallGate,
            CompatSurface::Signal,
            CompatSurface::ResourceDomain,
        ];

        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::OpenAt));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Pull));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Push));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Close));
        assert!(gate_has(&surfaces, |primitive| matches!(
            primitive,
            NativePrimitive::ObjectCtl { .. }
        )));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Await));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Exec));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Mmap));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::Mprotect));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Munmap));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::CapabilityDuplicate));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::CapabilitySend));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::CapabilityRecv));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::DomainCtl));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::GateCall));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::GateReturn));
        assert!(gate_has(&surfaces, |primitive| {
            *primitive
                == NativePrimitive::Clone {
                    profile: CloneProfile::NewProcessCow,
                }
        }));
        assert!(gate_has(&surfaces, |primitive| {
            *primitive
                == NativePrimitive::Clone {
                    profile: CloneProfile::NewThreadSharedVm,
                }
        }));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::EventDelivery));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::AbiSignalFrame));
        assert!(
            system_gate.contains(
                "netbsd_system_gate_canonical_native_primitives_cover_runner_requirements"
            )
        );
        for (_, token) in NETBSD_PERSONALITY_DENIED_ESCAPES {
            assert!(
                personality_abi.contains(token),
                "NetBSD ABI doc must keep denied escape visible: {token}"
            );
        }
    }

    #[test]
    fn compatibility_surfaces_have_layer_policy() {
        for entry in COMPATIBILITY_LOWERINGS {
            assert!(
                layer_for(entry.surface).is_some(),
                "missing layer policy for {:?}",
                entry.surface
            );
        }
        for (idx, entry) in COMPATIBILITY_LOWERINGS.iter().enumerate() {
            assert!(
                !COMPATIBILITY_LOWERINGS[..idx]
                    .iter()
                    .any(|seen| seen.surface == entry.surface),
                "duplicate lowering for {:?}",
                entry.surface
            );
        }
        for policy in COMPATIBILITY_SURFACE_POLICIES {
            assert!(
                !lowering_for(policy.surface).is_empty(),
                "missing lowering for policy surface {:?}",
                policy.surface
            );
        }
        for (idx, policy) in COMPATIBILITY_SURFACE_POLICIES.iter().enumerate() {
            assert!(
                !COMPATIBILITY_SURFACE_POLICIES[..idx]
                    .iter()
                    .any(|seen| seen.surface == policy.surface),
                "duplicate layer policy for {:?}",
                policy.surface
            );
        }
        assert_eq!(
            layer_for(CompatSurface::Errno),
            Some(CompatibilityLayer::RuntimeLibc)
        );
        assert_eq!(
            layer_for(CompatSurface::Signal),
            Some(CompatibilityLayer::Personality)
        );
        assert_eq!(
            layer_for(CompatSurface::ResourceDomain),
            Some(CompatibilityLayer::Native)
        );
    }

    #[test]
    fn netbsd_syscall_numbers_route_to_compat_surfaces() {
        assert_eq!(
            netbsd_syscall(2).map(|entry| entry.surface),
            Some(CompatSurface::Fork)
        );
        assert_eq!(
            netbsd_syscall(3).map(|entry| entry.surface),
            Some(CompatSurface::Read)
        );
        assert_eq!(
            netbsd_syscall(4).map(|entry| entry.surface),
            Some(CompatSurface::Write)
        );
        assert_eq!(
            netbsd_syscall(5).map(|entry| entry.surface),
            Some(CompatSurface::Open)
        );
        assert_eq!(
            netbsd_syscall(42).map(|entry| entry.surface),
            Some(CompatSurface::Pipe)
        );
        assert_eq!(
            netbsd_syscall(197).map(|entry| entry.surface),
            Some(CompatSurface::Mmap)
        );
        assert_eq!(
            netbsd_syscall(340).map(|entry| entry.surface),
            Some(CompatSurface::Signal)
        );
        assert_eq!(
            netbsd_syscall(468).map(|entry| entry.surface),
            Some(CompatSurface::Open)
        );
        assert!(netbsd_syscall_lowering(54).is_empty());
    }

    #[test]
    fn netbsd_syscall_dispatch_is_layered_over_native_lowerings() {
        for entry in NETBSD_SYSCALLS {
            assert_eq!(
                Some(entry.layer),
                layer_for(entry.surface),
                "layer mismatch for {}",
                entry.name
            );
            assert!(
                !netbsd_syscall_lowering(entry.number).is_empty(),
                "missing native lowering for {}",
                entry.name
            );
        }
    }

    #[test]
    fn netbsd_system_gate_syscalls_are_registered() {
        let names = [
            "fork",
            "read",
            "write",
            "open",
            "openat",
            "close",
            "compat_50_wait4",
            "__wait450",
            "chdir",
            "fchdir",
            "__getcwd",
            "chmod",
            "dup",
            "dup2",
            "fcntl",
            "pipe",
            "pipe2",
            "execve",
            "fexecve",
            "mmap",
            "mprotect",
            "munmap",
            "poll",
            "__select50",
            "epoll_create1",
            "epoll_ctl",
            "epoll_pwait2",
            "timerfd_create",
            "timerfd_settime",
            "timerfd_gettime",
            "__nanosleep50",
            "_lwp_create",
            "__socket30",
            "bind",
            "listen",
            "connect",
            "accept",
            "recvfrom",
            "sendto",
            "sendmsg",
            "recvmsg",
            "getsockname",
            "getsockopt",
            "setsockopt",
            "__sigaction_sigtramp",
            "__sigprocmask14",
            "kill",
            "compat_16___sigreturn14",
        ];
        for name in names {
            assert!(
                netbsd_syscall_by_name(name).is_some(),
                "missing NetBSD syscall dispatch entry for {name}"
            );
        }
    }
}
