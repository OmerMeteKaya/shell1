echo "TEST_H: sed -e flagleri AYNI satirda"
srcdirstrip='\.'; \
topsrcdirstrip='\.'; \
list='./Makefile.am ./configure foo.m4 bar.txt'; \
dist_files=`for file in $list; do echo $file; done | sed -e "s|^$srcdirstrip/||;t" -e "s|^$topsrcdirstrip/|./|;t"`; \
echo "H done dist_files=[$dist_files]"

echo "TEST_I: sed -e flagleri AYRI satirlarda, backslash-continuation ile"
srcdirstrip='\.'; \
topsrcdirstrip='\.'; \
list='./Makefile.am ./configure foo.m4 bar.txt'; \
  dist_files=`for file in $list; do echo $file; done | \
  sed -e "s|^$srcdirstrip/||;t" \
      -e "s|^$topsrcdirstrip/|./|;t"`; \
echo "I done dist_files=[$dist_files]"

echo "TEST_J: I + ekstra satir sonrasinda"
srcdirstrip='\.'; \
topsrcdirstrip='\.'; \
list='./Makefile.am ./configure foo.m4 bar.txt'; \
  dist_files=`for file in $list; do echo $file; done | \
  sed -e "s|^$srcdirstrip/||;t" \
      -e "s|^$topsrcdirstrip/|./|;t"`; \
echo "J done dist_files=[$dist_files]"; \
echo "J extra line"
