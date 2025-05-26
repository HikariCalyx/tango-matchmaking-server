# Tango-Matchmaking-Server

For the source code, see [here](https://github.com/HikariCalyx/trill/tree/main/tango-signaling-server) .

This repository served as file repository of Matchmaking server executable files.

# Supported OS for hosting

- Linux: with glibc 2.31 or newer (i.e. Debian 11 or newer). Supported architecture: amd64 (i.e. x86_64), arm64 (i.e. aarch64), armv7-hf (Raspberry Pi 2/3/Zero 2)
- Windows: Server 2016 or newer

# Server Hardware Requirements

- RAM: 512MB or greater
- Internet Connection: The server hardware/VM instance must not behind NAT. Most of VPS should work.

# Getting Started

There are 5 options to utilize matchmaking server. To use them, you'll need to configure environment variables.

- Self-Hosted TURN Server
- Cloudflare TURN Service
- Twilio
- Opentok
- Metered

