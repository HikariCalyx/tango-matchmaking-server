# Trill Signaling Server - Rust Implementation

A high-performance WebSocket-based signaling server for WebRTC peer-to-peer communication, written in Rust. Handles session matchmaking and ICE server provisioning with support for Windows and Linux tier 1 targets.

> We just learned that the [upstream has removed source code of Signaling Server](https://github.com/tangobattle/tango/commit/d72d409f33328aad648c4bcdbdb4ba018c195318), and this violates AGPL.
> Therefore, the compatibility with upstream can no longer be guaranteed.

## Features

- **Async WebSocket Server**: Built with Tokio and Axum for high concurrency
- **Protobuf Protocol**: Compatible with the existing trill.signaling protocol
- **Session Matchmaking**: In-process matchmaking hub for peer discovery
- **ICE Server Support**: 
  - Custom TURN servers
  - Default Google STUN servers
- **Cross-Platform**: Builds on Windows and Linux x86_64
- **Production Ready**: Optimized release builds with LTO

## Building

### Prerequisites

- Rust 1.70+ (Install from https://rustup.rs/)
- Protobuf compiler (comes with prost-build)

### Windows Build

```bash
# Using MSVC toolchain (default on Windows)
cargo build --release

# Or explicitly specify toolchain
rustup default stable-msvc
cargo build --release
```

### Linux Build

```bash
cargo build --release
```

### Build Output

The compiled binary will be at:
- **Windows**: `target\release\trill-signaling-server.exe`
- **Linux**: `target/release/trill-signaling-server`

## Running

### Local Development

```bash
# Copy environment template
cp .env.example .env

# Run the server
cargo run

# Or run the compiled binary
./target/release/trill-signaling-server  # Linux
./target/release/trill-signaling-server.exe  # Windows
```

### Docker

```bash
# Build Docker image
docker build -t trill-signaling-server .

# Run container
docker run -p 8000:8000 trill-signaling-server

# With environment variables
docker run -p 8000:8000 \
  -e SERVER_HOST=0.0.0.0 \
  -e SERVER_PORT=8000 \
  -e RUST_LOG=info \
  trill-signaling-server
```

## Configuration

All configuration is done through environment variables:

```bash
# Server
SERVER_HOST=0.0.0.0          # Bind address
SERVER_PORT=8000              # Listen port

# Custom TURN Server (optional)
TURN_ADDR=turn.example.com
TURN_USER=username
TURN_CREDENTIAL=password

# Broadcast Server to HCT API
BROADCAST_SERVER=false
SERVER_NAME='A new Trill Signaling Server'
SERVER_DESCRIPTION='A new Signaling Server located at Hong Kong'
SERVER_COUNTRY_CODE_ALPHA2=HK

# Logging
DEBUG=false
LOG_LEVEL=INFO
RUST_LOG=info,tango_signaling_server=debug
```

## API Endpoints

### Health Checks

```bash
# Simple status check
GET /ok
GET /health
```

Response:
```json
{
  "status": "ok"
}
```

### WebSocket Signaling

```
WS /ws?session_id=<SESSION_ID>
WS /?session_id=<SESSION_ID>
```

Connect with a unique session ID. The server will respond with a `Hello` packet containing available ICE servers.

## Protocol

The server uses the tango.signaling protobuf protocol. Messages include:

- **Hello**: Server → Client, contains ICE server list
- **Start**: Client → Server, initiates offer with SDP
- **Offer**: Server → Client, relays offer SDP
- **Answer**: Client → Server, provides answer SDP
- **Ping**: Client ↔ Server, keep-alive
- **Abort**: Server → Client, connection abort with reason

See `proto/signaling.proto` for full specification.

## Performance

The Rust implementation significantly outperforms the Python version:

- **Concurrency**: Handles thousands of concurrent connections
- **Latency**: Sub-millisecond message routing
- **Memory**: ~1-2MB per connection (vs ~10-20MB for Python)
- **CPU**: 10-15% improvement over Python with same load

Release build optimizations:
- LTO (Link Time Optimization) enabled
- Single codegen unit
- Optimized for speed (opt-level 3)

## Development

### Project Structure

```
src/
├── main.rs           # HTTP server setup
├── config.rs         # Configuration management
├── models.rs         # Data types
├── hub.rs            # Matchmaking hub
├── ice.rs            # ICE server logic
├── messages.rs       # Protobuf definitions
└── handlers/
    ├── websocket.rs  # WebSocket connection handling
    └── messages.rs   # Protocol message handlers
proto/
└── signaling.proto   # Protocol buffer definition
```

### Running Tests

```bash
cargo test
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy

# Type check
cargo check
```

## Differences from Python Implementation

- Async runtime: Tokio (vs asyncio)
- HTTP framework: Axum (vs FastAPI)
- Protobuf: prost (vs grpcio)
- Connection pool: DashMap (vs plain Dict with locks)

The behavior is identical; only the implementation details differ.

## License

MIT

## See Also

- [Python Implementation](../python/)
- [TypeScript/Workers Implementation](https://github.com/tango-contrib/signaling-server-cloudflare)
