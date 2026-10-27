# e2e_integrated, whole-state run 1 on 8d80354: a race in the test's reorg-recovery check

**What failed.** One of 20 checks failed (e2e-integrated-whole-state-1.log): "reorg recovery: Chrome via c now
shows the canonical version, identical on all nodes". All three nodes showed `LP10 site v1`, identical, but
the canonical index read afterwards was 1 (v2). The other 19 checks passed, including the replays of all
three nodes and identical roots at heights 1..102. The next run (whole-state 2) passed 20 of 20.

**Why.** The check compared views taken at one moment with a version read at a later moment, while the
version legitimately changed in between.
- During the partition, c published v2 on its minority branch.
- At the heal, c adopted the heavier branch and returned that branch's 7 transactions to its pool. By the
  D-7 relay policy (8584283) it offers them to its peers: `reinsertedOffered: 7, reinsertedUnoffered: 0`.
- The healed network included them a block or two later. The node logs here (node-a/b/c.log; heights and
  times in ms since the epoch the node prints):

  | event | node a | node c |
  |---|---|---|
  | reopened after the heal | 10,063,978: head 73 | 10,063,410: head 40, then 73-75 imported by 10,064,889 |
  | h=76, 5 transactions | 10,065,278 | 10,065,298 |
  | h=77, 2 transactions | 10,066,161 | 10,066,178 |

- So v2 became canonical about 1.3-2.2 s after the nodes reopened: after the Chrome reads began and before
  the version was read.

**Not a node defect.**
- Returning and re-offering the transactions is the specified behaviour: consensus.md, excluded transactions
  return after they are checked again; the D-7 relay policy.
- Every node agreed at every height, and v3 was "version 2" in this run as in every earlier one, so v2 was
  included in the end every time.
- In the 10 earlier integrated runs of accept-24cc167 and accept-8584283 (evidence/d7-bound/), the
  canonical index after the heal was 0 in six and 1 in four. Which one a run sees depends only on whether
  the re-inclusion lands before the check.
- The overlap with a low-priority (`nice 19`, one job) compile on this host may have widened the window. It
  did not cause the race.

**The fix (test only, prototypes/pocol-extension/tests/e2e_integrated.mjs).**
- The views now count only when every node's version is the same before and after them. They are retaken
  if it moved in between, for up to 120 s.
- The assertion is unchanged: the views are identical on all nodes and equal to that canonical version.
- The pools still settle before v3, as in 919a50c.
- The original log stays here unchanged. The reruns of the fixed test are in ../accept-<commit>/.
