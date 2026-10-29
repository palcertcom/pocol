# PoCol on five independent devices

This increment targets three Linux servers, one native Windows PC and one Android
ARM64 phone. Each runs the same Rust block validation, consensus, persistent state
and P2P engine. Android is not an RPC-only wallet. No assistant, central signing
service, Python, WSL or Docker is needed during normal operation.

**Experimental test currency only. Not a public-mainnet release.** The existing
launch gates in `development/public-network/PLAN.md` still apply. The network
closes at block 19,000 and ends at 20,000 (about 53/56 hours at the nominal block
interval). This release does not bypass those rules. The current resource model
has a 6 GiB node budget; running an early bounded pilot on a 4 GiB VPS is not a
worst-case capacity proof. Android's Java/runtime overhead is additional and a
physical handset has not yet been qualified. Android and Windows cannot yet be
claimed to have passed a real five-device network test.

## Once per device

- Linux: extract the artifact ZIP, then its `PoCol-linux-x86_64.tar.gz`
  (which preserves executable permissions), and run `sudo ./install-linux.sh`. It creates
  a dedicated account, a private local identity and a service, and prints only the
  public address. The service waits for the shared network file. It is enabled
  across reboots and restarts after failure. Only a fresh installation is accepted.
- Windows 10 x64 or newer: extract the checked release and double-click
  `Start-PoCol.cmd`. The first run creates an identity under `%LOCALAPPDATA%\PoCol`
  and displays its public address. It waits for the shared network file. This is
  a native console node, not a Windows service; keep its window open. Automatic
  startup after Windows reboot is not implemented by this increment.
- Android 8+ ARM64: install the test APK, open PoCol Testnet and copy its public
  address. The key stays in app-private storage and Android backup is disabled.
  The initial CI APK is debug-signed for this test; the unsigned release APK is
  also provided for an operator's retained signing key. Debug signing identities
  change between clean CI runs: do not uninstall a running node to replace it,
  because uninstalling erases its identity and state. A stable privately held
  release signing key is required for durable app updates.

The three prepared Linux endpoints are `108.165.128.89`, `108.165.128.90` and
`108.165.128.91`, P2P TCP port `30300`. This repository does not assert that the
port is open or that PoCol is running there. Only SSH was previously verified.
Do not open native RPC on the Internet. Windows and Android use outgoing P2P;
no home-router port forwarding is required for them to validate blocks.

## One shared genesis, then ordinary operation

The first network needs public identity addresses before its genesis can be
published. No private key needs to leave any device. Do not generate a separate
genesis per device, or invent fixed known keys to remove this one-time step.
An operator uses the existing `pocold init-testnet` public-input configuration,
selects initial miners/test allocations, and creates one invitation:

```sh
pocol network --from genesis --seed 108.165.128.89:30300 --seed 108.165.128.90:30300 --seed 108.165.128.91:30300 --out network.pocol.json
```

Publish its displayed fingerprint alongside the file through a trusted channel.
An invitation's checksum detects differences; it does not authenticate its
publisher. All participants use the same invitation and source compatibility ID.

- Linux: place the shared file at `/opt/pocol-standalone/network.pocol.json`, allow
  incoming TCP 30300, and start `pocol-standalone.service` once.
- Windows: place it beside `pocol.exe` and open `Start-PoCol.cmd` again.
- Android: tap **Import network file**, select the shared file, then **Start PoCol**.

Nodes reconnect to the configured seeds automatically, recover local durable
state and keep their original identities. There is no remote assistant control
channel. The phone runs a user-visible foreground service with a Stop button;
Android may still suspend/kill it because of system policy, heat or memory. It
requests a bounded renewable wake lock while running. After a phone reboot,
launch the app and press Start again; continuous mobile availability is not
promised. The server nodes should remain online.

Open joining means any compatible peer can connect and validate. Producing blocks
also requires membership in the existing registry. An initial genesis miner can
produce directly. A new identity must fund the existing bond and submit the
ordinary registration transaction. There is **no automatic faucet or automatic
registration transaction** in this increment; never describe joining alone as
mining eligibility. An operator may include all five public identities initially
or keep three server miners and use the PC/phone as independently validating
joining nodes. This choice is recorded in the shared genesis, never hard-coded
into the application.

## Identity and recovery

Linux/Android retain the strict 0600/0400, same-user, regular-file, single-link key
checks. Windows stores authenticated DPAPI ciphertext bound to the Windows user,
rejects raw Unix key files, reparse points, hard links and invalid ciphertext, and
never requests the user's Windows password. DPAPI deliberately does not use
machine-wide protection. The Windows file alone is not a portable recovery seed;
retain a suitable Windows profile backup. A portable encrypted export workflow
is not implemented. Do not upload private homes, key files or signing keystores.

A launcher lock prevents concurrent startup/calibration in the same home. The
first invitation binds the home; another network requires a separate home.
Damaged keys are refused, never silently replaced. Stop the node before offline
backup. The launcher checkpoints on a cooperative stop; a forced OS kill uses
existing journal recovery. Transport workers are process-scoped: the executable
or Android service process exits after the engine stops.

## Build and verification

`five-device-build.yml` builds exact source on Ubuntu and Windows, tests identity
and malformed invitations, and runs five local processes (three miners plus two
independent validating joiners), checking common confirmed roots and restart
identity. These checks are **not** a cross-OS or physical multi-host acceptance.
The Android job compiles the actual ARM64 Rust engine with NDK 27.2, packages its
JNI bridge into an APK and preserves the unsigned release APK as well. Android
runtime/permission/lifecycle tests on a physical device remain necessary.

- Rust: `cargo +1.99.1 build --locked --release --bins --manifest-path prototypes/pocold/Cargo.toml`
- Portable controls: `python deploy/standalone/test_launcher.py -v`
- Five-process check: `python deploy/standalone/test_launcher.py --integration`
- Android: SDK 35, NDK 27.2.12479018, JDK 17, Gradle 8.11.1; run
  `bash deploy/standalone/build-android.sh` after installing the ARM64 Rust target.

Every Rust source change updates the conservative compatibility fingerprint.
Upgrade all five devices to the same source before using a new network invitation.
Old b0181e7 binaries cannot connect to this increment.

Primary platform references:
- https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata
- https://developer.android.com/develop/background-work/services/fgs/service-types
- https://developer.android.com/ndk/guides/other_build_systems
- https://developer.android.com/build/releases/agp-8-9-0-release-notes
