if test -d "repro-3.11"; then find "repro-3.11" -type d ! -perm -200 -exec chmod u+w {} ';' && rm -rf "repro-3.11" || { sleep 1 && rm -rf "repro-3.11"; }; else :; fi
