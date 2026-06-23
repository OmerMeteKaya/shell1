echo "TEST_E: nokta-slash onekli liste"
srcdirstrip='\.'; \
topsrcdirstrip='\.'; \
list='./Makefile.am ./configure foo.m4 bar.txt'; \
dist_files=`for file in $list; do echo $file; done | sed -e "s|^$srcdirstrip/||;t" -e "s|^$topsrcdirstrip/|./|;t"`; \
echo "E done dist_files=[$dist_files]"

echo "TEST_F: E + case dist_files in */*) esac"
srcdirstrip='\.'; \
topsrcdirstrip='\.'; \
list='./Makefile.am ./configure foo.m4 bar.txt'; \
dist_files=`for file in $list; do echo $file; done | sed -e "s|^$srcdirstrip/||;t" -e "s|^$topsrcdirstrip/|./|;t"`; \
case $dist_files in \
  */*) echo "WOULD_MKDIR" ;; \
esac; \
echo "F done"

echo "TEST_G: F + backtickli srcdirstrip (gercek recipe gibi)"
srcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; \
topsrcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; \
list='./Makefile.am ./configure foo.m4 bar.txt'; \
dist_files=`for file in $list; do echo $file; done | sed -e "s|^$srcdirstrip/||;t" -e "s|^$topsrcdirstrip/|./|;t"`; \
case $dist_files in \
  */*) echo "WOULD_MKDIR" ;; \
esac; \
echo "G done"
