#!/usr/bin/env python3
"""Run TC-008 against one digest-pinned, ephemeral, loopback-only InfluxDB.

`--server v2` (default) is the supported OSS 2.9.1 target and every check must
pass. `--server v3` (TASK-046) runs the same tests and example against
InfluxDB 3 Core through its v2-compatible write API; their results are recorded
per test rather than failing the run, and the stored rows are dumped with SQL
for comparison, because v3 has no Flux query API.
"""
import argparse
import datetime
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import time
import urllib.error
import urllib.request

IMAGE = 'influxdb@sha256:a66a9e8b2c93f58bec088c17ff5876540caa095436df5a66e908bdbdb6a36205'
VERSION = '2.9.1'
# InfluxDB 3 Core `3-core` as of 2026-11-27 (TASK-046 S0).
IMAGE_V3 = 'influxdb@sha256:9eb55d1873ea629e497179cc465d1d345a78f4bce5c5bc318d3d3ac0b8647022'
VERSION_V3 = '3.12.3'
ORG = 'dot-logger-test'
BUCKET = 'tc008-exclusive'
TEST_LINE = re.compile(rb'^test (\S+) \.\.\. (ok|FAILED|ignored)$', re.MULTILINE)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--toolchain', default='1.96.0')
    parser.add_argument('--server', choices=['v2', 'v3'], default='v2')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--pipeline-example', action='store_true',
                        help='also verify TASK-020 S2 integration and its CLI against this owned DB')
    parser.add_argument('--example', action='store_true',
                        help='also run the influxdb_write example against this owned DB (TASK-045)')
    args = parser.parse_args()
    if args.pipeline_example and args.server == 'v3':
        parser.error('--pipeline-example is a v2 check (it reads back with Flux)')
    args.output.mkdir(parents=True, exist_ok=True)
    name = 'dot-logger-tc008-' + secrets.token_hex(6)
    created = False
    v3 = args.server == 'v3'
    version = VERSION_V3 if v3 else VERSION
    evidence = {'server': args.server, 'image': IMAGE_V3 if v3 else IMAGE, 'expected_version': version,
                'toolchain': args.toolchain,
                'scope': 'ephemeral Linux amd64 DB functionality; not performance/native OS acceptance'}
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    token = None
    try:
        # No existing volumes/configuration, no credentials in process arguments.
        if v3:
            subprocess.run(['docker', 'run', '--detach', '--name', name, '--label', 'dot-logger.test=tc008',
                            '--cpus', '2', '--memory', '512m', '--pids-limit', '256',
                            '--publish', '127.0.0.1::8181',
                            '--mount', 'type=tmpfs,destination=/var/lib/influxdb3,tmpfs-size=268435456,tmpfs-mode=1777',
                            IMAGE_V3, 'influxdb3', 'serve', '--node-id', 'node0', '--object-store', 'file',
                            '--data-dir', '/var/lib/influxdb3', '--disable-authz', 'health'],
                           check=True, stdout=subprocess.DEVNULL)
        else:
            subprocess.run(['docker', 'run', '--detach', '--name', name, '--label', 'dot-logger.test=tc008',
                            '--cpus', '2', '--memory', '512m', '--pids-limit', '128',
                            '--publish', '127.0.0.1::8086',
                            '--mount', 'type=tmpfs,destination=/var/lib/influxdb2,tmpfs-size=268435456',
                            '--mount', 'type=tmpfs,destination=/etc/influxdb2,tmpfs-size=16777216',
                            IMAGE], check=True, stdout=subprocess.DEVNULL)
        created = True
        port = subprocess.check_output(['docker', 'port', name, ('8181' if v3 else '8086') + '/tcp'],
                                       text=True).strip().split(':')[-1]
        endpoint = 'http://127.0.0.1:' + port
        deadline = time.monotonic() + 45
        while True:
            try:
                with opener.open(endpoint + '/health', timeout=1) as response:
                    body = response.read()
                if v3 and response.status == 200:
                    break
                health = json.loads(body)
                if health.get('status') == 'pass':
                    break
            except (OSError, ValueError, urllib.error.URLError):
                pass
            if time.monotonic() >= deadline:
                raise RuntimeError('isolated database did not become healthy')
            time.sleep(0.2)
        if v3:
            # The first admin token is created over HTTP while none exists.
            request = urllib.request.Request(endpoint + '/api/v3/configure/token/admin', data=b'', method='POST')
            with opener.open(request, timeout=15) as response:
                token = json.load(response)['token']
            bearer = {'Authorization': 'Bearer ' + token}
            with opener.open(urllib.request.Request(endpoint + '/ping', headers=bearer), timeout=5) as response:
                served = json.load(response).get('version', '')
            request = urllib.request.Request(endpoint + '/api/v3/configure/database',
                      data=json.dumps({'db': BUCKET}).encode(),
                      headers=dict(bearer, **{'Content-Type': 'application/json'}), method='POST')
            with opener.open(request, timeout=15) as response:
                assert response.status in (200, 201)
            setup = {'org': ORG, 'bucket': BUCKET}
            evidence.update(version=served)
        else:
            served = health.get('version', '')
            token = secrets.token_urlsafe(48)
            setup = {'username': 'tc008', 'password': secrets.token_urlsafe(24),
                     'org': ORG, 'bucket': BUCKET, 'token': token,
                     'retentionPeriodSeconds': 0}
            request = urllib.request.Request(endpoint + '/api/v2/setup',
                      data=json.dumps(setup).encode(), headers={'Content-Type': 'application/json'})
            with opener.open(request, timeout=15) as response:
                result = json.load(response)
            evidence.update(version=served, organization_id=result['org']['id'], bucket_id=result['bucket']['id'])
        if served.lstrip('v') != version:
            raise RuntimeError('database version differs from profile pin')
        # Synthetic protocol probe: preserve the server's answer without secrets.
        timestamp = time.time_ns()
        headers = {'Authorization': 'Token ' + token}
        line = ('tc_protocol,channel=7,device=robot_1 integer=1i,session="0000000000000000000000000000002a",'
                f'producer=0u,sequence=1u {timestamp}')
        from urllib.parse import urlencode
        write_url = endpoint + '/api/v2/write?' + urlencode({'org': setup['org'], 'bucket': setup['bucket'], 'precision': 'ns'})
        with opener.open(urllib.request.Request(write_url, data=line.encode(), headers=headers), timeout=5) as response:
            assert response.status == 204
        if v3:
            args.output.joinpath('protocol.jsonl').write_bytes(sql(opener, endpoint, token, 'SELECT * FROM tc_protocol'))
        else:
            flux = (f'from(bucket: "{setup["bucket"]}") |> range(start: time(v: {timestamp}), stop: time(v: {timestamp+100})) '
                    '|> filter(fn: (r) => r._measurement == "tc_protocol") '
                    '|> pivot(rowKey: ["_time"], columnKey: ["_field"], valueColumn: "_value") '
                    '|> map(fn: (r) => ({real_ns: int(v: r._time), value: r.integer, session: r.session, producer: r.producer, sequence: r.sequence}))')
            query_headers = dict(headers, **{'Content-Type': 'application/json', 'Accept': 'text/csv'})
            with opener.open(urllib.request.Request(endpoint + '/api/v2/query?' + urlencode({'org': setup['org']}), data=json.dumps({'query': flux, 'type': 'flux', 'dialect': {'annotations': ['datatype', 'group', 'default'], 'header': True}}).encode(), headers=query_headers), timeout=10) as response:
                args.output.joinpath('protocol.csv').write_bytes(response.read(65536))
        env = os.environ.copy()
        env.update(DOT_LOGGER_TEST_INFLUX_URL=endpoint, DOT_LOGGER_TEST_INFLUX_TOKEN=token,
                   DOT_LOGGER_TEST_INFLUX_ORG=setup['org'], DOT_LOGGER_TEST_INFLUX_BUCKET=setup['bucket'])
        cmd = ['cargo', '+' + args.toolchain, 'test', '--locked', '-p', 'dot-logger', '--features', 'influxdb',
               '--test', 'tc_008_influx_live', '--', '--ignored', '--test-threads=1']
        run = subprocess.run(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        # The harness must never print a token, including on failure.
        log = run.stdout.replace(token.encode(), b'[REDACTED]')
        args.output.joinpath('live.log').write_bytes(log)
        log_live = log
        evidence.update(command=cmd, exit_code=run.returncode,
                        tests={m.group(1).decode(): m.group(2).decode() for m in TEST_LINE.finditer(log)})
        if v3:
            print(f"InfluxDB {version}: live TC-008 " + json.dumps(evidence['tests'], sort_keys=True))
        elif run.returncode:
            print(log.decode()[-6000:])
            raise RuntimeError('live TC-008 failed; see live.log')
        else:
            print(f"InfluxDB {VERSION}: live TC-008 passed")
        if args.pipeline_example:
            commands = [
                ('pipeline-test', ['cargo', '+' + args.toolchain, 'test', '--locked',
                 '-p', 'dot-logger-client', '--features', 'integration-example',
                 '--test', 'pipeline', '--', '--ignored', '--test-threads=1']),
                ('pipeline-example', ['cargo', '+' + args.toolchain, 'run', '--locked',
                 '-p', 'dot-logger-client', '--features', 'integration-example',
                 '--example', 'dot_probe_pipeline', '--', '--run',
                 'tc_pipeline_cli_' + secrets.token_hex(6)]),
            ]
            evidence['pipeline_commands'] = []
            for label, command in commands:
                run = subprocess.run(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                log = run.stdout.replace(token.encode(), b'[REDACTED]')
                args.output.joinpath(label + '.log').write_bytes(log)
                evidence['pipeline_commands'].append({'command': command, 'exit_code': run.returncode})
                if run.returncode:
                    print(log.decode()[-6000:])
                    raise RuntimeError('TASK-020 S2 failed; see ' + label + '.log')
            print(f"InfluxDB {VERSION}: TASK-020 S2 integration and CLI passed")
        if args.example:
            command = ['cargo', '+' + args.toolchain, 'run', '--locked', '-p', 'dot-logger',
                       '--features', 'influxdb', '--example', 'influxdb_write']
            example_env = {key: value for key, value in env.items() if not key.startswith('INFLUX_')}
            # Without its settings the example must print usage and exit 2, not touch any DB.
            usage = subprocess.run(command, env=example_env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            args.output.joinpath('example-usage.log').write_bytes(usage.stdout)
            example_env.update(INFLUX_HOST=endpoint, INFLUX_TOKEN=token,
                               INFLUX_ORG=setup['org'], INFLUX_BUCKET=setup['bucket'])
            run = subprocess.run(command, env=example_env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            log = run.stdout.replace(token.encode(), b'[REDACTED]')
            args.output.joinpath('example.log').write_bytes(log)
            read_back = re.search(rb'^read back (\d+)/(\d+) numeric points$', log, re.MULTILINE)
            evidence['example'] = {'command': command, 'usage_exit_code': usage.returncode,
                                   'exit_code': run.returncode,
                                   'read_back': read_back.group(0).decode() if read_back else None}
            if usage.returncode != 2 or b'INFLUX_HOST' not in usage.stdout:
                print(usage.stdout.decode()[-6000:])
                raise RuntimeError('influxdb_write without settings must print usage and exit 2; see example-usage.log')
            if v3:
                print(f"InfluxDB {version}: influxdb_write exit {run.returncode}"
                      f" ({read_back.group(0).decode() if read_back else 'no read-back line'})")
            elif run.returncode or not read_back or read_back.group(1) != read_back.group(2) or read_back.group(1) == b'0':
                print(log.decode()[-6000:])
                raise RuntimeError('influxdb_write example failed; see example.log')
            else:
                print(f"InfluxDB {VERSION}: influxdb_write example passed ({read_back.group(0).decode()})")
        if v3:
            # What actually reached the DB, for comparison with each test's writes.
            tables = json.loads(b'[' + b','.join(sql(opener, endpoint, token,
                     "SELECT table_name FROM information_schema.tables WHERE table_schema = 'iox'").splitlines()) + b']')
            dump = b''
            for table in sorted(row['table_name'] for row in tables):
                if not re.fullmatch(r'[a-z0-9_]+', table):
                    raise RuntimeError('unexpected table name in the isolated database')
                rows = sql(opener, endpoint, token, f'SELECT * FROM "{table}" ORDER BY time')
                dump += json.dumps({'table': table}).encode() + b'\n' + rows
            args.output.joinpath('sql-dump.jsonl').write_bytes(dump)
            evidence['tables'] = sorted(row['table_name'] for row in tables)
            problems = v3_expectations(log_live, evidence, args.output)
            evidence['v3_problems'] = problems
            if problems:
                raise RuntimeError('InfluxDB 3 differs from the recorded support range: ' + '; '.join(problems))
            print(f"InfluxDB {version}: v2-compatible writes match; Flux queries are not served (404)")
    finally:
        if created:
            cleanup = subprocess.run(['docker', 'rm', '--force', '--volumes', name],
                                      stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            evidence['cleanup_exit_code'] = cleanup.returncode
            if cleanup.returncode:
                print('Could not remove this task container: ' + name)
        args.output.joinpath('environment.json').write_text(json.dumps(evidence, indent=2) + '\n')
        if evidence.get('cleanup_exit_code', 0):
            raise RuntimeError('isolated database cleanup failed')


# TASK-046: InfluxDB 3 Core accepts the v2 write API but serves no Flux query,
# and rejects a whole batch that contains a type conflict.
V3_ROUNDTRIP = [('integer', 9223372036854775807), ('integer', -9223372036854775808), ('boolean', True),
                ('float', 2.5), ('event', 'quoted"event')]
V3_EXAMPLE = [('1', 'float', 21.5), ('1', 'float', 21.75), ('2', 'event', 'door opened'), ('1', 'float', 22.0)]


def v3_expectations(live_log, evidence, output):
    """Each live test may fail only at its Flux read-back; stored rows must match the writes."""
    problems = []
    panics = re.findall(rb"thread '(\w+)'(?: \(\d+\))? panicked at [^\n]*\n([^\n]*)", live_log)
    failed = {name.decode(): message.decode() for name, message in panics}
    for test, outcome in evidence.get('tests', {}).items():
        if outcome == 'FAILED' and 'Http(404)' not in failed.get(test, ''):
            problems.append(f'{test} failed before its Flux query')
    if len(evidence.get('tests', {})) != 4:
        problems.append('expected four live tests')
    tables, table = {}, None
    for line in output.joinpath('sql-dump.jsonl').read_text().splitlines():
        row = json.loads(line)
        if list(row) == ['table']:
            table = row['table']
            tables[table] = []
        else:
            tables[table].append(row)
    roundtrip = [(field, row[field]) for row in tables.get('tc_roundtrip', [])
                 for field in ('integer', 'boolean', 'float', 'event') if field in row]
    if roundtrip != V3_ROUNDTRIP:
        problems.append('tc_roundtrip values differ: ' + repr(roundtrip))
    stamps = [unix_ns(row['time']) for row in tables.get('tc_roundtrip', [])]
    if [stamp - stamps[0] for stamp in stamps] != [0, 1, 2, 3, 4]:
        problems.append('tc_roundtrip nanosecond timestamps differ')
    if any('boolean' in row for row in tables.get('tc_partial', [])):
        problems.append('tc_partial kept valid rows of a rejected batch (now matches v2; update the support table)')
    if 'example' in evidence:
        example = [(row['channel'], field, row[field]) for row in tables.get('influxdb_write_example', [])
                   for field in ('float', 'event') if field in row]
        log = output.joinpath('example.log').read_bytes()
        if example != V3_EXAMPLE or b'write: Acknowledged' not in log or b'numeric query: Http(404)' not in log:
            problems.append('influxdb_write did not write and then fail only at its Flux read-back')
    return problems


def unix_ns(stamp):
    """SQL `time` such as 2026-11-27T09:41:57.292310069 (UTC, nanoseconds) as an integer."""
    seconds, _, fraction = stamp.partition('.')
    whole = datetime.datetime.fromisoformat(seconds).replace(tzinfo=datetime.timezone.utc).timestamp()
    return int(whole) * 1_000_000_000 + int(fraction.ljust(9, '0'))


def sql(opener, endpoint, token, query):
    """One v3 SQL query as JSON lines; the token never leaves the request."""
    request = urllib.request.Request(endpoint + '/api/v3/query_sql',
              data=json.dumps({'db': BUCKET, 'q': query, 'format': 'jsonl'}).encode(),
              headers={'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json'})
    with opener.open(request, timeout=10) as response:
        return response.read(4 * 1024 * 1024)


if __name__ == '__main__':
    main()
