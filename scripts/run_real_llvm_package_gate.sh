#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

build_dir="${LNP64_LLVM_BUILD_DIR:-target/llvm-lnp64-build}"
package_filter="${LNP64_LLVM_PACKAGE_FILTER:-all}"

split_filters() {
  local raw="$1"
  raw="${raw//,/ }"
  printf '%s\n' $raw
}

needed_elfs=()
for package in $(split_filters "$package_filter"); do
  case "$package" in
    all)
      needed_elfs+=(
        "$build_dir/lnp64-zlib-linked.elf"
        "$build_dir/lnp64-natsort-linked.elf"
        "$build_dir/lnp64-jsmn-linked.elf"
        "$build_dir/lnp64-inih-linked.elf"
        "$build_dir/lnp64-cwalk-linked.elf"
        "$build_dir/lnp64-sbase-echo-linked.elf"
        "$build_dir/lnp64-sbase-basename-linked.elf"
        "$build_dir/lnp64-sbase-dirname-linked.elf"
        "$build_dir/lnp64-sbase-cat-linked.elf"
        "$build_dir/lnp64-userland-ucat-linked.elf"
        "$build_dir/lnp64-userland-init-linked.elf"
        "$build_dir/lnp64-userland-lnpsh-linked.elf"
        "$build_dir/lnp64-userland-spawn-task-linked.elf"
        "$build_dir/lnp64-netbsd-init-linked.elf"
        "$build_dir/lnp64-netbsd-sh-linked.elf"
        "$build_dir/lnp64-netbsd-personality-clang-linked.elf"
        "$build_dir/lnp64-netbsd-loader-target-linked.elf"
        "$build_dir/lnp64-netbsd-elf-exec-test-linked.elf"
        "$build_dir/lnp64-netbsd-fork-wait-test-linked.elf"
        "$build_dir/lnp64-netbsd-thread-test-linked.elf"
        "$build_dir/lnp64-netbsd-poll-test-linked.elf"
        "$build_dir/lnp64-netbsd-signal-gate-test-linked.elf"
        "$build_dir/lnp64-netbsd-signal-fault-test-linked.elf"
        "$build_dir/lnp64-netbsd-timer-test-linked.elf"
        "$build_dir/lnp64-netbsd-mmap-test-linked.elf"
        "$build_dir/lnp64-netbsd-fd-passing-test-linked.elf"
        "$build_dir/lnp64-netbsd-namespace-test-linked.elf"
        "$build_dir/lnp64-netbsd-fs-service-test-linked.elf"
        "$build_dir/lnp64-netbsd-classifier-test-linked.elf"
        "$build_dir/lnp64-netbsd-socket-loopback-test-linked.elf"
        "$build_dir/lnp64-netbsd-gate-trace-test-linked.elf"
        "$build_dir/lnp64-netbsd-domain-nested-test-linked.elf"
        "$build_dir/lnp64-netbsd-domain-budget-test-linked.elf"
      )
      ;;
    zlib|natsort|jsmn|inih|cwalk)
      needed_elfs+=("$build_dir/lnp64-$package-linked.elf")
      ;;
    sbase)
      needed_elfs+=(
        "$build_dir/lnp64-sbase-echo-linked.elf"
        "$build_dir/lnp64-sbase-basename-linked.elf"
        "$build_dir/lnp64-sbase-dirname-linked.elf"
        "$build_dir/lnp64-sbase-cat-linked.elf"
      )
      ;;
    userland)
      needed_elfs+=(
        "$build_dir/lnp64-userland-ucat-linked.elf"
        "$build_dir/lnp64-userland-init-linked.elf"
        "$build_dir/lnp64-userland-lnpsh-linked.elf"
        "$build_dir/lnp64-userland-spawn-task-linked.elf"
      )
      ;;
    netbsd)
      needed_elfs+=(
        "$build_dir/lnp64-netbsd-init-linked.elf"
        "$build_dir/lnp64-netbsd-sh-linked.elf"
        "$build_dir/lnp64-netbsd-personality-clang-linked.elf"
        "$build_dir/lnp64-netbsd-loader-target-linked.elf"
        "$build_dir/lnp64-netbsd-elf-exec-test-linked.elf"
        "$build_dir/lnp64-netbsd-fork-wait-test-linked.elf"
        "$build_dir/lnp64-netbsd-thread-test-linked.elf"
        "$build_dir/lnp64-netbsd-poll-test-linked.elf"
        "$build_dir/lnp64-netbsd-signal-gate-test-linked.elf"
        "$build_dir/lnp64-netbsd-signal-fault-test-linked.elf"
        "$build_dir/lnp64-netbsd-timer-test-linked.elf"
        "$build_dir/lnp64-netbsd-mmap-test-linked.elf"
        "$build_dir/lnp64-netbsd-fd-passing-test-linked.elf"
        "$build_dir/lnp64-netbsd-namespace-test-linked.elf"
        "$build_dir/lnp64-netbsd-fs-service-test-linked.elf"
        "$build_dir/lnp64-netbsd-classifier-test-linked.elf"
        "$build_dir/lnp64-netbsd-socket-loopback-test-linked.elf"
        "$build_dir/lnp64-netbsd-gate-trace-test-linked.elf"
        "$build_dir/lnp64-netbsd-domain-nested-test-linked.elf"
        "$build_dir/lnp64-netbsd-domain-budget-test-linked.elf"
      )
      ;;
    *)
      printf 'unknown LNP64_LLVM_PACKAGE_FILTER item: %s\n' "$package" >&2
      printf 'expected one or more of: all,zlib,natsort,jsmn,inih,cwalk,sbase,userland,netbsd\n' >&2
      exit 2
      ;;
  esac
