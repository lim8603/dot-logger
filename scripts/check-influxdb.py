#!/usr/bin/env python3
"""Run TC-008 against one digest-pinned, ephemeral, loopback-only v2 DB."""
import argparse
import json
import os
from pathlib import Path
import secrets
import subprocess
import time
import urllib.error
import urllib.request

IMAGE = 'influxdb@sha256:a66a9e8b2c93f58bec088c17ff5876540caa095436df5a66e908bdbdb6a36205'
VERSION = '2.9.1'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--toolchain', default='1.96.0')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--pipeline-example', action='store_true',
                        help='also verify TASK-020 S2 integration and its CLI against this owned DB')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    name = 'dot-logger-tc008-' + secrets.token_hex(6)
    created = False
    evidence = {'image': IMAGE, 'expected_version': VERSION, 'toolchain': args.toolchain,
                'scope': 'ephemeral Linux amd64 DB functionality; not performance/native OS acceptance'}
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        # No existing volumes/configuration, no credentials in process arguments.
        subprocess.run(['docker', 'run', '--detach', '--name', name, '--label', 'dot-logger.test=tc008',
                        '--cpus', '2', '--memory', '512m', '--pids-limit', '128',
                        '--publish', '127.0.0.1::8086',
                        '--mount', 'type=tmpfs,destination=/var/lib/influxdb2,tmpfs-size=268435456',
                        '--mount', 'type=tmpfs,destination=/etc/influxdb2,tmpfs-size=16777216',
                        IMAGE], check=True, stdout=subprocess.DEVNULL)
        created = True
        port = subprocess.check_output(['docker', 'port', name, '8086/tcp'], text=True).strip().split(':')[-1]
        endpoint = 'http://127.0.0.1:' + port
        deadline = time.monotonic() + 45
        while True:
            try:
                with opener.open(endpoint + '/health', timeout=1) as response:
                    health = json.load(response)
                if health.get('status') == 'pass':
                    break
            except (OSError, urllib.error.URLError):
                pass
            if time.monotonic() >= deadline:
                raise RuntimeError('isolated database did not become healthy')
            time.sleep(0.2)
        if health.get('version', '').lstrip('v') != VERSION:
            raise RuntimeError('database version differs from profile pin')
        token = secrets.token_urlsafe(48)
        setup = {'username': 'tc008', 'password': secrets.token_urlsafe(24),
                 'org': 'dot-logger-test', 'bucket': 'tc008-exclusive', 'token': token,
                 'retentionPeriodSeconds': 0}
        request = urllib.request.Request(endpoint + '/api/v2/setup',
                  data=json.dumps(setup).encode(), headers={'Content-Type': 'application/json'})
        with opener.open(request, timeout=15) as response:
            result = json.load(response)
        # Synthetic protocol probe: preserve server CSV dialect without secrets.
        timestamp = time.time_ns()
        headers = {'Authorization': 'Token ' + token}
        line = ('tc_protocol,channel=7,device=robot_1 integer=1i,session="0000000000000000000000000000002a",'
                f'producer=0u,sequence=1u {timestamp}')
        from urllib.parse import urlencode
        write_url = endpoint + '/api/v2/write?' + urlencode({'org': setup['org'], 'bucket': setup['bucket'], 'precision': 'ns'})
        with opener.open(urllib.request.Request(write_url, data=line.encode(), headers=headers), timeout=5) as response:
            assert response.status == 204
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
        evidence.update(version=health['version'], organization_id=result['org']['id'],
                        bucket_id=result['bucket']['id'], command=cmd, exit_code=run.returncode)
        if run.returncode:
            print(log.decode()[-6000:])
            raise RuntimeError('live TC-008 failed; see live.log')
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


if __name__ == '__main__':
    main()
