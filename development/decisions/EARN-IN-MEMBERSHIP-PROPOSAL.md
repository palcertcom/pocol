# Earn-in membership with a 20 PCOL bond (proposal)

Status: **proposal only, for the owner's decision.** Nothing here is implemented or activated. It
changes consensus (the system contracts, execution and membership), so it needs a coordinated network
upgrade like H15000 and an explicit approval of each decision in section 8.

Request (owner, 2026-11-01): let a miner without PCOL join, through probationary membership funded by
its own rewards (option 5 of the alternatives), and lower the bond to **20 PCOL**.

## 1. Problem

Today a new identity must lock `B_reg` = 100 PCOL when it calls `register(address)`
(`MinerRegistry` fallback in `prototypes/lp4-execution/contracts/system/PoColSystemV1.sol`:
`msg.value` must equal `cfg(16)`, which genesis fixed and `ChainCaps` keeps immutable). Without a
record, an identity is not in `A_h`, gets no nonce range, and earns nothing. A miner who holds no
PCOL therefore has no way in.

The bond does three things today (threat model assumption A6, `development/m1/reference/threat.md`):
- it prices each identity, so seats in the active set (at most `M_max` = 64) and registry capacity
  cannot be filled for free;
- `punish` burns it on evidence, so misbehaviour costs the miner;
- with `R_max` = 1 registration or reactivation per block, it slows registration churn.

Any replacement must keep a cost for these three.

## 2. What stays unchanged

- Genesis, chain ID, balances, keys, the history before activation, and the fork schedule entries
  `[1,0]` and `[2,15000]`.
- The reward formula and issuance: subsidy 2 PCOL per block; winner `alpha` = 10%; proposer
  `gamma` = 5%; shares `p_i = R·c_i/Q` with `Q = max(C, κ·m)` = max(C, 64); the uncovered part is
  burned. Earn-in redirects where a probationer's payment goes. It creates no new coins.
- Nonce assignment, share and block targets, `W_act`, `Theta_inact`, `U` and the inactivity rules.
- Existing records. They keep their recorded bond of 100 PCOL, and exit refunds exactly that amount,
  because refunds already use the per-record bond word `w2`, not the current `B_reg`.

## 3. Proposed rules (protocol version 3, from height `H3`)

Fork schedule: `[[1,0],[2,15000],[3,H3]]`. Every rule below applies to blocks at height `≥ H3`.

**R1. Bond amount.** `B3` = 20 PCOL (`20·10^18` wei) for registrations from `H3`. It is a version-3
constant, because genesis CP field 16 (`B_reg`) cannot change.

**R2. Registration with a partial bond.** `register(address reward)` accepts any `msg.value` `v`
with `0 ≤ v ≤ B3`:
- `v = B3`: the record is full (`bondState` HELD), exactly as today.
- `v < B3`: the record is probationary. It has a new `bondState` ACCRUING (value 4), and `bond = v`.

Both cases share the `R_max` slot and the `M_max` capacity check.

**R3. Probation seats.** At most `P_max` records may be ACCRUING at once. Above that, `register`
with `v < B3` reverts `Capacity`. The count lives in a new Registry storage slot (slot 8 is unused;
slot 3 has only 5 reserved bits left).

**R4. Earning into the bond.** While a record is ACCRUING, every payment owed to it is added to its
bond instead of being paid to its reward address. That covers the winner share, the proposer share
(if R7 allows proposing) and its share payments. Once the bond reaches `B3`:
- the record becomes HELD in the same settlement;
- anything above `B3` is paid to the reward address as usual.

The funds never leave Escrow. `heldTotal` grows by the redirected amount, and the escrow invariant
`balance ≥ pending + heldTotal + burned` (`check_escrow_invariant`) keeps holding: settled pending
value moves into `heldTotal` instead of leaving.

**R5. Top-up.** A new `topUp()` call, made by the identity itself, adds `msg.value ≤ B3 − bond` to
an ACCRUING bond and promotes the record when the bond reaches `B3`. A miner who has PCOL, or who
receives some, can skip the wait.

**R6. Probation deadline.** A record still ACCRUING `T_prob` blocks after registration becomes
exited, with `exit = h` and `due = h + U`. At `due`, the existing sweep refunds the accrued bond to
the reward address and erases the record, which frees the seat. The accrued amount is the miner's own
earned reward, so it is refunded, not burned.

**R7. Proposer draw.** ACCRUING members keep a nonce range and earn shares and wins, but are left
out of the proposer draw. An absent proposer delays a block by `D_att` plus the fallback wait (about
67 s). Excluding probationers stops cheap identities from slowing the chain that way. This changes
`DrawWeights` in the node (`lp1-node/src/assignment.rs`) for version 3.

**R8. Exit, inactivity and punishment** follow the existing rules, applied to ACCRUING records:
- `deregister` and `removeInactive` work as for HELD records, and the accrued bond is refunded at
  `due`.
- `reactivate` keeps the original `T_prob` deadline.
- `punish` burns the accrued bond and tombs the record.
- `setRewardAddress` stays allowed. It changes nothing while payments accrue.

**R9. Durable reward for a self-registration.** PAY-02 judges a reward address on the
pre-transaction state and requires nonce > 0, so today a first-transaction registration reverts.
From `H3`, `register` treats `reward == msg.sender` as durable: the sender's nonce is at least 1
once the transaction runs. This removes the extra transfer a new miner must pay for.

## 4. A new miner still needs a little PCOL for fees

