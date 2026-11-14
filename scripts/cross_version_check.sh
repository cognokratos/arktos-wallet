#!/bin/sh
# Cross-version compatibility check for the encrypted database.
#
#   scripts/cross_version_check.sh OLD_BINARY NEW_BINARY
#
# OLD_BINARY creates a wallet and accounts; NEW_BINARY reopens the same
# SQLCipher database with the same keys. Checks that NEW_BINARY returns the
# stored accounts unchanged, decrypts the stored recovery phrase and derives
# fresh addresses identical to OLD_BINARY's, migrates idempotently, and that
# OLD_BINARY can still open the database afterwards. Uses throwaway random
# keys and a temporary directory; needs curl and python3. The server listens
# on port 8080, which must be free.
#
# Build OLD_BINARY from an earlier commit, e.g.:
#   git worktree add ../arktos-old <commit> && (cd ../arktos-old && cargo build)
set -eu

OLD=$1
NEW=$2
BASE="http://127.0.0.1:8080"
DIR=$(mktemp -d)
OUT="$DIR/out"
mkdir "$OUT"
PID=
trap '[ -n "$PID" ] && kill "$PID" 2>/dev/null; rm -rf "$DIR"' EXIT

DATABASE_KEY=$(head -c32 /dev/urandom | base64)
MASTER_KEY=$(head -c32 /dev/urandom | base64)
ADMIN_API_KEY=cross-version-admin-key

run() {
    env -i PATH="$PATH" HOME="$HOME" DATABASE_PATH="$DIR/arktos.db" \
        ADMIN_API_KEY="$ADMIN_API_KEY" DATABASE_KEY="$DATABASE_KEY" \
        MASTER_KEY="$MASTER_KEY" "$@"
}

start() {
    if curl -s -o /dev/null "$BASE/healthz"; then
        echo "port 8080 is already in use" >&2
        exit 1
    fi
    # Not via run(): a backgrounded function would make $! a subshell.
    env -i PATH="$PATH" HOME="$HOME" DATABASE_PATH="$DIR/arktos.db" \
        ADMIN_API_KEY="$ADMIN_API_KEY" DATABASE_KEY="$DATABASE_KEY" \
        MASTER_KEY="$MASTER_KEY" "$1" >"$OUT/server.log" 2>&1 &
    PID=$!
    i=0
    until curl -sf "$BASE/healthz" >/dev/null; do
        i=$((i + 1))
        [ "$i" -gt 100 ] && { cat "$OUT/server.log"; echo "server did not start" >&2; exit 1; }
        sleep 0.1
    done
}

stop() {
    kill "$PID"
    wait "$PID" 2>/dev/null || true
    PID=
}

META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"cross-version","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}'

# tool NAME ARGUMENTS_JSON: print the public fields of the structured result.
tool() {
    curl -s -X POST "$BASE/mcp" -H "X-API-KEY: $KEY" \
        -H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream' \
        -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: tools/call' -H "Mcp-Name: $1" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"$1\",\"arguments\":$2,$META}}" |
        python3 -c 'import json, sys
r = json.load(sys.stdin)["result"]
if r.get("isError"):
    sys.exit("tool error: " + json.dumps(r))
s = r["structuredContent"]
keep = ("wallet_name", "account_index", "derivation_path", "address", "network", "chain_id")
print(json.dumps({k: s[k] for k in keep if k in s}, sort_keys=True))'
}

addresses() { # INDEX...
    for i in "$@"; do
        tool get_bitcoin_address "{\"wallet_name\":\"w\",\"account_index\":$i}"
        tool get_ethereum_address "{\"wallet_name\":\"w\",\"account_index\":$i}"
    done
}

schema_version() {
    run "$1" db-info | sed -n 's/^schema version: *\([0-9]*\).*/\1/p'
}

# 1. OLD creates a wallet and stores accounts 0 and 7.
start "$OLD"
KEY=$(curl -s -X POST -H "X-API-KEY: $ADMIN_API_KEY" -H 'Content-Type: application/json' \
    -d '{"name":"cross-version"}' "$BASE/admin/api-keys" |
    python3 -c 'import json, sys; print(json.load(sys.stdin)["api_key"])')
tool create_wallet '{"wallet_name":"w"}' >/dev/null
addresses 0 7 >"$OUT/old-stored"
stop
mkdir "$DIR/snapshot"
cp "$DIR"/arktos.db* "$DIR/snapshot/"

# 2. OLD derives indices that the snapshot does not contain yet.
start "$OLD"
addresses 5 2147483647 >"$OUT/old-fresh"
stop
rm -f "$DIR"/arktos.db*
cp "$DIR/snapshot/"* "$DIR/"

# 3. NEW opens the snapshot: stored accounts, then fresh derivations that
#    decrypt the phrase OLD stored.
start "$NEW"
addresses 0 7 >"$OUT/new-stored"
addresses 5 2147483647 >"$OUT/new-fresh"
stop

cmp -s "$OUT/old-stored" "$OUT/new-stored" || { diff "$OUT/old-stored" "$OUT/new-stored"; exit 1; }
echo "ok: stored accounts identical ($(wc -l <"$OUT/old-stored" | tr -d ' ') results)"
cmp -s "$OUT/old-fresh" "$OUT/new-fresh" || { diff "$OUT/old-fresh" "$OUT/new-fresh"; exit 1; }
echo "ok: stored phrase decrypts; fresh derivations identical ($(wc -l <"$OUT/old-fresh" | tr -d ' ') results)"

# 4. Migration is idempotent and OLD can still open the database.
run "$NEW" migrate >/dev/null
run "$NEW" migrate >/dev/null
[ "$(schema_version "$NEW")" = "$(schema_version "$OLD")" ] || { echo "schema version changed" >&2; exit 1; }
echo "ok: repeated migrate is a no-op; schema version $(schema_version "$NEW")"
echo "ok: old binary reopens: $(run "$OLD" db-info | grep -E '^(sqlite version|sqlcipher):' | tr -s ' ' | paste -sd ';' -)"
echo "    new binary uses:      $(run "$NEW" db-info | grep -E '^(sqlite version|sqlcipher):' | tr -s ' ' | paste -sd ';' -)"
