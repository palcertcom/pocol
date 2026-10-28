# Round 4 evidence (2026-10-28)

Every result below names the tree it ran on. Runners for seeded faults and controls (mut.py, controls.py) stay
outside the repository; only their logs are here. Nothing here is an approval of an open decision
([../../proposals/DECISION-TABLE-2026-10-28.md](../../proposals/DECISION-TABLE-2026-10-28.md)).

Commits of this round (on claude/memory-r14-followup, after 260d021):

| commit | what |
|---|---|
| 3adc356 | pocold: a local storage failure (a state.redb commit, a journal read) stops the node (exit 70) and is never a validity verdict; the journal is streamed at open; `io-fault` test feature; OpenTiming |
| 16838eb | e2e: a failed run keeps its node logs, inputs and seeds before cleanup; `e2e_heal.mjs`; deterministic heals in `e2e_integrated` |
| 118afd7 | pocold: checkpoint open without per-record chain scans (prune cursor, one side scan, orphans only after a removal), with replay's version releases in the next block's commit; tests |

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

Input: the store a node at the network's state caps left (949,767 accounts, 3.8 M slots, 1.36 M trie nodes,
state.redb 2.06 GB), copied fresh for each run; the open resumes from the checkpoint at record 28,672 and replays
855 records. Every run ends on the same head. The 262–322 s of the earlier rounds were measured with other work
on the host; on this host and input, 16838eb takes 235–253 s (six runs).

Where the time went (16838eb with timers, [open/baseline-fine.log](open/baseline-fine.log), [open/prof-after.log](open/prof-after.log)):

| part | time |
|---|---:|
| block commits to state.redb (896 durable transactions, about 140 ms each) | 126 s |
| releases of the versions entries stopped holding (867 transactions, about 45 ms each; counted inside "retain") | 39 s |
| the rest of retain: whole-chain walks at every record (prune, three side scans, the orphan pass) | 33–36 s |
| restore of the checkpoint (each stored version read and checked) | 15 s |
| reset of the pins left by the previous run | 9–13 s |
| execution of the 855 blocks | 5 s |

Changes (118afd7), none of which skips a check or a durable write:
- the R21 prune visits each height once, and again only where a reorg, a rebuild or a restore could have
  given a block a state back (`Chain::prune_cursor`); debug builds assert the invariant;
- the side blocks are found once per retain, and the orphan pass runs only after a removal;
- during replay, a released version waits for the next block's commit and is released in that durable
  transaction (`StateStore::set_deferred`); a crash in between leaves the version pinned, which the next open
  resets, as after any crash.

Result, same input, alternating runs ([open/before-after-final.log](open/before-after-final.log)):

| | before (16838eb) | after (118afd7's node code) |
|---|---:|---:|
| open | 245.0 s, 243.1 s | 207.1 s, 214.5 s |
| retain during replay | 73.7 s, 74.5 s | 14.0 s, 16.1 s |
| validate (execution and block commits) | 132.6 s, 130.2 s | 156.1 s, 160.7 s (the releases now run inside the commits) |

What is left is the durable block commits (about 150 s). Not done: making them cheaper. One attempt was
measured and reverted: commits without an fsync during replay let state.redb grow from 1.3 GB to more than
5.6 GB in 90 s, because redb reuses no freed page until a durable commit; the disk filled and the node stopped
with a local storage failure, as designed ([open/unsynced-commits-fill-the-disk.log](open/unsynced-commits-fill-the-disk.log)).
The same full disk stopped the H_END run (§4) with exit 70.

Tests: `retain_prunes_each_canonical_height_once_and_again_where_a_state_came_back`,
`retain_prunes_side_blocks_made_canonical_below_the_cut_and_leaves_no_orphan`,
`a_late_release_leaves_the_rows_an_immediate_one_leaves`, the replay crash cases added to
`e4_checkpoint_crash_at_each_step`. Seeded faults: [mutants-prune.log](mutants-prune.log) (6 of 7 killed; the
survivor, a side-list reuse, was removed from the code), [mutants-late.log](mutants-late.log) (4 of 4 killed
after the test was strengthened; the earlier survivors are kept in the log).

## 4. Memory and storage

SECTION_MEMORY

## 6. Boot and performance

SECTION_BOOT

## Suites

SECTION_SUITES
