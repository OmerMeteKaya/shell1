echo "TEST_A: iki atama ardisik, \-continuation ile"
srcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; \
topsrcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; \
echo "A done srcdirstrip=[$srcdirstrip] topsrcdirstrip=[$topsrcdirstrip]"

echo "TEST_B: ayni ama araya list= ekle"
srcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; \
topsrcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; \
list='foo bar baz'; \
echo "B done list=[$list]"

echo "TEST_C: B + dist_files hesaplama (sed -e ile, cift -e flag)"
srcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; \
topsrcdirstrip=`echo "." | sed 's/[].[^$\\*]/\\\\&/g'`; \
list='foo bar baz'; \
dist_files=`for file in $list; do echo $file; done | sed -e "s|^$srcdirstrip/||;t" -e "s|^$topsrcdirstrip/|./|;t"`; \
echo "C done dist_files=[$dist_files]"

echo "TEST_D: sadece dist_files satiri, basit degerlerle"
srcdirstrip='\.'; \
topsrcdirstrip='\.'; \
list='foo bar baz'; \
dist_files=`for file in $list; do echo $file; done | sed -e "s|^$srcdirstrip/||;t" -e "s|^$topsrcdirstrip/|./|;t"`; \
echo "D done dist_files=[$dist_files]"
