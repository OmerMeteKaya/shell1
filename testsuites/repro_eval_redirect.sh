echo "TEST 1: eval CMD > file (orijinal kalip, gzip ile)"
rm -f /tmp/zesh_eval_redirect_test.gz
echo "hello world" | eval GZIP= gzip -c > /tmp/zesh_eval_redirect_test.gz
echo "exit: $?"
ls -la /tmp/zesh_eval_redirect_test.gz
file /tmp/zesh_eval_redirect_test.gz

echo "TEST 2: eval CMD > file (basit, echo ile)"
rm -f /tmp/zesh_eval_simple_test.txt
eval echo hello > /tmp/zesh_eval_simple_test.txt
echo "exit: $?"
cat /tmp/zesh_eval_simple_test.txt

echo "TEST 3: pipe sonrasi eval > file"
rm -f /tmp/zesh_eval_pipe_test.txt
echo "piped content" | eval cat > /tmp/zesh_eval_pipe_test.txt
echo "exit: $?"
cat /tmp/zesh_eval_pipe_test.txt

echo "TEST 4: eval'siz, ayni redirect (kontrol grubu)"
rm -f /tmp/zesh_no_eval_test.txt
echo "piped content" | cat > /tmp/zesh_no_eval_test.txt
echo "exit: $?"
cat /tmp/zesh_no_eval_test.txt
