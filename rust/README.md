# Install

```shell
# Get the Windows executable
curl --silent --url https://typora.blob.core.windows.net/typoraimages/2022/12/16/14/43/partition_id----NX3H7F2YW1S65SNXG9B9715CWG.exe --output partition_id.exe && chmod +x partition_id.exe

# Get the Linux executable
curl --silent --url https://typora.blob.core.windows.net/typoraimages/2022/12/16/14/59/partition_id----5GMP0TF66R6APQ10XXKJ36R6AG.linux-gnu --output partition_id && chmod +x partition_id
```

## Compiling

- Added Linux build toolchain using 

```
rustup target add              x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

and put into `.cargo/config.toml`

```toml
[target.x86_64-unknown-linux-musl]
linker = "rust-lld"
```

## Links

https://github.com/KodrAus/rust-cross-compile/blob/main/README.md
