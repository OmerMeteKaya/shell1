echo "=== A: ic pipe (script icinde echo|eval), eval+redirect ==="
rm -f /tmp/zesh_a.txt
~/shell/zesh/target/release/zesh -c 'echo content_a | eval FOO= cat > /tmp/zesh_a.txt'
ls -la /tmp/zesh_a.txt
cat /tmp/zesh_a.txt

echo "=== B: dis pipe (zesh -c disindan), eval+redirect ==="
rm -f /tmp/zesh_b.txt
echo content_b | ~/shell/zesh/target/release/zesh -c 'eval FOO= cat > /tmp/zesh_b.txt'
ls -la /tmp/zesh_b.txt
cat /tmp/zesh_b.txt

echo "=== C: dis pipe, eval YOK (kontrol grubu) ==="
rm -f /tmp/zesh_c.txt
echo content_c | ~/shell/zesh/target/release/zesh -c 'cat > /tmp/zesh_c.txt'
ls -la /tmp/zesh_c.txt
cat /tmp/zesh_c.txt

echo "=== D: dis pipe, eval VAR, redirect harici (zesh -c disinda) ==="
rm -f /tmp/zesh_d.txt
echo content_d | ~/shell/zesh/target/release/zesh -c 'eval FOO= cat' > /tmp/zesh_d.txt
ls -la /tmp/zesh_d.txt
cat /tmp/zesh_d.txt
