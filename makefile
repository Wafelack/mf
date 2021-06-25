PREFIX ?= /usr
BINARY := target/release/mf

all: src/
	cargo build --release
install: all
	strip $(BINARY)
	cp $(BINARY) $(PREFIX)/bin/
	cp mf.1 $(PREFIX)/share/man/man1/
