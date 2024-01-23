# Syncx
Syncx facilitates client-server interactions for file uploads and downloads, utilizing Google Cloud Storage. The server stores files uploaded by clients, who can later retrieve these files along with a Merkle proof to verify integrity. The project comprises four crates, implementing a simple yet effective architecture with gRPC communication, a background worker system, and MongoDB and Redis for data management.

## Crates Overview
- [merkle-tree](https://github.com/0xphen/syncx/tree/main/merkle-tree): Implements an efficient Merkle tree for data integrity verification. It generates trees from any data convertible to bytes and includes leaf indexing for fast Merkle proof verification.

- [common](https://github.com/0xphen/syncx/tree/main/common): A shared library between the client and server. It centralizes common functionalities to avoid code duplication and enhance maintainability.

- [client](https://github.com/0xphen/syncx/tree/main/client): A CLI-based client application. It handles file uploads to the server, file downloads, and locally stores the Merkle root for uploaded files.

- [server](https://github.com/0xphen/syncx/tree/main/server): Manages file storage on Google Cloud Storage, handles client requests for file uploads and downloads, and provides Merkle proofs for downloaded files. Client credentials are securely managed via MongoDB.


## Setup
### Dependencies
To set up this project, you need the protoc Protocol Buffers compiler, along with Protocol Buffers resource files.. For installing and setting up the Protocol Buffer compiler, refer to the [tonic](https://crates.io/crates/tonic) documentation. 

### Build
Ensure Rust and Clone the repository and install dependencies:
```
$ git clone git@github.com:0xphen/syncx.git
$ cd syncx
$ cargo build
```

## Usage
#### Run the server and worker
```
$ cargo run --bin server
```

In another terminal, run the worker:
```
$ cargo run --bin worker
```

#### Client
#### Register client on server 
```
$ cargo run create_account -p "<password>"  
```

##### Upload file(s) server
```
$ cargo run upload -d <path to directory>
```

##### Download file from server
```
$ cargo run download -f <name of file> -d <path to save download>
```

##### Read merkle root of uploaded files
```
$ cargo run merkleroot 
```
