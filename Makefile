# Makefile for RAG Demo
# 
# Note: S3 Vectors is currently in preview and only available in:
# - us-east-1
# - us-west-2
# Make sure to use one of these regions

EXAMPLE=rag_demo
DOCS_DIR=test_documents
QUERY="What is RAG?"
TOP_K=5
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=
AWS_SECRET_ACCESS_KEY=

export AWS_REGION
export AWS_ACCESS_KEY_ID
export AWS_SECRET_ACCESS_KEY

.PHONY: all build init ingest query interactive clean

all: build

build:
	cargo build --release --example $(EXAMPLE)

init:
	@echo "Creating model directory..."
	mkdir -p models/all-MiniLM-L6-v2
	@echo "Downloading config.json..."
	curl -L https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/config.json -o models/all-MiniLM-L6-v2/config.json
	@echo "Downloading tokenizer.json..."
	curl -L https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json -o models/all-MiniLM-L6-v2/tokenizer.json
	@echo "Downloading vocab.txt..."
	curl -L https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/vocab.txt -o models/all-MiniLM-L6-v2/vocab.txt
	@echo "Downloading special_tokens_map.json..."
	curl -L https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/special_tokens_map.json -o models/all-MiniLM-L6-v2/special_tokens_map.json
	@echo "Downloading model.safetensors..."
	curl -L https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/model.safetensors -o models/all-MiniLM-L6-v2/model.safetensors
	@echo "Model files downloaded."
	@echo "Running cargo example init..."
	cargo run --example $(EXAMPLE) -- init

ingest:
	cargo run --example $(EXAMPLE) -- ingest --directory $(DOCS_DIR)

query:
	cargo run --example $(EXAMPLE) -- query --query $(QUERY) --top-k $(TOP_K)

interactive:
	cargo run --example $(EXAMPLE) -- interactive

clean:
	cargo clean
