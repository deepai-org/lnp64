#!/usr/bin/env bash
set -euo pipefail

lnp64=(cargo run --release --quiet --)

"${lnp64[@]}" cc third_party/sbase/echo.c -o /tmp/sbase_echo.s
"${lnp64[@]}" cc third_party/sbase/cat.c -o /tmp/sbase_cat.s
"${lnp64[@]}" cc third_party/sbase/wc.c -o /tmp/sbase_wc.s
"${lnp64[@]}" cc third_party/sbase/yes.c -o /tmp/sbase_yes.s
"${lnp64[@]}" cc third_party/sbase/basename.c -o /tmp/sbase_basename.s
"${lnp64[@]}" cc third_party/sbase/dirname.c -o /tmp/sbase_dirname.s
"${lnp64[@]}" cc third_party/sbase/head.c -o /tmp/sbase_head.s
"${lnp64[@]}" cc third_party/sbase/tee.c -o /tmp/sbase_tee.s
"${lnp64[@]}" cc third_party/sbase/cksum.c -o /tmp/sbase_cksum.s
"${lnp64[@]}" cc third_party/sbase/tail.c -o /tmp/sbase_tail.s
"${lnp64[@]}" cc third_party/sbase/cmp.c -o /tmp/sbase_cmp.s
"${lnp64[@]}" cc third_party/jsmn/example/simple.c -o /tmp/jsmn_simple.s

echo "== sbase echo.c =="
echo_out=$("${lnp64[@]}" run /tmp/sbase_echo.s -- echo hello world)
test "$echo_out" = "hello world"
echo "$echo_out"
echo_n_out=$("${lnp64[@]}" run /tmp/sbase_echo.s -- echo -n hello world)
test "$echo_n_out" = "hello world"

echo "== sbase cat.c =="
cat_out=$("${lnp64[@]}" run /tmp/sbase_cat.s -- cat demos/cat_input.txt)
test "$cat_out" = "cat ok"
echo "$cat_out"

echo "== sbase wc.c =="
wc_out=$("${lnp64[@]}" run /tmp/sbase_wc.s -- wc demos/cat_input.txt)
test "$wc_out" = "1 2 7 demos/cat_input.txt"
echo "$wc_out"
wc_l_out=$("${lnp64[@]}" run /tmp/sbase_wc.s -- wc -l demos/cat_input.txt)
test "$wc_l_out" = "1 demos/cat_input.txt"
echo "$wc_l_out"

echo "== sbase yes.c =="
set +e
timeout 0.1s "${lnp64[@]}" run /tmp/sbase_yes.s -- yes ok > /tmp/sbase_yes.out
yes_status=$?
set -e
test "$yes_status" = "124"
yes_out=$(head -n 3 /tmp/sbase_yes.out)
test "$yes_out" = $'ok\nok\nok'
echo "$yes_out"

echo "== sbase basename.c =="
base_out=$("${lnp64[@]}" run /tmp/sbase_basename.s -- basename /usr/bin/example.txt .txt)
test "$base_out" = "example"
echo "$base_out"

echo "== sbase dirname.c =="
dir_out=$("${lnp64[@]}" run /tmp/sbase_dirname.s -- dirname /usr/bin/example.txt)
test "$dir_out" = "/usr/bin"
echo "$dir_out"

echo "== sbase head.c =="
head_input=/tmp/sbase_head_input.txt
for i in 1 2 3 4 5 6; do printf 'line%s\n' "$i"; done > "$head_input"
head_out=$("${lnp64[@]}" run /tmp/sbase_head.s -- head -n 3 "$head_input")
test "$head_out" = $'line1\nline2\nline3'
echo "$head_out"

echo "== sbase tee.c =="
rm -f /tmp/sbase_tee_file.txt
tee_out=$(printf 'tee ok\n' | "${lnp64[@]}" run /tmp/sbase_tee.s -- tee /tmp/sbase_tee_file.txt)
test "$tee_out" = "tee ok"
test "$(cat /tmp/sbase_tee_file.txt)" = "tee ok"
echo "$tee_out"

echo "== sbase cksum.c =="
cksum_out=$("${lnp64[@]}" run /tmp/sbase_cksum.s -- cksum demos/cat_input.txt)
cksum_expected=$(cksum demos/cat_input.txt)
test "$cksum_out" = "$cksum_expected"
echo "$cksum_out"

echo "== sbase tail.c =="
tail_input=/tmp/sbase_tail_input.txt
for i in 1 2 3 4 5; do printf 'tail%s\n' "$i"; done > "$tail_input"
tail_out=$("${lnp64[@]}" run /tmp/sbase_tail.s -- tail -n 2 "$tail_input")
test "$tail_out" = $'tail4\ntail5'
echo "$tail_out"

echo "== sbase cmp.c =="
printf 'cmp ok\n' > /tmp/sbase_cmp_a.txt
printf 'cmp ok\n' > /tmp/sbase_cmp_b.txt
printf 'cmp no\n' > /tmp/sbase_cmp_c.txt
"${lnp64[@]}" run /tmp/sbase_cmp.s -- cmp /tmp/sbase_cmp_a.txt /tmp/sbase_cmp_b.txt
set +e
"${lnp64[@]}" run /tmp/sbase_cmp.s -- cmp -s /tmp/sbase_cmp_a.txt /tmp/sbase_cmp_c.txt > /tmp/sbase_cmp_diff.out
cmp_status=$?
set -e
test "$cmp_status" = "1"
test ! -s /tmp/sbase_cmp_diff.out
echo "cmp ok"

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
