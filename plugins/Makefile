CC     = gcc
CFLAGS = -Wall -std=c99 -fPIC -I../include
PLUGINS = git.so

all: $(PLUGINS)

%.so: %.c
	$(CC) $(CFLAGS) -shared -o $@ $

clean:
	rm -f $(PLUGINS)