RUST_VERSION=1.83.0
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

.PHONY: publish
publish: # Publishes to crates.io
	cargo publish --verbose --features rest_auth,jsonfile_auth,cloud_storage

.PHONY: site-preview
site-preview: # Previews the documentation for Github Pages and Netlify
	doctave serve

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
