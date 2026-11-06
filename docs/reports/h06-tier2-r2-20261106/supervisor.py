import datetime, hashlib, json, os, pathlib, shlex, shutil, subprocess, sys, traceback
ROOT = pathlib.Path('/private/tmp/svc-h06-manifest')
EVD = pathlib.Path('/private/tmp/h06-tier2-r2-evidence')
DATA = pathlib.Path('/private/tmp/h06-tier2-pg')
ARCHIVE = ROOT / 'docs/reports/h06-tier2-r2-20261106'
PG = '/opt/homebrew/opt/postgresql@16/bin/'
ENV = {
    'HOME': '/Users/yanfenma', 'USER': 'yanfenma', 'LOGNAME': 'yanfenma',
    'PATH': PG + ':/Users/yanfenma/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin',
    'TMPDIR': '/private/tmp', 'PSQLRC': '/dev/null',
    'LC_ALL': 'C', 'LANG': 'C', 'TRUSTED_FLEET_LOCAL_PG': '1',
}
commands = []
result = {'source_head': 'bcd8950466091f4cb4e0133c431186d9267ac2c2',
          'base_head': '59daec042786f0416a6a9061c0b638ba56c0d83e',
          'environment': ENV, 'suite_rc': None, 'conformance_execution': 'NOT_RUN',
          'start_rc': None, 'stop_rc': None, 'attempts': 1,
          'production_db_connections': 0, 'production_db_or_service_writes': 0,
          'system_service_registration': 'NONE', 'error': None}

