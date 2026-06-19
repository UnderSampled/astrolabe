LZO_DIR := ../../lib/lzo
CC ?= gcc
CFLAGS ?= -std=c99 -Wall -Wextra -O0
CPPFLAGS := -I$(LZO_DIR)/minilzo -I$(LZO_DIR)/include
SOURCES := lzo1x.c $(LZO_DIR)/minilzo/minilzo.c

.PHONY: all clean

all: lzo1x

lzo1x: $(SOURCES)
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ $(SOURCES)

clean:
	rm -f lzo1x