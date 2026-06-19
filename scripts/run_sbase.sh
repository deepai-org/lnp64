#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${LNP64_BIN:-}" ]]; then
  lnp64=("$LNP64_BIN")
else
  lnp64=(cargo run --release --quiet --)
fi

"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/echo.c -o /tmp/sbase_echo.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/cat.c -o /tmp/sbase_cat.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/wc.c -o /tmp/sbase_wc.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/yes.c -o /tmp/sbase_yes.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/basename.c -o /tmp/sbase_basename.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/dirname.c -o /tmp/sbase_dirname.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/head.c -o /tmp/sbase_head.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/tee.c -o /tmp/sbase_tee.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/cksum.c -o /tmp/sbase_cksum.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/tail.c -o /tmp/sbase_tail.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/cmp.c -o /tmp/sbase_cmp.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/uniq.c -o /tmp/sbase_uniq.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/sort.c -o /tmp/sbase_sort.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/grep.c -o /tmp/sbase_grep.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/sed.c -o /tmp/sbase_sed.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/cp.c -o /tmp/sbase_cp.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/mv.c -o /tmp/sbase_mv.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/ls.c -o /tmp/sbase_ls.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/chmod.c -o /tmp/sbase_chmod.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/chown.c -o /tmp/sbase_chown.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/ln.c -o /tmp/sbase_ln.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/mkdir.c -o /tmp/sbase_mkdir.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/rm.c -o /tmp/sbase_rm.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/cut.c -o /tmp/sbase_cut.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/tr.c -o /tmp/sbase_tr.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/touch.c -o /tmp/sbase_touch.s
"${lnp64[@]}" cc --toy-bootstrap third_party/sbase/find.c -o /tmp/sbase_find.s
"${lnp64[@]}" cc --toy-bootstrap third_party/jsmn/example/simple.c -o /tmp/jsmn_simple.s
"${lnp64[@]}" cc --toy-bootstrap third_party/jsmn/test/tests.c -o /tmp/jsmn_tests.s

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

echo "== sbase uniq.c =="
printf 'alpha\nalpha\nbeta\nalpha\n' > /tmp/sbase_uniq_input.txt
uniq_out=$("${lnp64[@]}" run /tmp/sbase_uniq.s -- uniq /tmp/sbase_uniq_input.txt)
test "$uniq_out" = $'alpha\nbeta\nalpha'
echo "$uniq_out"

echo "== sbase grep.c =="
grep_out=$("${lnp64[@]}" run /tmp/sbase_grep.s -- grep beta /tmp/sbase_uniq_input.txt)
test "$grep_out" = "beta"
grep_count=$("${lnp64[@]}" run /tmp/sbase_grep.s -- grep -c alpha /tmp/sbase_uniq_input.txt)
test "$grep_count" = "3"
set +e
"${lnp64[@]}" run /tmp/sbase_grep.s -- grep zzz /tmp/sbase_uniq_input.txt > /tmp/sbase_grep_nomatch.out
grep_status=$?
set -e
test "$grep_status" = "1"
test ! -s /tmp/sbase_grep_nomatch.out
echo "$grep_out"

echo "== sbase sort.c =="
printf 'beta\nalpha\ngamma\n' > /tmp/sbase_sort_input.txt
sort_out=$("${lnp64[@]}" run /tmp/sbase_sort.s -- sort /tmp/sbase_sort_input.txt)
test "$sort_out" = $'alpha\nbeta\ngamma'
printf 'beta\nalpha\nbeta\n' > /tmp/sbase_sort_dup.txt
sort_u_out=$("${lnp64[@]}" run /tmp/sbase_sort.s -- sort -u /tmp/sbase_sort_dup.txt)
test "$sort_u_out" = $'alpha\nbeta'
sort_r_out=$("${lnp64[@]}" run /tmp/sbase_sort.s -- sort -r /tmp/sbase_sort_input.txt)
test "$sort_r_out" = $'gamma\nbeta\nalpha'
printf 'alpha\nbeta\ngamma\n' > /tmp/sbase_sort_sorted.txt
"${lnp64[@]}" run /tmp/sbase_sort.s -- sort -c /tmp/sbase_sort_sorted.txt
set +e
"${lnp64[@]}" run /tmp/sbase_sort.s -- sort -c /tmp/sbase_sort_input.txt > /tmp/sbase_sort_c.out 2>/tmp/sbase_sort_c.err
sort_c_status=$?
set -e
test "$sort_c_status" = "1"
echo "$sort_out"

