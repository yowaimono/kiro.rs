# Estimated prompt-cache accounting

Ported from M-JYuan/kiro.rs commit fc3528317bdc527d5c57212d646e9f379aada602 (MIT; LICENSE.cache-tracker).
This is a local prefix-accounting policy, not a Kiro-reported token measurement and not a response cache.
Every request still goes to Kiro. Kiro charges are unchanged by this bookkeeping.

Configuration:

```json
{"promptCacheAccountingEnabled": true, "promptCacheTtlSeconds": 300}
```

The feature defaults to false. Enable explicitly for downstream cache billing. Disabling reverts to the original usage shape.

Clients must send cache_control with type ephemeral on tool definitions, system blocks or message blocks. TTL 1h is supported only when the configured maximum permits it. No hints or prompts below the model threshold produce cache hits. The port follows the reference checkpoint/lookback/TTL policy; this policy is not proof of actual upstream cache lifetime.

Entries contain hashes and token estimates, are scoped to the actual successful credential and model, are held in memory, expire, and are bounded to 8192. A restart starts cold. Failed, malformed, incomplete or cancelled responses do not warm the table. Repeated concurrent cold requests may both be charged as creation (conservative; no speculative hit).

Ordinary JSON and /v1 and /cc/v1 SSE responses include cache_creation_input_tokens, cache_read_input_tokens, and 5m/1h creation detail. input_tokens excludes these cached portions so they are not billed twice. Values are clamped so input + read + creation equals the previous total; output is unchanged. Web-search-only requests keep their existing accounting and do not participate.

Responses identify the source in the x-kiro-cache-accounting header and usage.cache_accounting = {source: local_estimate, upstream_reported: false}. Sub2API may drop unknown metadata when proxying; its customer-facing documentation must call these estimated cache counters. Never advertise them as Kiro's measured cache tokens.

Rollback: disable promptCacheAccountingEnabled and restart kiro-rs, or restore the backed-up compose and config with the previous fixed image. Do not restore older credential files over newly refreshed credentials.
