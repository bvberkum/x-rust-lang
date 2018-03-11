default: hello_world bin/hello_cargo bin/naval_fate

%: %.rs
	rustc $<

bin/%: %/Cargo.toml
	mkdir -p bin
	cd $(*) && cargo build --release && mv target/release/$* ../bin/

.PHONY: default
