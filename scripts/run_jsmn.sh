#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --release --quiet --)
fi

"${lnp64[@]}" cc third_party/jsmn/example/simple.c -o /tmp/jsmn_simple.s
"${lnp64[@]}" cc third_party/jsmn/test/tests.c -o /tmp/jsmn_tests.s

echo "== jsmn example/simple.c =="
"${lnp64[@]}" run /tmp/jsmn_simple.s -- simple > /tmp/jsmn_simple.out
cat > /tmp/jsmn_simple.expected <<'EXPECTED'
- User: johndoe
- Admin: false
- UID: 1000
- Groups:
  * users
  * wheel
  * audio
  * video
EXPECTED
diff -u /tmp/jsmn_simple.expected /tmp/jsmn_simple.out
cat /tmp/jsmn_simple.out

echo "== jsmn test/tests.c =="
"${lnp64[@]}" run /tmp/jsmn_tests.s -- tests > /tmp/jsmn_tests.out
cat > /tmp/jsmn_tests.expected <<'EXPECTED'

PASSED: 16
FAILED: 0
EXPECTED
diff -u /tmp/jsmn_tests.expected /tmp/jsmn_tests.out
cat /tmp/jsmn_tests.out

printf '%s\n' "jsmn ok"
