RUST_VERSION=1.33
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
					printf("%-20s %s\n", $$1, $$2) \
				} \
			} else { \
				printf("%-20s %s\n", $$1, $$2) \
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
	docker build -t unftp-$*:$(DOCKER_TAG) -f $< .

##
.PHONY: docker-run-%
docker-run-%: docker-image-% # Run the % docker image in the foreground
	@echo docker run -ti --rm --net host --init $@

##
.PHONY: docker-image
docker-image: alpine.Dockerfile # Build the default docker image
	docker build -t bolcom/unftp:$(DOCKER_TAG) -f alpine.Dockerfile .

.PHONY: docker-run
docker-run: docker-image # Run the default docker image in the foreground
	docker run -ti --rm --net host --init bolcom/unftp:$(DOCKER_TAG)

.PHONY: docker-list
docker-list: # List the available docker images
	@echo $(DOCKER_IMAGES)
