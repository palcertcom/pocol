# Round 4 evidence (2026-10-28)

Every result below names the tree it ran on. Runners for seeded faults and controls (mut.py, controls.py) stay
outside the repository; only their logs are here. Nothing here is an approval of an open decision
([../../proposals/DECISION-TABLE-2026-10-28.md](../../proposals/DECISION-TABLE-2026-10-28.md)).

Commits of this round (on claude/memory-r14-followup, after 260d021):

| commit | what |
|---|---|
| 3adc356 | pocold: a local storage failure (a state.redb commit, a journal read) stops the node (exit 70) and is never a validity verdict; the journal is streamed at open; `io-fault` test feature; OpenTiming |
| 16838eb | e2e: a failed run keeps its node logs, inputs and seeds before cleanup; `e2e_heal.mjs`; deterministic heals in `e2e_integrated` |
| COMMIT_OPEN | pocold: checkpoint open without the per-record full scans (prune cursor, one side scan, orphans only after a removal) and with replay's trie commits unsynced between durable points; tests |

## 1. Seeded faults RV10, RV11, RV12, and the defect they led to

- RV10 (stored_bytes leaves out the release table), RV11 (open does not release what a checkpoint owes),
  RV12 (a checkpoint write that fails on I/O leaves the owed flag unlike the table): each is now detected by a
  test, with deterministic I/O fault injection (`--features io-fault`: a `redb` file backend that fails one write
  or one fsync after a named step). None is classed as equivalent. [mutants-r4.log](mutants-r4.log): RL1–RL16,
  RV1–RV12 and LF1–LF5 all KILLED on 260d021 + the round's tree (LF5 first SURVIVED, then KILLED after
  `a_journal_read_that_fails_in_an_import_stops_the_node` was added; both lines are in the log).
- **Defect found by the fault tests:** a state.redb commit that failed on I/O was taken as `Reject::Block`: the
  valid block and its descendants were marked invalid ([io-fault-before-fix.log](io-fault-before-fix.log):
  `Err(Block(...))`, `1 blocks marked invalid`). Fixed in 3adc356 (`Reject::Local`: the node logs FATAL and exits
  70; the next open recovers). A failed fsync can leave the commit durable; the tests accept "undone or whole"
  for a sync fault and require "undone" for a write fault.

## 2. Reorganization at a heal, both directions

- Rust, three nodes over TCP with no miner: `d133_heal_c_reorganizes_onto_the_heavier_a_b_branch` and
  `d133_heal_a_and_b_reorganize_onto_the_heavier_c_branch` (consensus.rs). The heavier branch has fewer blocks
  (signed, height 6, work 24,794 against fallback, height 18, work 20,590), so height cannot stand in for work.
- e2e, real pocold processes: [e2e-heal-run.log](e2e-heal-run.log), 12/12 PASS on 16838eb. Each scenario
  stops every miner and measures each head alone before the heal, so nothing can change between measurement
  and heal. The first run failed ([e2e-heal-first-run.log](e2e-heal-first-run.log)): a and b both mined on the
  a/b side and ended on two sibling tips of equal work, which D133 leaves to local import order; the test now
  lets only a mine there. That run's kept directory was deleted by mistake; only its console log remains.
- Negative controls ([controls-heal.log](controls-heal.log)): K1 fork choice takes the lighter branch, K2 a node
  never reorganizes, K3 a node never syncs. Each fails both Rust tests and e2e_heal (K1 and K2 already at the
  common prefix, K3 at the heal checks).

## 3. Failure evidence kept automatically

`harness.mjs` `keepOnFailure` (e2e_devnet, e2e_site, e2e_browser with Playwright traces, e2e_integrated,
e2e_heal) and `devnet.py keep_on_failure`: on a failure, before cleanup, the run directory (node logs, the
network's inputs, the seeds) is copied to `POCOL_E2E_KEEP` or `tests/failures/`, without `*.redb`, with a
manifest of SHA-256 sums, the commit, the pocold binary's hash and the environment. The two earlier failures
whose cause was not proven stay open: [../memory-r14/accept-e453851/CORRECTIONS-2026-10-28.md](../memory-r14/accept-e453851/CORRECTIONS-2026-10-28.md)
(e2e_integrated run 1) and [../memory-r14/accept-ad301e7/CORRECTIONS-2026-10-28.md](../memory-r14/accept-ad301e7/CORRECTIONS-2026-10-28.md)
(BR18). A passing rerun does not explain either.

## 5. Checkpoint open

SECTION_OPEN

## 4. Memory and storage

SECTION_MEMORY

## 6. Boot and performance

SECTION_BOOT

## Suites

SECTION_SUITES
