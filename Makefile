RUST_VERSION=1.99.1
DOCKER_TAG=$(shell git describe --tags --match 'v[0-9]*')
DOCKER_TEMPLATES:=$(wildcard *.Dockerfile.template)
DOCKER_FILES=$(DOCKER_TEMPLATES:%.template=%)
DOCKER_IMAGES=$(DOCKER_FILES:%.Dockerfile=%)
DOCKER_DEFAULT=alpine
DOCKER_NAMESPACE?=cloudinvoke

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
	docker build -t $(DOCKER_NAMESPACE)/unftp:$(DOCKER_TAG)-$* -f $< .

##
.PHONY: docker-run-%
docker-run-%: docker-image-% # Run the % docker image in the foreground
	@echo docker run -ti --rm --net host --init $@


##
.PHONY: docker-image
docker-image: $(DOCKER_DEFAULT).Dockerfile # Build the default docker image
	docker build -t $(DOCKER_NAMESPACE)/unftp:$(DOCKER_TAG)-$(DOCKER_DEFAULT) -f $(DOCKER_DEFAULT).Dockerfile .

.PHONY: docker-run
docker-run: docker-image # Run the default docker image in the foreground
	docker run -ti --rm --net host --init $(DOCKER_NAMESPACE)/unftp:$(DOCKER_TAG)-$(DOCKER_DEFAULT)

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
site: # Publishes the documentation to Netlify
	@set -eu; \
	if [ -e site ] || [ -L site ]; then \
		echo "Error: site directory already exists; move it before publishing" >&2; \
		exit 1; \
	fi; \
	git fetch origin netlify; \
	doctave build --release; \
	if [ ! -d site ]; then \
		echo "Error: site directory not found after build" >&2; \
		exit 1; \
	fi; \
	site_worktree=$$(mktemp -d); \
	trap 'git worktree remove --force "$$site_worktree" >/dev/null 2>&1 || rmdir "$$site_worktree" 2>/dev/null || true' 0; \
	git worktree add --detach "$$site_worktree" FETCH_HEAD; \
	git -C "$$site_worktree" rm -r -q -- .; \
	cp -R site/. "$$site_worktree/"; \
	git -C "$$site_worktree" add -A; \
	if git -C "$$site_worktree" diff --cached --quiet; then \
		echo "Documentation site is already up to date"; \
	else \
		git -C "$$site_worktree" commit -m "Update documentation site"; \
		git -C "$$site_worktree" push origin HEAD:refs/heads/netlify; \
	fi; \
	rm -r site

.PHONY: clean
clean: # Removes generated files
	cargo clean
	rm -rf release
	rm -f *.Dockerfile
	rm -rf site
