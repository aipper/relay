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
