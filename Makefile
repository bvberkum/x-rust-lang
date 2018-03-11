default: hello_world

%: %.rs
	rustc $<

.PHONY: default
