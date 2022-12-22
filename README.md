# Substrate &middot; [![Matrix](https://img.shields.io/matrix/cherry-technical:matrix.org)](https://matrix.to/#/#cherry-technical:matrix.org) [![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](docs/CONTRIBUTING.adoc) [![GitHub license](https://img.shields.io/badge/license-GPL3%2FApache2-blue)](#LICENSE)

<p align="center">
  <img src="/docs/media/cherry-horizontal.png">
</p>

🚧 **Warning** 🚧 This repo has been archived. Follow [Cherry Relay Node repository](https://github.com/CherryNetwork/cherry-relay-node) for all the updates of this project.

Decentralized File Storage, Smarter.

## Trying it out
We are building out a knowledge base and documentation. In the meanwhile, freestyle!

Deploy this node now with our easy [linux installation script](/scripts/run.sh)

## Contributions & Code of Conduct

Please follow the contributions guidelines as outlined in [`docs/CONTRIBUTING.adoc`](docs/CONTRIBUTING.adoc). In all communications and contributions, this project follows the [Contributor Covenant Code of Conduct](docs/CODE_OF_CONDUCT.md).

## Security

The security policy and procedures can be found in [`docs/SECURITY.md`](docs/SECURITY.md).

## License

- Substrate Primitives (`sp-*`), Frame (`frame-*`) and the pallets (`pallets-*`), binaries (`/bin`) and all other utilities are licensed under [Apache 2.0](LICENSE-APACHE2).
- Substrate Client (`/client/*` / `sc-*`) is licensed under [GPL v3.0 with a classpath linking exception](LICENSE-GPL3).

The reason for the split-licensing is to ensure that for the vast majority of teams using Substrate to create feature-chains, then all changes can be made entirely in Apache2-licensed code, allowing teams full freedom over what and how they release and giving licensing clarity to commercial teams.

In the interests of the community, we require any deeper improvements made to Substrate's core logic (e.g. Substrate's internal consensus, crypto or database code) to be contributed back so everyone can benefit.
