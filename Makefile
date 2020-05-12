RUST_VERSION=1.44.0
DOCKER_TAG=$(shell git describe --tags)
DOCKER_TEMPLATES:=$(wildcard *.Dockerfile.template)
DOCKER_FILES=$(DOCKER_TEMPLATES:%.template=%)
DOCKER_IMAGES=$(DOCKER_FILES:%.Dockerfile=%)

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
.PRECIOUS: %.Dockerfile
%.Dockerfile: %.Dockerfile.template # Generate the % Dockerfile from the template
	sed 's/%%RUST_VERSION%%/$(RUST_VERSION)/g; s/%%BUILD_VERSION%%/$(DOCKER_TAG)/g' $< > $@

##
.PHONY: docker-%
docker-image-%: %.Dockerfile # Build the % docker image
	cargo clean
	docker build -t bolcom/unftp:$(DOCKER_TAG)-$* -f $< .

##
.PHONY: docker-run-%
docker-run-%: docker-image-% # Run the % docker image in the foreground
	@echo docker run -ti --rm --net host --init $@

.PHONY: docker-list
docker-list: # List the available docker images
	@echo $(DOCKER_IMAGES)

##
.PHONY: pr-prep
pr-prep: # Runs checks to ensure you're ready for a pull request
	cargo fmt --all -- --check
	cargo clippy --all-features -- -D warnings
	cargo build --verbose --all --features rest_auth,jsonfile_auth,cloud_storage
	cargo test --verbose --all --features rest_auth,jsonfile_auth,cloud_storage
	cargo doc --all-features --no-deps

release-artifacts:
	rm -rf release && mkdir release
	cargo build --release --target x86_64-apple-darwin --features rest_auth,jsonfile_auth,cloud_storage
	cp target/x86_64-apple-darwin/release/unftp ./release/unftp_x86_64-apple-darwin
	md5 -r release/unftp_x86_64-apple-darwin > release/unftp_x86_64-apple-darwin.md5
	$(MAKE) docker-image-alpine
	docker run --name unftp-release-maker -d bolcom/unftp:$(DOCKER_TAG)-alpine sh
	docker cp unftp-release-maker:/unftp/unftp release/unftp_x86_64-unknown-linux-musl
	docker rm -vf unftp-release-maker
	md5 -r release/unftp_x86_64-unknown-linux-musl > release/unftp_x86_64-unknown-linux-musl.md5

clean:
	cargo clean
	rm -rf release
	rm *.Dockerfile
