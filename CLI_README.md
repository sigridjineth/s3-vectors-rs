# S3 Vectors CLI

## Installation

```bash
cargo build --release
```

## Usage

# S3 Vectors Interactive Mode

## Overview

S3 Vectors CLI now supports an interactive mode that provides a REPL (Read-Eval-Print Loop) experience for managing your vector data.

## Entering Interactive Mode

Simply run the CLI without any subcommands:

```bash
s3-vectors
```

You'll be greeted with an ASCII banner and helpful tips to get started.

## Available Commands

In interactive mode, you can use all the same commands as the regular CLI, but without the `s3-vectors` prefix:

### Bucket Management
- `bucket create my-vectors` - Create a new vector bucket
- `bucket list` - List all vector buckets
- `bucket get my-vectors` - Get bucket details
- `bucket delete my-vectors` - Delete a bucket

### Index Management
- `index create my-bucket my-index -d 384` - Create an index with 384 dimensions
- `index list my-bucket` - List indexes in a bucket
- `index get my-bucket my-index` - Get index details
- `index delete my-bucket my-index` - Delete an index

### Vector Operations
- `vector put my-bucket my-index key1 -d 0.1,0.2,0.3` - Add a vector
- `vector get my-bucket my-index key1` - Retrieve a vector
- `vector query my-bucket my-index -q 0.1,0.2,0.3 -t 10` - Query similar vectors
- `vector delete my-bucket my-index key1` - Delete a vector

### Policy Management
- `policy put my-bucket --file policy.json` - Set bucket policy
- `policy get my-bucket` - Get bucket policy
- `policy delete my-bucket` - Delete bucket policy

## Special Commands

- `help`, `/help`, `?` - Show available commands
- `clear`, `/clear` - Clear the screen
- `exit`, `quit`, `/exit`, `/quit` - Exit interactive mode

## Examples

```bash
# Start interactive mode
$ s3-vectors

# Create a bucket
s3-vectors> bucket create my-vectors

# Create an index
s3-vectors> index create my-vectors embeddings -d 384

# Add some vectors
s3-vectors> vector put my-vectors embeddings doc1 -d 0.1,0.2,0.3...
s3-vectors> vector put my-vectors embeddings doc2 -d 0.2,0.3,0.4...

# Query for similar vectors
s3-vectors> vector query my-vectors embeddings -q 0.15,0.25,0.35... -t 5

# Exit
s3-vectors> exit
```

## Features

- **Command History**: Use up/down arrows to navigate through previous commands
- **Auto-completion**: Tab completion for commands (when supported by terminal)
- **Colored Output**: Enhanced readability with syntax highlighting
- **Error Handling**: Friendly error messages guide you to correct usage

## Tips

1. All commands support the same options as the regular CLI
2. You can still use global options like `--output json` for different output formats
3. The interactive mode respects environment variables like `AWS_REGION` and `AWS_PROFILE`
4. Use `--verbose` flag with commands for detailed output

### Global Options

```bash
s3-vectors [OPTIONS] <COMMAND>

Options:
  -r, --region <REGION>     AWS region [default: us-east-1]
  -p, --profile <PROFILE>   AWS profile to use
  -o, --output <OUTPUT>     Output format [default: table] [possible values: json, table, yaml]
  --no-verify-ssl           Disable SSL certificate verification
  -v, --verbose             Enable verbose output
```

### Bucket Operations

```bash
# Create a vector bucket
s3-vectors bucket create my-vectors

# List all vector buckets
s3-vectors bucket list

# Get bucket details
s3-vectors bucket get my-vectors

# Delete a bucket (with confirmation)
s3-vectors bucket delete my-vectors
```

### Index Operations

```bash
# Create an index
s3-vectors index create my-vectors products -d 128 -m cosine

# List indexes in a bucket
s3-vectors index list my-vectors

# Get index details
s3-vectors index get my-vectors products

# Delete an index
s3-vectors index delete my-vectors products
```

### Vector Operations

```bash
# Put a single vector
s3-vectors vector put my-vectors products prod-1 -d "0.1,0.2,0.3..." -m '{"name":"Product 1"}'

# Put vectors from file (batch)
s3-vectors vector put my-vectors products -f vectors.json

# Get vectors by keys
s3-vectors vector get my-vectors products key1,key2,key3 --include-data --include-metadata

# List vectors
s3-vectors vector list my-vectors products -m 100

# Query vectors (similarity search)
s3-vectors vector query my-vectors products -q "0.1,0.2,0.3..." -t 10 -f '{"category":"electronics"}'

# Delete vectors
s3-vectors vector delete my-vectors products key1,key2,key3
```

### Policy Operations

```bash
# Put bucket policy from file
s3-vectors policy put my-vectors -f policy.json

# Put bucket policy inline
s3-vectors policy put my-vectors -p '{"Version":"2012-10-17",...}'

# Get bucket policy
s3-vectors policy get my-vectors

# Delete bucket policy
s3-vectors policy delete my-vectors
```

## Environment Variables

- `AWS_ACCESS_KEY_ID`: AWS access key
- `AWS_SECRET_ACCESS_KEY`: AWS secret key
- `AWS_SESSION_TOKEN`: Optional session token
- `AWS_REGION`: Default AWS region
- `AWS_PROFILE`: Default AWS profile

## Examples

### Create and populate a vector index

```bash
# Create bucket and index
s3-vectors bucket create product-search
s3-vectors index create product-search items -d 384 -m cosine

# Add vectors
s3-vectors vector put product-search items laptop-1 \
  -d "0.1,0.2,0.3..." \
  -m '{"name":"Gaming Laptop","price":1299.99,"category":"electronics"}'

# Query for similar products
s3-vectors vector query product-search items \
  -q "0.15,0.22,0.28..." \
  -t 5 \
  --include-metadata \
  --include-distance
```

### Export results as JSON

```bash
s3-vectors bucket list -o json > buckets.json
s3-vectors vector query my-bucket my-index -q "0.1,0.2..." -o json > results.json
```

### Batch operations with progress

When processing large numbers of vectors, the CLI automatically shows progress:

```bash
s3-vectors vector put my-bucket my-index -f large-dataset.json
# Shows progress bar for batches of 500 vectors
```

## Error Handling

The CLI provides detailed error messages and proper exit codes:
- Exit code 0: Success
- Exit code 1: General error
- Exit code 2: Invalid arguments
- Exit code 3: AWS authentication error
- Exit code 4: Resource not found

## Security

- Supports AWS IAM authentication
- Respects AWS credential chain (environment, profile, instance role)
- Optional SSL verification disable for development
- Bucket policies for fine-grained access control

## Notes

- S3 Vectors is currently in preview
- Limited to us-east-1 and us-west-2 regions during preview
- Maximum 500 vectors per batch operation
- Vector dimensions: 1-4096
- Supported distance metrics: euclidean, cosine