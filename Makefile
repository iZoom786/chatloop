.PHONY: help build test clean docker-build docker-run split-model lint fmt

# Default target
.DEFAULT_GOAL := help

# Variables
CARGO := cargo
DOCKER := docker
PYTHON := python3
MODEL_NAME ?= meta-llama/Llama-2-7b-hf
NUM_PARTITIONS ?= 4

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build all Rust crates
	@echo "Building ChatLoop..."
	$(CARGO) build --release

build-dev: ## Build in development mode
	$(CARGO) build

test: ## Run all tests
	$(CARGO) test --all

test-release: ## Run tests in release mode
	$(CARGO) test --release --all

clean: ## Clean build artifacts
	$(CARGO) clean
	rm -rf models/*.safetensors
	rm -rf models/partitions/*

lint: ## Run linters
	$(CARGO) clippy --all-targets --all-features -- -D warnings

fmt: ## Format code
	$(CARGO) fmt

fmt-check: ## Check formatting
	$(CARGO) fmt -- --check

docker-base: ## Build base Docker image
	$(DOCKER) build -f docker/base.Dockerfile -t chatloop-base:latest .

docker-worker: docker-base ## Build worker Docker image
	$(DOCKER) build -f docker/worker.Dockerfile -t chatloop-worker:latest .

docker-coordinator: docker-base ## Build coordinator Docker image
	$(DOCKER) build -f docker/coordinator.Dockerfile -t chatloop-coordinator:latest .

docker-build: docker-base docker-worker docker-coordinator ## Build all Docker images

docker-run-worker: ## Run a single worker (for testing)
	$(DOCKER) run -it --rm \
		-p 50051:50051 \
		-p 9091:9091 \
		-v $(PWD)/models:/home/chatloop/models:ro \
		-v $(PWD)/configs/worker-config.yaml:/home/chatloop/configs/worker-config.yaml \
		-e CHATLOOP_CONFIG=/home/chatloop/configs/worker-config.yaml \
		chatloop-worker:latest

docker-run-coordinator: ## Run coordinator (for testing)
	$(DOCKER) run -it --rm \
		-p 50050:50050 \
		-p 9091:9091 \
		-v $(PWD)/configs/coordinator-config.yaml:/home/chatloop/configs/coordinator-config.yaml \
		-e CHATLOOP_CONFIG=/home/chatloop/configs/coordinator-config.yaml \
		chatloop-coordinator:latest

docker-compose-up: ## Start all services with docker-compose
	$(DOCKER)-compose up -d

docker-compose-down: ## Stop all services
	$(DOCKER)-compose down

split-model: ## Split a model into partitions
	@echo "Splitting model: $(MODEL_NAME) into $(NUM_PARTITIONS) partitions..."
	$(PYTHON) python/model_splitter/split_model.py \
		--model $(MODEL_NAME) \
		--output ./models/partitions \
		--num-partitions $(NUM_PARTITIONS)

split-model-int8: ## Split model with INT8 quantization
	@echo "Splitting model with INT8 quantization..."
	$(PYTHON) python/model_splitter/split_model.py \
		--model $(MODEL_NAME) \
		--output ./models/partitions \
		--num-partitions $(NUM_PARTITIONS) \
		--quantization int8

install-python-deps: ## Install Python dependencies
	pip install -r python/model_splitter/requirements.txt

proto: ## Regenerate protobuf files
	$(CARGO) build --package chatloop-proto

bench: ## Run benchmarks
	$(CARGO) bench --all

deps: ## Update dependencies
	$(CARGO) update

check: fmt-check lint test ## Run all checks (format, lint, test)

ci: check test-release ## Run CI pipeline

# Development helpers
dev-setup: install-python-deps ## Set up development environment
	@echo "Development environment setup complete!"
	@echo "1. Split a model: make split-model MODEL_NAME=your/model"
	@echo "2. Build images: make docker-build"
	@echo "3. Run services: make docker-compose-up"

dev-deploy: docker-build ## Quick deploy for development
	$(DOCKER)-compose up -d
	@echo "Services deployed. Check logs with: docker-compose logs -f"

# YARN deployment (requires YARN cluster)
yarn-deploy-workers: ## Deploy workers to YARN
	@echo "Deploying $(NUM_PARTITIONS) workers to YARN..."
	@for i in $$(seq 0 $$($(NUM_PARTITIONS) - 1)); do \
		echo "Deploying worker $$i..."; \
		yarn app -install chatloop-worker; \
		yarn app -start chatloop-worker -Dworker.id=$$i -Dworker.port=$$((50051 + $$i)); \
	done

yarn-deploy-coordinator: ## Deploy coordinator to YARN
	@echo "Deploying coordinator to YARN..."
	yarn app -install chatloop-coordinator
	yarn app -start chatloop-coordinator

yarn-deploy: yarn-deploy-coordinator yarn-deploy-workers ## Deploy full stack to YARN
