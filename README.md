## Ngok

A minimal `ngrok` implementation in Rust, for educational purpose.
This work is largely based on [rathole][0], especially [the very first commit][1].
_This is by no mean an idiomatic or correct Rust implementation. I am mastering
Rust and fairly new to writing networking code with `tokio`_.

## Quick Start

There are two components in `ngok`:

- `src/server.rs`: the server consists of two part, the control server and the proxy server.
  - The control server take in request from the clients, assign domain name and setup proxy server
  for each clients.
  - The proxy server will receive requests through the provided domain name and proxy it to the client.
running locally in the user machine.
- `src/client.rs`: the client receive the requests from the internet through the server,
and further proxy it to the web server running locally by the user.

### Setting Up

So in order to see this code in action, you'll have to run 3 service:

#### Running the server

The `server` binary expect the `domains` to be provided. These are the domains that
users from the internet will send requests to. Since we are running locally, we could
mock those domains and have it work locally by manually editing our hosts file at
`/etc/hosts`. Hence, this would only work in your local machine

```
# /etc/hosts

127.0.0.1 a.domain.com
127.0.0.1 b.domain.com
127.0.0.1 c.domain.com
```
Then, we can pass these to the `server` and run it:

```bash
RUST_LOG=info cargo run --bin server -- --domains a.domain.com --domains b.domain.com --domains c.domain.com
```

By default, the server start at port 3001.

```
Apr 30 09:58:49.694  INFO server: Listening on TCP: 0.0.0.0:3001
```

### Implementation and limitations

Currently, it's a half baked implementation. It basically:

- Have a server to listen to incoming `Init` packet from a client and reply
  it with a domain the client can use.
- Upon running the client, it will send an `Init` packet, to get the domain name.
- Then, the client send a `DataInit` packet to the server, setting up another TCP tunnel
  listening to request from the server and proxy it to the local server. It's currently
  hardcoded to forward to `localhost:4000`
- Once the server receive an `DataInit` packet, it will spin up a `TcpListener`
  on the assigned port for the particular domain name, which is hardcoded as
  well. This will then allow the public internet to send request to the domain,
  which will be proxy to the client local server through the TCP tunnel we
  setup.
- Once the client tunnal receive and respond with a request, the whole TCP
  tunnel will be shutdown, and it will resend the `DataInit` packet again to
  setup another TCP tunnel.
- Same applied to the `TcpListener` that is exposed to the internet. Hence,
  it's not an efficient implementation as it will spin up and shutdown 2 TCP
  connection on every request.

On top of that:

- It only support 3 domain name, that means it currently support at most 3
  client at a time.
- It doesn't work with websocket.
- ~The domain name is not recycle. This mean that if a client lost it's
  connection, the domain assigned do not goes back to the domain name pool.~
- ~It doesn't support logging HTTP request and response time.~

### Flow

The following flow will be the initial first attempt to make a minimal reverse
proxy. It's subject to changes as we involved our implementation.

1. First, a client send an `Init` packet to setup the _control channel_ with the
   server. Then, the server will assigned a domain and port for the control
   channel and reply the domain name to the client.

```
        Init
Client ------> Server
       <------
       Domain
```

2. Once the client receive the domain name, it send an `Ack` packet and the
   server will proceed to update the state to keep track of the control channel
   spawned.

```
         Ack
Client ------> Server (proceed to store the control channel information in the state)
```

3. Client will then proceed to setup a _data channel_ with the server and
   a `TcpStream` to connect to the local service, waiting for the incoming
   request from the server, so it can proxy the request to the local service and
   respond back to the server through the data channel.

```
                TCP connection            DataInit
Local Service <----------------> Client -----------> Server
```

4. When server receive the `DataInit` packet, it start listening to the
   particular port assigned to the domain name and start to forward any request
   to the _data channel_ to the client, which the client will proxy it to the
   local service and reply the responses to us through the same data channel,
   which we then return to the incoming request.

```
            TCP listener            Proxy request
Internet <---------------> Server <---------------> Server
```

5. Once a request is served, the _data channel_, TCP listner on the server and
   TCP connection to the local service from the client will all be shutdown.
   Step 3, 4 and 5 will be repeated.

### References

- [ngrok](https://github.com/inconshreveable/ngrok/blob/master/docs/DEVELOPMENT.md): To understand how to setup tunneling on a high level.
- [rathole][0]: The first commit of a lightweight Rust `ngrok` liked implementation. Use TCP all the way.
- [tunnelto](https://github.com/agrinman/tunnelto): Another Rust `ngrok` liked implementation. It use websockets to setup the control channel.
- [tokio.rs](https://tokio.rs/tokio/tutorial): Tokio tutorials.