echo "== sbase sed.c =="
printf 'alpha\nbeta\n' > /tmp/sbase_sed_input.txt
sed_sub=$("${lnp64[@]}" run /tmp/sbase_sed.s -- sed 's/beta/BETA/' /tmp/sbase_sed_input.txt)
test "$sed_sub" = $'alpha\nBETA'
sed_nomatch=$("${lnp64[@]}" run /tmp/sbase_sed.s -- sed 's/zzz/XXX/' /tmp/sbase_sed_input.txt)
test "$sed_nomatch" = $'alpha\nbeta'
sed_print=$("${lnp64[@]}" run /tmp/sbase_sed.s -- sed -n 's/beta/BETA/p' /tmp/sbase_sed_input.txt)
test "$sed_print" = "BETA"
sed_addr=$("${lnp64[@]}" run /tmp/sbase_sed.s -- sed -n '/beta/p' /tmp/sbase_sed_input.txt)
test "$sed_addr" = "beta"
echo "$sed_sub"

echo "== sbase cp.c / mv.c / ls.c =="
rm -rf /tmp/sbase_fs_smoke
mkdir -p /tmp/sbase_fs_smoke/dir
printf 'copy me\n' > /tmp/sbase_fs_smoke/src.txt
"${lnp64[@]}" run /tmp/sbase_cp.s -- cp /tmp/sbase_fs_smoke/src.txt /tmp/sbase_fs_smoke/copy.txt
test "$(cat /tmp/sbase_fs_smoke/copy.txt)" = "copy me"
mkdir -p /tmp/sbase_fs_smoke/tree/sub
printf 'root\n' > /tmp/sbase_fs_smoke/tree/root.txt
printf 'child\n' > /tmp/sbase_fs_smoke/tree/sub/child.txt
"${lnp64[@]}" run /tmp/sbase_cp.s -- cp -r /tmp/sbase_fs_smoke/tree /tmp/sbase_fs_smoke/tree_copy
test "$(cat /tmp/sbase_fs_smoke/tree_copy/root.txt)" = "root"
test "$(cat /tmp/sbase_fs_smoke/tree_copy/sub/child.txt)" = "child"
"${lnp64[@]}" run /tmp/sbase_mv.s -- mv /tmp/sbase_fs_smoke/copy.txt /tmp/sbase_fs_smoke/moved.txt
test "$(cat /tmp/sbase_fs_smoke/moved.txt)" = "copy me"
ls_out=$("${lnp64[@]}" run /tmp/sbase_ls.s -- ls /tmp/sbase_fs_smoke)
test "$ls_out" = $'dir\nmoved.txt\nsrc.txt\ntree\ntree_copy'
echo "$ls_out"

echo "== sbase chmod.c / chown.c / ln.c =="
"${lnp64[@]}" run /tmp/sbase_chmod.s -- chmod 600 /tmp/sbase_fs_smoke/src.txt
mode=$(stat -c '%a' /tmp/sbase_fs_smoke/src.txt)
test "$mode" = "600"
"${lnp64[@]}" run /tmp/sbase_chmod.s -- chmod -R 700 /tmp/sbase_fs_smoke/tree_copy
test "$(stat -c '%a' /tmp/sbase_fs_smoke/tree_copy)" = "700"
test "$(stat -c '%a' /tmp/sbase_fs_smoke/tree_copy/sub/child.txt)" = "700"
uid=$(id -u)
gid=$(id -g)
"${lnp64[@]}" run /tmp/sbase_chown.s -- chown "$uid:$gid" /tmp/sbase_fs_smoke/src.txt
owner=$(stat -c '%u:%g' /tmp/sbase_fs_smoke/src.txt)
test "$owner" = "$uid:$gid"
"${lnp64[@]}" run /tmp/sbase_chown.s -- chown -R "$uid:$gid" /tmp/sbase_fs_smoke/tree_copy
test "$(stat -c '%u:%g' /tmp/sbase_fs_smoke/tree_copy/sub/child.txt)" = "$uid:$gid"
"${lnp64[@]}" run /tmp/sbase_ln.s -- ln /tmp/sbase_fs_smoke/src.txt /tmp/sbase_fs_smoke/hard.txt
test "$(cat /tmp/sbase_fs_smoke/hard.txt)" = "copy me"
"${lnp64[@]}" run /tmp/sbase_ln.s -- ln -s /tmp/sbase_fs_smoke/src.txt /tmp/sbase_fs_smoke/sym.txt
test "$(readlink /tmp/sbase_fs_smoke/sym.txt)" = "/tmp/sbase_fs_smoke/src.txt"
echo "chmod/chown/ln ok"

