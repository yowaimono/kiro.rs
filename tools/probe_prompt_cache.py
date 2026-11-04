"""Bounded acceptance probe. Run against an isolated account; never prints API keys."""
import argparse
import json
import time
import urllib.request
import uuid


def call(base, key, path, body):
    request = urllib.request.Request(base.rstrip('/') + path, data=json.dumps(body).encode(),
        headers={'x-api-key': key, 'anthropic-version': '2023-06-01', 'Content-Type': 'application/json'})
    with urllib.request.urlopen(request, timeout=90) as response:
        source = response.headers.get('x-kiro-cache-accounting')
        raw = response.read().decode()
        if not body['stream']:
            usage = json.loads(raw)['usage']
        else:
            events = [json.loads(line[6:]) for line in raw.splitlines() if line.startswith('data: ')]
            starts = [event['message']['usage'] for event in events if event.get('type') == 'message_start']
            finals = [event['usage'] for event in events if event.get('type') == 'message_delta']
            assert len(starts) == 1 and len(finals) == 1, 'Missing final SSE usage'
            assert any(event.get('type') == 'message_stop' for event in events), 'Incomplete SSE response'
            usage = finals[0]
            for field in ('cache_read_input_tokens', 'cache_creation_input_tokens'):
                assert starts[0][field] == usage[field], 'Start/final cache counters differ'
        assert source == 'local-estimate', 'Missing estimate source header'
        assert usage['cache_accounting'] == {'source': 'local_estimate', 'upstream_reported': False}
        for name in ('input_tokens', 'cache_read_input_tokens', 'cache_creation_input_tokens'):
            assert isinstance(usage[name], int) and usage[name] >= 0
        creation = usage['cache_creation']
        assert creation['ephemeral_5m_input_tokens'] + creation['ephemeral_1h_input_tokens'] == usage['cache_creation_input_tokens']
        return usage


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--base', default='http://127.0.0.1:18991')
    parser.add_argument('--config', required=True)
    options = parser.parse_args()
    with open(options.config, encoding='utf-8') as handle:
        key = json.load(handle)['apiKey']
    for name, path, streaming in [('json', '/v1/messages', False), ('sse', '/v1/messages', True), ('buffered_sse', '/cc/v1/messages', True)]:
        seed = uuid.uuid4().hex
        prefix = ' '.join(f'{seed} cache acceptance record {i}: alpha beta gamma delta epsilon' for i in range(500))
        body = {'model': 'claude-haiku-4-5-20251001', 'max_tokens': 16, 'stream': streaming,
            'system': [{'type': 'text', 'text': prefix, 'cache_control': {'type': 'ephemeral'}}],
            'messages': [{'role': 'user', 'content': 'Reply with exactly OK'}]}
        first = call(options.base, key, path, body)
        assert first['cache_read_input_tokens'] == 0 and first['cache_creation_input_tokens'] > 0, 'Cold prefix should be creation'
        time.sleep(1)
        second = call(options.base, key, path, body)
        assert second['cache_read_input_tokens'] > 0 and second['cache_creation_input_tokens'] == 0, 'Repeated prefix should read'
        print(json.dumps({'mode': name, 'first': first, 'repeat': second}), flush=True)
    body['stream'] = False
    body['system'][0].pop('cache_control')
    last = call(options.base, key, '/v1/messages', body)
    assert last['cache_read_input_tokens'] == 0 and last['cache_creation_input_tokens'] == 0
    print(json.dumps({'mode': 'no_hint', 'usage': last}), flush=True)


if __name__ == '__main__':
    main()
