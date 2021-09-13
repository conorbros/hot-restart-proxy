# Hot Restart Proxy

Demo project for testing out how a proxy would hot restart itself without dropping any connections while handling configuration changes. Uses IPC with unix sockets to share the socket FDs between processes.

## Usage

Run the node program with and have the ports you want it to listen on as arguments: `node node_tcp_server.js 5431 5432 5433 5434`.

`config/config.yaml`determines the upstreams (sockets that the node program is listening on) to forward requests to. This can be changed between restarts and the changes will be reflected in the new process.

Run the first instance of the proxy with `cargo run`. Test the proxy with `telnet 127.0.0.1 8080`by sending some data through.

Run the second instance of the proxy with `cargo run -- -t`.
