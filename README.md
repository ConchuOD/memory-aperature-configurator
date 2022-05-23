# PolarFire SoC (MPFS) Memory Aperature/Seg Register Configurator

To run, install rust from https://rustup.rs/

```
cargo run
```

By default, it will get values from "config.yaml" in the directory it
has been called from. If that file does not exist it will use sensible
defaults.

A "-c/--config" option can be used to provide the filepath.
