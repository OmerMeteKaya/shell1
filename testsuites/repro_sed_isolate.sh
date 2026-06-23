echo "TEST1 start"
srcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`
echo "TEST1 srcdirstrip=[$srcdirstrip]"
echo "TEST2 start"
x=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; echo "TEST2 x=[$x]"
echo "TEST3 start"
echo "." | sed 's/[].[^$\\*]/\\\\&/g'
echo "TEST3 done"
echo "TEST4 start (sed without the backslash-heavy pattern)"
echo "." | sed 's/a/b/g'
echo "TEST4 done"
echo "TEST5 start (just the pattern chars, simpler)"
echo "." | sed 's/[].[^$\\*]/X/g'
echo "TEST5 done"
