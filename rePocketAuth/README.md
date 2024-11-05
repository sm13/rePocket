# rePocketAuth

Host application to register and authorize a Pocket application.

`rePocketAuth` does three things:

1. It communicates with Pocket via the [POST API](https://getpocket.com/developer/docs/authentication)

2. It runs a local web server to listen to the redirection

3. Creates a file with the `consumer_key` and `access_token` for `rePocket`

## Register an App with Pocket

In order to get the this working you'll have to register an *Application* (as in software application) with Pocket. This will provide you with the `consumer_key`, **your** consumer key.

## Building and Running

To build `rePocketAuth` , first you need to create a certificate for the web server. For instance:

```bash
openssl req -new -newkey rsa:4096 -x509 -sha256 -days 365 -nodes -out rePocket.crt -keyout rePocket.key
```

Place and rename the files as follows

```bash
# +── data
#     └── cert
#         ├── rePocket.crt
#         └── rePocket.key
```

```bash
# From the repository root
cd rePocketAuth
cargo build --release
```

Once built, you can run it using cargo **from the same location**.

```bash
cargo run
```

This will ask you for your consumer key and then open a browser to guide you through the authorization process. Once authorized it will redirect you to a local URL. You may close the browser and return to the terminal and continue following instructions. If the program exists without reporting errors, your App has been authorized and you just got yourself a key token pair!