done

artifacts_ready=1
for elf in "${needed_elfs[@]}"; do
  if [[ ! -s "$elf" ]]; then
    artifacts_ready=0
    break
  fi
done

if [[ "$artifacts_ready" != "1" ]]; then
  if [[ -z "${LNP64_LLVM_DOCKER_SKIP_BUILD:-}" &&
    -x "$build_dir/bin/clang" &&
    -x "$build_dir/bin/ld.lld" ]]; then
    export LNP64_LLVM_DOCKER_SKIP_BUILD=1
  fi
  bash scripts/run_real_llvm_lnp64_docker.sh
  printf '%s\n' "real LLVM package artifacts refreshed by Docker gate"
fi

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64_bin="$LNP64_BIN"
else
  cargo build --quiet --bin lnp64
  lnp64_bin="${CARGO_TARGET_DIR:-target}/debug/lnp64"
fi
if [[ ! -x "$lnp64_bin" ]]; then
  printf 'missing lnp64 binary: %s\n' "$lnp64_bin" >&2
  exit 1
fi

run_elf_probe() {
  local linked_probe="$1"
  shift
  local run_args=()
  local has_arg_marker=0
  local item
  for item in "$@"; do
    if [[ "$item" == "--expect" ]]; then
      has_arg_marker=1
      break
    fi
  done
  if [[ "$has_arg_marker" == "1" ]]; then
    while [[ "$#" -gt 0 && "$1" != "--expect" ]]; do
      run_args+=("$1")
      shift
    done
    shift
  fi
  "$lnp64_bin" elf-plan "$linked_probe" >/dev/null
  local run_elf_output
  run_elf_output="$("$lnp64_bin" run-elf "$linked_probe" "${run_args[@]}")"
  grep -q 'exit=0' <<<"$run_elf_output"
  local expected
  for expected in "$@"; do
    grep -q "$expected" <<<"$run_elf_output"
  done
}

run_elf_report() {
  local message="$1"
  local linked_probe="$2"
  shift 2
  run_elf_probe "$linked_probe" "$@"
  printf '%s: %s\n' "$message" "$linked_probe"
}

