srcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; \
topsrcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; \
list='./Makefile.am ./configure foo.m4 bar.txt'; \
  dist_files=`for file in $list; do echo $file; done | \
  sed -e "s|^$srcdirstrip/||;t" \
      -e "s|^$topsrcdirstrip/|./|;t"`; \
echo "srcdirstrip=[$srcdirstrip]"; \
echo "topsrcdirstrip=[$topsrcdirstrip]"; \
echo "dist_files=[$dist_files]"; \
case $dist_files in \
  */*) echo "WOULD_MKDIR" ;; \
esac; \
echo "REACHED_END"
