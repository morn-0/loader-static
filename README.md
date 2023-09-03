```shell
RUSTFLAGS='-C target-feature=-crt-static' cargo build --release && gcc -static hello.c -o hello
./target/release/loader-static ./hello
```
