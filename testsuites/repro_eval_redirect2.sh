echo "TEST 5: pipe + eval + FOO= (bos deger) + cat + redirect"
rm -f /tmp/zesh_t5.txt
echo "content5" | eval FOO= cat > /tmp/zesh_t5.txt
echo "exit5: $?"
cat -v /tmp/zesh_t5.txt

echo "TEST 6: pipe + eval + FOO=bar (dolu deger) + cat + redirect"
rm -f /tmp/zesh_t6.txt
echo "content6" | eval FOO=bar cat > /tmp/zesh_t6.txt
echo "exit6: $?"
cat -v /tmp/zesh_t6.txt

echo "TEST 7: eval YOK, sadece env-var atamali komut + pipe + redirect"
rm -f /tmp/zesh_t7.txt
echo "content7" | FOO= cat > /tmp/zesh_t7.txt
echo "exit7: $?"
cat -v /tmp/zesh_t7.txt
