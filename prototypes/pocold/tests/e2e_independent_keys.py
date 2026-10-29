#!/usr/bin/env python3
"""Four real nodes, private key files, external genesis, transfer and late registration.

All endpoints are loopback. This is not public-network acceptance.
Secret files stay in a 0700 local work directory; never upload that directory.
"""
import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'deploy/private-testnet'))
from private_testnet import rpc
from runtime import digest

BIN = Path(os.environ.get('POCOLD_BIN_DIR', str(ROOT / 'prototypes/pocold/target/release')))
LOCAL = ['--delta', '1', '--phi', '1', '--t-exec', '1', '--delta-rel', '8', '--gamma', '5']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--work', type=Path, required=True)
    args = parser.parse_args()
    work = args.work.resolve()
    work.mkdir(mode=0o700, parents=True, exist_ok=False)
    keys = work / 'secrets'
    keys.mkdir(mode=0o700)
    evidence = work / 'public-evidence'
    evidence.mkdir()
    addresses = {}
    for name in ['m1', 'm2', 'm3', 'm4', 'r1', 'r2', 'r3', 'r4', 'alice', 'bob']:
        addresses[name] = subprocess.check_output([str(BIN / 'pocol-key'), 'generate', '--out',
            str(keys / (name + '.pocol-key'))], text=True).strip()
    config = {'chainId': 777914, 'genesisTime': int(time.time()) + 5, 'registrationPerBlock': 1,
        'miners': [{'id': addresses[f'm{i}'], 'reward': addresses[f'r{i}']} for i in range(1, 4)],
        'accounts': [{'address': addresses[name], 'wei': str(10**24)} for name in ['alice', 'bob', 'm4', 'r4']]}
    (evidence / 'genesis-input.json').write_text(json.dumps(config, indent=2))
    result = subprocess.run([str(BIN / 'pocold'), 'init-testnet', '--config', str(evidence / 'genesis-input.json'),
         '--out', str(work / 'net')], capture_output=True, text=True)
    (evidence / 'init.log').write_text(result.stdout + result.stderr)
    result.check_returncode()
    ports = []
    held = []
    for _ in range(8):
        sock = socket.socket(); sock.bind(('127.0.0.1', 0)); held.append(sock)
        ports.append(sock.getsockname()[1])
    for sock in held: sock.close()
    urls = [f'http://127.0.0.1:{p}' for p in ports[4:]]
    processes, logs, checks = {}, [], []

    def start(i):
        log = (evidence / f'node{i+1}.log').open('ab'); logs.append(log)
        command = [str(BIN / 'pocold'), 'run', '--network', str(work / 'net'), '--data', str(work / f'data{i}'),
            '--name', f'node{i+1}', '--miner-key-file', str(keys / f'm{i+1}.pocol-key'),
            '--listen', f'127.0.0.1:{ports[i]}', '--rpc', f'127.0.0.1:{ports[i+4]}', '--state-store', '--log'] + LOCAL
        for j in range(4):
            if j != i: command += ['--peer', f'127.0.0.1:{ports[j]}']
        processes[i] = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)

    def stop(i):
        process = processes.pop(i); process.terminate()
        try: process.wait(timeout=30)
        except subprocess.TimeoutExpired: process.kill(); process.wait(timeout=10)

    def wait(predicate, label, timeout=240):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            if any(p.poll() is not None for p in processes.values()): raise RuntimeError('node exited; see logs')
            try:
                if predicate(): return
            except (OSError, ValueError, KeyError, TypeError): pass
            time.sleep(1)
        raise RuntimeError('timeout: ' + label)

    def ok(name): checks.append(name); print('PASS ' + name, flush=True)

    def send(key, to, value, nonce, data=None, gas=21000):
        command = [str(BIN / 'pocol-sign'), '--chain-id', '777914', '--key-file', str(keys / (key + '.pocol-key')),
                   '--to', to, '--value', str(value), '--nonce', str(nonce), '--gas', str(gas)]
        if data: command += ['--data', data]
        raw = subprocess.check_output(command, text=True).strip()
        return rpc(urls[0], 'eth_sendRawTransaction', [raw])

    def confirmed(tx, expected='0x1'):
        def same():
            receipts = [rpc(url, 'eth_getTransactionReceipt', [tx]) for url in urls]
            if not all(receipts) or len({r['blockHash'] for r in receipts}) != 1: return False
            height = int(receipts[0]['blockNumber'], 16)
            return all(r['status'] == expected and rpc(url, 'pocol_status')['head']['height'] >= height + 6 and
                       rpc(url, 'eth_getBlockByNumber', [hex(height), False])['hash'] == r['blockHash']
                       for url, r in zip(urls, receipts))
        wait(same, 'common confirmed receipt ' + tx)

    try:
        for i in range(4): start(i)
        wait(lambda: all(rpc(u, 'pocol_status')['peers'] for u in urls), 'peer connections', 60)
        assert all(rpc(urls[i], 'pocol_status')['miner'].lower() == addresses[f'm{i+1}'] for i in range(4))
        assert all(len(rpc(u, 'pocol_getMiners')) == 3 for u in urls)
        ok('four nodes load independent keys; fourth key is not in genesis membership')
        transfer = send('alice', addresses['bob'], 12345, 0)
        durable = send('r4', addresses['r4'], 0, 0)
        confirmed(transfer); confirmed(durable)
        assert all(int(rpc(u, 'eth_getBalance', [addresses['bob'], 'latest']), 16) == 10**24 + 12345 for u in urls)
        ok('independently signed transfer converges; future reward account becomes durable')
        # keccak256("register(address)")[:4], independently checked with @noble/hashes.
        calldata = '0x4420e486' + addresses['r4'][2:].rjust(64, '0')
        registry = '0x0000000000000000000000000000000000c0c001'
        registration = send('m4', registry, 100 * 10**18, 0, calldata, 1000000)
        confirmed(registration)
        assert all(any(m['id'].lower() == addresses['m4'] and m['status'] == 1 for m in rpc(u, 'pocol_getMiners')) for u in urls)
        wait(lambda: rpc(urls[3], 'pocol_status')['stats']['mined'] > 0, 'new miner produces a block')
        ok('non-genesis miner registers through a transaction and produces real blocks')
        duplicate = send('m4', registry, 100 * 10**18, 1, calldata, 1000000)
        confirmed(duplicate, '0x0')
        assert all(len(rpc(u, 'pocol_getMiners')) == 4 for u in urls)
        ok('duplicate registration reverts without adding a fifth member')
        source = work / 'site-input'
        source.mkdir()
        (source / 'index.html').write_text('<!doctype html><html lang="en"><title>PoCol test</title><h1>Independent keys</h1></html>')
        published = subprocess.run([str(BIN / 'pocol-site'), 'publish', '--rpc', f'127.0.0.1:{ports[4]}',
            '--chain-id', '777914', '--key-file', str(keys / 'alice.pocol-key'), '--dir', str(source)],
            capture_output=True, text=True, timeout=300)
        (evidence / 'publish.log').write_text(published.stdout + published.stderr)
        published.check_returncode()
        site = json.loads(published.stdout)['site']
        for i in range(4):
            dest = work / f'fetched-{i}'
            def fetch():
                p = subprocess.run([str(BIN / 'pocol-site'), 'fetch', '--rpc', f'127.0.0.1:{ports[i+4]}',
                    '--site', site, '--out', str(dest)], capture_output=True, text=True, timeout=20)
                return p.returncode == 0
            wait(fetch, 'verified site fetch', 120)
            assert (dest / 'index.html').read_bytes() == (source / 'index.html').read_bytes()
        ok('site published with independent account key and verified byte-for-byte on all four nodes')
        height = min(rpc(u, 'pocol_status')['head']['height'] for u in urls) - 6
        blocks = [rpc(u, 'eth_getBlockByNumber', [hex(height), False]) for u in urls]
        assert len({(b['hash'], b['stateRoot'], b['receiptsRoot']) for b in blocks}) == 1
        stop(3); start(3)
        wait(lambda: rpc(urls[3], 'eth_getBlockByNumber', [hex(height), False])['hash'] == blocks[0]['hash'], 'restart', 60)
        assert rpc(urls[3], 'pocol_status')['miner'].lower() == addresses['m4']
        ok('four nodes agree on roots; newly registered miner restarts with its original identity')
        report = {'result': 'PASS', 'scope': 'local independent-key/registration integration', 'checks': checks,
                  'publicLaunchReady': False, 'realMultiHostProven': False, 'binarySha256': digest(BIN / 'pocold'),
                  'commonHeight': height, 'commonHash': blocks[0]['hash'], 'site': site,
                  'transactions': {'transfer': transfer, 'registration': registration, 'duplicate': duplicate}}
        (evidence / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    finally:
        for i in list(processes): stop(i)
        for log in logs: log.close()


if __name__ == '__main__': main()
