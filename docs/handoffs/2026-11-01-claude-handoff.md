# Package access and repository status

Package: PoCol-Complete-Handoff-2026-11-01.zip
Size: 205231789 bytes
SHA-256: b9a3885a630bd7f32270488f741b68a5c923e3fc48c1478499ceb887c520d2f3
Download (requires the owner's ChatGPT access): https://chatgpt.com/library?file_id=libfile_728e657a5cb08191ad53987fbfdb668f&account_id=personal

The binary ZIP has NOT been uploaded to this repository by this handoff. The owner should download it locally and give Claude its local path. Do not assume Claude can access the ChatGPT link. Store large binary assets according to repository policy (for example release assets), with checksums; do not embed them as base64 text in Git.

This document is being added on a branch based on repository tree dc64bf1e14501ed75761cd02ef7c1fc1f17e4a82. The package baseline is the older pinned commit below. Compare against the actual current branch; never reset newer work to the package baseline.

Claude: locate and verify the ZIP, read this report and its internal CLAUDE-HANDOFF.md, inspect repository instructions and changes, then integrate on a review branch. The package does not contain the entire baseline Git repository offline. Do not deploy or replace wallet/state files automatically. Report exactly which files were integrated and which checks remain outstanding.

---

# PoCol — updated handoff, 2026-11-01

This report records user-supplied deployment evidence through 2026-10-31. It is not a fresh live audit. Start by inspecting the repository and its AGENTS.md. Preserve unrelated changes and create a review branch. Do not deploy automatically.

## Package layout and integration

- `repository-overlay/`: editable extension source and relevant deployment Python files, based on commit `c68dd1b146212e3f095dd386c888728fb29548b8`, with English extension 0.4.1 changes applied. Paths are relative to the repository root. Compare before copying.
- `websites/`: complete deployable document roots after merging the H15000 website packages with the corrected English-extension 0.4.1 patch. These are deployable output, not a claim to contain the original IDE TypeScript development project. Preserve existing API configuration and review diffs before deployment.
- `native/`: original 0.4.0 Windows/Linux distributions and the unsigned Android distribution. Native binaries were not rebuilt for this handoff.
- `extension/`: complete built English 0.4.1 Chrome extension; retain the same installed extension identity and local data when updating.
- `network/`: exact public invitation and fork schedules. No wallet keys, private chain databases, SSH keys, signing keystores or backups are included.
- `handoff/`: original release guidance and current integrity reports. Original guidance is historical; this file takes precedence on current state and outstanding work.

The full baseline Git repository is NOT bundled offline. Obtain the exact baseline using `Get-Baseline-Source.ps1` or an existing checkout of the pinned commit, then compare/apply `repository-overlay/`. The baseline contains core, wallet, IDE and other development source. This avoids substituting compiled website assets for editable application source. Binary distributions and complete published website files ARE included offline.

## Confirmed network upgrade

The existing chain was retained. No fresh genesis, reset or balance migration was performed. All three Linux nodes were upgraded to native 0.4.0 and successfully crossed activation height 15000.

| Setting | Value |
|---|---|
| chainId | 1599846715 |
| genesisHash | `0xbe137bfdb4fdf332f2e1500ee1a67e0b22ce6bdf57efa46fb4ea7b0debe509ca` |
| protocolVersion | 2 |
| compatibilitySha256 | `6cf38a14aa01ca3d824b0309564942cc926ca7232e2a5b2672a17e8c99e35476` |
| consensusCompatibilitySha256 | `0xc76db596bd2b6e9727423f2c5c118dd2026a273e19abfc77b987392586fd8fb8` |
| forkSchedule | `[[1,0],[2,15000]]` |
| after activation | activeVersion 2, effectiveNonceMode 1, nextBlockNonceMode 1 |
| invitation SHA-256 | `f4ed1373019563106a4ba0af9f063c874c667a5be50a2b42f32f16e2dcbeaf3b` |

The immutable base `nonceMode: 0` and `profileKind/nodeKind: Simnet` still appear in RPC. Do not blindly rename consensus identifiers or rewrite genesis to match UI branding. Public display values are `PoCol Mainnet` and `mainnet`. The previously inspected invitation had `publicLaunchReady:false`; branding is not a launch readiness audit.

Nodes: pocol-test-1 = 108.165.128.89, pocol-test-2 = .90, pocol-test-3 = .91. P2P TCP 30300. Service `pocol-standalone.service`, user/group `pocol-standalone`, programs `/opt/pocol-standalone`, home `/var/lib/pocol-standalone`, RPC `127.0.0.1:8545`. Preserve wallet identities, state and service hardening. Backups were verified on all three servers before upgrading.

All nodes returned the same canonical block at height 16650: hash `0xceb83d7e8ef59146b8cf0b55c4278cea4def53e2265ac4594d186e1450db06d1`, state root `0xd5ce7de1163b5d82d01691c65aba511c1319a5ff65ef0b2db258d8262746c0ad`. State stores were healthy; historyDamaged was false. These are historical observations, not current heads.

## Connectivity fix

The VPSes could not ARP each other despite /24 addresses: neighbours remained INCOMPLETE. All six directed peer connections were repaired with specific peer /32 routes via `108.165.128.1`, interface `ens3`, static metric 42700 in main table. Default routes and original network configuration were preserved.

Persistent additions were saved to `/etc/netplan/90-pocol-peer-routes.yaml`, two peer routes per server. Isolated generation verified only the expected additions. No live netplan reload or reboot was performed. Persistence after reboot remains UNTESTED. A fresh chain would not itself fix this networking problem.

## Gateways

Server 1 runs `pocol-explorer-gateway.service` and `pocol-studio-gateway.service`. Sources are installed under `/opt/pocol-explorer-gateway/` and `/opt/pocol-studio-gateway/`. Explorer gateway listens on 8088; Studio gateway uses loopback 8089. Updated files were gateway.py, pocol_rewards.py and pocol_studio_gateway.py. Trusted server schedule is numeric `[[1,0],[2,15000]]`; preserve existing transaction database, GeoIP modules and service isolation. Both services passed local health checks and public RPC reads after restart.

Public endpoints:
- https://gateway.pocol.net/studio/health
- https://gateway.pocol.net/studio/rpc
- https://explorer.pocol.net/api/rpc.php
- https://pocol.net/api/reward.php

The reward endpoint is GET; do not assume an arbitrary reward method is exposed through the Explorer RPC whitelist. One historical pending reward snapshot had gross 2 PCOL, projected payout 1.3890625 PCOL, projected burn 0.6109375 PCOL, one beneficiary and 40 shares. Projections are not settled payments. Full nonce activation does not change the reward formula or guarantee every eligible miner receives each block reward.

## Websites and visual requirements

Domains map directly to the matching `websites/` folders: pocol.net, explorer.pocol.net, studio.pocol.net, ide.pocol.net. Actual cPanel document roots were `~/pocol.net`, `~/explorer.pocol.net`, `~/studio.pocol.net`, `~/ide.pocol.net`.

Requirements carried forward: use PoCol visual identity and supplied matching icons; English default language and starter content; currency ticker **PCOL** while brand remains **PoCol**; market Studio and IDE on the main website with screenshots, extension details and usage steps. Do not globally replace every occurrence of test in source, fixtures or protocol identifiers.

The four H15000 website entry files and public invitation were originally verified against uploaded packages with HTTP 200. Public RPC responses then showed active version 2 and the matching schedule and hashes.

A later English-extension-only patch previously stored files with UNIX mode 0600. Main, Studio and IDE subsequently displayed LiteSpeed 403; Explorer was unaffected. The corrected latest patch uses regular-file mode 0644. This handoff merges that corrected patch and assigns explicit 0644 to ALL archived files. However, restoration of the live sites after re-upload has NOT been confirmed. Diagnose document-root ownership/permissions, index file placement, .htaccess and hosting error logs; do not assume the archive alone proves the live cause is resolved. Do not chmod everything 777 or delete hosting configuration blindly.

## Chrome extension

Original deployed version 0.4.0 had bilingual Arabic/English options and publisher UI. Latest version 0.4.1 removes Arabic interface strings and the publisher language toggle, with English/LTR fixed there. Options labels and empty history state are English. Package versions and corresponding README versions were updated. Storage, wallet identity, jobs and publication history should remain intact.

The English release was previously validated with 42 publisher tests and 13 contract UI tests (55 total), syntax checks and an Arabic codepoint scan. Re-run checks after integration; browser QA and user installation confirmation remain outstanding. Use trusted browser schedule `[[1,"0"],[2,"15000"]]`. The website extension links and versioned downloads have been updated to 0.4.1; native apps remain 0.4.0.

## Windows

Staged executable folder: `D:\PoCol\PoCol-Wallet-0.4.0-907e33aa`. Existing home: `C:\Users\Lenovo\AppData\Local\PoCol`. Backup: `D:\PoCol\PoCol-Backup-0.4.0-acb3f761`; all 14 files were hash-verified. Existing home and adjacent invitation were upgraded with unchanged wallet identity, state, permissions, genesis and allocation.

The latest screenshot confirmed GUI version 0.4.0, 3 peers and local block 13538, with mining enabled. It did NOT establish full synchronization. Compare canonical block hashes with healthy seeds and verify current schedule, balances and head before claiming completion. Never reinitialize the user's existing home or replace wallet files. Do not downgrade to old binaries/configuration after post-activation history is present.

## Android — deferred

User explicitly deferred the Android upgrade and plans a private Play signing key. Installed package `org.pocol.node`, version `0.3.1-wallet-testnet`, has an old debug signing certificate. Local debug.keystore has a DIFFERENT certificate. The supplied 0.4.0 Android distribution is unsigned; do not advertise it as a ready in-place update. A newly chosen signing key cannot normally update the old installed package directly.

An offline Android height inspection found highest stored block 14054. That height snapshot is NOT a full wallet backup. Do not uninstall or clear application data without verified wallet recovery/export and an agreed migration plan.

## TLS observation

Windows Python 3.11 reported certificate-expired verification failures. Server OpenSSL chain/hostname verification succeeded, and Windows curl 8.13 Schannel reached health and Explorer with strict TLS and HTTP 200. Exact Python trust-store cause is unresolved. Keep TLS verification enabled; do not ship -k or certificate-verification bypasses.

## Next actions, in order

1. Read repository instructions; obtain/verify pinned baseline and compare overlay with current branch. Review pending upstream work before applying.
2. Restore and verify main/Studio/IDE HTTP 200, key assets and downloads; verify RPC identity and schedule. Preserve API/local hosting configuration.
3. Confirm English extension update, trusted schedule and Studio/IDE operation using the same installed profile.
4. Confirm Windows synchronization and canonical hashes without resetting wallet/state.
5. Test persistent peer routes during a controlled maintenance reboot, one server at a time.
6. Address Android signing and safe data migration separately.
7. Re-run appropriate source tests, generate release notes and propose a reviewable repository change. Use repository release/LFS policy for binaries rather than blindly committing large distributions.

Do not claim a fresh mainnet launch, a completed audit, resolved 403, completed Windows sync or successful Android migration without new evidence.