Earn-in removes the bond, but not the transaction fee. Gas fees are never zero, and an account with
0 PCOL cannot send `register`.
- **Amount needed:** with R9, one transaction is enough. The wallet's fee cap for a registry
  transaction is about 0.001 PCOL at the current base fee plus its 1 gwei tip. The fee actually
  charged is lower.
- **Options:**
  - a fee faucet that sends a few thousandths of a PCOL per identity. The value at risk is
    negligible, unlike a 100 or 20 PCOL sponsorship;
  - any existing user sends that dust.
- **Not proposed:** a fee-free registration transaction type. It would be a larger consensus
  change, with its own spam risk.

## 5. Time to earn the 20 PCOL bond

Paid value per block ≈ 2 PCOL × (0.10 winner + 0.85 × C/64 shares). With about C ≈ 33 shares per
block (`m` = 32 plus the winner), that is ≈ 1.08 PCOL, leaving out the proposer share per R7. A
probationer with a fraction `f` of the network's hash rate earns about `f × 1.08` PCOL per block, so
it needs about `18.6 / f` blocks (`T_blk` = 10 s):

| Hash share `f` | Blocks | Time |
|---|---:|---:|
| 1/2 | 37 | about 6 min |
| 1/8 | 149 | about 25 min |
| 1/32 | 594 | about 1.7 h |
| 1/100 | 1,857 | about 5.2 h |
| 1/1000 | 18,570 | about 52 h |

These are expected values. Luck, fees and the burned uncovered part move them. `T_prob` sets the
smallest hash share that can finish probation: 7 days (60,480 blocks) needs `f` ≥ about 0.03%;
24 hours (8,640 blocks) needs `f` ≥ about 0.22%.

## 6. Security analysis

| Concern | Today (100 PCOL) | With this proposal |
|---|---|---|
| Filling the 64 full seats | 6,400 PCOL locked | 1,280 PCOL locked, or the hash work to earn it (about 1,190 blocks of the whole network's paid reward) |
| Filling probation seats | not applicable | `P_max` identities that keep sharing. Cheap in hash, but it only delays newcomers (onboarding denial), never block production (R7). Bounded by `P_max`, `T_prob` and `R_max` |
| Penalty for misbehaviour | burns 100 PCOL | burns 20 PCOL, or the accrued part of it: 5 times less deterrence |
| Taking a sponsor's money and leaving | possible with plain sponsorship | not applicable: a probationer's bond is its own earned reward |
| Slowing blocks with absent proposers | full members only | still full members only (R7) |
| Supply | unchanged | unchanged: redirected rewards are existing issuance, and burns are unchanged |

**Residual risks the owner accepts with this proposal:**
- An attacker with modest hash rate can keep the `P_max` probation seats occupied, delaying honest
  newcomers by up to `T_prob`. They can still join at once with a full 20 PCOL bond or a top-up.
- The lower bond makes full seats 5 times cheaper to occupy, and evidence penalties 5 times smaller.

## 7. Implementation and rollout (after approval)

This is consensus-critical. A node without these rules forks off at `H3`.

1. **Specification:** normative text for R1–R9, with independent vectors for registration,
   accrual, promotion, deadline, punishment and the escrow invariant.
2. **System contracts v3:** `MinerRegistry` (partial bond, ACCRUING state, seat count, `topUp`,
   deadline in the sweep, `accrue` callable only by Escrow, R9) and `RewardEscrow` (redirect in
   `payTo` and `distribute`). The storage layout stays compatible; only new values and slots are added.
3. **Execution:** today `apply` has no protocol version (H15000 changed only the node's nonce
   domain). Version 3 needs the execution to learn the version, and to install the v3 Registry and
   Escrow code at `H3` while keeping their storage. This is a new code-replacement mechanism. It must
   be deterministic, journaled and replayable, and it must account for the state summary's code
   inventory.
4. **Node:** the fork schedule with version 3; the R7 proposer draw; the handshake schedule check;
   status and RPC fields.
5. **Clients:**
   - the wallet engine and Windows wallet: registering with 0 to 20 PCOL, probation progress and
     top-up;
   - the extension, Explorer and gateway reward accounting (`deploy/rewards`): show and account
     ACCRUING records;
   - Android: deferred, as now.
6. **Tests:** unit tests and vectors; devnet tests across activation, in both directions of a reorg
   around `H3`; replay from genesis; mutation checks; a multi-process upgrade rehearsal.
7. **Rollout:** like H15000. Agree on a future `H3`, upgrade and verify every server, the Windows
   wallets and the extension's trusted schedule before `H3`, and keep backups. Choosing and
   deploying `H3` is the owner's call.

## 8. Decisions for the owner

| ID | Decision | Proposed value |
|---|---|---|
| E1 | New bond `B3` | 20 PCOL (as requested) |
| E2 | Probation seats `P_max` | 8 |
| E3 | Probation deadline `T_prob` | 60,480 blocks (7 days), so weak devices can finish; or 8,640 (24 h) to turn seats over faster |
| E4 | At the deadline, refund or burn the accrued bond | refund: it is the miner's own reward |
| E5 | Leave probationers out of the proposer draw (R7) | yes |
| E6 | Allow `topUp` (R5) | yes, by the identity itself |
| E7 | Existing 100 PCOL bonds | keep them until exit; no forced migration |
| E8 | Accept `reward == msg.sender` as durable from `H3` (R9) | yes |
| E9 | Activation height `H3` | chosen later, when the release is ready |

Not proposed here: changing `M_max`, `R_max`, rewards or issuance, or a fee-free transaction type.