run_package() {
  local package="$1"
  case "$package" in
    zlib|natsort|jsmn|inih|cwalk)
      run_elf_report "real LLVM LNP64 run-elf $package package execution passed" \
        "$build_dir/lnp64-$package-linked.elf"
      ;;
    sbase)
      run_elf_report "real LLVM LNP64 run-elf sbase echo execution passed" \
        "$build_dir/lnp64-sbase-echo-linked.elf" \
        echo hello clang --expect 'hello clang'
      run_elf_report "real LLVM LNP64 run-elf sbase basename execution passed" \
        "$build_dir/lnp64-sbase-basename-linked.elf" \
        basename /usr/local/bin/clang --expect '^clang$'
      run_elf_report "real LLVM LNP64 run-elf sbase dirname execution passed" \
        "$build_dir/lnp64-sbase-dirname-linked.elf" \
        dirname /usr/local/bin/clang --expect '^/usr/local/bin$'
      local sbase_fixture_root="$build_dir/sbase-fixture-root"
      mkdir -p "$sbase_fixture_root/input"
      printf 'cat via clang\n' >"$sbase_fixture_root/input/cat.txt"
      "$lnp64_bin" elf-plan "$build_dir/lnp64-sbase-cat-linked.elf" >/dev/null
      local sbase_cat_output
      sbase_cat_output="$("$lnp64_bin" run-elf --namespace-root "$sbase_fixture_root" \
        "$build_dir/lnp64-sbase-cat-linked.elf" cat input/cat.txt)"
      grep -q '^cat via clang$' <<<"$sbase_cat_output"
      grep -q 'exit=0' <<<"$sbase_cat_output"
      printf 'real LLVM LNP64 run-elf sbase cat execution passed: %s\n' \
        "$build_dir/lnp64-sbase-cat-linked.elf"
      ;;
    userland)
      local userland_fixture_root="$build_dir/userland-fixture-root"
      mkdir -p "$userland_fixture_root/dev" "$userland_fixture_root/etc"
      printf 'welcome from clang ucat\n' >"$userland_fixture_root/etc/motd"
      printf 'console\nnull\nrandom\n' >"$userland_fixture_root/dev/devices"
      "$lnp64_bin" elf-plan "$build_dir/lnp64-userland-ucat-linked.elf" >/dev/null
      local userland_ucat_output
      userland_ucat_output="$("$lnp64_bin" run-elf --namespace-root "$userland_fixture_root" \
        "$build_dir/lnp64-userland-ucat-linked.elf" ucat etc/motd)"
      grep -q '^welcome from clang ucat$' <<<"$userland_ucat_output"
      grep -q 'exit=0' <<<"$userland_ucat_output"
      printf 'real LLVM LNP64 run-elf userland ucat execution passed: %s\n' \
        "$build_dir/lnp64-userland-ucat-linked.elf"
      "$lnp64_bin" elf-plan "$build_dir/lnp64-userland-init-linked.elf" >/dev/null
      local userland_init_output
      userland_init_output="$("$lnp64_bin" run-elf --namespace-root "$userland_fixture_root" \
        "$build_dir/lnp64-userland-init-linked.elf" init /)"
      grep -q '^lnp64 clang init: boot$' <<<"$userland_init_output"
      grep -q '^lnp64 clang init: root /$' <<<"$userland_init_output"
      grep -q '^welcome from clang ucat$' <<<"$userland_init_output"
      grep -q 'exit=0' <<<"$userland_init_output"
      printf 'real LLVM LNP64 run-elf userland init execution passed: %s\n' \
        "$build_dir/lnp64-userland-init-linked.elf"
      "$lnp64_bin" elf-plan "$build_dir/lnp64-userland-lnpsh-linked.elf" >/dev/null
      local userland_lnpsh_output
      userland_lnpsh_output="$("$lnp64_bin" run-elf --namespace-root "$userland_fixture_root" \
        "$build_dir/lnp64-userland-lnpsh-linked.elf" lnpsh)"
      grep -q '^lnpsh clang: scripted console$' <<<"$userland_lnpsh_output"
      grep -q '^welcome from clang ucat$' <<<"$userland_lnpsh_output"
      grep -q '^console$' <<<"$userland_lnpsh_output"
      grep -q '^random$' <<<"$userland_lnpsh_output"
      grep -q 'exit=0' <<<"$userland_lnpsh_output"
      printf 'real LLVM LNP64 run-elf userland lnpsh execution passed: %s\n' \
        "$build_dir/lnp64-userland-lnpsh-linked.elf"
      "$lnp64_bin" elf-plan "$build_dir/lnp64-userland-spawn-task-linked.elf" >/dev/null
      local userland_spawn_output
      userland_spawn_output="$("$lnp64_bin" run-elf \
        "$build_dir/lnp64-userland-spawn-task-linked.elf" spawn-task)"
      grep -q '^userland spawn: parent$' <<<"$userland_spawn_output"
      grep -q '^userland spawn: child$' <<<"$userland_spawn_output"
      grep -q '^userland spawn: joined$' <<<"$userland_spawn_output"
      grep -q 'exit=0' <<<"$userland_spawn_output"
      printf 'real LLVM LNP64 run-elf userland spawn task execution passed: %s\n' \
        "$build_dir/lnp64-userland-spawn-task-linked.elf"
      ;;
    netbsd)
      run_elf_report "real LLVM LNP64 run-elf NetBSD personality clang smoke passed" \
        "$build_dir/lnp64-netbsd-personality-clang-linked.elf" \
        'netbsd clang personality init' \
        'netbsd clang personality shell' \
        'netbsd clang personality smoke ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD loader target child passed" \
        "$build_dir/lnp64-netbsd-loader-target-linked.elf" \
        'loader_target ok'
      local netbsd_elf_exec_fixture_root="$build_dir/netbsd-elf-exec-fixture-root"
      rm -rf "$netbsd_elf_exec_fixture_root"
      mkdir -p "$netbsd_elf_exec_fixture_root/bin"
      cp "$build_dir/lnp64-netbsd-loader-target-linked.elf" \
        "$netbsd_elf_exec_fixture_root/bin/loader_target.elf"
      "$lnp64_bin" elf-plan "$build_dir/lnp64-netbsd-elf-exec-test-linked.elf" \
        >/dev/null
      local netbsd_elf_exec_output
      netbsd_elf_exec_output="$("$lnp64_bin" run-elf --namespace-root \
        "$netbsd_elf_exec_fixture_root" \
        "$build_dir/lnp64-netbsd-elf-exec-test-linked.elf")"
      grep -q 'loader_target ok' <<<"$netbsd_elf_exec_output"
      grep -q 'elf_exec_test ok' <<<"$netbsd_elf_exec_output"
      grep -q 'exit=0' <<<"$netbsd_elf_exec_output"
      printf 'real LLVM LNP64 run-elf NetBSD ELF exec parent passed: %s\n' \
        "$build_dir/lnp64-netbsd-elf-exec-test-linked.elf"
      run_elf_report "real LLVM LNP64 run-elf NetBSD fork/wait child passed" \
        "$build_dir/lnp64-netbsd-fork-wait-test-linked.elf" \
        'fork_wait_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD thread child passed" \
        "$build_dir/lnp64-netbsd-thread-test-linked.elf" \
        'thread_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD poll child passed" \
        "$build_dir/lnp64-netbsd-poll-test-linked.elf" \
        'poll_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD signal gate child passed" \
        "$build_dir/lnp64-netbsd-signal-gate-test-linked.elf" \
        'signal_gate_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD signal fault child passed" \
        "$build_dir/lnp64-netbsd-signal-fault-test-linked.elf" \
        'signal_fault_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD timer child passed" \
        "$build_dir/lnp64-netbsd-timer-test-linked.elf" \
        'timer_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD mmap child passed" \
        "$build_dir/lnp64-netbsd-mmap-test-linked.elf" \
        'mmap_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD fd passing child passed" \
        "$build_dir/lnp64-netbsd-fd-passing-test-linked.elf" \
        'fd_passing_test ok'
      local netbsd_namespace_fixture_root="$build_dir/netbsd-namespace-fixture-root"
      rm -rf "$netbsd_namespace_fixture_root"
      mkdir -p "$netbsd_namespace_fixture_root/etc" \
        "$netbsd_namespace_fixture_root/tmp"
      printf 'welcome\n' >"$netbsd_namespace_fixture_root/etc/motd"
      "$lnp64_bin" elf-plan "$build_dir/lnp64-netbsd-namespace-test-linked.elf" \
        >/dev/null
      local netbsd_namespace_output
      netbsd_namespace_output="$("$lnp64_bin" run-elf --namespace-root \
        "$netbsd_namespace_fixture_root" \
        "$build_dir/lnp64-netbsd-namespace-test-linked.elf")"
      grep -q 'namespace_test ok' <<<"$netbsd_namespace_output"
      grep -q 'exit=0' <<<"$netbsd_namespace_output"
      printf 'real LLVM LNP64 run-elf NetBSD namespace child passed: %s\n' \
        "$build_dir/lnp64-netbsd-namespace-test-linked.elf"
      local netbsd_fixture_root="$build_dir/netbsd-fixture-root"
      rm -rf "$netbsd_fixture_root"
      mkdir -p "$netbsd_fixture_root/etc" "$netbsd_fixture_root/tmp"
      local netbsd_fs_image="$netbsd_fixture_root/etc/netbsd_personality.fs"
      truncate -s 512 "$netbsd_fs_image"
      put_netbsd_fs_image() {
        local offset="$1"
        local bytes="$2"
        printf '%b' "$bytes" | dd of="$netbsd_fs_image" bs=1 seek="$offset" \
          conv=notrunc status=none
      }
      put_netbsd_fs_image 0 'LNPFS2\n0'
      put_netbsd_fs_image 64 '1d11/\0'
      put_netbsd_fs_image 100 'x'
      put_netbsd_fs_image 128 '1d11/etc\0'
      put_netbsd_fs_image 164 'x'
      put_netbsd_fs_image 192 '1f11/etc/motd\0'
      put_netbsd_fs_image 228 'r'
      put_netbsd_fs_image 232 'welcome\0'
      put_netbsd_fs_image 256 '1d11/tmp\0'
      put_netbsd_fs_image 292 'x'
      "$lnp64_bin" elf-plan "$build_dir/lnp64-netbsd-fs-service-test-linked.elf" \
        >/dev/null
      local netbsd_fs_service_output
      netbsd_fs_service_output="$("$lnp64_bin" run-elf --namespace-root \
        "$netbsd_fixture_root" \
        "$build_dir/lnp64-netbsd-fs-service-test-linked.elf")"
      grep -q 'fs_service_test ok' <<<"$netbsd_fs_service_output"
      grep -q 'exit=0' <<<"$netbsd_fs_service_output"
      printf 'real LLVM LNP64 run-elf NetBSD fs service child passed: %s\n' \
        "$build_dir/lnp64-netbsd-fs-service-test-linked.elf"
      run_elf_report "real LLVM LNP64 run-elf NetBSD classifier child passed" \
        "$build_dir/lnp64-netbsd-classifier-test-linked.elf" \
        'classifier_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD socket loopback child passed" \
        "$build_dir/lnp64-netbsd-socket-loopback-test-linked.elf" \
        'socket_loopback_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD gate trace child passed" \
        "$build_dir/lnp64-netbsd-gate-trace-test-linked.elf" \
        'gate_trace_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD domain nested child passed" \
        "$build_dir/lnp64-netbsd-domain-nested-test-linked.elf" \
        'domain_nested_test ok'
      run_elf_report "real LLVM LNP64 run-elf NetBSD domain budget child passed" \
        "$build_dir/lnp64-netbsd-domain-budget-test-linked.elf" \
        'domain_budget_test ok'
      local netbsd_system_root="$build_dir/netbsd-system-fixture-root"
      rm -rf "$netbsd_system_root"
      mkdir -p "$netbsd_system_root/bin" "$netbsd_system_root/etc" \
        "$netbsd_system_root/tmp"
      printf 'welcome\n' >"$netbsd_system_root/etc/motd"
      local netbsd_system_fs_image="$netbsd_system_root/etc/netbsd_personality.fs"
      truncate -s 512 "$netbsd_system_fs_image"
      put_netbsd_system_fs_image() {
        local offset="$1"
        local bytes="$2"
        printf '%b' "$bytes" | dd of="$netbsd_system_fs_image" bs=1 \
          seek="$offset" conv=notrunc status=none
      }
      put_netbsd_system_fs_image 0 'LNPFS2\n0'
      put_netbsd_system_fs_image 64 '1d11/\0'
      put_netbsd_system_fs_image 100 'x'
      put_netbsd_system_fs_image 128 '1d11/etc\0'
      put_netbsd_system_fs_image 164 'x'
      put_netbsd_system_fs_image 192 '1f11/etc/motd\0'
      put_netbsd_system_fs_image 228 'r'
      put_netbsd_system_fs_image 232 'welcome\0'
      put_netbsd_system_fs_image 256 '1d11/tmp\0'
      put_netbsd_system_fs_image 292 'x'
      cp "$build_dir/lnp64-netbsd-sh-linked.elf" \
        "$netbsd_system_root/bin/netbsd_sh.elf"
      cp "$build_dir/lnp64-netbsd-loader-target-linked.elf" \
        "$netbsd_system_root/bin/loader_target.elf"
      cp "$build_dir/lnp64-netbsd-elf-exec-test-linked.elf" \
        "$netbsd_system_root/bin/elf_exec_test.elf"
      cp "$build_dir/lnp64-netbsd-fork-wait-test-linked.elf" \
        "$netbsd_system_root/bin/fork_wait_test.elf"
      cp "$build_dir/lnp64-netbsd-thread-test-linked.elf" \
        "$netbsd_system_root/bin/thread_test.elf"
      cp "$build_dir/lnp64-netbsd-poll-test-linked.elf" \
        "$netbsd_system_root/bin/poll_test.elf"
      cp "$build_dir/lnp64-netbsd-signal-gate-test-linked.elf" \
        "$netbsd_system_root/bin/signal_gate_test.elf"
      cp "$build_dir/lnp64-netbsd-signal-fault-test-linked.elf" \
        "$netbsd_system_root/bin/signal_fault_test.elf"
      cp "$build_dir/lnp64-netbsd-timer-test-linked.elf" \
        "$netbsd_system_root/bin/timer_test.elf"
      cp "$build_dir/lnp64-netbsd-mmap-test-linked.elf" \
        "$netbsd_system_root/bin/mmap_test.elf"
      cp "$build_dir/lnp64-netbsd-fd-passing-test-linked.elf" \
        "$netbsd_system_root/bin/fd_passing_test.elf"
      cp "$build_dir/lnp64-netbsd-namespace-test-linked.elf" \
        "$netbsd_system_root/bin/namespace_test.elf"
      cp "$build_dir/lnp64-netbsd-fs-service-test-linked.elf" \
        "$netbsd_system_root/bin/fs_service_test.elf"
      cp "$build_dir/lnp64-netbsd-classifier-test-linked.elf" \
        "$netbsd_system_root/bin/classifier_test.elf"
      cp "$build_dir/lnp64-netbsd-socket-loopback-test-linked.elf" \
        "$netbsd_system_root/bin/socket_loopback_test.elf"
      cp "$build_dir/lnp64-netbsd-gate-trace-test-linked.elf" \
        "$netbsd_system_root/bin/gate_trace_test.elf"
      cp "$build_dir/lnp64-netbsd-domain-nested-test-linked.elf" \
        "$netbsd_system_root/bin/domain_nested_test.elf"
      cp "$build_dir/lnp64-netbsd-domain-budget-test-linked.elf" \
        "$netbsd_system_root/bin/domain_budget_test.elf"
      "$lnp64_bin" elf-plan "$build_dir/lnp64-netbsd-init-linked.elf" \
        >/dev/null
      local netbsd_system_output
      netbsd_system_output="$("$lnp64_bin" run-elf --namespace-root \
        "$netbsd_system_root" "$build_dir/lnp64-netbsd-init-linked.elf" init /)"
      grep -q 'lnp64-netbsd-personality: supervisor boot' \
        <<<"$netbsd_system_output"
      grep -q 'fork_wait_test ok' <<<"$netbsd_system_output"
      grep -q 'loader_target ok' <<<"$netbsd_system_output"
      grep -q 'elf_exec_test ok' <<<"$netbsd_system_output"
      grep -q 'netbsd personality system ok' <<<"$netbsd_system_output"
      grep -q 'exit=0' <<<"$netbsd_system_output"
      printf 'real LLVM LNP64 run-elf NetBSD init/shell system passed: %s\n' \
        "$build_dir/lnp64-netbsd-init-linked.elf"
      ;;
    all)
      for selected in zlib natsort jsmn inih cwalk sbase userland netbsd; do
        run_package "$selected"
      done
      ;;
  esac
}

for package in $(split_filters "$package_filter"); do
  run_package "$package"
done

printf 'real LLVM package gate ok: %s\n' "$package_filter"
