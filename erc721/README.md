#README#

## Preparation

For building the smart contracts found in this folder you will need to have [`cargo-contract`](https://github.com/paritytech/cargo-contract) installed.

```
cargo install cargo-contract --vers 0.8.0 --force --locked
```

## Testing the contract

To run off-chain tests that written at the end of the contract you can run the following command

```
cargo +nightly test
```

## Building the contract

Run the following command to compile your smart contract:

```
cargo +nightly contract build

```

This command will compile the code into a Wasm binary, a metadata file (which contains the contract's ABI) and a .contract file which bundles both. This .contract file can be used for deploying your contract to your chain. If all goes well, you should see these files in a target folder
