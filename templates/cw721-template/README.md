# CW721 Template

A template for generating custom cw721 NFT contracts. Based on [cw-template](https://github.com/CosmWasm/cw-template).

## Creating a new repo from template

Assuming you have a recent version of Rust and Cargo installed
(via [rustup](https://rustup.rs/)),
then the following should get you a new repo to start a contract:

Install [cargo-generate](https://github.com/ashleygwilliams/cargo-generate) and cargo-run-script.
Unless you did that before, run this line now:

```sh
cargo install cargo-generate --features vendored-openssl
cargo install cargo-run-script
```

Now, use it to create your new contract.

Go to the folder in which you want to place it and run:

**Latest**

```sh
cargo generate --git https://github.com/CosmWasm/cw-nfts.git --name PROJECT_NAME
```

**Older Versions**

Pass version as branch flag:

```sh
cargo generate --git https://github.com/CosmWasm/cw-nfts.git --branch <version> --name PROJECT_NAME
```

Example:

```sh
cargo generate --git https://github.com/CosmWasm/cw-nfts.git --branch 0.16 --name PROJECT_NAME
```

You will now have a new folder called `PROJECT_NAME` (I hope you changed that to something else)
containing a simple working contract and build system that you can customize.
