# 08 - API Server Architecture

## Forge HTTP REST API Server (v2.0.0)

> **Updated (2026-11-08):** The configuration, middleware and security sections below were rewritten from the code (they described an API open to every origin with no authentication). The MCP server is separate: `forge mcp` speaks MCP over stdio (rmcp) and does not go through this HTTP API. Handlers call the synchronous core functions directly; there is no `spawn_blocking`.

**Last Updated:** 2025-11-25

**Status:** Complete

**Coverage:** Enterprise HTTP API server implementation

---

## Overview

The Forge API Server provides a production-ready HTTP REST API for enterprise integrations. Built on Axum and Tokio, it exposes all Forge operations through RESTful endpoints.

```
┌─────────────────────────────────────────────────────────────────┐
│                      External Systems                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐ │
│  │ Web Apps │  │   CI/CD  │  │ Scripts  │  │ AI Agents (MCP)  │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬─────────┘ │
└───────┼─────────────┼─────────────┼─────────────────┼───────────┘
        │             │             │                 │
        └──────────┬──┴─────────────┴─────────────────┘
                   │ HTTP/JSON
                   ▼
┌──────────────────────────────────────────────────────────────────┐
│                    Forge API Server                               │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                   Tower Middleware Stack                    │  │
│  │  ┌────────────┐  ┌──────────────┐  ┌────────────────────┐  │  │
│  │  │ Host/token │  │   Tracing    │  │ CORS (opt-in only) │  │  │
│  │  └────────────┘  └──────────────┘  └────────────────────┘  │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                    Axum Router                              │  │
│  │  ┌──────────┐  ┌────────────┐  ┌──────────────────────┐   │  │
│  │  │ /health  │  │ /version   │  │ /api/v1/*            │   │  │
│  │  └──────────┘  └────────────┘  └──────────────────────┘   │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                    Request Handlers                         │  │
│  │  ┌──────────┐  ┌────────────┐  ┌───────┐  ┌──────────────┐│  │
│  │  │ validate │  │ calculate  │  │ audit │  │ export/import││  │
│  │  └──────────┘  └────────────┘  └───────┘  └──────────────┘│  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                    CLI Command Layer                        │  │
│  │             (Reuses existing CLI functions)                 │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

## Module Structure

```
src/api/
├── mod.rs           # Module exports
├── server.rs        # Axum server configuration
└── handlers.rs      # Request handlers
```

### Module Dependencies

```
src/api/
    ├── mod.rs
    │   └── exports: run_api_server, server::*, handlers::*
    │
    ├── server.rs
    │   ├── depends: axum, tower_http, tracing
    │   └── provides: ApiConfig, AppState, run_api_server()
    │
    └── handlers.rs
        ├── depends: axum, serde, uuid
        ├── depends: crate::cli (validate, calculate, audit, export, import)
        └── provides: All endpoint handlers
```

---

## API Endpoints

### Core Operations

| Endpoint | Method | Description | Request Body |
|----------|--------|-------------|--------------|
| `/api/v1/validate` | POST | Validate YAML model | `{"file_path": "model.yaml"}` |
| `/api/v1/calculate` | POST | Calculate formulas | `{"file_path": "model.yaml", "dry_run": false}` |
| `/api/v1/audit` | POST | Audit variable | `{"file_path": "model.yaml", "variable": "total"}` |
| `/api/v1/export` | POST | Export to Excel | `{"yaml_path": "model.yaml", "excel_path": "out.xlsx"}` |
| `/api/v1/import` | POST | Import from Excel | `{"excel_path": "in.xlsx", "yaml_path": "out.yaml"}` |

### Utility Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | API documentation and endpoint list |
| `/health` | GET | Health check (returns `{"status": "healthy"}`) |
| `/version` | GET | Server version and feature list |

---

## Request/Response Format

### Standard Response Wrapper

All responses use a consistent JSON envelope:

```json
{
  "success": true,
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "data": { ... },
  "error": null
}
```

### Response Types

```rust
// Success response
ApiResponse::ok(data)
// -> { "success": true, "request_id": "...", "data": {...} }

// A failed validation or calculation is still `ok`: the failure is in `data`
// -> { "success": true, "request_id": "...", "data": { "valid": false, ... } }
```

---

## Configuration

### Server Configuration

```rust
pub struct ApiConfig {
    pub host: String,              // Default: "127.0.0.1"
    pub port: u16,                 // Default: 8080
    pub token: Option<String>,     // Required on /api/v1/* when set
    pub cors_origins: Vec<String>, // None by default
}
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `FORGE_HOST` | Host address to bind | `127.0.0.1` |
| `FORGE_PORT` | Port to listen on | `8080` |
| `FORGE_API_TOKEN` | Token required on `/api/v1/*` | none |

### CLI Arguments

```bash
forge serve [OPTIONS]

Options:
  -H, --host <HOST>  Host address [default: 127.0.0.1] [env: FORGE_HOST]
  -p, --port <PORT>  Port number [default: 8080] [env: FORGE_PORT]
      --token <TOKEN>          Token required on /api/v1/* [env: FORGE_API_TOKEN]
      --cors-origin <ORIGIN>   Browser origin allowed to call the API (repeatable; none by default)
  -h, --help         Print help
  -V, --version      Print version
```

---

## Middleware Stack

### Layers

`build_router` in `src/api/server.rs`:

- a guard on every route: it rejects a request whose `Host` header is not this machine (403, against DNS rebinding) when the server listens on a loopback address, and a request to `/api/v1/*` without `Authorization: Bearer <token>` (401) when a token is set;
- a CORS layer only when `--cors-origin` is given: those origins, `GET` and `POST`, `Content-Type` and `Authorization` headers; `*` is refused;
- `TraceLayer` for request tracing.

