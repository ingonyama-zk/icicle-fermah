# Groth16 Installation and Usage Guide

## CUDA Backend Installation

1. **Download the Release Binaries**
   - Download the latest release binaries from the [ICICLE Releases](https://github.com/ingonyama-zk/icicle/releases/).

2. **Install the Binaries**
   - Extract the downloaded binaries to a directory

3. **Set the Installation Path**
   - Set the `ICICLE_BACKEND_INSTALL_DIR` environment variable to the installation path:

### Running Groth16

1. **Run on CUDA Device**
   ```bash
   LD_LIBRARY_PATH=./lib ./groth16 --device CUDA
   ```

2. **Run on CPU**
   ```bash
   LD_LIBRARY_PATH=./lib ./groth16 --device CPU
   ```

### Executing Proofs

To execute a proof, specify the paths for the witness file, zkey file, and the output paths for the proof and public JSONs.

Example:
```bash
prove --witness ./witness.wtns --zkey ./circuit.zkey --proof ./proof.json --public ./public.json
```

### User R and S example usage
```bash
cargo run --release
> prove
witness: "./witness.wtns"
zkey: "./circuit.zkey"
proof: "./proof.json"
public: "./public.json"
Setting device CPU
r: 0x0623f8560f3409aa993a42ed0148f8f3cf35589f4029ae643c78894ef2470192
s: 0x2bb69b1c7ad8b4d7bc8db0eb3f77e1fbcee86ea2418df76258527d70594196a8
command completed in 758.362907ms
COMMAND_COMPLETED
> prove --r FF 
witness: "./witness.wtns"
zkey: "./circuit.zkey"
proof: "./proof.json"
public: "./public.json"
r: 0x00000000000000000000000000000000000000000000000000000000000000ff
s: 0x1065b11af11fe429650626c76e71b23167985d194c54baa62518724ea919e6a2
command completed in 652.623865ms
COMMAND_COMPLETED
> prove --r 1B4F --s 4E12
witness: "./witness.wtns"
zkey: "./circuit.zkey"
proof: "./proof.json"
public: "./public.json"
r: 0x0000000000000000000000000000000000000000000000000000000000001b4f
s: 0x0000000000000000000000000000000000000000000000000000000000004e12
command completed in 658.64536ms
COMMAND_COMPLETED
```