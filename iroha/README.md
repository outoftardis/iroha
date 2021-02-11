# Iroha Application

## Description

When you start your own ledger, Iroha Application will make peers in it up and running
based on predefined configuration.

## Usage

### Docker Compose Deployment

#### Prepare Key Pairs

Before deployment each Peer should generate own pair of crypthographic keys. In our example we will use `Ed25519` and 
[iroha_crypto_cli](https://github.com/hyperledger/iroha/blob/iroha2-dev/iroha_crypto_cli/README.md) tool. This tool is a recommended way to generate iroha keys.

```bash
./iroha_crypto_cli
```

As a result you will see something like that:

```bash
Public key (multihash): ed20bdf918243253b1e731fa096194c8928da37c4d3226f97eebd18cf5523d758d6c
Private key: 0311152fad9308482f51ca2832fdfab18e1c74f36c6adb198e3ef0213fe42fd8bdf918243253b1e731fa096194c8928da37c4d3226f97eebd18cf5523d758d6c
Digest function: ed25519
```

Paste these values into `docker-compose.yml` environment variables for the first Iroha Peer:

```yaml
version: "3.3"
services:
  iroha:
    build:
      context: ./
      dockerfile: Dockerfile.debug
    image: iroha:debug
    environment:
      TORII_URL: iroha:1337
      IROHA_PUBLIC_KEY: 'ed20bdf918243253b1e731fa096194c8928da37c4d3226f97eebd18cf5523d758d6c'
      IROHA_PRIVATE_KEY: '{"digest_function": "ed25519", "payload": "0311152fad9308482f51ca2832fdfab18e1c74f36c6adb198e3ef0213fe42fd8bdf918243253b1e731fa096194c8928da37c4d3226f97eebd18cf5523d758d6c"}'
...
```

Repeat this for each Peer, and do not forget to update `IROHA_TRUSTED_PEERS` correspondingly. 

Also take a look at the reference configurations for a [single peer](https://github.com/hyperledger/iroha/blob/iroha2-dev/docker-compose-single.yml)
and for [multiple peers](https://github.com/hyperledger/iroha/blob/iroha2-dev/docker-compose.yml).

#### Build Binaries

```bash
cargo build
```

#### Build Images

```bash
docker-compose build
```

#### Run Containers

```bash
docker-compose up
```

If you want to keep containers up and running after closing the terminal, use *detached* flag:

```bash
docker-compose up -d
```

#### Stop Containers

```bash
docker-compose stop
```

#### Remove Containers

```bash
docker-compose down
```

### Want to help us develop Iroha?

That's great! 
Check out [this document](https://github.com/hyperledger/iroha/blob/iroha2-dev/CONTRIBUTING.md)

## Need help?

* Join [Telegram chat](https://t.me/hyperledgeriroha) or [Hyperledger RocketChat](https://chat.hyperledger.org/channel/iroha) where the maintainers, contributors and fellow users are ready to help you. 
You can also discuss your concerns and proposals and simply chat about Iroha there or in Gitter [![Join the chat at https://gitter.im/hyperledger-iroha/Lobby](https://badges.gitter.im/hyperledger-iroha/Lobby.svg)](https://gitter.im/hyperledger-iroha/Lobby)
* Submit issues and improvement suggestions via [Hyperledger Jira](https://jira.hyperledger.org/secure/CreateIssue!default.jspa) 
* Subscribe to our [mailing list](https://lists.hyperledger.org/g/iroha) to receive the latest and most important news and spread your word within Iroha community

## License

Iroha codebase is licensed under the Apache License,
Version 2.0 (the "License"); you may not use this file except
in compliance with the License. You may obtain a copy of the
License at http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

Iroha documentation files are made available under the Creative Commons
Attribution 4.0 International License (CC-BY-4.0), available at
http://creativecommons.org/licenses/by/4.0/