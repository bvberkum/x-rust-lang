default: build
all: test

BIN += hello_world \
	bin/hello_cargo \
	bin/naval_fate \
	bin/tutorial_01 \
	bin/simple_tcp_client \
	bin/hello_world_tcp
TEST += test-hello_world test-hello_cargo test-naval_fate test-hello_world_tcp

build: $(BIN)
	
%: %.rs
	rustc $<
	du -h $@

bin/%: %/src/*.rs %/Cargo.toml
	mkdir -p bin
	cd $(*) && cargo build --release && mv target/release/$* ../bin/
	du -hs bin/*

clean:
	rm $(BIN)

clean-all:
	git clean -dfx

test:: build $(TEST)

test-hello_world: hello_world
	./$<

test-hello_cargo: bin/hello_cargo
	./$<

test-naval_fate: bin/naval_fate
	./$< --version

#test-tutorial_01: bin/tutorial_01
#	echo 52 | ./$<

test-hello_world_tcp: bin/hello_world_tcp
	./$< &
	{ echo spawn telnet localhost 12345; \
	  echo expect \"hello world\"; \
	} | expect
	killall hello_world_tcp

.PHONY: default all build clean clean-all test $(TEST)
