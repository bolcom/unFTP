RUST_VERSION=1.92.0
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
site: # Publishes to the documentation to Netlify
	doctave build --release
	@if [ ! -d "site" ]; then \
		echo "Error: site directory not found after build"; \
		exit 1; \
	fi; \
	current_branch=$$(git branch --show-current); \
	site_tar=$$(mktemp -t site-XXXXXX.tar.gz); \
	tar -czf $$site_tar -C site .; \
	if git show-ref --verify --quiet refs/heads/netlify; then \
		git checkout netlify; \
	else \
		git checkout -b netlify; \
	fi; \
	find . -mindepth 1 -maxdepth 1 ! -name '.git' -exec rm -rf {} + 2>/dev/null || true; \
	tar -xzf $$site_tar; \
	rm -f $$site_tar; \
	git add -A; \
	git commit -m "Update documentation site" || true; \
	git push origin netlify; \
	git checkout $$current_branch; \
	rm -rf site

.PHONY: clean
clean: # Removes generated files
	cargo clean
	rm -rf release
	rm -f *.Dockerfile
	rm -rf site