Forge refuses to listen on a non-loopback address without a token.

---

## Graceful Shutdown

The server handles shutdown signals gracefully:

```rust
async fn shutdown_signal() {
    // Handle Ctrl+C
    let ctrl_c = tokio::signal::ctrl_c();

    // Handle SIGTERM (Unix)
    #[cfg(unix)]
    let terminate = tokio::signal::unix::signal(SignalKind::terminate());

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
```

---

## Sequence Diagram

```
Client                 API Server             CLI Layer            Forge Core
   │                       │                      │                    │
   │  POST /api/v1/validate                       │                    │
   │ ──────────────────────>                      │                    │
   │  {"file_path": "x.yaml"}                     │                    │
   │                       │                      │                    │
   │                       │  cli_validate(path)  │                    │
   │                       │ ──────────────────────>                   │
   │                       │                      │                    │
   │                       │                      │  parse_model()     │
   │                       │                      │ ────────────────────>
   │                       │                      │                    │
   │                       │                      │  <── ParsedModel ──│
   │                       │                      │                    │
   │                       │                      │  ArrayCalculator   │
   │                       │                      │     .validate()    │
   │                       │                      │ ────────────────────>
   │                       │                      │                    │
   │                       │  <── Ok/Err ─────────│                    │
   │                       │                      │                    │
   │  <── ApiResponse ─────│                      │                    │
   │  {"success": true,    │                      │                    │
   │   "data": {...}}      │                      │                    │
   │                       │                      │                    │
```

---

## Error Handling

### HTTP Status Codes

| Code | Usage |
|------|-------|
| 200 | Every handled request, including a failed validation or calculation (see `data`) |
| 400 | Malformed request body |
| 401 | `/api/v1/*` without the token, when one is set |
| 403 | `Host` header that is not this machine |
| 404 | Unknown endpoint |
| 500 | Internal server error |

### Error Response Format

```json
{
  "success": true,
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "data": {
    "valid": false,
    "file_path": "model.yaml",
    "message": "Error: File not found"
  }
}
```

Note: a failed validation or calculation comes back with HTTP 200 and `success: true`; the failure is in `data` (`valid: false` or `calculated: false`).

---

## Testing

### Unit Tests

```
api::handlers::tests::
├── test_api_response_ok_creates_success_response
├── test_api_response_ok_with_struct
├── test_api_response_err_creates_error_response
├── test_api_response_request_id_is_unique
├── test_validate_response_default
├── test_calculate_response_default
├── test_audit_response_default
├── test_export_response_default
├── test_import_response_default
├── test_validate_request_deserialize
├── test_calculate_request_deserialize_with_dry_run
├── test_calculate_request_deserialize_dry_run_defaults_false
├── test_audit_request_deserialize
├── test_export_request_deserialize
├── test_import_request_deserialize
├── test_health_response_serialize
├── test_version_response_serialize
├── test_validate_response_serialize
├── test_calculate_response_serialize
├── test_api_response_serializes_without_none_fields
├── test_api_response_error_serializes_without_data
├── test_endpoint_info_serialize
└── test_root_response_has_all_endpoints

api::server::tests::
├── test_default_config
├── test_config_custom_values
├── test_config_clone
├── test_config_address_format
├── test_app_state_version
├── test_app_state_clone
└── test_app_state_in_arc
```

### Test Coverage

- **Request deserialization:** All request types
- **Response serialization:** All response types
- **Default values:** All response structs
- **UUID generation:** Request ID uniqueness
- **Configuration:** ApiConfig and AppState

---

## Performance Considerations

### Async Runtime

- Uses Tokio multi-threaded runtime
- All handlers are async

### Connection Handling

- HTTP/1.1 keep-alive supported
- Connection pooling via Hyper

### Memory Usage

- Stateless handlers (no session storage)
- Shared state via `Arc<AppState>`
- Request-scoped allocations

---

## Security Considerations

### Current Implementation

- Listens on 127.0.0.1 by default and rejects requests whose `Host` header is not this machine
- No CORS unless `--cors-origin` names an origin
- A bearer token (`--token` or `FORGE_API_TOKEN`) on `/api/v1/*`; required to listen on another address
- The API reads and writes the files requests name, as the user running it
- No rate limiting and no HTTPS (use a reverse proxy)

### Production Recommendations

1. **Keep the token secret** and prefer the environment variable to `--token`
2. **Implement rate limiting** (tower-governor)
3. **Enable HTTPS** (reverse proxy or rustls)
4. **Add request validation** (payload size limits)

---

## Integration Examples

### cURL

```bash
# Validate
curl -X POST http://localhost:8080/api/v1/validate \
  -H "Content-Type: application/json" \
  -d '{"file_path": "model.yaml"}'

# Calculate with dry-run
curl -X POST http://localhost:8080/api/v1/calculate \
  -H "Content-Type: application/json" \
  -d '{"file_path": "model.yaml", "dry_run": true}'
```

### JavaScript/Fetch

```javascript
const response = await fetch('http://localhost:8080/api/v1/validate', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ file_path: 'model.yaml' })
});
const result = await response.json();
console.log(result.data.valid);
```

### Python/Requests

```python
import requests

response = requests.post(
    'http://localhost:8080/api/v1/validate',
    json={'file_path': 'model.yaml'}
)
result = response.json()
print(result['data']['valid'])
```

---

## Related Documentation

- [00-OVERVIEW](00-OVERVIEW.md) - System context
- [01-COMPONENT-ARCHITECTURE](01-COMPONENT-ARCHITECTURE.md) - Module interactions
- [06-CLI-ARCHITECTURE](06-CLI-ARCHITECTURE.md) - CLI commands (reused by API)
- [07-TESTING-ARCHITECTURE](07-TESTING-ARCHITECTURE.md) - Test strategy
