# hostd Local API (unix socket, MVP)

`hostd` exposes a local control API over a unix domain socket (HTTP).

Default socket path (dev): `/tmp/relay-hostd.sock`  
Recommended on Linux: `/run/relay/hostd.sock`

Examples (requires `curl`):

```sh
curl --unix-socket /tmp/relay-hostd.sock http://localhost/runs \
  -H 'content-type: application/json' \
  -d '{"tool":"codex","cmd":"bash -lc \"echo hi; read -p \\\"Proceed? \\\" x; echo ok\"","cwd":null}'
```

List runs:

```sh
curl --unix-socket /tmp/relay-hostd.sock http://localhost/runs
```

Send input:

```sh
curl --unix-socket /tmp/relay-hostd.sock http://localhost/runs/<run_id>/input \
  -H 'content-type: application/json' \
  -d '{"input_id":"<uuid>","actor":"cli","text":"y\n"}'
```

Stop:

```sh
curl --unix-socket /tmp/relay-hostd.sock http://localhost/runs/<run_id>/stop \
  -H 'content-type: application/json' \
  -d '{"signal":"term"}'
```

## Filesystem (scoped to run cwd)

All filesystem and git endpoints are scoped to the run's working directory (`cwd`) and only allow relative paths.

Read file (UTF-8, max 1MiB):

```sh
curl --unix-socket /tmp/relay-hostd.sock \
  "http://localhost/runs/<run_id>/fs/read?path=README.md"
```

Search with ripgrep (requires `rg` in PATH, max 200 matches):

```sh
curl --unix-socket /tmp/relay-hostd.sock \
  "http://localhost/runs/<run_id>/fs/search?q=TODO"
```

## Git (scoped to run cwd)

Status:

```sh
curl --unix-socket /tmp/relay-hostd.sock \
  "http://localhost/runs/<run_id>/git/status"
```

Diff (optional path filter):

```sh
curl --unix-socket /tmp/relay-hostd.sock \
  "http://localhost/runs/<run_id>/git/diff"

curl --unix-socket /tmp/relay-hostd.sock \
  "http://localhost/runs/<run_id>/git/diff?path=src/main.rs"
```
