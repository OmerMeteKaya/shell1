echo "MINIMAL TEST: backtick icinde multi-line backslash-continued sed -e -e"
x=`echo hello | \
sed -e "s/h/H/" \
    -e "s/o/O/"`
echo "result=[$x]"

echo "MINIMAL TEST 2: backtick icinde, sadece TEK -e, multi-line"
y=`echo hello | \
sed -e "s/h/H/"`
echo "result2=[$y]"

echo "MINIMAL TEST 3: backtick icinde multi-line ama sed -e yok"
z=`echo hello | \
cat`
echo "result3=[$z]"

echo "MINIMAL TEST 4: backtick icinde multi-line, grep ile"
w=`echo hello | \
grep hello`
echo "result4=[$w]"
