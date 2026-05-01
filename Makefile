
## SECONDEXPANSION allows us to use $$ variables in prerequisites
.SECONDEXPANSION:

.DEFAULT_GOAL := binaries

ALL_BINARIES = mz-monitoring-build mz-monitoring-check
BUILD_BIN_BASENAME = $(notdir $@)

SOURCES_mz-monitoring-build = $(shell find crates/mz-monitoring-build -type f)
SOURCES_mz-monitoring-check = $(shell find crates/mz-monitoring-check -type f)

target/debug/mz-monitoring-%: $$(SOURCES_mz-monitoring-%)
	cargo build --bin "$(BUILD_BIN_BASENAME)"

## YAGNI
# target/release/mz-monitoring-%: $$(SOURCES_mz-monitoring-%)
# 	cargo build --release --bin "$(BUILD_BIN_BASENAME)"

binaries: $(addprefix target/debug/,$(ALL_BINARIES))
.PHONY: binaries
