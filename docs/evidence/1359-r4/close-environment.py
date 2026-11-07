#!/usr/bin/env python3
"""Stop only identity-verified R4-owned fixtures after the recorded full run."""
import datetime
import json
import pathlib
import subprocess

OUT = pathlib.Path(__file__).resolve().parent
PG = pathlib.Path('/opt/homebrew/opt/postgresql@16/bin')


def main():
    full = json.loads((OUT / 'full.json').read_text())
    assert full.get('ended_at'), 'full command has not ended'
    clusters = json.loads((OUT / 'environment.json').read_text())
    for cluster in clusters:
        port, directory = cluster['port'], cluster['data_directory']
        identity = subprocess.check_output([
            str(PG / 'psql'), '-h', '127.0.0.1', '-p', str(port), '-U',
            'postgres', '-d', 'postgres', '-XAt', '-v', 'ON_ERROR_STOP=1',
            '-c', "SELECT current_setting('data_directory') || '|' || current_setting('port') || '|' || current_setting('listen_addresses')"
        ], text=True).strip()
        assert identity == f'{directory}|{port}|127.0.0.1', identity
        cluster['identity_before_stop'] = identity
    for cluster in clusters:
        command = [str(PG / 'pg_ctl'), '-D', cluster['data_directory'],
                   '-m', 'fast', '-w', 'stop']
        result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (OUT / f"pg-stop-{cluster['port']}.log").write_bytes(result.stdout)
        cluster.update(command=command, rc=result.returncode,
                       stopped_at=datetime.datetime.now(datetime.timezone.utc).isoformat())
        (OUT / 'cleanup.json').write_text(json.dumps(clusters, indent=2) + '\n')
        assert result.returncode == 0, result.stdout.decode()
    print(json.dumps(clusters, indent=2))


if __name__ == '__main__':
    main()
