# Rust MCS Web Interface

Web interface for the [Rust MCS](https://github.com/SergeiGL/Rust_MCS) global minimization algorithm.

![MCS Web Interface Screenshot](img.png "The Rust MCS Web Interface")

## Quick Start

### Prerequisites

- [Git](https://git-scm.com/downloads)
- [Docker](https://www.docker.com/products/docker-desktop/)

### Installation and Launch

1. Clone the repository:
   ```bash
   git clone https://github.com/SergeiGL/Rust_MCS_web
   cd Rust_MCS_web
   ```

2. Start the application:
   ```bash
   docker compose build --no-cache
   ```
   ```bash
   docker compose up
   ```
3. Access the interface in your browser:
   ```
   http://localhost:3000
   ```

## Acknowledgments

- Based on the [Rust MCS](https://github.com/SergeiGL/Rust_MCS) crate
- Original MCS algorithm by Huyer and Neumaier