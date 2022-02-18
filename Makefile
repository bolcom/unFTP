RUST_VERSION=1.58.0
DOCKER_TAG=$(shell git describe --tags)
DOCKER_TEMPLATES:=$(wildcard *.Dockerfile.template)
DOCKER_FILES=$(DOCKER_TEMPLATES:%.template=%)
DOCKER_IMAGES=$(DOCKER_FILES:%.Dockerfile=%)
DOCKER_DEFAULT=alpine

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


##
.PHONY: docker-image
docker-image: $(DOCKER_DEFAULT).Dockerfile # Build the default docker image
	docker build -t bolcom/unftp:$(DOCKER_DEFAULT)-$(DOCKER_TAG) -f $(DOCKER_DEFAULT).Dockerfile .

.PHONY: docker-run
docker-run: docker-image # Run the default docker image in the foreground
	docker run -ti --rm --net host --init bolcom/unftp:$(DOCKER_DEFAULT)-$(DOCKER_TAG)

.PHONY: docker-list
docker-list: # List the available docker images
	@echo $(DOCKER_IMAGES)

##
.PHONY: image-tag
image-tag: # Prints the tag that will be used for docker images
	@echo $(DOCKER_TAG)

.PHONY: pr-prep
pr-prep: # Runs checks to ensure you're ready for a pull request
	cargo fmt --all -- --check
	cargo clippy --all-features --workspace -- -D warnings
	cargo test --verbose  --workspace --all --all-features
	cargo doc  --workspace --all-features --no-deps
	cargo build --verbose  --workspace --all --all-features

.PHONY: release-artifacts
release-artifacts: # Generates artifacts for a release
	rm -rf release && mkdir release
	cargo build --release --target x86_64-apple-darwin --features rest_auth,jsonfile_auth,cloud_storage
	cp target/x86_64-apple-darwin/release/unftp ./release/unftp_x86_64-apple-darwin
	md5 -r release/unftp_x86_64-apple-darwin > release/unftp_x86_64-apple-darwin.md5
	$(MAKE) docker-image-alpine
	docker run --rm bolcom/unftp:$(DOCKER_TAG)-alpine cat /unftp/unftp > release/unftp_x86_64-unknown-linux-musl
	md5 -r release/unftp_x86_64-unknown-linux-musl > release/unftp_x86_64-unknown-linux-musl.md5
	$(MAKE) docker-image-gnubuilder
	docker run --rm bolcom/unftp:$(DOCKER_TAG)-gnubuilder > release/unftp_x86_64-unknown-linux-gnu
	md5 -r release/unftp_x86_64-unknown-linux-gnu > release/unftp_x86_64-unknown-linux-gnu.md5

.PHONY: publish
publish: # Publishes to crates.io
	cargo publish --verbose --features rest_auth,jsonfile_auth,cloud_storage

.PHONY: site
site: # Publishes to the documentation to Github Pages and Netlify
	sed -i '' 's|base_path: /|base_path: /unFTP|g' doctave.yaml
	doctave build --release	
	gh-pages -d site -b gh-pages
	rm -rf site
	sed -i '' 's|base_path: /unFTP|base_path: /|g' doctave.yaml
	doctave build --release
	gh-pages -d site -b netlify
	rm -rf site

.PHONY: clean
clean: # Removes generated files
	cargo clean
	rm -rf release
	rm -f *.Dockerfile
	rm -rf site
