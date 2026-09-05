# Schema

Current version: **1** (`warehouse_common::SCHEMA_VERSION`).

Every response carries a `schema` field. Compare it against the version you built against
before trusting a document — an instance serving a different schema may return bodies you
would misread. The manifest is the cheapest place to check.

Bump the version on any change to a catalog document that is not purely additive. Adding
a field is additive; renaming one, removing one, or changing its type is not.

## Authentication

Every route except `GET /system/health` requires `Authorization: Bearer <token>` when the
instance sets `WAREHOUSE_TOKEN`. Failures return `401` with the error body below.

## Errors

```json
{ "error": { "id": "catalog/not_found", "msg": "This instance does not serve that catalog" } }
```

| id | status | meaning |
|---|---|---|
| `auth/invalid` | 401 | missing or wrong bearer token |
| `catalog/not_found` | 404 | no such catalog, or this instance doesn't serve it |
| `catalog/unavailable` | 503 | the catalog exists but hasn't been resolved yet |

`503` is normal on a cold instance with no persisted data. Keep whatever you already hold
and retry; it is not a reason to fail your own request.

## `GET /catalog`

```json
{
  "schema": 1,
  "catalogs": [
    {
      "id": "rust",
      "etag": "\"2356072a28da1b13...\"",
      "updated_at": "2026-09-05T20:03:25.693199Z",
      "stale": false
    }
  ]
}
```

`etag` identifies the document's content — refetch a catalog only when it changes. A
catalog the instance has never resolved is absent from the list entirely.

`stale: true` means the last refresh attempt failed; the previous document is still being
served and is usually still worth using.

## `GET /system/status`

```json
{
  "schema": 1,
  "version": "0.1.0",
  "catalogs": [
    {
      "id": "rust",
      "resolved": true,
      "stale": false,
      "last_success": "2026-09-05T20:03:25.693199Z",
      "last_failure": null,
      "last_error": null,
      "consecutive_failures": 0,
      "next_attempt": "2026-09-06T20:12:12.693219Z"
    }
  ]
}
```

`last_error` names the upstream that failed, so a Mojang outage reads differently from a
Docker Hub rate limit.

## `GET /catalog/minecraft`

```json
{
  "schema": 1,
  "updated_at": "2026-09-05T14:48:49Z",
  "versions": [
    {
      "id": "1.21.4",
      "recommended_java": 21,
      "data_pack": true,
      "distributions": {
        "paper": { "kind": "range", "min": 83, "max": 121, "excluded": [95, 108] },
        "fabric": { "kind": "set", "values": ["0.19.5", "0.19.4"] },
        "neoforge": { "kind": "range", "prefix": "21.4.", "min": 1, "max": 60, "excluded": [] }
      }
    }
  ]
}
```

`versions` is ordered newest first and holds full releases only — no snapshots, nothing
older than `1.7.10`.

`distributions` keys are `paper`, `purpur`, `fabric`, `quilt`, `forge`, `neoforge`. A
distribution absent from the map has no build for that release. **`vanilla` never
appears**: Mojang's own server exists for every listed release and has no build numbers,
so listing it would carry no information.

### `BuildSet`

Two shapes, tagged by `kind`:

- `range` — dense integer build numbers: every value from `min` to `max` inclusive except
  those in `excluded`. An optional `prefix` is prepended to a build number to form the
  full version string; NeoForge uses it, so build `60` with prefix `21.4.` means loader
  `21.4.60`.
- `set` — opaque version strings in `values`, newest first. Fabric, Quilt and Forge
  loaders use this.

## `GET /catalog/minecraft-proxy`

```json
{
  "schema": 1,
  "updated_at": "2026-09-05T14:46:48Z",
  "velocity": [
    { "id": "3.4.0-SNAPSHOT", "java": 17, "builds": { "kind": "range", "min": 1, "max": 500, "excluded": [] } }
  ],
  "bungeecord": [1800, 1799, 1798]
}
```

`velocity` is newest first and `java` is the minimum feature version that line requires.
`bungeecord` holds the 50 most recent successful build numbers, newest first.

## `GET /catalog/java`

```json
{
  "schema": 1,
  "updated_at": "2026-09-05T14:47:29Z",
  "majors": [
    { "major": 21, "versions": ["21.0.12_8", "21.0.11_9"] },
    { "major": 8, "versions": ["8u502-b07"] }
  ]
}
```

Temurin publishes two version shapes and both appear as the string upstream uses:
`major.minor.patch_build` for Java 9 and later, `8uPATCH-bBUILD` for Java 8. `majors` and
each `versions` list are newest first.

## `GET /catalog/runtime/{runtime}`

`{runtime}` is `go`, `node`, `python` or `rust`. Java is **not** valid here — it has its
own route because its versions don't fit a triple.

```json
{
  "schema": 1,
  "runtime": "rust",
  "updated_at": "2026-09-05T20:03:25.693199Z",
  "versions": ["1.98.0", "1.97.1", "1.97.0"]
}
```

Versions are `major.minor.patch` strings, newest first. These come from the five newest
Docker Hub tag pages, so they are recent releases rather than full history, and Node and
Python drop versions below their supported floor.

## `POST /catalog/{id}/refresh`

Queues a rebuild and returns `204` immediately. The refresh happens in the background;
`GET /system/status` reports how it went. Refreshing does not bypass the Docker Hub
interval gate.
