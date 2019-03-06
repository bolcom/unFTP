RUST_VERSION=1.33
DOCKER_TAG=$(shell git describe --tags)
DOCKER_TEMPLATES:=$(wildcard *.Dockerfile.template)
DOCKER_FILES=$(DOCKER_TEMPLATES:%.template=%)
DOCKER_IMAGES=$(DOCKER_FILES:%.Dockerfile=%)

.PHONY: help
help: # Show available `make` commands
	@awk -F'#' '/^[%a-z][.A-Za-z0-9]+/ { if (NF > 1) { sub(/:[^#]*/, ""); printf("%-15s %s\n", $$1, $$2)}}' Makefile

.PRECIOUS: %.Dockerfile
%.Dockerfile: %.Dockerfile.template # Generate the given Dockerfile from the template
	sed 's/%%RUST_VERSION%%/$(RUST_VERSION)/g' $< > $@

.PHONY: docker-%
docker-%: %.Dockerfile # Build the given docker image
	docker build -t unftp-$*:$(DOCKER_TAG) -f $< .

.PHONY: docker-run-%
docker-run-%: docker-% # Run the given docker image in the foreground
	@echo docker run -ti --rm --net host --init $@

.PHONY: docker-list
docker-list: # List the available docker images
	@echo $(DOCKER_IMAGES)
