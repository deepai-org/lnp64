"""Shared executable mirror of the M1 top-level RTL-to-Lean refinement step.

This module is the single source of the packed-bit codec, the LNP64 rights
model, and the `check_top_m1_refinement_step` relation used by the top-level
program smoke (`scripts/run_rtl_top_program_smoke.sh`) and by the standalone
witness checker (`scripts/check_rtl_top_m1_witness.py`). Keeping one copy means
the executable refinement relation cannot drift between the producer that emits
the `lnp64_top_m1_refinement_witness_v1` artifact and the consumer that
re-checks it offline.

The functions raise `SystemExit` on the first violation so both the simulation
gate and the offline checker fail closed with the same message.
"""

import hashlib
import json

# Fields in the M1 state projection that are taxonomy/outcome markers rather
# than authority-bearing projections. Everything else in the projection is
# compared for unauthorized change across a transition.
NON_AUTHORITY_STATE_FIELDS = frozenset(
    {
        "op",
        "status",
        "stale_rejected",
        "revoked_rejected",
        "failed_no_authority",
        "full_was_explicit",
    }
)

# Effective top-level RTL rights-mask bits.
TOP_RIGHT_PULL = 0x1
TOP_RIGHT_PUSH = 0x2
TOP_RIGHT_DUP = 0x40
TOP_RIGHT_TRANSFER = 0x100

# Modeled M1 rights bits used by the Lean/Python mirror.
MODELED_RIGHT_PUSH = 0x1
MODELED_RIGHT_PULL = 0x2
MODELED_RIGHT_DUP = 0x4


def authority_projection_fields(state_fields) -> tuple[str, ...]:
    """Projection fields whose change a transition must justify."""
    return tuple(field for field in state_fields if field and field not in NON_AUTHORITY_STATE_FIELDS)


def decode_packed_bits(bits: str, fields: tuple[str, ...], widths: tuple[int, ...]) -> dict[str, int]:
    total_width = sum(widths)
    try:
        raw = int(bits, 16)
    except ValueError as exc:
        raise SystemExit(f"invalid top-level M1 packed commit bits {bits!r}") from exc
    if raw >= (1 << total_width):
        raise SystemExit(
            f"top-level M1 packed commit bits exceed schema width {total_width}: 0x{bits}"
        )
    decoded = {}
    shift = total_width
    for field, width in zip(fields, widths, strict=True):
        shift -= width
        decoded[field] = (raw >> shift) & ((1 << width) - 1)
    if shift != 0:
        raise SystemExit("internal top-level M1 packed commit decoder did not consume all bits")
    return decoded


