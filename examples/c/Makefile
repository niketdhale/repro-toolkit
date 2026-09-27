# Builds repro-toolkit-ffi (if needed) and this C example against it.
#
# Usage:
#   make run
#   make run PDX=/path/to/other.pdx SEQUENCE=/path/to/custom-sequence.json

FFI_DIR := ../../crates/repro-toolkit-ffi
TARGET_DIR := ../../target/release
LIB_NAME := repro_toolkit_ffi

PDX ?= ../../samples/sample.pdx
SEQUENCE ?= ../../samples/custom-sequence.example.json

.PHONY: all lib run clean

all: main

lib:
	cargo build --release --manifest-path $(FFI_DIR)/Cargo.toml

main: main.c repro_toolkit.h lib
	$(CC) -Wall -Wextra -o main main.c -I. -L$(TARGET_DIR) -l$(LIB_NAME)

run: main
	LD_LIBRARY_PATH=$(TARGET_DIR):$$LD_LIBRARY_PATH ./main $(PDX) $(SEQUENCE)

clean:
	rm -f main
