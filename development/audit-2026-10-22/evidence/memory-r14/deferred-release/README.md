# D-6: a dropped checkpoint's nodes released in steps between events (`93a40d0`, `22a82c2`)

At the network's state caps, a checkpoint that dropped the oldest one held the consensus thread 10.3–13.7 s
(scale-d30dc71/ckpt-instrumented.log). It released every node only the dropped checkpoint held inside its own
transaction. This is D-6's local mechanism; no consensus, validity or specification rule changes.

## The mechanism

- **state.redb** keeps a `release` table: node id → the references it still owes. A node's stored count is the
  stored nodes that name it, plus its pins, plus what it owes.
- **`write_checkpoint`:**
  - first finishes, in its own transaction, what an earlier write left owed;
  - then takes the dropped checkpoint's pins off and owes each of its roots one reference.

  So the store owes at most one dropped checkpoint.
- **`release_step(budget)`:**
  - works off up to `budget` owed rows in one transaction, fsynced since `22a82c2`;
  - a node left with no reference is deleted, and each of its children is owed one;
  - a step that fails marks the store damaged, and the next open rebuilds it.
- **`Node::run`** takes one release step (RELEASE_SLICE, 256) whenever no event is waiting, or one between two
  events while work is owed. The pool's revalidation slices go first.
- **Open** finishes everything owed (`release_all`) before it reads a checkpoint.
- **A13:** the state.redb file term counts (KEPT + 2) sets of held versions, plus a release row for at most each
  node of the owed set. Network store mode: 5,581,321,485 B (5.198 GiB), 5.941 GiB required, under the 5.25 GiB
  model cap (A13-DERIVATION §3c).

## Files

| file | tree | what |
|---|---|---|
| `release-93a40d0.log` | `206eaa4` + the change committed as `93a40d0` | lp4 and pocold release suites: 451 passed, 0 failed |
| `crash-test-93a40d0.log` | the same | `--features crash-test`: crashes inside release steps 1, 20 and 60, and kills at increasing delays |
| `crash-test-22a82c2.log` | the tree committed as `22a82c2` (steps fsynced) | the same |
| `mutants-deferred-release.log` | `22a82c2` + `e4_checkpoint_release_runs_at_open_and_between_events` | seeded faults (below) |
| scale-d30dc71/`ckpt-*.log` | `93a40d0`, `22a82c2` | the cost at the network's state caps (scale-d30dc71/README.md, § After the deferred release) |

## What the crash tests show

- After a crash inside a release step, the node opens as replay from genesis gives it, from the same journal.
  It resumes from a checkpoint, and the next run imports every block. This holds with the steps fsynced or not.
- `e4_checkpoint_survives_kills` takes a release step after each import, as the event loop does, and is killed
  at 11 increasing delays.

## The tests

- **`state_store::a_dropped_checkpoint_is_released_in_steps_and_ends_as_a_release_in_the_write`:**
  - five checkpoints, keeping two, and releases in steps of 3 (161 steps);
  - after every step the counts recount, and every pinned version reads as its state;
  - the dropped checkpoint's own nodes end neither stored nor cached;
  - the next write finishes what is left owed first;
  - a store that owes work reopens owing it, and `release_all` finishes it.
- **`consensus::e4_checkpoint_release_runs_at_open_and_between_events`** (added with this evidence):
  - a node that stopped owing a dropped checkpoint's nodes works them off at open;
  - a running node with no event but its ticks works off what a new checkpoint owes;
  - it then imports a block from its channel;
  - the counts recount at each stage.
- **`network_budget`:** the file term with the owed set and its rows.

## Seeded faults

See `mutants-deferred-release.log`.