def now(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def save(name, value): (EVD / name).write_text(json.dumps(value, indent=2, ensure_ascii=False) + chr(10))
def run(name, args, required=False):
    item = {'name': name, 'argv': args, 'command': shlex.join(args), 'start': now()}
    print('RUN ' + name + ': ' + item['command'], flush=True)
    with (EVD / (name + '.log')).open('wb') as log:
        process = subprocess.Popen(args, cwd=ROOT, env=ENV, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        while True:
            data = process.stdout.read1(65536)
            if not data: break
            log.write(data); log.flush()
            sys.stdout.buffer.write(data); sys.stdout.buffer.flush()
        rc = process.wait()
    item.update(end=now(), rc=rc)
    commands.append(item); save('commands.json', commands)
    print('RC ' + name + '=' + str(rc), flush=True)
    if required and rc != 0: raise RuntimeError(name + ' rc=' + str(rc))
    return rc

def source_hashes():
    paths = subprocess.check_output(['git', 'ls-files', '-z', 'src', 'tests', 'migrations', 'Cargo.toml', 'Cargo.lock', 'workflow_trusted_fleet_successor_plan_v2.json'], cwd=ROOT, env=ENV).decode().split(chr(0))
    return {p: digest(ROOT / p) for p in paths if p}

started_attempt = False
before = None
try:
    save('environment.json', ENV)
    before = source_hashes(); save('source-hashes-before.json', before)
    result['runner_sha256'] = digest(ROOT / 'scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh')
    result['fixture_bindings'] = []
    for p in [ROOT / 'workflow_trusted_fleet_successor_plan_v2.json', pathlib.Path('/Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json')]:
        binding = {'path': str(p), 'size': p.stat().st_size, 'sha256': digest(p)}
        result['fixture_bindings'].append(binding)
        assert binding['size'] == 540472 and binding['sha256'] == '0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606', binding
    assert not ARCHIVE.exists(), 'archive already exists'
    assert (DATA.stat().st_mode & 0o777) == 0o700
    assert (DATA / 'PG_VERSION').read_text().strip() == '16'
    assert not (DATA / 'postmaster.pid').exists()
    assert not list(DATA.glob('.s.PGSQL.*'))
    shutil.copyfile(DATA / 'start.log', EVD / 'regulator-start.log')
    result['regulator_start_log_sha256'] = digest(DATA / 'start.log')
    shutil.copyfile(ROOT / 'scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh', EVD / 'runner.sh')
    run('git-before', ['git', 'status', '--short'], True)
    run('runner-r2-diff', ['git', 'diff', 'HEAD', '--', 'scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh'], True)
    run('runner-full-diff', ['git', 'diff', '5a7dc9ee2dde1dd730dae94afb39f116160aec2c', '--', 'scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh'], True)
    run('tool-version', [PG + 'pg_ctl', '--version'], True)
    run('runner-syntax', ['/bin/bash', '-n', 'scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh'], True)
    status = run('pg-status-before', [PG + 'pg_ctl', '-D', str(DATA), 'status'])
    assert status == 3, 'cluster is already running or status unknown'
    started_attempt = True
    result['start_rc'] = run('pg-start', [PG + 'pg_ctl', '-D', str(DATA), '-l', str(EVD / 'postgres.log'), '-w', '-t', '30', '-o', "-p 55432 -h '' -k /private/tmp/h06-tier2-pg -c unix_socket_permissions=0700 -c log_connections=on -c log_disconnections=on", 'start'])
    if result['start_rc'] != 0: raise RuntimeError('pg-start failed')
    psql = [PG + 'psql', 'postgres://postgres@localhost:55432/postgres?host=/private/tmp/h06-tier2-pg', '-XAt', '-v', 'ON_ERROR_STOP=1']
    identity = "SELECT json_build_object('data_directory', current_setting('data_directory'), 'port', current_setting('port'), 'socket', current_setting('unix_socket_directories'), 'listen_addresses', current_setting('listen_addresses'), 'server_version_num', current_setting('server_version_num'), 'user', current_user, 'is_superuser', current_setting('is_superuser'), 'server_ip', inet_server_addr(), 'database', current_database(), 'databases', (SELECT json_agg(datname ORDER BY datname) FROM pg_database))"
    run('pg-identity', psql + ['-c', identity], True)
    observed = json.loads((EVD / 'pg-identity.log').read_text())
    result['identity'] = observed
    assert observed['data_directory'] == str(DATA) and observed['port'] == '55432' and observed['socket'] == str(DATA)
    assert observed['listen_addresses'] == '' and observed['server_ip'] is None
    assert 160000 <= int(observed['server_version_num']) <= 169999
    assert observed['user'] == 'postgres' and observed['is_superuser'] == 'on' and observed['database'] == 'postgres'
    assert observed['databases'] == ['postgres', 'template0', 'template1']
    result['conformance_execution'] = 'EXECUTED'
    result['suite_rc'] = run('conformance', ['/bin/bash', 'scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh'])
    if result['suite_rc'] != 0: raise RuntimeError('conformance failed rc=' + str(result['suite_rc']))
except BaseException as exc:
    result['error'] = str(exc)
    (EVD / 'supervisor-error.log').write_text(traceback.format_exc())
    print('STOPPED: ' + str(exc), flush=True)
finally:
    if started_attempt:
        try:
            result['stop_rc'] = run('pg-stop', [PG + 'pg_ctl', '-D', str(DATA), '-m', 'fast', '-w', '-t', '30', 'stop'])
        except BaseException as exc:
            result['cleanup_error'] = str(exc)
    try:
        result['status_after_rc'] = run('pg-status-after', [PG + 'pg_ctl', '-D', str(DATA), 'status'])
        result['postmaster_pid_exists'] = (DATA / 'postmaster.pid').exists()
        result['socket_paths_after'] = [str(p) for p in DATA.glob('.s.PGSQL.*')]
        result['data_mode_after'] = oct(DATA.stat().st_mode & 0o777)
        result['data_retained'] = DATA.exists()
        inspected = subprocess.run(['/bin/ps', '-axo', 'pid=,command='], env=ENV, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
        lines = inspected.stdout.decode().splitlines()
        matched = [line for line in lines if ('postgres -D ' + str(DATA)) in line or ('/bin/postgres ' in line and str(DATA) in line)]
        result['postgres_processes_after'] = matched
        save('process-inspection.json', {'method': 'ps -axo pid=,command=; retain only postgres processes for the exact data directory', 'matches': matched, 'rc': inspected.returncode, 'observed_at': now()})
        after = source_hashes(); save('source-hashes-after.json', after)
        result['source_hashes_unchanged'] = before == after
        result['regulator_start_log_unchanged'] = digest(DATA / 'start.log') == result.get('regulator_start_log_sha256')
        result['result'] = 'PASS' if (result['suite_rc'] == 0 and result['stop_rc'] == 0 and result['status_after_rc'] == 3 and not result['postmaster_pid_exists'] and not result['socket_paths_after'] and not matched and result['source_hashes_unchanged'] and result['regulator_start_log_unchanged'] and result['error'] is None) else 'STOPPED'
    except BaseException as exc:
        result['postcheck_error'] = str(exc); result['result'] = 'STOPPED'
    result['completed_at'] = now()
    save('result.json', result); save('commands.json', commands)
    sums = ''.join(digest(p) + '  ' + p.name + chr(10) for p in sorted(EVD.iterdir()) if p.is_file() and p.name != 'SHA256SUMS')
    (EVD / 'SHA256SUMS').write_text(sums)
    if not ARCHIVE.exists(): shutil.copytree(EVD, ARCHIVE)
    print('FINAL ' + json.dumps(result, ensure_ascii=False), flush=True)
sys.exit(0 if result.get('result') == 'PASS' else 1)
