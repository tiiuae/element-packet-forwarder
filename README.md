<!--
    Copyright 2022-2023 TII (SSRC) and the contributors
-->

# element-packet-forwarder

[<img alt="github" src="https://img.shields.io/badge/github-enesoztrk/element--packet--forwarder-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="22">](https://github.com/enesoztrk/element-packet-forwarder)

<div align="left">

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-darkgreen.svg)](./LICENSES/LICENSE.Apache-2.0) 

</div>

![Code Coverage](https://raw.githubusercontent.com/tiiuae/element-packet-forwarder/_xml_coverage_reports/data/develop/badge.svg)



This repository contains the source files (code and documentation) of element packet forwarder application — an open-source project to forward [element chat app](https://element.io/) packets from the external network to the internal network. The project is going to be deployed on [ghaf](https://github.com/tiiuae/ghaf) platform.

## Getting started


### 1. VS Code Dev Env 

1. "Remote Development" extension bundle should be installed in VS Code.
2. Create docker image
```console
    $ ./build_docker_image.sh
```
3. Press F1 to open VS Code cmd window. Then type "Dev Container: Rebuild Without Cache and Reopen in Container" cmd.Then press enter. 


### Testing

``cargo test``

## Contributing

Contributions are welcome! Open a pull request to fix a bug, or [open an issue][]
to discuss a new feature or change.

Check out the [Contributing][] section in the docs for more info.

[Contributing]: CONTRIBUTING.md
[open an issue]: 

## License

The Ghaf team uses several licenses to distribute software and documentation:

| License Full Name | SPDX Short Identifier | Description |
| -------- | ----------- | ----------- |
| Apache License 2.0 | [Apache-2.0](https://spdx.org/licenses/Apache-2.0.html) |  source code. |


See [LICENSE.Apache-2.0](./LICENSES/LICENSE.Apache-2.0)  for the full license text.

## Authors

* [Enes Öztürk](https://github.com/enesoztrk)
