#!/usr/bin/env python3
"""Compare checked-in prost fields with the portable protobuf schema."""
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
proto = (ROOT / 'proto/logger.proto').read_text()
paths = ['crates/dot-logger/src/adapters/remote/wire.rs', 'crates/dot-logger-client/src/wire.rs']
expected = {}
scalars = {'uint32', 'uint64', 'sint64', 'bool', 'bytes'}
for name, body in re.findall(r'message\s+(\w+)\s*\{(.*?)\}', proto, re.S):
    fields = {}
    for modifier, kind, field, tag in re.findall(r'(?m)^\s*(?:(optional|repeated)\s+)?(\w+)\s+(\w+)\s*=\s*(\d+)\s*;', body):
        wire_kind = kind if kind in scalars else 'message'
        modifier = modifier or ('optional' if wire_kind == 'message' else '')
        assert int(tag) not in fields, (name, tag)
        fields[int(tag)] = (field, wire_kind, modifier)
    expected[name] = fields
for path in paths:
    rust = (ROOT / path).read_text()
    actual = {}
    for name, body in re.findall(r'pub\(crate\) struct\s+(\w+)\s*\{(.*?)\n\}', rust, re.S):
        fields = {}
        for attributes, field in re.findall(r'#\[prost\((.*?)\)\]\s*pub\s+(\w+)\s*:', body, re.S):
            tag = re.search(r'tag\s*=\s*"(\d+)"', attributes)
            assert tag, (name, field)
            kind = attributes.split(',')[0].strip().split('=')[0].strip()
            modifier = 'repeated' if 'repeated' in attributes else 'optional' if 'optional' in attributes else ''
            fields[int(tag.group(1))] = (field, kind, modifier)
        actual[name] = fields
    assert expected == actual, f'protobuf/prost field or wire-type drift: {path}'
compiler = shutil.which('protoc')
if compiler:
    with tempfile.TemporaryDirectory() as directory:
        subprocess.run([compiler, '--proto_path=' + str(ROOT / 'proto'),
                        '--descriptor_set_out=' + str(Path(directory) / 'schema.pb'),
                        str(ROOT / 'proto/logger.proto')], check=True)
print(f'Protocol: {len(expected)} messages / {sum(map(len, expected.values()))} fields match in server and client; '
      f'protoc {"Pass" if compiler else "unavailable (field comparison Pass)"}')
