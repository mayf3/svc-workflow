#!/usr/bin/env python3
"""Create only new R4-owned disposable PostgreSQL fixtures; fail closed."""
import datetime
import hashlib
import json
import pathlib
import socket
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[3]
OUT = pathlib.Path(__file__).resolve().parent
PG = pathlib.Path('/opt/homebrew/opt/postgresql@16/bin')
CLUSTERS = [(56589, pathlib.Path('/private/tmp/h1359-r4-pg')),
            (55432, pathlib.Path('/private/tmp/h1359-r4-pg-55432')),
            (55483, pathlib.Path('/private/tmp/h1359-r4-pg-55483'))]


def execute(args, log, capture=False):
    log.write(('COMMAND ' + json.dumps([str(a) for a in args]) + '\n').encode())
    log.flush()
    result = subprocess.run(args, stdout=subprocess.PIPE if capture else log,
                            stderr=log, check=True)
    if capture:
        log.write(result.stdout)
        log.flush()
        return result.stdout.decode().strip()


def psql(port, database='postgres'):
    return [str(PG / 'psql'), '-h', '127.0.0.1', '-p', str(port), '-U',
            'postgres', '-d', database, '-XAt', '-v', 'ON_ERROR_STOP=1']


def main():
    # Preflight every endpoint and path before creating any cluster.
    for port, base in CLUSTERS:
        assert not base.exists(), f'existing path forbidden: {base}'
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', port))
    plan = pathlib.Path('/Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json')
    assert hashlib.sha256(plan.read_bytes()).hexdigest() == '0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606'
    states = []
    with (OUT / 'full-environment.log').open('wb') as log:
        for port, base in CLUSTERS:
            base.mkdir()
            execute([str(PG / 'initdb'), '-D', str(base / 'data'), '-U',
                     'postgres', '--auth=trust', '--no-locale', '-E', 'UTF8'], log)
            execute([str(PG / 'pg_ctl'), '-D', str(base / 'data'), '-l',
                     str(base / 'server.log'), '-o',
                     f'-h 127.0.0.1 -p {port} -k {base}', '-w', 'start'], log)
            identity = execute(psql(port) + ['-c', "SELECT current_setting('data_directory') || '|' || current_setting('port') || '|' || current_setting('listen_addresses')"], log, True)
            assert identity == f'{base}/data|{port}|127.0.0.1', identity
            states.append({'port': port, 'data_directory': str(base / 'data'),
                           'identity': identity})
            (OUT / 'environment.json').write_text(json.dumps(states, indent=2) + '\n')
        execute(psql(55483) + ['-c', 'CREATE ROLE swaar4 LOGIN SUPERUSER'], log)
        for database in ['svc_workflow_test', 'svc_workflow_cutover_test', 'auth_cutover_test']:
            execute([str(PG / 'createdb'), '-h', '127.0.0.1', '-p', '56589',
                     '-U', 'postgres', database], log)
        for migration in sorted((ROOT / 'migrations').glob('*.sql')):
            execute(psql(56589, 'svc_workflow_cutover_test') + ['-1', '-f', str(migration)], log)
    print(json.dumps({'prepared_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                      'clusters': states, 'rc': 0}))


if __name__ == '__main__':
    main()
