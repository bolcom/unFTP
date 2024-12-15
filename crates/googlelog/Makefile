.PHONY: help
help: # Show available `make` commands
	@awk -F'#' '\
	BEGIN{n=split("$(DOCKER_IMAGES)", docker_images, " ")} \
	/^[%a-z][.A-Za-z0-9]+/ { \
		if (NF > 1) { \
			sub(/:[^#]*/, ""); \
			if ($$1 ~ /%/ && $$1 ~ /[Dd]ocker/) { \
				line=$$0; \
				for (i=1; i<=n; ++i) { \
					$$0 = line; \
					gsub(/%/, docker_images[i]); \
					printf("%-25s %s\n", $$1, $$2) \
				} \
			} else { \
				printf("%-25s %s\n", $$1, $$2) \
			} \
		} \
	}\
	/^##/ { printf("\n") }' Makefile

##
.PHONY: pr-prep
pr-prep: # Runs checks to ensure you're ready for a pull request
	cargo fmt --all -- --check
	cargo clippy -- -D warnings
	cargo test
	cargo test --doc
	cargo build
	cargo doc --no-deps --features shipper

publish:
	cargo publish --verbose
