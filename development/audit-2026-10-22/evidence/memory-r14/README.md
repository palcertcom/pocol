# Memory bound recount and R14/R19 workload gap (2026-10-27)

Branch `claude/memory-r14-followup` (palcertcom/pocol#2, draft), from the merged `13f1eac`. Commits:
- `892dca6`: the four A13 reductions and their tests;
- `2839f2a`: the store-mode import fix, the R14/R19 measurement and its documents.

Host: 4 vCPU KVM, ext4 on virtio; rustc 1.99.1; release builds. Nothing else ran during the timing runs.
The mutation runs used a separate worktree.

## Files

| file | what | result |
|---|---|---|
| a13-models.log | `network_budget` on `2839f2a`'s tree | network store **5.086 GiB, gate 5.81 GiB**; whole-state 24.98 GiB; devnet store 2.44, whole-state 2.99; state.redb file 2.20 TiB (563 regions); 5/5 |
| lp4-release.log | lp4 release suite, `--no-fail-fast`, on the tree committed as `892dca6` | 200 passed, 0 failed |
| pocold-release.log | pocold release suite on that tree before the pin was updated | 234 passed, **1 failed**: `a13_bound_breakdown`, the pinned devnet model total, which the boxed entries changed (3,287,955,262 → 3,233,597,728 B). The pin is updated in `892dca6`, and the test then passed |
| pocold-release-2.log | pocold release suite on `2839f2a` (the restore fix included) | **236 passed, 0 failed, 24 ignored** |
| r14-gap-rootcheck-on.log | `jm_r14_workload_gap` on `892dca6` with pocold's test-only root checks on (`POCOL_ROOT_CHECK` unset) | kept as the record of the correction: every block also ran on a whole-state reference path; 1,246 s |
| r14-gap-rootcheck-off.log | the same with `POCOL_ROOT_CHECK=0`, as in the node binary | 136 s; the table in A13-DERIVATION §3b |
| r14-gap-after-fix.log | the same on `2839f2a` (the restore fix) | 113 s; store-mode imports equal their validation |
| d2-gate.log | admission of the pocold devnet and a P11-shaped devnet at t_exec = 1 and 2 s (a throwaway test, not committed) | at 2 s: R2, R3 and R7 fail; with Δ_rel and Γ raised, R3 and R7 still fail; admitted only with dFbWait and D_att (CP) raised |
| mutants.log | mutation controls (the runner is kept outside the repository) | 24 of 25 killed, 1 equivalent; see below |

## What the measurement shows (A13-DERIVATION §3b)

All times are with the checks off, in ms (validate median / import).

| kind | whole-state, near-cap state | store, near-cap state, after the fix |
|---|---:|---:|
| GAS_LIMIT compute loop (the calibration's workload) | 24 / 32 | 43 / 53 |
| 380 transfers | 72 / 71 | 113 / 100 |
| MODEXP, even modulus (two shapes) | 1,265–1,325 / 1,260–1,359 | 1,355–1,399 / 1,524–1,591 |
| BN pairing | 304 / 286 | 383 / 330 |
| keccak over 32 KiB | 207 / 193 | 204 / 321 |
| cold CALLs | 65 / 83 | 343 / 318 |

- Calibration: 9–10 ms per phase in memory, 12–19 ms through the store. t_exec is 1,000 ms.
- **Only MODEXP with an even modulus exceeds t_exec.** That is D-2.
- **Store-mode import before the fix:** MODEXP 2,535–2,609 ms and transfers 255 ms, because every import
  re-executed the head.

## Mutation controls (mutants.log)

25 mutants of the new code, each applied to a separate worktree of `2839f2a`, run against the tests named
in the log, then restored. Baseline unmutated: 7 + 1 + 5 tests pass.

| group | mutants | result |
|---|---|---|
| view.rs, the pool layer without code copies (V1–V5) | 5 | 5 killed |
| node_trie.rs, the per-node constants and the batch bounds (N1–N11) | 11 | 10 killed in the first run; N11 (`ROW` 36 → 32) survived, see below |
| resources.rs, the A13 terms (P1–P8) | 8 | 6 killed in the first run; P6 equivalent; P8 survived, see below |
| chain.rs, the store-mode import fix (F1) | 1 | killed |

- **P8** (the map slot of a block entry counted ×3 instead of ×4) survived `network_budget`, whose network
  figures do not pin the devnet total. Rerun against `consensus::a13_bound_breakdown`, which pins it: killed.
- **N11** survived both node_trie runs, also after `stored()` in node_trie/tests.rs was made to count a row's
  32 + 4 bytes itself: the bounds the tests check have slack, so a row 4 bytes short still fits. Killed by
  the new `state_store::a13_row_is_what_state_redb_keeps_per_node`, which checks `ROW` plus each node's
  encoding against the rows state.redb keeps (its `stored_bytes`), exactly.
- **P6** removes `min(n·whole, …)` from `chain_versions_bytes`. It is equivalent: every term of
  `block_bytes(wa,ws,ka,ks)` is at most the same term of `version_bytes(ka,ks)` (leaves `min(w,k) ≤ k`,
  branches `min(64·keys, child) ≤ keys`, children `≤ 2·child`), so `whole + (n−1)·block ≤ n·whole` and the
  `min` never picks its first argument. No test can kill it.

Result: 24 of 25 killed, 1 equivalent.

## Not established here

- Acceptance.
- Measured RSS at network scale.
- The network profile booting.
- Any adopted decision: D-2 and the calibration workload are rows 1–2 of proposals/OPEN-DECISIONS.md.
