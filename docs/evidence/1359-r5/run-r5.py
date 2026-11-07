#!/usr/bin/env python3
"""Exact R5 stash/control/restore and isolated test evidence; no production I/O."""
import datetime
import hashlib
import json
import os
import pathlib
import socket
import subprocess
import sys
import time

ROOT = pathlib.Path('/private/tmp/h1359-outbox')
OUT = pathlib.Path('/private/tmp/h1359-r5-control')
PG = pathlib.Path('/opt/homebrew/opt/postgresql@16/bin')
HEAD = '8aa864ab9ff70107faf8e016a04d32f4e78ad141'

def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def write(name, value):
    (OUT / name).write_text(json.dumps(value, indent=2) + '\n')

def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def manifest():
    paths = subprocess.check_output(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=ROOT).decode().split('\0')
    return {p: sha(ROOT / p) for p in sorted(set(paths)) if p and (ROOT / p).is_file()}

def execute(tag, command, env=None, required=True):
    meta = {'command': [str(s) for s in command], 'cwd': str(ROOT), 'started_at': now(),
            'head': git('rev-parse', 'HEAD'), 'status': git('status', '--porcelain'),
            'scope': 'task-owned disposable PostgreSQL and loopback fixtures only'}
    if env:
        meta['environment'] = {k: env[k] for k in ['TEST_DATABASE_URL', 'DATABASE_URL', 'AA_RESULT_DIR', 'RUSTFLAGS', 'CARGO_TARGET_DIR'] if k in env}
    meta['hashes'] = {p: sha(ROOT / p) for p in ['src/forum_sync.rs', 'src/store/postgres/outbox.rs', 'tests/48_outbox_three_channels.rs', 'tests/67_authorized_actions_r4.rs', 'tests/common/admission.rs']}
    start = time.monotonic()
    with (OUT / (tag + '.log')).open('wb') as log:
        r = subprocess.run([str(s) for s in command], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
    meta.update(rc=r.returncode, ended_at=now(), elapsed_seconds=round(time.monotonic()-start, 3), log_sha256=sha(OUT / (tag + '.log')))
    write(tag + '.json', meta)
    print(json.dumps({'tag': tag, 'rc': r.returncode, 'elapsed_seconds': meta['elapsed_seconds']}), flush=True)
    if required and r.returncode:
        raise RuntimeError(f'{tag} failed rc={r.returncode}; dependent work stopped')
    return meta

def psql(port, sql):
    return subprocess.check_output([str(PG / 'psql'), '-h', '127.0.0.1', '-p', str(port), '-U', 'postgres', '-d', 'postgres', '-XAt', '-v', 'ON_ERROR_STOP=1', '-c', sql], text=True).strip()

def prepare():
    clusters = [{'port': p, 'base': f'/private/tmp/h1359-r5-pg-{p}'} for p in [55483, 56591]]
    for c in clusters:
        assert not pathlib.Path(c['base']).exists(), c
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', c['port']))
    for c in clusters:
        base = pathlib.Path(c['base']); base.mkdir()
        execute(f'init-{c["port"]}', [PG/'initdb', '-D', base/'data', '-U', 'postgres', '--auth=trust', '--no-locale', '-E', 'UTF8'])
        execute(f'start-{c["port"]}', [PG/'pg_ctl', '-D', base/'data', '-l', base/'server.log', '-o', f'-h 127.0.0.1 -p {c["port"]} -k {base}', '-w', 'start'])
        c['identity'] = psql(c['port'], "SELECT current_setting('data_directory')||'|'||current_setting('port')||'|'||current_setting('listen_addresses')")
        assert c['identity'] == f'{base}/data|{c["port"]}|127.0.0.1', c
        write('environment.json', clusters)
    psql(55483, 'CREATE ROLE swaar4 LOGIN SUPERUSER')
    for port, db, user in [(55483, 'h1359_r5_authorized_actions', 'swaar4'), (56591, 'h1359_r5_outbox', 'postgres'), (56591, 'svc_prereq_20261029_impl', 'postgres')]:
        execute('createdb-' + db, [PG/'createdb', '-h', '127.0.0.1', '-p', str(port), '-U', user, db])

def env_for(group):
    env = os.environ.copy()
    for k in list(env):
        if k.startswith('CARGO_TARGET_') and k.endswith('_RUNNER'):
            del env[k]
    for k in ['AA_SHARED_BASELINE', 'AA_REUSE_CANCELLED_RED', 'ACB_OLD_BINARY', 'CANONICAL_CREATE_TEST_PRINCIPAL_ID']:
        env.pop(k, None)
    env['CARGO_TARGET_DIR'] = str(ROOT / 'target')
    env['RUSTFLAGS'] = '--cfg trusted_fleet_cutover_conformance --check-cfg=cfg(trusted_fleet_cutover_conformance)'
    env['TEST_DATABASE_URL'] = ('postgres://swaar4@127.0.0.1:55483/h1359_r5_authorized_actions' if group == '67' else 'postgres://postgres@127.0.0.1:56591/' + ('svc_prereq_20261029_impl' if group == '45' else 'h1359_r5_outbox'))
    env['DATABASE_URL'] = env['TEST_DATABASE_URL']
    env['AA_RESULT_DIR'] = str(OUT / 'aa-results')
    pathlib.Path(env['AA_RESULT_DIR']).mkdir(exist_ok=True)
    return env

def stash():
    assert git('rev-parse', 'HEAD') == HEAD
    assert git('branch', '--show-current') == 'fix/outbox-channel-isolation'
    write('pre-stash-manifest.json', manifest())
    write('pre-stash-state.json', {'head': HEAD, 'status': git('status', '--porcelain'), 'stash_list': git('stash', 'list'), 'at': now()})
    execute('stash-push', ['git', 'stash', 'push', '--include-untracked', '-m', 'h1359-r5-exact-candidate-before-clean67'])
    stash_sha = git('rev-parse', 'refs/stash')
    assert git('status', '--porcelain') == ''
    write('clean-state.json', {'head': git('rev-parse', 'HEAD'), 'status': git('status', '--porcelain'), 'stash_sha': stash_sha, 'at': now()})

def restore():
    state = json.loads((OUT/'clean-state.json').read_text())
    assert git('rev-parse', 'refs/stash') == state['stash_sha']
    execute('stash-pop', ['git', 'stash', 'pop', 'stash@{0}'])
    expected = json.loads((OUT/'pre-stash-manifest.json').read_text())
    actual = manifest()
    changed = {p: {'expected': expected.get(p), 'actual': actual.get(p)} for p in expected.keys() | actual.keys() if expected.get(p) != actual.get(p)}
    write('restoration.json', {'at': now(), 'head': git('rev-parse', 'HEAD'), 'status': git('status', '--porcelain'), 'manifest_entries': len(expected), 'mismatches': changed})
    assert not changed, changed

def close():
    clusters = json.loads((OUT/'environment.json').read_text())
    for c in clusters:
        identity = psql(c['port'], "SELECT current_setting('data_directory')||'|'||current_setting('port')||'|'||current_setting('listen_addresses')")
        assert identity == c['identity'], identity
        c['identity_before_stop'] = identity
    for c in clusters:
        r = execute(f'stop-{c["port"]}', [PG/'pg_ctl', '-D', pathlib.Path(c['base'])/'data', '-m', 'fast', '-w', 'stop'])
        c.update(stop_rc=r['rc'], stopped_at=r['ended_at'])
        c['postmaster_pid_exists'] = (pathlib.Path(c['base'])/'data/postmaster.pid').exists()
        assert not c['postmaster_pid_exists']
        with socket.socket() as sock:
            c['port_accepting'] = sock.connect_ex(('127.0.0.1', c['port'])) == 0
        assert not c['port_accepting']
        write('cleanup.json', clusters)

action = sys.argv[1]
if action == 'stash': stash()
elif action == 'prepare': prepare()
elif action == 'clean67':
    assert git('rev-parse', 'HEAD') == HEAD and not git('status', '--porcelain')
    execute('clean67', ['cargo', 'test', '--locked', '--jobs', '4', '--test', '67_authorized_actions_r4', '--', '--test-threads=1', '--nocapture'], env_for('67'), required=False)
elif action == 'restore': restore()
elif action == 'candidate48':
    execute('candidate48', ['cargo', 'test', '--locked', '--jobs', '4', '--test', '48_outbox_three_channels', '--', '--test-threads=1', '--nocapture'], env_for('48'))
elif action == 'candidate-lib':
    execute('candidate-lib', ['cargo', 'test', '--locked', '--jobs', '4', '--lib', '--', '--test-threads=1', '--nocapture'], env_for('48'))
elif action == 'close': close()
else: raise ValueError(action)
