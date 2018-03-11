#$(info Executable: $(MAKE))
#$(info Makefiles: $(MAKEFILE_LIST))
#$(info Targets: $(MAKECMDGOALS))

BIN_NAME := $(shell basename $$(pwd))
BIN := ./target/debug/$(BIN_NAME)
$(info Makefile for $(BIN_NAME))

default: $(BIN)
	
$(BIN): src/main.rs
	cargo build

run: $(BIN) ; $<

clean: ; rm $(BIN)

.PHONY: default run clean