echo "== sbase mkdir.c / rm.c / cut.c / tr.c =="
"${lnp64[@]}" run /tmp/sbase_mkdir.s -- mkdir /tmp/sbase_fs_smoke/newdir
test -d /tmp/sbase_fs_smoke/newdir
printf 'remove me\n' > /tmp/sbase_fs_smoke/remove.txt
"${lnp64[@]}" run /tmp/sbase_rm.s -- rm /tmp/sbase_fs_smoke/remove.txt
test ! -e /tmp/sbase_fs_smoke/remove.txt
"${lnp64[@]}" run /tmp/sbase_rm.s -- rm -r /tmp/sbase_fs_smoke/tree_copy
test ! -e /tmp/sbase_fs_smoke/tree_copy
printf 'abcde\n' > /tmp/sbase_cut_input.txt
cut_out=$("${lnp64[@]}" run /tmp/sbase_cut.s -- cut -c 2-4 /tmp/sbase_cut_input.txt)
test "$cut_out" = "bcd"
printf 'a:b:c\n' > /tmp/sbase_cut_fields.txt
cut_field_out=$("${lnp64[@]}" run /tmp/sbase_cut.s -- cut -d : -f 2 /tmp/sbase_cut_fields.txt)
test "$cut_field_out" = "b"
tr_out=$(printf 'abc\n' | "${lnp64[@]}" run /tmp/sbase_tr.s -- tr a-z A-Z)
test "$tr_out" = "ABC"
echo "mkdir/rm/cut/tr ok"

echo "== sbase touch.c =="
rm -f /tmp/sbase_touch_file.txt
"${lnp64[@]}" run /tmp/sbase_touch.s -- touch /tmp/sbase_touch_file.txt
test -f /tmp/sbase_touch_file.txt
"${lnp64[@]}" run /tmp/sbase_touch.s -- touch /tmp/sbase_touch_file.txt
rm -f /tmp/sbase_touch_ref.txt /tmp/sbase_touch_copy.txt
printf ref > /tmp/sbase_touch_ref.txt
printf copy > /tmp/sbase_touch_copy.txt
touch -t 200102030405 /tmp/sbase_touch_ref.txt
"${lnp64[@]}" run /tmp/sbase_touch.s -- touch -r /tmp/sbase_touch_ref.txt /tmp/sbase_touch_copy.txt
test "$(stat -c '%Y' /tmp/sbase_touch_copy.txt)" = "$(stat -c '%Y' /tmp/sbase_touch_ref.txt)"
echo "touch ok"

echo "== sbase find.c =="
rm -rf /tmp/sbase_find_smoke
mkdir -p /tmp/sbase_find_smoke/sub
printf 'root\n' > /tmp/sbase_find_smoke/root.txt
printf 'child\n' > /tmp/sbase_find_smoke/sub/child.txt
find_out=$("${lnp64[@]}" run /tmp/sbase_find.s -- find /tmp/sbase_find_smoke)
test "$find_out" = $'/tmp/sbase_find_smoke\n/tmp/sbase_find_smoke/root.txt\n/tmp/sbase_find_smoke/sub\n/tmp/sbase_find_smoke/sub/child.txt'
find_name_out=$("${lnp64[@]}" run /tmp/sbase_find.s -- find /tmp/sbase_find_smoke -name child.txt)
test "$find_name_out" = "/tmp/sbase_find_smoke/sub/child.txt"
find_type_out=$("${lnp64[@]}" run /tmp/sbase_find.s -- find /tmp/sbase_find_smoke -type f)
test "$find_type_out" = $'/tmp/sbase_find_smoke/root.txt\n/tmp/sbase_find_smoke/sub/child.txt'
echo "$find_out"

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
