# Admin UI local preview

The admin UI uses the existing React, Vite and Tailwind stack. Brand colors are
semantic CSS variables in `src/index.css`. Logo and favicon assets in `public/`
are derived from the user supplied cloud logo; no account credentials are included.

## Development

```powershell
$env:KIRO_ADMIN_API_TARGET = "http://127.0.0.1:8889"
pnpm install --frozen-lockfile
pnpm dev --host 127.0.0.1 --port 18889
```

Open `http://127.0.0.1:18889/admin/`. Without an override, the API proxy retains
its existing default, `http://localhost:8080`.

## Docker preview of frontend changes

From this directory, run `pnpm build`. Then from the repository root:

```powershell
docker build -f admin-ui/Dockerfile.preview -t kiro-admin:local admin-ui
```

The preview image serves the built admin UI and forwards API requests to
`kiro-rs:8990`. Run it on the same Docker network as a backend service named
`kiro-rs`, and publish the preview container port 80 to a local host port.
The backend keeps its existing config/credentials mount. Nginx disables proxy
buffering so streaming API responses continue to pass through.

The original root Dockerfile still builds a single Rust image with the admin UI
embedded, including these frontend changes. The preview image is only a faster
local option for frontend work.
