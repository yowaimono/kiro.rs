# Client API keys

The administration console has a **客户端密钥** page (/admin/#client-keys). These keys authenticate requests to /v1 and /cc/v1; they are separate from both the admin key and Kiro upstream account credentials.

Administrators can create a named key, copy the full secret once, list masked metadata, disable/re-enable it, or delete it with confirmation. Managed keys share the entire account pool and do not implement per-key quotas, model groups or rate limits. Disabling/deleting affects new requests immediately; already authenticated in-flight streams finish normally.

The existing config.json apiKey remains valid for backward compatibility, including current Sub2API integrations. It is identified as a legacy key in the UI and cannot be viewed or revoked there. Rotate it through the service configuration and restart; do not use the admin key as the client key.

## Persistence and security

The service stores client_api_keys.json next to the selected configuration file. The existing /app/config bind mount therefore persists managed keys. No new database or migration is required. Back up this file with the configuration. Stores contain SHA-256 hashes plus masked key fragments and metadata; the full managed secret is never recoverable. New files use mode 0600 on Unix, atomic replacement, and serialized mutations. Write failures leave live authentication unchanged. A malformed existing store stops startup instead of discarding keys. A missing store starts empty and is created on the first successful mutation. One service process should own the file; concurrent replicas do not synchronize key state.

Raw secrets are returned only by POST /api/admin/client-keys and never included in list responses, logs, browser storage or query caches. Key administration requires the existing admin key. Metadata responses use Cache-Control: no-store. The UI clears query data on logout. UUID v4 generation is backed by OS randomness; two UUIDs provide 244 bits for each key.

## Admin API

All routes require x-api-key or Authorization: Bearer with the ADMIN key.

- GET /api/admin/client-keys → {keys: [...masked metadata], legacyKeyEnabled: boolean}
- POST /api/admin/client-keys with {name: string} → 201 {key: secret, item: metadata}
- POST /api/admin/client-keys/{id}/disabled with {disabled: boolean} → metadata
- DELETE /api/admin/client-keys/{id} → 204

Names must contain 1–80 characters after trimming and no control characters; at most 1000 managed keys may be stored. The client sends its generated secret via x-api-key or Authorization: Bearer for model/message/count-token calls. Disabled and deleted secrets receive 401.

This feature does not change upstream credentials or prompt-cache accounting. Deploying the backend alone does not update a separate nginx admin UI container: rebuild both the full kiro-rs image and the branded admin-ui image when releasing this feature.
