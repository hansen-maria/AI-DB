.PHONY: build run stop restart logs clean help

# Variables
IMAGE_NAME = ai-db
CONTAINER_NAME = ai-db
PORT = 80

help: ## Show Help
	@echo "Available Commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build Docker Image
	docker build -t $(IMAGE_NAME) .

run: ## Start Container
	docker-compose up -d

stop: ## Stop Container
	docker-compose down

restart: ## Restart Container
	docker-compose restart

logs: ## Show Logs
	docker-compose logs -f

clean: ## Remove Container und Images
	docker-compose down
	docker rmi $(IMAGE_NAME)

rebuild: clean build run ## Clean Build

status: ## Show Container Status
	docker-compose ps

shell: ## Open Shell in Container
	docker exec -it $(CONTAINER_NAME) /bin/sh

deploy: build run ## Build Deployment
	@echo "Deployment successfully!"
	@echo "App running on http://localhost:$(PORT)"