def sha256_json(data: object) -> str:
    payload = json.dumps(data, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(payload).hexdigest()


def rights_subset(child: int, parent: int) -> bool:
    return (child & ~parent) == 0


def top_rights_to_modeled_mask(raw: int) -> int:
    modeled = 0
    if raw & TOP_RIGHT_PUSH:
        modeled |= MODELED_RIGHT_PUSH
    if raw & TOP_RIGHT_PULL:
        modeled |= MODELED_RIGHT_PULL
    if raw & TOP_RIGHT_DUP:
        modeled |= MODELED_RIGHT_DUP
    return modeled


def modeled_rights(record: dict, field: str) -> int:
    value = record.get(field)
    if not isinstance(value, int) or value < 0:
        raise SystemExit(f"top-level M1 field {field} is not a nonnegative rights mask: {value!r}")
    return top_rights_to_modeled_mask(value)


def check_top_m1_optional_cap_zero(
    prefix: str,
    state: dict,
    idx: int,
    label: str,
) -> None:
    nonzero = [
        field
        for field in (
            f"{prefix}_object_id",
            f"{prefix}_generation",
            f"{prefix}_domain_id",
            f"{prefix}_lineage_epoch",
            f"{prefix}_sealed",
            f"{prefix}_rights",
        )
        if state[field] != 0
    ]
    if nonzero:
        raise SystemExit(
            f"top-level M1 {label} state {idx} invalid {prefix} projection "
            f"retained authority field(s): {nonzero}"
        )


def check_top_m1_optional_authority_slots(
    state: dict,
    idx: int,
    label: str,
) -> None:
    if state["sent_valid"] not in (0, 1):
        raise SystemExit(
            f"top-level M1 {label} state {idx} has non-boolean sent_valid "
            f"{state['sent_valid']!r}"
        )
    if state["minted_valid"] not in (0, 1):
        raise SystemExit(
            f"top-level M1 {label} state {idx} has non-boolean minted_valid "
            f"{state['minted_valid']!r}"
        )
    if state["sent_valid"] == 0:
        check_top_m1_optional_cap_zero("sent", state, idx, label)
    elif state["transfer_valid"] != 1:
        raise SystemExit(
            f"top-level M1 {label} state {idx} has sent_valid without transfer_valid"
        )
    if state["minted_valid"] == 0:
        check_top_m1_optional_cap_zero("minted", state, idx, label)
        if state["created_object_created"] != 0 or state["created_object_gen"] != 0:
            raise SystemExit(
                f"top-level M1 {label} state {idx} invalid minted projection "
                "retained created-object witness"
            )


def check_top_m1_projection_matches_commit(
    prefix: str,
    commit: dict,
    state: dict,
    idx: int,
    transition: str,
) -> None:
    for state_field, commit_field in (
        (f"{prefix}_object_id", "object_id"),
        (f"{prefix}_generation", "fdr_gen"),
        (f"{prefix}_domain_id", "domain_id"),
        (f"{prefix}_rights", "rights_mask"),
        (f"{prefix}_lineage_epoch", "lineage_epoch"),
        (f"{prefix}_sealed", "sealed"),
    ):
        if state[state_field] != commit[commit_field]:
            raise SystemExit(
                f"top-level M1 {transition} {idx} {prefix} projection {state_field} "
                f"does not match commit {commit_field}: "
                f"state={state[state_field]} commit={commit[commit_field]}"
            )


def check_top_m1_non_ok_transition(
    idx: int,
    commit: dict,
    pre_state: dict,
    post_state: dict,
    authority_fields: tuple[str, ...],
) -> None:
    drift = [
        (field, pre_state[field], post_state[field])
        for field in authority_fields
        if pre_state[field] != post_state[field]
    ]
    if drift:
        raise SystemExit(
            f"top-level M1 non-OK commit {idx} changed authority projection: {drift}"
        )
    status = commit["status"]
    if status == 116 and post_state["stale_rejected"] != 1:
        raise SystemExit(f"top-level M1 non-OK commit {idx} did not mark stale rejection")
    if status in (1, 9) and post_state["failed_no_authority"] != 1:
        raise SystemExit(f"top-level M1 non-OK commit {idx} did not mark failed authority")
    if status == 11 and post_state["full_was_explicit"] != 1:
        raise SystemExit(f"top-level M1 non-OK commit {idx} did not mark explicit full queue")


def check_top_m1_authority_projection_unchanged(
    transition: str,
    idx: int,
    pre_state: dict,
    post_state: dict,
    authority_fields: tuple[str, ...],
) -> None:
    drift = [
        (field, pre_state[field], post_state[field])
        for field in authority_fields
        if pre_state[field] != post_state[field]
    ]
    if drift:
        raise SystemExit(f"top-level M1 {transition} {idx} changed authority projection: {drift}")


def check_top_m1_refinement_step(
    idx: int,
    commit: dict,
    pre_state: dict,
    post_state: dict,
    commit_ops: dict[str, int],
    authority_fields: tuple[str, ...],
) -> None:
    """Executable top-level mirror of the current M1 RTL-to-Lean step shape."""
    if commit["status"] != 0:
        check_top_m1_non_ok_transition(
            idx,
            commit,
            pre_state,
            post_state,
            authority_fields,
        )
        return

    op = commit["op"]
    if op == commit_ops["CapDup"]:
        if pre_state["root_rights"] & TOP_RIGHT_DUP == 0:
            raise SystemExit(f"top-level M1 capDup {idx} accepted without DUP right")
        if not rights_subset(commit["rights_mask"], pre_state["root_rights"]):
            raise SystemExit(
                f"top-level M1 capDup {idx} amplified rights: "
                f"commit={commit['rights_mask']} pre_root={pre_state['root_rights']}"
            )
        if not rights_subset(
            modeled_rights(commit, "rights_mask"),
            modeled_rights(pre_state, "root_rights"),
        ):
            raise SystemExit(
                f"top-level M1 capDup {idx} amplified modeled M1 rights: "
                f"commit={modeled_rights(commit, 'rights_mask')} "
                f"pre_root={modeled_rights(pre_state, 'root_rights')}"
            )
        if commit["object_id"] != pre_state["root_object_id"]:
            raise SystemExit(f"top-level M1 capDup {idx} changed object lineage")
        check_top_m1_projection_matches_commit("consumer", commit, post_state, idx, "capDup")
        return

    if op == commit_ops["CapSend"]:
        if pre_state["sent_valid"] != 0:
            raise SystemExit(f"top-level M1 capSend {idx} accepted with occupied transfer slot")
        if pre_state["consumer_rights"] & TOP_RIGHT_TRANSFER == 0:
            raise SystemExit(f"top-level M1 capSend {idx} accepted without TRANSFER right")
        if not rights_subset(commit["rights_mask"], pre_state["consumer_rights"]):
            raise SystemExit(
                f"top-level M1 capSend {idx} amplified rights: "
                f"commit={commit['rights_mask']} pre_consumer={pre_state['consumer_rights']}"
            )
        if not rights_subset(
            modeled_rights(commit, "rights_mask"),
            modeled_rights(pre_state, "consumer_rights"),
        ):
            raise SystemExit(
                f"top-level M1 capSend {idx} amplified modeled M1 rights: "
                f"commit={modeled_rights(commit, 'rights_mask')} "
                f"pre_consumer={modeled_rights(pre_state, 'consumer_rights')}"
            )
        if post_state["sent_valid"] != 1:
            raise SystemExit(f"top-level M1 capSend {idx} did not publish a sent cap")
        if post_state["transfer_valid"] != 1:
            raise SystemExit(f"top-level M1 capSend {idx} did not publish valid-transfer witness")
        check_top_m1_projection_matches_commit("sent", commit, post_state, idx, "capSend")
        return

    if op == commit_ops["CapRecv"]:
        if pre_state["sent_valid"] != 1:
            raise SystemExit(f"top-level M1 capRecv {idx} accepted without a sent cap")
        if not rights_subset(commit["rights_mask"], pre_state["sent_rights"]):
            raise SystemExit(
                f"top-level M1 capRecv {idx} amplified rights: "
                f"commit={commit['rights_mask']} pre_sent={pre_state['sent_rights']}"
            )
        if not rights_subset(
            modeled_rights(commit, "rights_mask"),
            modeled_rights(pre_state, "sent_rights"),
        ):
            raise SystemExit(
                f"top-level M1 capRecv {idx} amplified modeled M1 rights: "
                f"commit={modeled_rights(commit, 'rights_mask')} "
                f"pre_sent={modeled_rights(pre_state, 'sent_rights')}"
            )
        if post_state["sent_valid"] != 0:
            raise SystemExit(f"top-level M1 capRecv {idx} left a sent cap queued")
        if post_state["transfer_valid"] != 1:
            raise SystemExit(f"top-level M1 capRecv {idx} did not preserve valid-transfer witness")
        check_top_m1_projection_matches_commit("consumer", commit, post_state, idx, "capRecv")
        return

    if op == commit_ops["CapRevoke"]:
        if pre_state["root_rights"] & 0x80 == 0:
            raise SystemExit(f"top-level M1 capRevoke {idx} accepted without REVOKE right")
        if not rights_subset(commit["rights_mask"], pre_state["root_rights"]):
            raise SystemExit(
                f"top-level M1 capRevoke {idx} commit rights exceed pre root rights: "
                f"commit={commit['rights_mask']} pre_root={pre_state['root_rights']}"
            )
        if not rights_subset(
            modeled_rights(commit, "rights_mask"),
            modeled_rights(pre_state, "root_rights"),
        ):
            raise SystemExit(
                f"top-level M1 capRevoke {idx} amplified modeled M1 rights: "
                f"commit={modeled_rights(commit, 'rights_mask')} "
                f"pre_root={modeled_rights(pre_state, 'root_rights')}"
            )
        if commit["object_id"] != pre_state["root_object_id"]:
            raise SystemExit(f"top-level M1 capRevoke {idx} changed root object lineage")
        if commit["lineage_epoch"] != pre_state["root_lineage_epoch"]:
            raise SystemExit(f"top-level M1 capRevoke {idx} changed root lineage epoch")
        if post_state["has_revoked_generation"] != 1:
            raise SystemExit(f"top-level M1 capRevoke {idx} did not publish revoked generation")
        if post_state["object_gen"] != commit["fdr_gen"]:
            raise SystemExit(
                f"top-level M1 capRevoke {idx} object generation does not match commit: "
                f"post={post_state['object_gen']} commit={commit['fdr_gen']}"
            )
        if post_state["revoked_generation"] != commit["fdr_gen"]:
            raise SystemExit(
                f"top-level M1 capRevoke {idx} revoked generation does not match commit: "
                f"post={post_state['revoked_generation']} commit={commit['fdr_gen']}"
            )
        if post_state["root_generation"] != commit["fdr_gen"]:
            raise SystemExit(
                f"top-level M1 capRevoke {idx} root generation does not match commit: "
                f"post={post_state['root_generation']} commit={commit['fdr_gen']}"
            )
        if post_state["root_rights"] != 0:
            raise SystemExit(f"top-level M1 capRevoke {idx} left root authority live")
        return

    if op == commit_ops["ObjectCreate"]:
        if post_state["created_object_created"] != 1:
            raise SystemExit(f"top-level M1 objectCreate {idx} did not mark created object")
        if post_state["minted_valid"] != 1:
            raise SystemExit(f"top-level M1 objectCreate {idx} did not mint a cap")
        check_top_m1_projection_matches_commit("minted", commit, post_state, idx, "objectCreate")
        return

    if op == commit_ops["Push"]:
        if pre_state["root_rights"] & TOP_RIGHT_PUSH == 0:
            raise SystemExit(f"top-level M1 push {idx} accepted without PUSH right")
        check_top_m1_projection_matches_commit("root", commit, pre_state, idx, "push")
        check_top_m1_authority_projection_unchanged(
            "push",
            idx,
            pre_state,
            post_state,
            authority_fields,
        )
        return

    if op == commit_ops["Pull"]:
        if pre_state["consumer_rights"] & TOP_RIGHT_PULL == 0:
            raise SystemExit(f"top-level M1 pull {idx} accepted without PULL right")
        check_top_m1_projection_matches_commit("consumer", commit, pre_state, idx, "pull")
        check_top_m1_authority_projection_unchanged(
            "pull",
            idx,
            pre_state,
            post_state,
            authority_fields,
        )
        return

    raise SystemExit(f"top-level M1 accepted unsupported transition op {op}")
