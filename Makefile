default: build
all: test

BIN += hello_world bin/hello_cargo bin/naval_fate
TEST += test-hello_world test-hello_cargo test-naval_fate

build: $(BIN)
	
%: %.rs
	rustc $<
	du -h $@

bin/%: %/Cargo.toml
	mkdir -p bin
	cd $(*) && cargo build --release && mv target/release/$* ../bin/
	du -hs bin/*

clean:
	rm $(BIN)

test:: build $(TEST)

test-hello_world:
	./hello_world

test-hello_cargo:
	./bin/hello_cargo

test-naval_fate:
	./bin/naval_fate --version

.PHONY: default all build clean test $(TEST)
